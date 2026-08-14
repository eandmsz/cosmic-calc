use crate::config::*;
use crate::color::Rgba;
use crate::theme::ThemeKind;
use std::path::PathBuf;

#[test]
fn defaults_sit_in_valid_ranges() {
    let c = Config::default();
    assert_eq!(c.significant_digits, 15);
    assert_eq!(c.window_startup_width, 300);
    assert_eq!(c.window_startup_height, 700);
    assert_eq!(c.font, "Adwaita Sans");
    assert_eq!(c.mode, Mode::Scientific);
    assert_eq!(c.theme_kind, ThemeKind::Cosmic);
    assert!(c.rand_min_incl < c.rand_max_excl);
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
    assert_eq!(c.mode, Mode::Scientific);
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
        mode: Mode::Basic,
        ..Config::default()
    };
    cfg.apply_theme_preset(ThemeKind::RedmondDark);
    cfg.save_at(&path).expect("save");

    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.significant_digits, 9);
    assert_eq!(back.window_startup_width, 444);
    assert_eq!(back.mode, Mode::Basic);
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
    std::fs::write(
        &path,
        "significant_digits = 42\nwindow_startup_width = 2\n",
    )
    .unwrap();

    let cfg = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(cfg.significant_digits, MAX_SIGNIFICANT_DIGITS);
    assert_eq!(cfg.window_startup_width, MIN_WINDOW_DIM);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
