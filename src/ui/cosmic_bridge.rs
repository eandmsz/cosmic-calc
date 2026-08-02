//! Glue between libcosmic's live desktop palette and our own
//! `CosmicOverride` struct. Keeping this isolated means `theme.rs`
//! does not depend on libcosmic, so its unit tests can run without
//! a compositor – only this bridge pulls in the heavy crate.
//!
//! The mapping follows the Phase-2 spec:
//!
//! * `bg_color`                  → window / app background
//! * `primary_container_color`   → side-panel background
//! * `on_bg_color`               → interface text
//! * `primary_component_color`   → control-component tint (science /
//!                                  second / top-row / negate buttons)
//! * `accent_color`              → accent (equals / basic-op buttons)
//! * `is_dark`                   → whether to darken (×0.8) or lighten
//!                                  (×1.2) the number/decimal buttons
//!                                  derived from the component tint

use cosmic::cosmic_theme;
use cosmic::cosmic_theme::palette::Srgba;

use crate::color::Rgba;
use crate::theme::CosmicOverride;

/// Convert a palette::Srgba (float RGBA in 0..=1) to our 8-bit
/// `Rgba`. Out-of-range channel values – which palette will produce
/// after gamut-mapping – are clamped by `Rgba::from_f32`.
fn srgba_to_rgba(c: Srgba) -> Rgba {
    Rgba::from_f32(c.red, c.green, c.blue, c.alpha)
}

/// Build a `CosmicOverride` from the running desktop theme. Designed
/// to be called whenever the ambient `cosmic_theme::Theme` changes so
/// the calculator's Cosmic preset tracks the desktop.
pub fn override_from_cosmic(theme: &cosmic_theme::Theme) -> CosmicOverride {
    CosmicOverride {
        window_bg: srgba_to_rgba(theme.bg_color()),
        container_bg: srgba_to_rgba(theme.primary_container_color()),
        interface_text: srgba_to_rgba(theme.on_bg_color()),
        component_tint: srgba_to_rgba(theme.primary_component_color()),
        accent: srgba_to_rgba(theme.accent_color()),
        is_dark: theme.is_dark,
    }
}
