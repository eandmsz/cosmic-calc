//! Library root for the GUI crate. The calculation core lives in the
//! separate `cosmic-calc-core` package and is re-exported here, so UI
//! modules keep referring to `crate::engine`, `crate::config`, and so
//! on, and embedders get one import for the whole calculator.
//!
//! The split exists so the core can be tested without libcosmic: run
//! `cargo test -p cosmic-calc-core` for the engine, formatter, config
//! and clipboard suites, and `cargo test` for the UI on top.

pub use cosmic_calc_core::{
    clipboard, color, config, engine, history, locale, memory, props, rng, theme,
};

pub mod ui;

#[cfg(test)]
mod tests;
