//! Test suite for the pure calculation core. Each submodule covers the
//! source file it is named after; none of them need a running
//! compositor, so `cargo test -p cosmic-calc-core` builds and runs in
//! seconds without the GUI dependency tree.

mod clipboard;
mod clipboard_spec;
mod color;
mod config;
mod engine_input;
mod engine_integration;
mod history;
mod locale;
mod memory;
mod props;
mod rng;
mod theme;
