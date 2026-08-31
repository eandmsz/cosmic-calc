//! Theme data model: a fixed set of named palettes, each spelling out
//! every colour the window is painted with.
//!
//! There is no arithmetic here and none anywhere downstream. A button
//! carries a colour for its fill, its label and its border, in each of
//! its three states, and the renderer draws what the table says.
//!
//! # Reading a palette
//!
//! Each arm of [`presets::preset`] is one palette, and each names
//! three window surfaces, two text colours, an accent, a border
//! percentage, and then one entry per button category — see [`Theme`]
//! for what the categories are.
//!
//! A category is a three-by-three grid: a row per colour, a column
//! per state.
//!
//! ```text
//! science: ButtonColors::grid(
//!     //               resting            hover              pressed
//!     StateColors::new(rgba("#3E4247FF"), rgba("#52575EFF"), rgba("#383B40FF")), // fill
//!     StateColors::new(rgba("#D4D4D4FF"), rgba("#D4D4D4FF"), rgba("#D4D4D4FF")), // label
//!     StateColors::new(rgba("#B0B0B0FF"), rgba("#B0B0B0FF"), rgba("#B0B0B0FF")), // border
//! ),
//! ```
//!
//! Borders are opt-in per palette
//! ([`Theme::button_border_percent`], a percentage of the button's
//! height). Most palettes leave it at zero and their border colours
//! are written down and waiting rather than on screen; Cupertino Dark
//! and Cyberpunk ask for one.
//!
//! Colours are written as `#RRGGBBAA`, the same spelling `config.toml`
//! uses — see [`crate::color`]. The alpha channel is live: a fill of
//! `#00000000` is a button drawn by its border alone over whatever is
//! behind it.
//!
//! # Where a palette actually comes from
//!
//! The tables in [`presets`] are defaults. `config.toml` carries every
//! palette in full, and the file is what the window is painted with,
//! so a user can retune any colour, any border and any button label
//! without rebuilding. [`ThemeTable`] is that file section: it reads
//! leniently, repairs anything it cannot use, and fills in whatever
//! the file leaves out.
//!
//! [`ThemeKind::Cosmic`] is the one palette that is not fixed. It
//! tracks the running COSMIC desktop and takes that desktop's own
//! per-state component colours — base, hover, pressed, its text and
//! its border — rather than deriving any of them. See
//! [`apply_cosmic_override`].

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::color::Rgba;
use crate::config::{sanitized_font_name, ButtonShape, FontWeight};
use crate::lenient::{color_of, Lenient};

mod presets;

/// One button in one state: the three colours it is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonFace {
    /// What the button is filled with.
    pub background: Rgba,
    /// What its label — the digit, the operator, the function name —
    /// is drawn in. The font colour.
    pub text: Rgba,
    /// What its outline is drawn in, where the palette asks for one
    /// — see [`Theme::button_border_percent`].
    pub border: Rgba,
}

impl ButtonFace {
    /// In the order the fields are declared: fill, label, border.
    pub const fn new(background: Rgba, text: Rgba, border: Rgba) -> Self {
        Self {
            background,
            text,
            border,
        }
    }
}

/// One colour of a button across the three states it is drawn in —
/// one row of the grid a category is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateColors {
    /// At rest.
    pub resting: Rgba,
    /// Under the pointer.
    pub hover: Rgba,
    /// While the button is held down.
    pub pressed: Rgba,
}

impl StateColors {
    /// In the order the columns read: resting, hover, pressed.
    pub const fn new(resting: Rgba, hover: Rgba, pressed: Rgba) -> Self {
        Self {
            resting,
            hover,
            pressed,
        }
    }

    /// The same colour in all three states, for a row that does not
    /// answer the pointer.
    pub const fn flat(color: Rgba) -> Self {
        Self::new(color, color, color)
    }
}

/// A button category in each of the three states it is drawn in, and
/// the one thing about a category that is not a colour: the weight
/// its labels are set at.
///
/// Nothing is derived from anything: a theme that wants its hover to
/// be darker than its base, or its pressed label a different colour
/// from its resting one, simply says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonColors {
    /// At rest.
    pub normal: ButtonFace,
    /// Under the pointer.
    pub hover: ButtonFace,
    /// While the button is held down.
    pub pressed: ButtonFace,
    /// Which of the palette family's faces this category's labels
    /// are set in, where the category asks for one of its own.
    ///
    /// `None` — which is what almost every category everywhere says —
    /// is the palette's own [`Theme::font_weight`], so a palette that
    /// wants one weight throughout names it once and says nothing
    /// down here. A category that names one gets it, and only it: the
    /// digits can be set heavier than the operators around them
    /// without a second family or a second palette, which is how a
    /// good many desktop calculators draw their keypad.
    ///
    /// The family is not part of this. A palette is one face, and a
    /// second family on the same keypad is a different palette rather
    /// than a heavier group of keys.
    pub font_weight: Option<FontWeight>,
}

impl ButtonColors {
    /// In the order the fields are declared: resting, hover, pressed.
    /// The labels take the palette's own weight; see
    /// [`ButtonColors::weighted`] for a category that wants its own.
    pub const fn new(normal: ButtonFace, hover: ButtonFace, pressed: ButtonFace) -> Self {
        Self {
            normal,
            hover,
            pressed,
            font_weight: None,
        }
    }

    /// This category with its labels set at `weight` rather than at
    /// the palette's own.
    pub const fn weighted(self, weight: FontWeight) -> Self {
        Self {
            font_weight: Some(weight),
            ..self
        }
    }

    /// A category written the way the palette tables and `config.toml`
    /// write it: three rows — fill, label, border — each giving the
    /// colour at rest, under the pointer and while held.
    pub const fn grid(fill: StateColors, label: StateColors, border: StateColors) -> Self {
        Self::new(
            ButtonFace::new(fill.resting, label.resting, border.resting),
            ButtonFace::new(fill.hover, label.hover, border.hover),
            ButtonFace::new(fill.pressed, label.pressed, border.pressed),
        )
    }

    /// Every state drawn the same way, for a button whose appearance
    /// does not answer to the pointer — the latched `2nd` key, a
    /// selected row in the settings panel.
    pub const fn flat(face: ButtonFace) -> Self {
        Self::new(face, face, face)
    }

    /// The fill row: what the button is filled with in each state.
    pub const fn fill_row(&self) -> StateColors {
        StateColors::new(
            self.normal.background,
            self.hover.background,
            self.pressed.background,
        )
    }

    /// The label row: the font colour in each state.
    pub const fn label_row(&self) -> StateColors {
        StateColors::new(self.normal.text, self.hover.text, self.pressed.text)
    }

    /// The border row: the outline colour in each state.
    pub const fn border_row(&self) -> StateColors {
        StateColors::new(self.normal.border, self.hover.border, self.pressed.border)
    }
}

/// Largest border a theme may ask for, as a percentage of the
/// button's height. A quarter of the button is already a frame rather
/// than an outline; past that there is no room left for a label.
pub const MAX_BORDER_PERCENT: f32 = 25.0;

/// Longest label a theme may name itself with. The name is a row in
/// the settings panel's palette list, and one that runs off the end
/// of its row is no use to anybody.
pub const MAX_DISPLAY_NAME_LEN: usize = 32;

/// Named colour palette. Every button category plus the three
/// surfaces — window, display, side panel — has a dedicated slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Theme {
    /// Which preset this palette is. Fixed by the build: the file may
    /// retune a palette but cannot invent one, because every other
    /// part of the app names a palette by this.
    ///
    /// Not written into `config.toml`: the palette's entry is already
    /// keyed by it — `[themes.RedmondLight]` — and a second copy
    /// inside the entry is one more thing that can disagree with the
    /// name above it.
    #[serde(skip_serializing)]
    pub id: ThemeKind,
    /// What the settings panel writes on the row that selects it.
    pub display_name: String,
    /// The family this palette is drawn in.
    ///
    /// A palette is a look rather than a set of colours, and the face
    /// its text is set in is as much a part of that as its accent: a
    /// Cupertino palette wants the face that desktop is drawn in, a
    /// Cyberpunk one a terminal face. So the font travels with the
    /// palette — switching palettes switches the family with it, and
    /// picking a family in the settings panel changes the palette on
    /// screen and leaves the other eighteen as they were.
    ///
    /// The name is kept exactly as it stands, whether or not the host
    /// has the family. A machine without it is drawn in the best
    /// installed family from [`crate::config::RECOMMENDED_FONTS`] —
    /// see `ui::font::resolved_font` — and installing the one the
    /// palette names is all it takes to get it, without an edit
    /// anywhere.
    pub font: String,
    /// Which of that family's faces to draw in.
    ///
    /// Only honoured while the family itself is installed: a palette
    /// falling back to a recommended family is drawn at the default
    /// weight, since a Black chosen for one face says nothing about
    /// how another should be set.
    pub font_weight: FontWeight,
    /// How round this palette's buttons are drawn — see
    /// [`ButtonShape`].
    ///
    /// A corner is part of a look in the way a colour or a face is: a
    /// Cupertino keypad is pills and a Wolfenstein one is corners, and
    /// a single setting for the whole app meant picking one and
    /// wearing it on all twenty. So the shape travels with the palette
    /// like the font does — switching palettes switches it, and
    /// choosing one in the settings panel changes the palette on
    /// screen and leaves the other nineteen alone.
    ///
    /// A palette that copies a desktop copies its corner: Cupertino
    /// is pills, Redmond is corners, and Cosmic Desktop asks for
    /// `Auto` — the running desktop's own — because tracking that
    /// desktop is what it is for. The rest ask for `Auto` too, which
    /// is where every palette started, so switching to one of them
    /// changes nothing about the keypad's shape.
    pub button_shape: ButtonShape,
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
    /// the `×` the calculator fills in for the user.
    pub text_inactive: Rgba,
    /// The switches and sliders in the settings panel. Under the
    /// Cosmic preset this is the desktop's own accent colour.
    pub accent: Rgba,
    /// How thick a button's border is drawn, as a percentage of the
    /// button's height — see [`Theme::border_width`]. `0` is no
    /// border at all, which is what most shipped palettes ask for.
    pub button_border_percent: f32,
    /// The scientific keys that have no category of their own: the
    /// roots, the logarithms, the powers, the constants, `!`, `EE`.
    pub science: ButtonColors,
    pub second: ButtonColors,
    pub toprow: ButtonColors,
    /// `AC`/`C` and backspace. The same colours as `toprow` in every
    /// shipped theme — they are a slot of their own so a theme can
    /// mark the two keys that take something away.
    pub delete: ButtonColors,
    /// `(` and `)`.
    pub bracket: ButtonColors,
    pub basicop: ButtonColors,
    pub equals: ButtonColors,
    pub percent: ButtonColors,
    /// `1/x`.
    pub reciprocal: ButtonColors,
    /// The twelve trigonometric functions: `sin`, `cos`, `tan`, their
    /// hyperbolic forms and all six inverses.
    pub trig: ButtonColors,
    /// `rand`.
    pub rand: ButtonColors,
    pub negate: ButtonColors,
    pub decimal: ButtonColors,
    pub number: ButtonColors,
}

impl Theme {
    /// Width, in logical pixels, of the border on a button `height`
    /// pixels tall.
    ///
    /// The thickness a theme carries is a percentage of the button's
    /// own height rather than a pixel count, so a window dragged
    /// twice as wide keeps the proportion instead of thinning the
    /// outline to a hairline.
    ///
    /// The result is rounded to a whole logical pixel. A border is a
    /// hairline of solid colour, and it is the one place a fractional
    /// width really shows: at 0.4px the renderer has no pixel to put
    /// it in and draws a shimmering grey smear instead of a line.
    /// Rounding pins it to 1px, 2px, 3px — crisp at every size, and
    /// on a HiDPI screen a whole logical pixel is a whole number of
    /// physical ones too.
    ///
    /// A theme that asks for a border always gets at least one pixel
    /// of it, and the width is capped at [`MAX_BORDER_PERCENT`] of
    /// the button so no value can swallow the label.
    pub fn border_width(&self, button_height: f32) -> f32 {
        if self.button_border_percent <= 0.0 || button_height <= 0.0 {
            return 0.0;
        }
        let percent = self.button_border_percent.min(MAX_BORDER_PERCENT);
        let cap = (button_height * MAX_BORDER_PERCENT / 100.0).max(1.0);
        (button_height * percent / 100.0).round().clamp(1.0, cap)
    }

    /// Snap the two fields a hand-edited file can put out of range
    /// back into it: a name that would not fit on its button, and a
    /// border thickness the renderer could not draw.
    fn sanitize(&mut self) {
        let fallback = presets::preset(self.id);
        self.display_name = sanitized_display_name(&self.display_name, &fallback.display_name);
        self.font = sanitized_font_name(&self.font, &fallback.font);
        if !self.button_border_percent.is_finite() {
            self.button_border_percent = fallback.button_border_percent;
        }
        self.button_border_percent = self.button_border_percent.clamp(0.0, MAX_BORDER_PERCENT);
    }
}

/// A theme's name with everything a text renderer has no business
/// being handed taken out of it, or `fallback` when nothing usable is
/// left.
///
/// The name is drawn on a row of the settings panel and nowhere else
/// — it is never a path, a command or markup — so the risks are a
/// label that breaks the panel's layout and a label that lies about
/// which palette it selects. Control characters (a newline, a NUL) do the first;
/// bidirectional overrides and zero-width joiners do the second, by
/// letting a name render as something other than the characters it is
/// made of. Both go, and what is left is capped at
/// [`MAX_DISPLAY_NAME_LEN`] characters.
fn sanitized_display_name(raw: &str, fallback: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| is_safe_label_char(*c))
        .take(MAX_DISPLAY_NAME_LEN)
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

/// Whether a character may appear in a theme's name. Everything is
/// allowed except control characters and the invisible formatting
/// codepoints — zero-width spaces and joiners, the bidirectional
/// overrides, the byte-order mark, the line and paragraph separators.
fn is_safe_label_char(c: char) -> bool {
    !c.is_control()
        && !matches!(c,
            '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2028}' | '\u{2029}'
            | '\u{2060}'..='\u{2064}'
            | '\u{FEFF}')
}

/// Which of the shipped palettes is in force.
///
/// There is no "custom" member: a theme is one of these, and
/// `config.toml` records which. The file carries the colours of all
/// of them — see [`ThemeTable`] — so retuning a palette is an edit
/// rather than a new entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeKind {
    CupertinoDark,
    CupertinoLight,
    RedmondDark,
    RedmondLight,
    HighContrastDark,
    HighContrastLight,
    /// The palette a fresh `config.toml` starts on, wherever it sits
    /// in the list the settings panel offers.
    #[default]
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
    Wolfenstein,
}

impl ThemeKind {
    /// Every palette, in the order the settings panel offers them.
    pub const ALL: [ThemeKind; 20] = [
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
        ThemeKind::Wolfenstein,
    ];

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
            ThemeKind::Wolfenstein => "Wolfenstein",
        }
    }

    /// The palette this key names, or `None` for one no build knows —
    /// a palette dropped since the file was written, a typo, the
    /// `Custom` entry earlier versions had.
    pub fn from_key(key: &str) -> Option<ThemeKind> {
        ThemeKind::ALL.into_iter().find(|kind| kind.key() == key)
    }

    /// The palette this preset ships with, before `config.toml` has
    /// its say. [`ThemeTable`] is what the window is painted from.
    pub fn get(self) -> Theme {
        presets::preset(self)
    }
}

// ---------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------

// A `theme_kind` the build does not know falls back to the default
// rather than failing the whole config load and taking every other
// setting with it. Same repair-rather-than-reject rule the numeric
// fields follow.
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
        Ok(ThemeKind::from_key(&name).unwrap_or_default())
    }
}

/// A category as `config.toml` writes it: three rows of three
/// colours, the same grid the palette tables use, and under them the
/// weight — only where the category asks for one of its own, since a
/// `font_weight` on every one of the fourteen would be fourteen lines
/// saying what the palette already said once at the top.
impl Serialize for ButtonColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = 3 + usize::from(self.font_weight.is_some());
        let mut row = serializer.serialize_struct("ButtonColors", fields)?;
        row.serialize_field("fill", &self.fill_row())?;
        row.serialize_field("label", &self.label_row())?;
        row.serialize_field("border", &self.border_row())?;
        if let Some(weight) = self.font_weight {
            row.serialize_field("font_weight", &weight)?;
        }
        row.end()
    }
}

/// One row, written the way [`crate::layout`] writes a keypad row:
/// the three values in order, separated by spaces, on one line. A
/// grid of nine colours is nine words in three lines that way,
/// against the fifteen a list of lists would take.
impl Serialize for StateColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!(
            "{} {} {}",
            self.resting.to_hex_string(),
            self.hover.to_hex_string(),
            self.pressed.to_hex_string()
        ))
    }
}

// ---------------------------------------------------------------------
// The `config.toml` theme section
// ---------------------------------------------------------------------

/// Every palette, as `config.toml` carries them.
///
/// One `themes` table holds the lot, and each palette is a sub-table
/// under its own id, so every line of a palette says which palette it
/// belongs to:
///
/// ```toml
/// [themes.RedmondLight]
/// display_name = "Redmond Light"
/// app_bg = "#F0F0F0FF"
///
/// [themes.RedmondLight.number]
/// fill   = "#FFFFFFFF #FFFFFFFF #E5E5E5FF"
/// label  = "#000000FF #000000FF #000000FF"
/// border = "#000000FF #000000FF #000000FF"
/// ```
///
/// Earlier versions wrote a `[[themes]]` array with the id inside
/// each entry, and those files still load — the id is read from the
/// entry when the list is an array — so an upgrade is a load and the
/// next save, not a hand-migration.
///
/// The file is authoritative: what the user leaves in it is what the
/// window is painted with. It is also hand-edited, so nothing in it
/// is trusted. Reading one is a repair pass rather than a parse —
/// every value that is not a colour, a number in range or a name that
/// fits on a button is replaced by the shipped one, an entry naming a
/// palette this build does not have is dropped, a palette named twice
/// keeps its first entry, and a palette the file leaves out is added
/// back. What comes out is always all twenty, in the order the
/// settings panel offers them, whatever went in.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTable(Vec<Theme>);

impl Default for ThemeTable {
    fn default() -> Self {
        Self(ThemeKind::ALL.into_iter().map(presets::preset).collect())
    }
}

impl ThemeTable {
    /// The palette a preset is currently painted in.
    pub fn get(&self, kind: ThemeKind) -> Theme {
        self.0
            .iter()
            .find(|t| t.id == kind)
            .cloned()
            .unwrap_or_else(|| presets::preset(kind))
    }

    /// What the settings panel writes on the row for `kind`.
    pub fn display_name(&self, kind: ThemeKind) -> &str {
        self.0
            .iter()
            .find(|t| t.id == kind)
            .map(|t| t.display_name.as_str())
            .unwrap_or("")
    }

    /// The family `kind` asks to be drawn in — see [`Theme::font`].
    /// Borrowed rather than handed back with the whole palette: the
    /// view asks for it once per widget it draws, and cloning
    /// a whole palette's worth of colours for a family name is a lot
    /// of copying per frame.
    pub fn font(&self, kind: ThemeKind) -> &str {
        self.0
            .iter()
            .find(|t| t.id == kind)
            .map(|t| t.font.as_str())
            .unwrap_or(crate::config::DEFAULT_FONT)
    }

    /// The weight `kind` asks for.
    pub fn font_weight(&self, kind: ThemeKind) -> FontWeight {
        self.0
            .iter()
            .find(|t| t.id == kind)
            .map(|t| t.font_weight)
            .unwrap_or_default()
    }

    /// Give `kind` a family, held to what a family name can be. A
    /// palette the table does not carry is not created for it: the
    /// table always holds all twenty after
    /// [`ThemeTable::normalize`], and one that does not has bigger
    /// problems than a font.
    pub fn set_font(&mut self, kind: ThemeKind, font: String) {
        if let Some(theme) = self.0.iter_mut().find(|t| t.id == kind) {
            theme.font = sanitized_font_name(&font, &presets::preset(kind).font);
        }
    }

    /// Give `kind` a weight.
    pub fn set_font_weight(&mut self, kind: ThemeKind, weight: FontWeight) {
        if let Some(theme) = self.0.iter_mut().find(|t| t.id == kind) {
            theme.font_weight = weight;
        }
    }

    /// How round `kind` asks for its buttons — see
    /// [`Theme::button_shape`].
    pub fn button_shape(&self, kind: ThemeKind) -> ButtonShape {
        self.0
            .iter()
            .find(|t| t.id == kind)
            .map(|t| t.button_shape)
            .unwrap_or_default()
    }

    /// Give `kind` a shape.
    pub fn set_button_shape(&mut self, kind: ThemeKind, shape: ButtonShape) {
        if let Some(theme) = self.0.iter_mut().find(|t| t.id == kind) {
            theme.button_shape = shape;
        }
    }

    /// Whether `kind` is still wearing the shape its preset ships,
    /// which is the question the top-of-file `button_shape` is
    /// answered with — the same rule the face follows, and for the
    /// same reason: a file written before the shape moved into the
    /// palette carried one setting for the whole app, and it belongs
    /// to the palette that was on screen when it was written. See
    /// [`crate::config::Config::button_shape_in_force`].
    pub fn wears_preset_shape(&self, kind: ThemeKind) -> bool {
        self.button_shape(kind) == presets::preset(kind).button_shape
    }

    /// Whether `kind` is still wearing the family and weight its
    /// preset ships, which is what the table holds for a palette the
    /// file said nothing about.
    ///
    /// The question the top-of-file `font`/`font_weight` pair is
    /// answered with: a palette that has a face of its own — from the
    /// settings panel, or from a hand-edit of its entry — keeps it,
    /// and one that does not takes the pair, which is how a file
    /// written before the font moved into the palette is read. See
    /// [`crate::config::Config::font_in_force`].
    pub fn wears_preset_face(&self, kind: ThemeKind) -> bool {
        let preset = presets::preset(kind);
        self.font(kind) == preset.font && self.font_weight(kind) == preset.font_weight
    }

    /// Put the table back in a state the rest of the app can rely on:
    /// each of the twenty presets present exactly once, in
    /// [`ThemeKind::ALL`] order, each with a drawable name and border.
    pub fn normalize(&mut self) {
        for theme in &mut self.0 {
            theme.sanitize();
        }
        let mut kept: Vec<Theme> = Vec::with_capacity(ThemeKind::ALL.len());
        for kind in ThemeKind::ALL {
            let found = self.0.iter().find(|t| t.id == kind).cloned();
            kept.push(found.unwrap_or_else(|| presets::preset(kind)));
        }
        self.0 = kept;
    }
}

/// One table of palettes, each under its own id, in the order the
/// settings panel offers them. The id is the key rather than a field
/// of the entry, so `[themes.HighContrastDark.number]` says which
/// palette's digits it is describing without the reader having to
/// scroll back up the file to find out.
impl Serialize for ThemeTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for theme in &self.0 {
            map.serialize_entry(theme.id.key(), theme)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ThemeTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Entries;

        impl<'de> Visitor<'de> for Entries {
            type Value = Vec<Theme>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a table of palettes keyed by id")
            }

            /// The current shape: `[themes.RedmondLight]`, the key
            /// naming the palette the entry retunes.
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    // Read the entry as a plain value first, so one
                    // written as something other than a table — a
                    // string, a number — is skipped rather than
                    // failing the whole file with it.
                    let value = map.next_value::<toml::Value>()?;
                    let Some(id) = ThemeKind::from_key(key.trim()) else {
                        continue;
                    };
                    if let Ok(raw) = RawTheme::deserialize(value) {
                        out.push(raw.resolve(id));
                    }
                }
                Ok(out)
            }

            /// What earlier versions wrote: a `[[themes]]` array
            /// with the id inside each entry. Read so those files
            /// keep every colour the user tuned in them; the next
            /// save writes the table above.
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(raw) = seq.next_element::<RawTheme>()? {
                    if let Some(theme) = raw.resolve_by_own_id() {
                        out.push(theme);
                    }
                }
                Ok(out)
            }
        }

        // Either shape, told apart by what the file actually holds
        // rather than by which one this build would have written.
        let mut table = ThemeTable(deserializer.deserialize_any(Entries)?);
        table.normalize();
        Ok(table)
    }
}

/// A palette as it comes off disk, before any of it is believed.
///
/// Every field is optional and every field reads through a type that
/// cannot fail: a colour that is not `#RRGGBBAA`, a border percentage
/// that is not a number, a whole category written as a string — none
/// of them is an error, each is simply absent, and
/// [`RawTheme::resolve`] puts the shipped value in its place. One bad
/// character in one colour must not cost the user every other setting
/// in the file.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawTheme {
    /// Only a `[[themes]]` array written by an older version carries
    /// one; in the table the key above the entry names the palette.
    id: Lenient<String>,
    display_name: Lenient<String>,
    font: Lenient<String>,
    font_weight: Lenient<FontWeight>,
    button_shape: Lenient<ButtonShape>,
    app_bg: Lenient<Rgba>,
    display_bg: Lenient<Rgba>,
    sidepanel_bg: Lenient<Rgba>,
    text_active: Lenient<Rgba>,
    text_inactive: Lenient<Rgba>,
    accent: Lenient<Rgba>,
    /// Called `button_border_thickness` in earlier versions, and
    /// still read under that name so a file written by one of them
    /// keeps the border its user asked for.
    #[serde(alias = "button_border_thickness")]
    button_border_percent: Lenient<f32>,
    science: RawGroup,
    second: RawGroup,
    toprow: RawGroup,
    delete: RawGroup,
    bracket: RawGroup,
    basicop: RawGroup,
    equals: RawGroup,
    percent: RawGroup,
    reciprocal: RawGroup,
    trig: RawGroup,
    rand: RawGroup,
    negate: RawGroup,
    decimal: RawGroup,
    number: RawGroup,
}

impl RawTheme {
    /// The palette this entry describes, as an older `[[themes]]`
    /// array named it: from the `id` field inside the entry itself.
    /// `None` when it names one the build does not have — the only
    /// thing in the file that cannot be repaired, because the rest of
    /// the app addresses a palette by its id.
    fn resolve_by_own_id(self) -> Option<Theme> {
        let id = ThemeKind::from_key(self.id.0.as_deref()?.trim())?;
        Some(self.resolve(id))
    }

    /// This entry as the palette `id` names, with the shipped value
    /// in place of everything the file did not give usably.
    fn resolve(self, id: ThemeKind) -> Theme {
        let base = presets::preset(id);
        Theme {
            id,
            display_name: sanitized_display_name(
                self.display_name.0.as_deref().unwrap_or_default(),
                &base.display_name,
            ),
            font: sanitized_font_name(self.font.0.as_deref().unwrap_or_default(), &base.font),
            font_weight: self.font_weight.0.unwrap_or(base.font_weight),
            button_shape: self.button_shape.0.unwrap_or(base.button_shape),
            app_bg: self.app_bg.0.unwrap_or(base.app_bg),
            display_bg: self.display_bg.0.unwrap_or(base.display_bg),
            sidepanel_bg: self.sidepanel_bg.0.unwrap_or(base.sidepanel_bg),
            text_active: self.text_active.0.unwrap_or(base.text_active),
            text_inactive: self.text_inactive.0.unwrap_or(base.text_inactive),
            accent: self.accent.0.unwrap_or(base.accent),
            button_border_percent: self
                .button_border_percent
                .0
                .filter(|t| t.is_finite())
                .unwrap_or(base.button_border_percent)
                .clamp(0.0, MAX_BORDER_PERCENT),
            science: self.science.resolve(base.science),
            second: self.second.resolve(base.second),
            toprow: self.toprow.resolve(base.toprow),
            delete: self.delete.resolve(base.delete),
            bracket: self.bracket.resolve(base.bracket),
            basicop: self.basicop.resolve(base.basicop),
            equals: self.equals.resolve(base.equals),
            percent: self.percent.resolve(base.percent),
            reciprocal: self.reciprocal.resolve(base.reciprocal),
            trig: self.trig.resolve(base.trig),
            rand: self.rand.resolve(base.rand),
            negate: self.negate.resolve(base.negate),
            decimal: self.decimal.resolve(base.decimal),
            number: self.number.resolve(base.number),
        }
    }
}

/// One button category off disk: three rows, each of up to three
/// colours, any of which may be missing or unusable, and the weight
/// the category asks its labels be set at.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawGroup {
    fill: RawRow,
    label: RawRow,
    border: RawRow,
    font_weight: Lenient<FontWeight>,
}

impl RawGroup {
    fn resolve(self, base: ButtonColors) -> ButtonColors {
        let colors = ButtonColors::grid(
            self.fill.resolve(base.fill_row()),
            self.label.resolve(base.label_row()),
            self.border.resolve(base.border_row()),
        );
        ButtonColors {
            // Same rule the colours follow: what the file does not
            // give usably is the shipped value, which for all but a
            // couple of categories is no weight of their own — and
            // that is the palette's own weight when the keypad is
            // drawn. A category that wants a different one says so,
            // and one that wants the palette's own where the shipped
            // category asks for a weight says *that*, by naming it.
            font_weight: self.font_weight.0.or(base.font_weight),
            ..colors
        }
    }
}

/// One row off disk — `"#RRGGBBAA #RRGGBBAA #RRGGBBAA"`, or the same
/// three as a TOML list — with each slot filled in only if the file
/// gave a colour for it.
#[derive(Debug, Default)]
struct RawRow([Option<Rgba>; 3]);

impl RawRow {
    fn resolve(self, base: StateColors) -> StateColors {
        StateColors::new(
            self.0[0].unwrap_or(base.resting),
            self.0[1].unwrap_or(base.hover),
            self.0[2].unwrap_or(base.pressed),
        )
    }
}

impl<'de> Deserialize<'de> for RawRow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut row = RawRow::default();
        // Written as one line of three, which is how it is saved, or
        // as a list of three, which is how somebody reaching for TOML
        // would write it. Both mean the same row.
        match toml::Value::deserialize(deserializer)? {
            toml::Value::String(line) => {
                for (slot, word) in row.0.iter_mut().zip(line.split_whitespace()) {
                    *slot = Rgba::parse_hex_str(word).ok();
                }
            }
            toml::Value::Array(values) => {
                for (slot, value) in row.0.iter_mut().zip(&values) {
                    *slot = color_of(value);
                }
            }
            _ => {}
        }
        Ok(row)
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
    /// This component as a button category's three states: the
    /// desktop's own fills, wearing its own text and border colours.
    pub fn colors(self) -> ButtonColors {
        ButtonColors::grid(
            StateColors::new(self.base, self.hover, self.pressed),
            StateColors::flat(self.text),
            StateColors::flat(self.border),
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
    /// The component drawn on a container: everything but the digits,
    /// the decimal point and the accent keys.
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
/// derived. The fields it does not mention — the name, the border
/// thickness — are the preset's own, and so is every category's
/// weight: the desktop publishes colours, and how heavily the digits
/// are set is still the palette's to say.
pub fn apply_cosmic_override(base: Theme, over: CosmicOverride) -> Theme {
    let component = over.component.colors();
    let surface = over.surface_component.colors();
    let accent = over.accent.colors();
    // The desktop's colours, still set at the weight the palette
    // asked this category for.
    let weighed = |colors: ButtonColors, was: ButtonColors| ButtonColors {
        font_weight: was.font_weight,
        ..colors
    };
    Theme {
        app_bg: over.window_bg,
        display_bg: over.window_bg,
        sidepanel_bg: over.container_bg,
        text_active: over.interface_text,
        text_inactive: over.interface_text_dim,
        accent: over.accent.base,
        science: weighed(component, base.science),
        second: weighed(component, base.second),
        toprow: weighed(component, base.toprow),
        delete: weighed(component, base.delete),
        bracket: weighed(component, base.bracket),
        percent: weighed(component, base.percent),
        reciprocal: weighed(component, base.reciprocal),
        trig: weighed(component, base.trig),
        rand: weighed(component, base.rand),
        negate: weighed(component, base.negate),
        basicop: weighed(accent, base.basicop),
        equals: weighed(accent, base.equals),
        number: weighed(surface, base.number),
        decimal: weighed(surface, base.decimal),
        ..base
    }
}
