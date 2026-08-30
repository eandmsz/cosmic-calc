use crate::color::rgba;
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
fn the_corner_radius_presets_are_offered_as_what_they_draw() {
    // The settings panel offers these under "Button corner radius",
    // so each one says what it rounds off. The two rounded presets
    // are a fraction of the button's height on the keypad, not a
    // pixel count, and a fixed number would stop being true the
    // moment the window was dragged.
    let names: Vec<_> = ButtonShape::ALL.iter().map(|s| s.display_name()).collect();
    assert_eq!(names, ["System", "50%", "25%", "0%"]);

    // The stored pair is a separate thing: the buttons outside the
    // keypad have no height to scale against and take a fixed radius.
    // `System` has none of its own — that is what defers to the
    // desktop.
    assert_eq!(ButtonShape::Auto.resolved(), None);
    assert_eq!(ButtonShape::Square.resolved(), Some((0.0, 1.0)));
    assert!(
        ButtonShape::Round.resolved().unwrap().0 > ButtonShape::SlightlyRound.resolved().unwrap().0
    );

    // The key the file records is the variant's own, so renaming a
    // label cannot orphan a config somebody already has.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrap {
        shape: ButtonShape,
    }
    for shape in ButtonShape::ALL {
        let toml = toml::to_string(&Wrap { shape }).unwrap();
        let back: Wrap = toml::from_str(&toml).unwrap();
        assert_eq!(back.shape, shape, "{toml}");
    }
    let back: Wrap = toml::from_str(r#"shape = "auto""#).unwrap();
    assert_eq!(back.shape, ButtonShape::Auto);
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
fn the_palette_in_force_comes_out_of_the_file() {
    // `theme_kind` says which palette is on; the palette itself is
    // the entry the file carries for it, which starts life as the
    // shipped one and is the user's to retune.
    let mut c = Config {
        theme_kind: ThemeKind::CupertinoDark,
        ..Config::default()
    };
    assert_eq!(c.theme().display_name, "Cupertino Dark");
    assert_eq!(c.theme().app_bg, rgba("#283133FF"));
    assert_eq!(c.theme(), ThemeKind::CupertinoDark.get());

    // Retune it, and that is what the window is painted with.
    let mut retuned = ThemeKind::CupertinoDark.get();
    retuned.app_bg = rgba("#0A0B0CFF");
    retuned.display_name = "Mine".to_string();
    c.themes = toml_themes(&[retuned]);
    assert_eq!(c.theme().app_bg, rgba("#0A0B0CFF"));
    assert_eq!(c.theme_display_name(ThemeKind::CupertinoDark), "Mine");
    // Every other palette is untouched.
    assert_eq!(c.themes.get(ThemeKind::Tokyo), ThemeKind::Tokyo.get());
}

/// A [`ThemeTable`] built the way a config file builds one: written
/// out and read back, so the test exercises the same path the app
/// does rather than a constructor only it can reach.
fn toml_themes(themes: &[crate::theme::Theme]) -> crate::theme::ThemeTable {
    #[derive(serde::Serialize)]
    struct Wrap {
        themes: Vec<crate::theme::Theme>,
    }
    #[derive(serde::Deserialize)]
    struct Read {
        themes: crate::theme::ThemeTable,
    }
    let text = toml::to_string_pretty(&Wrap {
        themes: themes.to_vec(),
    })
    .expect("serialise");
    toml::from_str::<Read>(&text).expect("parse").themes
}

#[test]
fn round_trip_through_toml() {
    let c = Config::default();
    let s = toml::to_string(&c).expect("serialize");
    let back: Config = toml::from_str(&s).expect("deserialize");
    assert_eq!(back.significant_digits, c.significant_digits);
    assert_eq!(back.mode, c.mode);
    assert_eq!(back.theme_kind, c.theme_kind);
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
    cfg.theme_kind = ThemeKind::RedmondDark;
    cfg.save_at(&path).expect("save");

    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.significant_digits, 9);
    assert_eq!(back.window_startup_width, 444);
    assert_eq!(back.mode, Mode::Scientific);
    assert_eq!(back.theme_kind, ThemeKind::RedmondDark);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_retuned_palette_survives_a_save_and_a_reload() {
    // The whole point of carrying the palettes in the file: an edit
    // there has to come back out of it, through the same write and
    // read the running app does.
    let path = scratch_path("retuned-theme");
    let mut mine = ThemeKind::Barbie.get();
    mine.display_name = "Not Barbie".to_string();
    mine.app_bg = rgba("#0F0E0DFF");
    mine.button_border_thickness = 2.5;
    mine.number = crate::theme::ButtonColors::grid(
        crate::theme::StateColors::new(rgba("#111111FF"), rgba("#222222FF"), rgba("#333333FF")),
        crate::theme::StateColors::flat(rgba("#EEEEEEFF")),
        crate::theme::StateColors::flat(rgba("#999999FF")),
    );
    let mut cfg = Config {
        theme_kind: ThemeKind::Barbie,
        ..Config::default()
    };
    cfg.themes = toml_themes(&[mine.clone()]);
    cfg.save_at(&path).expect("save");

    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.theme(), mine);
    assert_eq!(back.theme_display_name(ThemeKind::Barbie), "Not Barbie");
    // And the file names the build that wrote it.
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(
        body.starts_with(&format!("version = \"{CONFIG_VERSION}\"")),
        "{body}"
    );
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
fn a_theme_name_the_build_does_not_know_falls_back() {
    // A file naming the `Custom` palette earlier versions had — or
    // any other name this build does not ship — loads with the
    // default theme rather than failing and taking every other
    // setting down with it.
    let cfg: Config =
        toml::from_str("significant_digits = 9\ntheme_kind = \"Custom\"\n").expect("load");
    assert_eq!(cfg.theme_kind, ThemeKind::default());
    assert_eq!(cfg.significant_digits, 9);
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
// Theme storage format
// =====================================================================

#[test]
fn the_file_names_the_palette_in_force_and_carries_them_all() {
    let toml = toml::to_string_pretty(&Config::default()).expect("serialise");
    assert!(
        toml.contains(r#"theme_kind = "Cosmic""#),
        "expected the palette's name:\n{toml}"
    );
    // And the palettes themselves, in full, so any of them can be
    // retuned by hand.
    for kind in ThemeKind::ALL {
        assert!(
            toml.contains(&format!("id = \"{}\"", kind.key())),
            "{kind:?}"
        );
    }
    assert!(toml.contains("app_bg"), "{toml}");
    assert!(toml.contains("[themes.number]"), "{toml}");
    assert!(
        toml.contains(r#"display_name = "HighContrast Dark""#),
        "{toml}"
    );
}

#[test]
fn the_file_records_the_version_that_wrote_it() {
    let toml = toml::to_string_pretty(&Config::default()).expect("serialise");
    assert!(
        toml.contains(&format!("version = \"{CONFIG_VERSION}\"")),
        "{toml}"
    );

    // Whatever an older file says, a load stamps this build's version
    // on it: the value on disk always names the build that wrote it.
    let mut cfg: Config = toml::from_str("version = \"0.0.1\"\n").expect("load");
    assert_eq!(cfg.version, "0.0.1");
    cfg.validate_and_clamp();
    assert_eq!(cfg.version, CONFIG_VERSION);

    // And a file that spells it as something other than a string is
    // still a file: nothing about a version costs the user the rest
    // of their settings.
    let cfg: Config = toml::from_str("version = 3\nsignificant_digits = 9\n").expect("load");
    assert_eq!(cfg.significant_digits, 9);
}

#[test]
fn a_file_that_still_carries_a_palette_loads_without_it() {
    // Older versions wrote the whole palette into the file and let it
    // be hand-edited. Those files must keep loading — the section is
    // simply ignored, and the next save leaves it out.
    let raw = r##"
        significant_digits = 9
        theme_kind = "CupertinoDark"

        [theme]
        name = "Custom"
        app_bg = { r = 27, g = 27, b = 27, a = 255 }
        sidepanel_bg = "#272727FF"
        text_active = "#E7E7E7FF"
    "##;
    let mut cfg: Config = toml::from_str(raw).expect("an old file must still load");
    cfg.validate_and_clamp();
    assert_eq!(cfg.significant_digits, 9);
    assert_eq!(cfg.theme_kind, ThemeKind::CupertinoDark);
    // The old single-palette section is not the new list, so it is
    // ignored and the palette is the shipped one until the user
    // retunes it in `themes`.
    assert_eq!(cfg.theme().app_bg, rgba("#283133FF"));
    let rewritten = toml::to_string_pretty(&cfg).expect("serialise");
    assert!(!rewritten.contains("[theme]\n"), "{rewritten}");
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

#[test]
fn font_weights_name_the_faces_a_font_ships() {
    use crate::config::FontWeight;

    // The nine steps, lightest first, at the numbers a face carries.
    assert_eq!(FontWeight::ALL.len(), 9);
    assert_eq!(FontWeight::default(), FontWeight::Regular);
    assert_eq!(FontWeight::Regular.value(), 400);
    assert_eq!(FontWeight::Black.value(), 900);
    let mut sorted = FontWeight::ALL;
    sorted.sort();
    assert_eq!(sorted, FontWeight::ALL);

    // A face is free to carry any number in the range — a variable
    // font's instances often do — and lands on the step nearest it.
    assert_eq!(FontWeight::nearest(400), FontWeight::Regular);
    assert_eq!(FontWeight::nearest(430), FontWeight::Regular);
    assert_eq!(FontWeight::nearest(560), FontWeight::SemiBold);
    assert_eq!(FontWeight::nearest(0), FontWeight::Thin);
    assert_eq!(FontWeight::nearest(2000), FontWeight::Black);
    // A tie goes to the lighter step.
    assert_eq!(FontWeight::nearest(450), FontWeight::Regular);
}

#[test]
fn the_font_weight_round_trips_through_the_file() {
    use crate::config::FontWeight;

    let path = scratch_path("font-weight");
    let written = Config {
        font_weight: FontWeight::SemiBold,
        ..Config::default()
    };
    written.save_at(&path).expect("save");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.font_weight, FontWeight::SemiBold);
    // Spelled readably in the file rather than as a number.
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(body.contains("font_weight = \"semi_bold\""), "{body}");

    // And a file written before the field existed still loads, at the
    // weight a font ships as its own.
    std::fs::write(&path, "font = \"Adwaita Sans\"\n").expect("write");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.font_weight, FontWeight::Regular);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
