//! Per-category button styling. Every [`Button`] variant is mapped to
//! one of the theme's colour slots (science / second / toprow /
//! basicop / equals / negate / decimal / number); this module turns a
//! slot colour into a libcosmic `ButtonClass::Custom` so the keypad
//! paints each key in the palette's dedicated colour.
//!
//! Hover and pressed variants are derived from the base colour via the
//! existing `Rgba` helpers: `hover()` (HSV lift + hue shift toward
//! yellow) and `scaled(factor)` (linear RGB multiply). Disabled state
//! is a darker tint of the base.
//!
//! Styling uses only pure helpers; the category mapping is tested
//! here, the colour construction is simple enough that a snapshot-style
//! test is not worth its maintenance cost.
//!
//! Keeping this helper self-contained means `keypad.rs` stays focused
//! on layout and the theme types keep no libcosmic dependency.
//! The category function lives alongside the `Button` enum so the
//! lookup is trivially exhaustive.

use cosmic::iced::border::Radius;
use cosmic::iced::{Background, Color};
use cosmic::widget::button::{ButtonClass, Style};

use crate::color::Rgba;
use crate::theme::Theme;
use crate::ui::buttons::Button;

/// Which palette slot drives a button's background colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Science,
    Second,
    TopRow,
    BasicOp,
    Equals,
    Negate,
    Decimal,
    Number,
}

impl Category {
    /// Pick the slot colour from an active palette.
    pub fn color(self, theme: &Theme) -> Rgba {
        match self {
            Self::Science => theme.science_button,
            Self::Second => theme.second_button,
            Self::TopRow => theme.toprow_button,
            Self::BasicOp => theme.basicop_button,
            Self::Equals => theme.equals_button,
            Self::Negate => theme.negate_button,
            Self::Decimal => theme.decimal_button,
            Self::Number => theme.number_button,
        }
    }
}

/// Assign a [`Category`] to every [`Button`] variant. The mapping
/// follows the Phase-4 spec: digits / decimal / negate / equals get
/// their own slots; basic operators share one; everything else is
/// either "top-row" control or "science".
pub fn category_for(button: Button) -> Category {
    use Button::*;
    match button {
        Num(_) => Category::Number,
        Decimal => Category::Decimal,
        Negate => Category::Negate,
        Equals => Category::Equals,
        Add | Sub | Mul | Div => Category::BasicOp,
        Second => Category::Second,
        Clear | Backspace | LeftParen | RightParen | CursorLeft | CursorRight | ToggleMode
        | ToggleAngleMode | ToggleHistoryPanel | ToggleSettingsPanel | MemClear | MemRecall
        | MemAdd | MemSub => Category::TopRow,
        _ => Category::Science,
    }
}

/// Build a [`ButtonClass`] that paints the button in `base` with text
/// rendered in `text` and the corners rounded to `corner_radius`
/// pixels. Hover lightens via `Rgba::hover`, pressed darkens to 90 %
/// luminance, disabled darkens to 70 % luminance.
pub fn class(base: Rgba, text: Rgba, corner_radius: f32) -> ButtonClass {
    let active_bg = rgba_to_color(base);
    let hovered_bg = rgba_to_color(base.hover());
    let pressed_bg = rgba_to_color(base.scaled(0.9));
    let disabled_bg = rgba_to_color(base.scaled(0.7));
    let txt = rgba_to_color(text);
    let txt_dim = rgba_to_color(text.inactive());
    let radius = Radius::from(corner_radius);

    ButtonClass::Custom {
        active: Box::new(move |_focused, _theme| build_style(active_bg, txt, radius)),
        hovered: Box::new(move |_focused, _theme| build_style(hovered_bg, txt, radius)),
        pressed: Box::new(move |_focused, _theme| build_style(pressed_bg, txt, radius)),
        disabled: Box::new(move |_theme| build_style(disabled_bg, txt_dim, radius)),
    }
}

/// Convenience: colour a button straight from a [`Theme`] given its
/// [`Button`] tag, applying the user-configured corner radius.
pub fn class_for(theme: &Theme, button: Button, corner_radius: f32) -> ButtonClass {
    let cat = category_for(button);
    class(cat.color(theme), theme.text_active, corner_radius)
}

/// Same as [`class_for`] but renders the button in its pressed colour
/// even when not actively pressed by the mouse. Used to flash the
/// keypad cell that matches a keyboard activation, so the user can see
/// which button their keystroke hit.
pub fn class_for_flashed(theme: &Theme, button: Button, corner_radius: f32) -> ButtonClass {
    let cat = category_for(button);
    class_flashed(cat.color(theme), theme.text_active, corner_radius)
}

/// Like [`class`] but the active variant uses the pressed colour, so
/// the button looks "held down" without any pointer interaction.
fn class_flashed(base: Rgba, text: Rgba, corner_radius: f32) -> ButtonClass {
    let pressed_bg = rgba_to_color(base.scaled(0.9));
    let hovered_bg = rgba_to_color(base.hover());
    let disabled_bg = rgba_to_color(base.scaled(0.7));
    let txt = rgba_to_color(text);
    let txt_dim = rgba_to_color(text.inactive());
    let radius = Radius::from(corner_radius);

    ButtonClass::Custom {
        active: Box::new(move |_focused, _theme| build_style(pressed_bg, txt, radius)),
        hovered: Box::new(move |_focused, _theme| build_style(hovered_bg, txt, radius)),
        pressed: Box::new(move |_focused, _theme| build_style(pressed_bg, txt, radius)),
        disabled: Box::new(move |_theme| build_style(disabled_bg, txt_dim, radius)),
    }
}

// ---------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------

fn rgba_to_color(c: Rgba) -> Color {
    let (r, g, b, a) = c.to_f32();
    Color::from_rgba(r, g, b, a)
}

fn build_style(background: Color, text: Color, radius: Radius) -> Style {
    let mut s = Style::new();
    s.background = Some(Background::Color(background));
    s.text_color = Some(text);
    s.icon_color = Some(text);
    s.border_radius = radius;
    s
}
