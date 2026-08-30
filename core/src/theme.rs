//! Theme data model: a fixed set of named palettes, each spelling out
//! every colour the window is painted with.
//!
//! There is no arithmetic here and none anywhere downstream. A button
//! carries a colour for its fill, its label and its border, in each of
//! its three states, and the renderer draws what the table says. The
//! hover and pressed shades used to be computed from the base colour —
//! an HSV lift toward yellow for hover, a 10 % darkening for pressed —
//! and the label colour was one value for the whole theme, lifted
//! straight onto an accent-coloured key where it had no contrast to
//! spare. A formula cannot know that; a table can, so the table is
//! what a theme is now.
//!
//! Colours are written as `#RRGGBBAA`, the same spelling `config.toml`
//! uses — see [`crate::color`]. The alpha channel is live: a fill of
//! `#00000000` is a button drawn by its border alone over whatever is
//! behind it.
//!
//! [`ThemeKind::Cosmic`] is the one palette that is not fixed. It
//! tracks the running COSMIC desktop, and takes that desktop's own
//! per-state component colours — base, hover, pressed, its text and
//! its border — rather than deriving any of them. See
//! [`apply_cosmic_override`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::color::{rgba, Rgba};

/// One button in one state: what it is filled with, what its label is
/// drawn in, and what its border is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonFace {
    pub background: Rgba,
    pub text: Rgba,
    pub border: Rgba,
}

impl ButtonFace {
    pub const fn new(background: Rgba, text: Rgba, border: Rgba) -> Self {
        Self {
            background,
            text,
            border,
        }
    }
}

/// A button category in each of the three states it is drawn in.
///
/// Nothing is derived from anything: a theme that wants its hover to
/// be darker than its base, or its pressed label a different colour
/// from its resting one, simply says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonColors {
    pub normal: ButtonFace,
    pub hover: ButtonFace,
    pub pressed: ButtonFace,
}

impl ButtonColors {
    pub const fn new(normal: ButtonFace, hover: ButtonFace, pressed: ButtonFace) -> Self {
        Self {
            normal,
            hover,
            pressed,
        }
    }

    /// Every state drawn the same way, for a button whose appearance
    /// does not answer to the pointer — the latched `2nd` key, a
    /// selected row in the settings panel.
    pub const fn flat(face: ButtonFace) -> Self {
        Self::new(face, face, face)
    }
}

/// Largest border a theme may ask for, as a percentage of the
/// button's height. A quarter of the button is already a frame rather
/// than an outline; past that there is no room left for a label.
pub const MAX_BORDER_THICKNESS: f32 = 25.0;

/// Named colour palette. Every button category plus the three
/// surfaces — window, display, side panel — has a dedicated slot.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    /// Behind the window as a whole: the keypad's gaps, the top bar.
    pub app_bg: Rgba,
    /// Behind the expression display — the caption, the readout, and
    /// the row of number properties and the memory register under
    /// them. Its own slot rather than the window's, so the display
    /// can be a panel of its own against the keypad.
    pub display_bg: Rgba,
    /// Behind the history and settings side panels.
    pub sidepanel_bg: Rgba,
    /// Text that is not on a button: the readout, the property labels
    /// that hold, the memory register with something in it, and the
    /// text in the side panels.
    pub text_active: Rgba,
    /// The dim counterpart: the caption above the readout, a property
    /// that does not hold, the memory register while it is empty, and
    /// the `×` the calculator fills in for the user. Its own colour
    /// rather than a fixed fraction of `text_active`'s alpha, which
    /// is what it used to be and what left it unreadable on some
    /// backgrounds.
    pub text_inactive: Rgba,
    /// The switches and sliders in the settings panel. Under the
    /// Cosmic preset this is the desktop's own accent colour.
    pub accent: Rgba,
    /// How thick a button's border is drawn, as a percentage of the
    /// button's height — see [`Theme::border_width`]. `0` is no
    /// border at all, which is what every shipped theme asks for.
    pub button_border_thickness: f32,
    pub science: ButtonColors,
    pub second: ButtonColors,
    pub toprow: ButtonColors,
    /// `AC`/`C` and backspace. The same colours as `toprow` in every
    /// shipped theme — they are a slot of their own so a theme can
    /// mark the two keys that take something away.
    pub delete: ButtonColors,
    pub basicop: ButtonColors,
    pub equals: ButtonColors,
    pub negate: ButtonColors,
    pub decimal: ButtonColors,
    pub number: ButtonColors,
}

impl Theme {
    /// Width, in logical pixels, of the border on a button `height`
    /// pixels tall.
    ///
    /// The thickness a theme carries is a percentage of the button's
    /// own height rather than a pixel count. A window dragged twice
    /// as wide grows every button with it, and a border pinned to
    /// pixels would go from an outline to a hairline as that
    /// happened; a percentage keeps the proportion. It also keeps the
    /// small rows in the settings panel from wearing the same heavy
    /// line as a keypad key three times their size.
    ///
    /// The result is rounded to a whole logical pixel, and this is
    /// the part worth saying out loud. A border is a hairline of
    /// solid colour, and it is the one place a fractional width
    /// really shows: at 0.4px the renderer has no pixel to put it in
    /// and draws a grey smear instead of a line, and the smear
    /// changes shade with every pixel the window is dragged — an
    /// outline that shimmers while the window resizes. Rounding pins
    /// it to 1px, 2px, 3px: crisp at every size, and on a HiDPI
    /// screen a whole logical pixel is a whole number of physical
    /// ones too, so it stays crisp there. The width then grows in
    /// visible steps rather than continuously, which is what a border
    /// should do — it is either one pixel or two, never one and a
    /// bit.
    ///
    /// A theme that asks for a border always gets at least one pixel
    /// of it, so a thin setting on a small button is drawn faintly
    /// rather than not at all, and the width is capped at
    /// [`MAX_BORDER_THICKNESS`] of the button so no value can swallow
    /// the label.
    pub fn border_width(&self, button_height: f32) -> f32 {
        if self.button_border_thickness <= 0.0 || button_height <= 0.0 {
            return 0.0;
        }
        let percent = self.button_border_thickness.min(MAX_BORDER_THICKNESS);
        let cap = (button_height * MAX_BORDER_THICKNESS / 100.0).max(1.0);
        (button_height * percent / 100.0).round().clamp(1.0, cap)
    }
}

/// Which of the shipped palettes is in force.
///
/// There is no "custom" member and no palette in `config.toml`: a
/// theme is one of these, and the file records which. Editing a
/// palette by hand meant a hundred-odd colours in the config file and
/// a second copy of every theme to keep in step with the shipped one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeKind {
    #[default]
    CupertinoDark,
    CupertinoLight,
    RedmondDark,
    RedmondLight,
    HighContrastDark,
    HighContrastLight,
    Cosmic,
    Texas,
    Tokyo,
    Cyberpunk,
    Plastic,
    Crystal,
    Barbie,
    TouchLight,
    TouchDark,
    EmeraldLight,
    EmeraldDark,
    FlatOrangeDark,
    FlatGreenLight,
}

impl ThemeKind {
    /// Every palette, in the order the settings panel offers them.
    pub const ALL: [ThemeKind; 19] = [
        ThemeKind::CupertinoDark,
        ThemeKind::CupertinoLight,
        ThemeKind::RedmondDark,
        ThemeKind::RedmondLight,
        ThemeKind::HighContrastDark,
        ThemeKind::HighContrastLight,
        ThemeKind::Cosmic,
        ThemeKind::Texas,
        ThemeKind::Tokyo,
        ThemeKind::Cyberpunk,
        ThemeKind::Plastic,
        ThemeKind::Crystal,
        ThemeKind::Barbie,
        ThemeKind::TouchLight,
        ThemeKind::TouchDark,
        ThemeKind::EmeraldLight,
        ThemeKind::EmeraldDark,
        ThemeKind::FlatOrangeDark,
        ThemeKind::FlatGreenLight,
    ];

    /// Human-readable name – what the settings panel shows and what
    /// the resulting [`Theme`] carries.
    pub fn display_name(self) -> &'static str {
        match self {
            ThemeKind::CupertinoDark => "Cupertino Dark",
            ThemeKind::CupertinoLight => "Cupertino Light",
            ThemeKind::RedmondDark => "Redmond Dark",
            ThemeKind::RedmondLight => "Redmond Light",
            ThemeKind::HighContrastDark => "High Contrast Dark",
            ThemeKind::HighContrastLight => "High Contrast Light",
            ThemeKind::Cosmic => "Cosmic",
            ThemeKind::Texas => "Texas",
            ThemeKind::Tokyo => "Tokyo",
            ThemeKind::Cyberpunk => "Cyberpunk",
            ThemeKind::Plastic => "Plastic",
            ThemeKind::Crystal => "Crystal",
            ThemeKind::Barbie => "Barbie",
            ThemeKind::TouchLight => "Touch Light",
            ThemeKind::TouchDark => "Touch Dark",
            ThemeKind::EmeraldLight => "Emerald Light",
            ThemeKind::EmeraldDark => "Emerald Dark",
            ThemeKind::FlatOrangeDark => "Flat Orange Dark",
            ThemeKind::FlatGreenLight => "Flat Green Light",
        }
    }

    /// The name `config.toml` records, which is the variant's own —
    /// unchanged from the files earlier versions wrote.
    pub fn key(self) -> &'static str {
        match self {
            ThemeKind::CupertinoDark => "CupertinoDark",
            ThemeKind::CupertinoLight => "CupertinoLight",
            ThemeKind::RedmondDark => "RedmondDark",
            ThemeKind::RedmondLight => "RedmondLight",
            ThemeKind::HighContrastDark => "HighContrastDark",
            ThemeKind::HighContrastLight => "HighContrastLight",
            ThemeKind::Cosmic => "Cosmic",
            ThemeKind::Texas => "Texas",
            ThemeKind::Tokyo => "Tokyo",
            ThemeKind::Cyberpunk => "Cyberpunk",
            ThemeKind::Plastic => "Plastic",
            ThemeKind::Crystal => "Crystal",
            ThemeKind::Barbie => "Barbie",
            ThemeKind::TouchLight => "TouchLight",
            ThemeKind::TouchDark => "TouchDark",
            ThemeKind::EmeraldLight => "EmeraldLight",
            ThemeKind::EmeraldDark => "EmeraldDark",
            ThemeKind::FlatOrangeDark => "FlatOrangeDark",
            ThemeKind::FlatGreenLight => "FlatGreenLight",
        }
    }

    /// Enumerate every preset in display order (for building the
    /// settings list or a docs table).
    pub fn all() -> [ThemeKind; 19] {
        Self::ALL
    }

    /// The palette this preset stands for.
    pub fn get(self) -> Theme {
        match self {
            ThemeKind::CupertinoDark => Theme {
                name: "Cupertino Dark".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::CupertinoLight => Theme {
                name: "Cupertino Light".to_string(),
                app_bg: rgba("#4C4C4CFF"),
                display_bg: rgba("#4C4C4CFF"),
                sidepanel_bg: rgba("#4C4C4CFF"),
                text_active: rgba("#FFFFFFFF"),
                text_inactive: rgba("#FFFFFF4D"),
                accent: rgba("#00525AFF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#D6D6D6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#EDEDEDFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#C1C1C1FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#D6D6D6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#EDEDEDFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#C1C1C1FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#D6D6D6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#EDEDEDFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#C1C1C1FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#D6D6D6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#EDEDEDFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#C1C1C1FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#F5923DFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#FFA03FFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#DD8337FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#00525AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#006771FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#004A51FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#D6D6D6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#EDEDEDFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#C1C1C1FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#F7F7F7FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#CACACAFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#F7F7F7FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#CACACAFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
            },
            ThemeKind::RedmondDark => Theme {
                name: "Redmond Dark".to_string(),
                app_bg: rgba("#202020FF"),
                display_bg: rgba("#202020FF"),
                sidepanel_bg: rgba("#202020FF"),
                text_active: rgba("#FFFFFFFF"),
                text_inactive: rgba("#FFFFFF4D"),
                accent: rgba("#4CC2FFFF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#333333FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4A4A4AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#2E2E2EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#333333FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4A4A4AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#2E2E2EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#333333FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4A4A4AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#2E2E2EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#333333FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4A4A4AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#2E2E2EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#333333FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4A4A4AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#2E2E2EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#4CC2FFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#4CCFFFFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#44AFE6FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#3C3C3CFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#535353FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#363636FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#3C3C3CFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#535353FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#363636FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#3C3C3CFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#535353FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#363636FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
            },
            ThemeKind::RedmondLight => Theme {
                name: "Redmond Light".to_string(),
                app_bg: rgba("#F3F3F3FF"),
                display_bg: rgba("#F3F3F3FF"),
                sidepanel_bg: rgba("#F3F3F3FF"),
                text_active: rgba("#000000FF"),
                text_inactive: rgba("#0000004D"),
                accent: rgba("#0067C0FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#F9F9F9FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#F9F9F9FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#F9F9F9FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#F9F9F9FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#F9F9F9FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E0E0E0FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#0067C0FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#0073D7FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#005DADFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E6E6E6FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E6E6E6FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FFFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#E6E6E6FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
            },
            ThemeKind::HighContrastDark => Theme {
                name: "High Contrast Dark".to_string(),
                app_bg: rgba("#242424FF"),
                display_bg: rgba("#242424FF"),
                sidepanel_bg: rgba("#F3F3F3FF"),
                text_active: rgba("#FFFFFFFF"),
                text_inactive: rgba("#FFFFFF4D"),
                accent: rgba("#FFFFFFFF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#1A1A1AFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#313131FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#171717FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#0F0E0EFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#262323FF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                    ButtonFace::new(rgba("#0E0D0DFF"), rgba("#FFFFFFFF"), rgba("#FFFFFFFF")),
                ),
            },
            ThemeKind::HighContrastLight => Theme {
                name: "High Contrast Light".to_string(),
                app_bg: rgba("#DBDBDBFF"),
                display_bg: rgba("#DBDBDBFF"),
                sidepanel_bg: rgba("#DBDBDBFF"),
                text_active: rgba("#000000FF"),
                text_inactive: rgba("#0000004D"),
                accent: rgba("#000000FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#E5E5E5FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FCFCFCFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#CECECEFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#F0F1F1FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#FEFFFFFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#D8D9D9FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
            },
            ThemeKind::Cosmic => Theme {
                name: "Cosmic".to_string(),
                app_bg: rgba("#1B1B1BFF"),
                display_bg: rgba("#1B1B1BFF"),
                sidepanel_bg: rgba("#272727FF"),
                text_active: rgba("#E7E7E7FF"),
                text_inactive: rgba("#E7E7E74D"),
                accent: rgba("#61CDDCFF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#636363FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#7A7A7AFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#595959FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#636363FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#7A7A7AFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#595959FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#636363FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#7A7A7AFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#595959FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#636363FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#7A7A7AFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#595959FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#61CDDCFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#6BE2F3FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#57B9C6FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#61CDDCFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#6BE2F3FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#57B9C6FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#636363FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#7A7A7AFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#595959FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#4F4F4FFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#666666FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#474747FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#4F4F4FFF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#666666FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                    ButtonFace::new(rgba("#474747FF"), rgba("#E7E7E7FF"), rgba("#E7E7E7FF")),
                ),
            },
            ThemeKind::Texas => Theme {
                name: "Texas".to_string(),
                app_bg: rgba("#1E2329FF"),
                display_bg: rgba("#1E2329FF"),
                sidepanel_bg: rgba("#1E2329FF"),
                text_active: rgba("#000000FF"),
                text_inactive: rgba("#0000004D"),
                accent: rgba("#324C67FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#1C1F27FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#2C313EFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#191C23FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#687B99FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#788DB0FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#5E6F8AFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#1C1F27FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#2C313EFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#191C23FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#1C1F27FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#2C313EFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#191C23FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#324C67FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#3D5D7EFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#2D445DFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#324C67FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#3D5D7EFF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#2D445DFF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#707070FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#878787FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#656565FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#707070FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#878787FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#656565FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#707070FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#878787FF"), rgba("#000000FF"), rgba("#000000FF")),
                    ButtonFace::new(rgba("#656565FF"), rgba("#000000FF"), rgba("#000000FF")),
                ),
            },
            ThemeKind::Tokyo => Theme {
                name: "Tokyo".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::Cyberpunk => Theme {
                name: "Cyberpunk".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::Plastic => Theme {
                name: "Plastic".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::Crystal => Theme {
                name: "Crystal".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::Barbie => Theme {
                name: "Barbie".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::TouchLight => Theme {
                name: "Touch Light".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::TouchDark => Theme {
                name: "Touch Dark".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::EmeraldLight => Theme {
                name: "Emerald Light".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::EmeraldDark => Theme {
                name: "Emerald Dark".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::FlatOrangeDark => Theme {
                name: "Flat Orange Dark".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
            ThemeKind::FlatGreenLight => Theme {
                name: "Flat Green Light".to_string(),
                app_bg: rgba("#283133FF"),
                display_bg: rgba("#283133FF"),
                sidepanel_bg: rgba("#283133FF"),
                text_active: rgba("#D4D4D4FF"),
                text_inactive: rgba("#D4D4D44D"),
                accent: rgba("#FF9600FF"),
                button_border_thickness: 0.0,
                science: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                second: ButtonColors::new(
                    ButtonFace::new(rgba("#3E4247FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#52575EFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#383B40FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                toprow: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                delete: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                basicop: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                equals: ButtonColors::new(
                    ButtonFace::new(rgba("#FF9600FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#FFB000FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#E68700FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                negate: ButtonColors::new(
                    ButtonFace::new(rgba("#888A8BFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#9EA1A2FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#7A7C7DFF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                decimal: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
                number: ButtonColors::new(
                    ButtonFace::new(rgba("#585E60FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#6D7477FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                    ButtonFace::new(rgba("#4F5556FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")),
                ),
            },
        }
    }
}

// ---------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------

// Written and read by name, and read leniently: a `theme_kind` the
// build does not know — a palette removed since the file was written,
// a typo, the `Custom` entry earlier versions had — falls back to the
// default rather than failing the whole config load and taking every
// other setting with it. Same repair-rather-than-reject rule the
// numeric fields follow.
impl Serialize for ThemeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.key())
    }
}

impl<'de> Deserialize<'de> for ThemeKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(ThemeKind::ALL
            .into_iter()
            .find(|kind| kind.key() == name)
            .unwrap_or_default())
    }
}

// ---------------------------------------------------------------------
// Cosmic-desktop override
// ---------------------------------------------------------------------

/// One of the running desktop's components, in the states it draws
/// itself in. The UI layer fills this in from `cosmic_theme::Component`
/// and hands it to [`apply_cosmic_override`]; the type is plain RGBA
/// here so this module does not depend on libcosmic at all, which is
/// what lets its tests run without a compositor.
///
/// The desktop already publishes a hover, a pressed, a text and a
/// border colour for every component it draws, so the Cosmic preset
/// takes those rather than computing anything: its buttons hover the
/// way the rest of the desktop's buttons hover, and an accent-coloured
/// key wears the accent's *own* text colour instead of the window's,
/// which is where the contrast used to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CosmicComponent {
    pub base: Rgba,
    pub hover: Rgba,
    pub pressed: Rgba,
    pub text: Rgba,
    pub border: Rgba,
}

impl CosmicComponent {
    /// This component as a button category's three states.
    pub fn colors(self) -> ButtonColors {
        ButtonColors::new(
            ButtonFace::new(self.base, self.text, self.border),
            ButtonFace::new(self.hover, self.text, self.border),
            ButtonFace::new(self.pressed, self.text, self.border),
        )
    }
}

/// Colour hooks extracted from the running COSMIC desktop theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CosmicOverride {
    pub window_bg: Rgba,
    pub container_bg: Rgba,
    pub interface_text: Rgba,
    /// Text on the window background at the desktop's own dim level.
    pub interface_text_dim: Rgba,
    /// The component drawn on a container: the science, `2nd`,
    /// top-row, delete and negate keys.
    pub component: CosmicComponent,
    /// The component drawn on the window background: the digits and
    /// the decimal point, which read as a group of their own.
    pub surface_component: CosmicComponent,
    /// The accent: the basic operators and `=`, and the switches and
    /// sliders in the settings panel.
    pub accent: CosmicComponent,
}

/// Overlay a running COSMIC palette on top of the Cosmic preset.
/// Every colour it touches is one the desktop published; nothing is
/// derived. The fields it does not mention — the border thickness —
/// are the preset's own.
pub fn apply_cosmic_override(base: Theme, over: CosmicOverride) -> Theme {
    let component = over.component.colors();
    let surface = over.surface_component.colors();
    let accent = over.accent.colors();
    Theme {
        app_bg: over.window_bg,
        display_bg: over.window_bg,
        sidepanel_bg: over.container_bg,
        text_active: over.interface_text,
        text_inactive: over.interface_text_dim,
        accent: over.accent.base,
        science: component,
        second: component,
        toprow: component,
        delete: component,
        negate: component,
        basicop: accent,
        equals: accent,
        number: surface,
        decimal: surface,
        ..base
    }
}
