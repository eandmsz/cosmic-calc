//! Test suite for the UI layer. Everything here needs the libcosmic
//! types; the calculation core is tested separately in
//! `cosmic-calc-core`, which compiles without them.

mod button_style;
mod buttons;
mod cosmic_bridge;
mod display;
mod display_scaling;
mod font_metrics;
mod keymap;
mod keypad;
mod keys;
mod panel_width;

/// The version stamped into `config.toml` comes from the core crate,
/// which is versioned separately from the binary. They ship as one
/// program, so a file that named one of them and not the other would
/// tell a later release the wrong thing about what wrote it.
#[test]
fn the_config_version_is_the_application_version() {
    assert_eq!(crate::config::CONFIG_VERSION, env!("CARGO_PKG_VERSION"));
}
