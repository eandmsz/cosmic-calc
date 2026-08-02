//! Library root. Re-exports the engine, history, memory, config, and
//! clipboard modules so integration tests (and future embedders) can
//! drive the calculator without touching the UI layer. The binary in
//! `main.rs` depends on this same crate through its `cosmic_calc::*`
//! paths.

pub mod clipboard;
pub mod color;
pub mod config;
pub mod engine;
pub mod history;
pub mod locale;
pub mod memory;
pub mod props;
pub mod rng;
pub mod theme;
pub mod ui;

#[cfg(test)]
mod tests;
