use crate::color::Rgba;
use crate::config::*;
use crate::engine::Notation;
use crate::theme::ThemeKind;
use std::path::PathBuf;

#[test]
fn defaults_sit_in_valid_ranges() {
    let c = Config::default();
    assert_eq!(c.significant_digits, 15);
    assert_eq!(c.window_startup_width, 300);
    assert_eq!(c.window_startup_height, 700);
    assert_eq!(c.font, "Adwaita Sans");
    // First run opens on the Basic keypad.
    assert_eq!(c.mode, Mode::Basic);
    assert_eq!(c.theme_kind, ThemeKind::Cosmic);
    assert!(c.rand_min_incl < c.rand_max_excl);
}

#[test]
fn the_debug_toggle_picks_the_notation_and_survives_a_save() {
    let mut c = Config::default();
    // Pretty by default: the toggle is a debugging aid, not the
    // everyday rendering.
    assert!(!c.debug_raw_formula);
    assert_eq!(c.notation(), Notation::Pretty);

    c.debug_raw_formula = true;
    assert_eq!(c.notation(), Notation::Raw);

    let text = toml::to_string_pretty(&c).unwrap();
    let back: Config = toml::from_str(&text).unwrap();
    assert!(back.debug_raw_formula);
    assert_eq!(back.notation(), Notation::Raw);
}

#[test]
fn validate_clamps_out_of_range_fields() {
    let mut c = Config {
        button_corner_radius: -5.0,
        significant_digits: 99,
        window_startup_width: 0,
        window_startup_height: 999_999,
        rand_decimals: 40,
        font: "   ".to_string(),
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.button_corner_radius, 0.0);
    assert_eq!(c.significant_digits, MAX_SIGNIFICANT_DIGITS);
    assert_eq!(c.window_startup_width, MIN_WINDOW_DIM);
    assert_eq!(c.window_startup_height, MAX_WINDOW_DIM);
    assert_eq!(c.rand_decimals, MAX_RAND_DECIMALS);
    assert_eq!(c.font, DEFAULT_FONT);
}

#[test]
fn validate_resets_rand_range_if_inverted() {
    let mut c = Config {
        rand_min_incl: 5.0,
        rand_max_excl: 1.0,
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.rand_min_incl, 0.0);
    assert_eq!(c.rand_max_excl, 1.0);
}

#[test]
fn validate_resets_rand_range_on_nan() {
    let mut c = Config {
        rand_min_incl: f64::NAN,
        rand_max_excl: 1.0,
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.rand_min_incl, 0.0);
    assert_eq!(c.rand_max_excl, 1.0);
}

#[test]
fn apply_preset_replaces_full_palette() {
    let mut c = Config::default();
    c.apply_theme_preset(ThemeKind::CupertinoDark);
    assert_eq!(c.theme_kind, ThemeKind::CupertinoDark);
    assert_eq!(c.theme.name, "Cupertino Dark");
    assert_eq!(c.theme.app_bg, Rgba::from_hex(0x28_31_33_FF));
}

#[test]
fn mark_custom_preserves_edited_palette() {
    let mut c = Config::default();
    c.theme.app_bg = Rgba::from_hex(0xAB_CD_EF_FF);
    c.mark_theme_custom();
    assert_eq!(c.theme_kind, ThemeKind::Custom);
    assert_eq!(c.theme.app_bg, Rgba::from_hex(0xAB_CD_EF_FF));
    assert_eq!(c.theme.name, "Custom");
}

#[test]
fn round_trip_through_toml() {
    let c = Config::default();
    let s = toml::to_string(&c).expect("serialize");
    assert!(
        s.contains("#"),
        "theme colours should serialize as hex strings: {s}"
    );
    let back: Config = toml::from_str(&s).expect("deserialize");
    assert_eq!(back.significant_digits, c.significant_digits);
    assert_eq!(back.mode, c.mode);
    assert_eq!(back.theme_kind, c.theme_kind);
    assert_eq!(back.theme.app_bg, c.theme.app_bg);
    assert_eq!(back.decimal_separator, c.decimal_separator);
}

#[test]
fn partial_toml_picks_up_defaults() {
    // Only significant_digits set; everything else should default.
    let toml_src = "significant_digits = 7\n";
    let c: Config = toml::from_str(toml_src).expect("partial load");
    assert_eq!(c.significant_digits, 7);
    assert_eq!(c.window_startup_width, DEFAULT_WINDOW_WIDTH);
    assert_eq!(c.mode, Mode::Basic);
}

/// Build a per-process scratch path so tests can run in parallel
/// without colliding. We use nanosecond timestamps plus the PID;
/// the tempdir itself is wiped at the end of each test.
fn scratch_path(label: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "cosmic-calc-test-{}-{}-{}/config.toml",
        label,
        std::process::id(),
        nanos
    ))
}

#[test]
fn load_or_create_default_creates_missing_file() {
    let path = scratch_path("missing");
    assert!(!path.exists());
    let cfg = Config::load_or_create_default_at(&path).expect("load");
    assert!(path.exists(), "file should be written on first load");
    assert_eq!(cfg.significant_digits, DEFAULT_SIGNIFICANT_DIGITS);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn save_and_reload_round_trip() {
    let path = scratch_path("roundtrip");
    let mut cfg = Config {
        significant_digits: 9,
        window_startup_width: 444,
        // Not the default, so the round-trip has something to prove.
        mode: Mode::Scientific,
        ..Config::default()
    };
    cfg.apply_theme_preset(ThemeKind::RedmondDark);
    cfg.save_at(&path).expect("save");

    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.significant_digits, 9);
    assert_eq!(back.window_startup_width, 444);
    assert_eq!(back.mode, Mode::Scientific);
    assert_eq!(back.theme_kind, ThemeKind::RedmondDark);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn corner_radius_clamps_to_max() {
    let mut c = Config {
        button_corner_radius: 200.0,
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.button_corner_radius, MAX_CORNER_RADIUS);
}

#[test]
fn theme_name_stays_in_sync_with_preset() {
    let mut c = Config::default();
    c.apply_theme_preset(ThemeKind::CupertinoLight);
    c.theme.name = "oops".to_string(); // simulate stale state
    c.validate_and_clamp();
    assert_eq!(c.theme.name, "Cupertino Light");
}

#[test]
fn max_decimals_for_rand_max_shrinks_with_int_part() {
    // Default 1.0 → values are 0.x, 1 int digit, 14 decimals fit.
    assert_eq!(max_decimals_for_rand_max(1.0), 14);
    // 10.0 → values can reach 9.999..., still 1 int digit.
    assert_eq!(max_decimals_for_rand_max(10.0), 14);
    // 100.0 → values up to 99.999..., 2 int digits.
    assert_eq!(max_decimals_for_rand_max(100.0), 13);
    // 1e15 → values up to ~999999999999999.999..., 15 int digits.
    assert_eq!(max_decimals_for_rand_max(1e15), 0);
    // Beyond the 15-digit cap stays at 0; never goes negative.
    assert_eq!(max_decimals_for_rand_max(1e20), 0);
}

#[test]
fn max_decimals_for_rand_max_handles_non_positive_max() {
    // Invalid bounds aren't reachable in steady state but the
    // helper has to return SOMETHING – pick the most permissive
    // fallback so the slider doesn't collapse to zero by accident
    // mid-typing.
    assert_eq!(max_decimals_for_rand_max(0.0), 14);
    assert_eq!(max_decimals_for_rand_max(-1.0), 14);
    assert_eq!(max_decimals_for_rand_max(f64::NAN), 14);
}

#[test]
fn load_clamps_out_of_range_values() {
    let path = scratch_path("clamp");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "significant_digits = 42\nwindow_startup_width = 2\n").unwrap();

    let cfg = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(cfg.significant_digits, MAX_SIGNIFICANT_DIGITS);
    assert_eq!(cfg.window_startup_width, MIN_WINDOW_DIM);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// =====================================================================
// Colour storage format
// =====================================================================

#[test]
fn colours_are_stored_as_compact_rgba_hex() {
    let toml = toml::to_string_pretty(&Config::default()).expect("serialise");

    // Structured: the palette is one named table, not eleven loose
    // top-level keys.
    assert!(
        toml.contains("[theme]"),
        "expected a [theme] table:\n{toml}"
    );

    // Compact: each colour is a single `#RRGGBBAA` string. The type
    // also accepts the older `{ r, g, b, a }` table on the way in, so
    // this asserts the *written* form has not regressed to it.
    assert!(
        toml.contains(r##"app_bg = "#1B1B1BFF""##),
        "expected #RRGGBBAA hex:\n{toml}"
    );
    assert!(
        !toml.contains("[theme.app_bg]"),
        "colours must not expand into per-channel tables:\n{toml}"
    );

    // Every slot in the palette, uppercase hex, alpha always present.
    let hex = regex_lite_hex_lines(&toml);
    assert_eq!(hex.len(), 11, "expected 11 colour slots, got {hex:?}");
    for line in &hex {
        let value = line.split('=').nth(1).unwrap().trim().trim_matches('"');
        assert_eq!(value.len(), 9, "{value:?} should be #RRGGBBAA");
        assert!(value.starts_with('#'), "{value:?} should start with #");
        assert!(
            value[1..]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_lowercase()),
            "{value:?} should be uppercase hex"
        );
    }
}

/// Collect the `key = "#RRGGBBAA"` lines without pulling in a regex
/// crate for one assertion.
fn regex_lite_hex_lines(toml: &str) -> Vec<&str> {
    toml.lines().filter(|l| l.contains("= \"#")).collect()
}

#[test]
fn colours_round_trip_through_the_hex_form() {
    let mut cfg = Config::default();
    cfg.apply_theme_preset(ThemeKind::CupertinoDark);
    let toml = toml::to_string_pretty(&cfg).expect("serialise");
    let back: Config = toml::from_str(&toml).expect("deserialise");
    assert_eq!(back.theme, cfg.theme);
}

#[test]
fn legacy_per_channel_colour_tables_still_load() {
    // Older config files stored colours as a table of channels. They
    // must keep loading, and get rewritten in the compact form on the
    // next save.
    let raw = r##"
        [theme]
        name = "Custom"
        app_bg = { r = 27, g = 27, b = 27, a = 255 }
        sidepanel_bg = "#272727FF"
        text_active = "#E7E7E7FF"
        science_button = "#636363FF"
        second_button = "#636363FF"
        toprow_button = "#636363FF"
        basicop_button = "#61CDDCFF"
        equals_button = "#61CDDCFF"
        negate_button = "#636363FF"
        decimal_button = "#4F4F4FFF"
        number_button = "#4F4F4FFF"
    "##;
    let cfg: Config = toml::from_str(raw).expect("legacy form must deserialise");
    assert_eq!(cfg.theme.app_bg, Rgba::from_hex(0x1B_1B_1B_FF));
    let rewritten = toml::to_string_pretty(&cfg).expect("serialise");
    assert!(rewritten.contains(r##"app_bg = "#1B1B1BFF""##));
}

#[test]
fn the_new_toggles_default_to_what_the_app_always_did() {
    let c = Config::default();
    // The memory register is on: it used to be shown unconditionally,
    // just in a place only the history panel could reach.
    assert!(c.show_memory);
    // The window size has always been remembered.
    assert!(c.save_window_size);
    // The history never has, so keeping it is the user's to ask for.
    assert!(!c.save_history);
    assert!(c.history.is_empty());
    // Either half of the row under the display is reason enough to
    // draw it.
    assert!(c.status_row_visible());
    assert!(!Config {
        show_memory: false,
        ..Config::default()
    }
    .status_row_visible());
}

#[test]
fn a_stored_history_is_kept_only_while_the_toggle_is_on() {
    use crate::history::{StoredEntry, HISTORY_CAPACITY};
    let rows: Vec<StoredEntry> = (0..HISTORY_CAPACITY + 4)
        .map(|i| StoredEntry {
            expression: format!("{i}"),
            result: format!("{i}"),
        })
        .collect();

    // Off, whatever is in the file is dropped on load — including a
    // hand-edited list and the leftovers of the toggle being turned
    // off while the app was not running.
    let mut c = Config {
        save_history: false,
        history: rows.clone(),
        ..Config::default()
    };
    c.validate_and_clamp();
    assert!(c.history.is_empty());

    // On, only as much of it as the panel would hold, newest kept.
    let mut c = Config {
        save_history: true,
        history: rows,
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.history.len(), HISTORY_CAPACITY);
    assert_eq!(c.history[0].result, "4");
}

#[test]
fn the_new_fields_round_trip_through_the_file() {
    use crate::history::StoredEntry;
    let c = Config {
        save_history: true,
        show_memory: false,
        save_window_size: false,
        history: vec![StoredEntry {
            expression: "2^3".to_string(),
            result: "8".to_string(),
        }],
        ..Config::default()
    };
    let text = toml::to_string_pretty(&c).expect("serialises");
    let mut back: Config = toml::from_str(&text).expect("parses");
    back.validate_and_clamp();
    assert!(back.save_history);
    assert!(!back.show_memory);
    assert!(!back.save_window_size);
    assert_eq!(back.history.len(), 1);
    assert_eq!(back.history[0].expression, "2^3");
}

#[test]
fn a_pinned_minimum_width_is_held_to_the_window_range() {
    // Zero passes through: it is the "let the keypad decide" value,
    // not a width.
    let mut c = Config {
        min_window_width: AUTO_MIN_WINDOW_WIDTH,
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.min_window_width, AUTO_MIN_WINDOW_WIDTH);
    assert_eq!(c.pinned_min_window_width(), None);

    // Anything else is a width, and is held to the same range the
    // startup dimensions are.
    for (given, expected) in [
        (5u32, MIN_WINDOW_DIM),
        (250, 250),
        (MAX_WINDOW_DIM + 1000, MAX_WINDOW_DIM),
    ] {
        let mut c = Config {
            min_window_width: given,
            ..Config::default()
        };
        c.validate_and_clamp();
        assert_eq!(c.min_window_width, expected, "given {given}");
        assert_eq!(c.pinned_min_window_width(), Some(expected as f32));
    }

    // And it round-trips through the file like every other field.
    let c = Config {
        min_window_width: 240,
        ..Config::default()
    };
    let back: Config = toml::from_str(&toml::to_string_pretty(&c).unwrap()).unwrap();
    assert_eq!(back.min_window_width, 240);
}
