//! Per-category button styling. Every [`Button`] variant is mapped to
//! one of the theme's colour groups (science / second / toprow /
//! delete / basicop / equals / negate / decimal / number); this module
//! turns that group into a libcosmic `ButtonClass::Custom` so the
//! keypad paints each key in the palette's dedicated colours.
//!
//! Nothing is computed. The group carries a fill, a label colour and a
//! border colour for each of the three states, and each state is drawn
//! with the three it names — see [`crate::theme::ButtonColors`]. Hover
//! used to be an HSV lift of the base and pressed a 10 % darkening of
//! it, with one label colour for the whole theme however little
//! contrast it had against the key it landed on.
//!
//! The border is drawn inside the button's own rectangle, so its width
//! never moves anything: turning one on changes what a key looks like
//! and not where it or its neighbours sit. How wide it comes out is
//! [`Theme::border_width`], which is the one place a number is worked
//! out, and it needs the button's height — which is why the class
//! builders take one.
//!
//! Keeping this helper self-contained means `keypad.rs` stays focused
//! on layout and the theme types keep no libcosmic dependency.

use cosmic::iced::border::Radius;
use cosmic::iced::{Background, Color};
use cosmic::widget::button::{ButtonClass, Style};

use crate::color::Rgba;
use crate::theme::{ButtonColors, ButtonFace, Theme};
use crate::ui::buttons::Button;

/// Which palette group drives a button's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Science,
    Second,
    TopRow,
    /// `AC`/`C` and backspace — the two keys that take something
    /// away. Their own group so a theme can mark them; the shipped
    /// ones paint them exactly as the top row.
    Delete,
    BasicOp,
    Equals,
    Negate,
    Decimal,
    Number,
}

impl Category {
    /// Pick the group from an active palette.
    pub fn colors(self, theme: &Theme) -> ButtonColors {
        match self {
            Self::Science => theme.science,
            Self::Second => theme.second,
            Self::TopRow => theme.toprow,
            Self::Delete => theme.delete,
            Self::BasicOp => theme.basicop,
            Self::Equals => theme.equals,
            Self::Negate => theme.negate,
            Self::Decimal => theme.decimal,
            Self::Number => theme.number,
        }
    }
}

/// Assign a [`Category`] to every [`Button`] variant. The mapping
/// follows the Phase-4 spec: digits / decimal / negate / equals get
/// their own slots; basic operators share one; clear and backspace
/// share the delete slot; everything else is either "top-row" control
/// or "science".
pub fn category_for(button: Button) -> Category {
    use Button::*;
    match button {
        Num(_) => Category::Number,
        Decimal => Category::Decimal,
        Negate => Category::Negate,
        Equals => Category::Equals,
        Add | Sub | Mul | Div => Category::BasicOp,
        Second => Category::Second,
        Clear | Backspace => Category::Delete,
        LeftParen | RightParen | CursorLeft | CursorRight | CursorHome | CursorEnd | ToggleMode
        | ToggleAngleMode | ToggleHistoryPanel | ToggleSettingsPanel | MemClear | MemRecall
        | MemAdd | MemSub => Category::TopRow,
        _ => Category::Science,
    }
}

/// Build a [`ButtonClass`] that draws each state in the colours
/// `colors` names for it, with corners rounded to `corner_radius` and
/// a border `border_width` pixels wide drawn inside the button.
///
/// No button in this app is ever disabled — every one is built with an
/// `on_press` — so the disabled variant is the resting one rather than
/// a dimmed invention of a colour the theme has not asked for.
pub fn class(colors: ButtonColors, corner_radius: f32, border_width: f32) -> ButtonClass {
    let radius = Radius::from(corner_radius);
    let normal = style_of(colors.normal, radius, border_width);
    let hovered = style_of(colors.hover, radius, border_width);
    let pressed = style_of(colors.pressed, radius, border_width);

    ButtonClass::Custom {
        active: Box::new(move |_focused, _theme| normal),
        hovered: Box::new(move |_focused, _theme| hovered),
        pressed: Box::new(move |_focused, _theme| pressed),
        disabled: Box::new(move |_theme| normal),
    }
}

/// Colour a button straight from a [`Theme`] given its [`Button`] tag,
/// applying the user-configured corner radius and the theme's border
/// at the width `height` earns it.
pub fn class_for(theme: &Theme, button: Button, corner_radius: f32, height: f32) -> ButtonClass {
    let colors = category_for(button).colors(theme);
    class(colors, corner_radius, theme.border_width(height))
}

/// Same as [`class_for`] but the resting state is drawn in the pressed
/// colours, so the button looks held down without any pointer
/// interaction. Used to flash the keypad cell that matches a keyboard
/// activation, so the user can see which button their keystroke hit.
pub fn class_for_flashed(
    theme: &Theme,
    button: Button,
    corner_radius: f32,
    height: f32,
) -> ButtonClass {
    let colors = category_for(button).colors(theme);
    class(
        ButtonColors::new(colors.pressed, colors.hover, colors.pressed),
        corner_radius,
        theme.border_width(height),
    )
}

/// Colour a latched toggle that is currently on: the `2nd` key while
/// its table is the one on screen, the chosen row in a settings
/// list.
///
/// The button swaps to the palette's text colour with the window
/// background as its label, so "armed" reads at a glance in every
/// theme. It holds that appearance under the pointer as well — a
/// latch is showing state rather than inviting a press, and a hover
/// shade there was easy to read as an ordinary press.
pub fn class_for_toggled(theme: &Theme, corner_radius: f32, height: f32) -> ButtonClass {
    let face = ButtonFace::new(theme.text_active, theme.app_bg, theme.text_active);
    class(
        ButtonColors::flat(face),
        corner_radius,
        theme.border_width(height),
    )
}

// ---------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------

pub(crate) fn rgba_to_color(c: Rgba) -> Color {
    let (r, g, b, a) = c.to_f32();
    Color::from_rgba(r, g, b, a)
}

fn style_of(face: ButtonFace, radius: Radius, border_width: f32) -> Style {
    let text = rgba_to_color(face.text);
    let mut s = Style::new();
    s.background = Some(Background::Color(rgba_to_color(face.background)));
    s.text_color = Some(text);
    s.icon_color = Some(text);
    s.border_radius = radius;
    s.border_width = border_width;
    s.border_color = rgba_to_color(face.border);
    s
}
