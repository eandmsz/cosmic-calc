//! UI module root. Phase-1 provides a minimal libcosmic application
//! so the binary compiles and launches; the visual keypad, history
//! panel, and memory display are placeholders that a later phase will
//! flesh out. The goal here is to keep the crate building end-to-end
//! while the engine stabilises.

pub mod app;
pub mod button_style;
pub mod buttons;
pub mod cosmic_bridge;
pub mod display;
pub mod font;
pub mod keypad;
pub mod keys;
pub mod panels;

pub use app::AppModel;
pub use buttons::{apply_button, Button, ButtonEffect, ClearMode, MemoryOp, UiState};
pub use cosmic_bridge::override_from_cosmic;
