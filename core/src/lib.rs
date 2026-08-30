//! Calculation core: everything the calculator does that does not need
//! a window. The tokenizer/parser/evaluator pipeline, the display
//! formatter, persisted configuration, themes, locale handling,
//! clipboard sanitising, history and memory all live here.
//!
//! Deliberately free of any GUI dependency, so `cargo test -p
//! cosmic-calc-core` compiles in seconds rather than pulling in
//! libcosmic and wgpu. The `cosmic-calc` binary crate re-exports every
//! module below, so UI code keeps referring to them as
//! `crate::engine`, `crate::config`, and so on.

pub mod clipboard;
pub mod color;
pub mod config;
pub mod engine;
pub mod history;
pub mod layout;
mod lenient;
pub mod locale;
pub mod memory;
pub mod props;
pub mod rng;
pub mod theme;

#[cfg(test)]
mod tests;
