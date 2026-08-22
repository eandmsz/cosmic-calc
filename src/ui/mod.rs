//! UI module root: the libcosmic application, the keypad grid, the
//! expression display, the side panels and the keyboard bindings.
//!
//! The calculation itself lives in `cosmic-calc-core`, which this crate
//! re-exports; nothing under `ui` should hold state the core could own
//! instead.

pub mod app;
pub mod button_style;
pub mod buttons;
pub mod cosmic_bridge;
pub mod display;
pub mod display_metrics;
pub mod font;
pub mod font_metrics;
pub mod keymap;
pub mod keypad;
pub mod keys;
pub mod panels;

pub use app::AppModel;
pub use buttons::{
    apply_button, apply_resolved_button, resolve_for_keyboard, Button, ButtonEffect, ClearMode,
    MemoryOp, UiState,
};
pub use cosmic_bridge::override_from_cosmic;
