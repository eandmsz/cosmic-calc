//! Glue between libcosmic's live desktop palette and our own
//! `CosmicOverride` struct. Keeping this isolated means `theme.rs`
//! does not depend on libcosmic, so its unit tests can run without
//! a compositor – only this bridge pulls in the heavy crate.
//!
//! The desktop publishes each of its components in every state it
//! draws that component in — a base, a hover, a pressed, the text on
//! it and its border — so the Cosmic preset takes those verbatim
//! rather than deriving hover and pressed from the base. The keys
//! then hover the way the rest of the desktop's buttons hover, and an
//! accent-coloured key wears the accent's *own* text colour. That
//! last part is where contrast is won or lost: the window's text
//! colour is chosen to read against the window, and on a bright
//! accent fill it has nothing to spare.
//!
//! The mapping:
//!
//! * `background.base` → window and display background
//! * `primary.base` → side-panel background
//! * `background.on` → interface text
//! * `primary.component` → the control keys (science, `2nd`, top row,
//!   delete, negate)
//! * `background.component` → the digits and the decimal point, which
//!   the desktop draws a shade apart from the container's own
//!   components and which read as a group of their own
//! * `accent` → the basic operators, `=`, and the settings panel's
//!   switches and sliders

use cosmic::cosmic_theme;
use cosmic::cosmic_theme::palette::Srgba;

use crate::color::Rgba;
use crate::theme::{CosmicComponent, CosmicOverride};

/// How dim the desktop's interface text is drawn where the calculator
/// wants a secondary reading — the caption above the readout, a
/// number property that does not hold. COSMIC's own secondary text
/// sits at this alpha.
const DIM_TEXT_ALPHA: u8 = 0x80;

/// Convert a palette::Srgba (float RGBA in 0..=1) to our 8-bit
/// `Rgba`. Out-of-range channel values – which palette will produce
/// after gamut-mapping – are clamped by `Rgba::from_f32`.
fn srgba_to_rgba(c: Srgba) -> Rgba {
    Rgba::from_f32(c.red, c.green, c.blue, c.alpha)
}

/// One of the desktop's components in the states it draws itself in.
fn component(c: &cosmic_theme::Component) -> CosmicComponent {
    CosmicComponent {
        base: srgba_to_rgba(c.base),
        hover: srgba_to_rgba(c.hover),
        pressed: srgba_to_rgba(c.pressed),
        text: srgba_to_rgba(c.on),
        border: srgba_to_rgba(c.border),
    }
}

/// Build a `CosmicOverride` from the running desktop theme. Designed
/// to be called whenever the ambient `cosmic_theme::Theme` changes so
/// the calculator's Cosmic preset tracks the desktop.
pub fn override_from_cosmic(theme: &cosmic_theme::Theme) -> CosmicOverride {
    let interface_text = srgba_to_rgba(theme.on_bg_color());
    CosmicOverride {
        window_bg: srgba_to_rgba(theme.bg_color()),
        container_bg: srgba_to_rgba(theme.primary_container_color()),
        interface_text,
        interface_text_dim: Rgba {
            a: DIM_TEXT_ALPHA,
            ..interface_text
        },
        component: component(&theme.primary.component),
        surface_component: component(&theme.background.component),
        accent: component(&theme.accent),
    }
}
