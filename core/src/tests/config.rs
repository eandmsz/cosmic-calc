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
    // The font is the palette's now, and Cosmic Desktop — the palette
    // a fresh file opens on — asks for Open Sans.
    assert_eq!(c.font(), "Open Sans");
    assert_eq!(c.font(), DEFAULT_FONT);
    assert_eq!(c.font_weight(), FontWeight::Regular);
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
    // Offered most rounded first, so the row reads down a scale
    // rather than jumping about it.
    let names: Vec<_> = ButtonShape::ALL.iter().map(|s| s.display_name()).collect();
    assert_eq!(names, ["System", "50%", "25%", "10%", "0%"]);

    // The stored pair is a separate thing: the buttons outside the
    // keypad have no height to scale against and take a fixed radius.
    // `System` has none of its own — that is what defers to the
    // desktop.
    assert_eq!(ButtonShape::Auto.resolved(), None);
    assert_eq!(ButtonShape::Square.resolved(), Some((0.0, 1.0)));
    let radius = |shape: ButtonShape| shape.resolved().unwrap().0;
    assert!(radius(ButtonShape::Round) > radius(ButtonShape::SlightlyRound));
    assert!(radius(ButtonShape::SlightlyRound) > radius(ButtonShape::BarelyRound));
    assert!(radius(ButtonShape::BarelyRound) > radius(ButtonShape::Square));
    // Every one of them keeps the gap at a quarter of the corner, so
    // a keypad's spacing tracks its shape.
    for shape in [
        ButtonShape::Round,
        ButtonShape::SlightlyRound,
        ButtonShape::BarelyRound,
    ] {
        let (r, spacing) = shape.resolved().unwrap();
        assert!((spacing - r * 0.25).abs() < 1e-6, "{shape:?}");
    }

    // The file records the same name the panel offers, so a shape in
    // `config.toml` says what the keypad is drawn like rather than
    // naming a preset the reader has to look up.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrap {
        shape: ButtonShape,
    }
    for shape in ButtonShape::ALL {
        let toml = toml::to_string(&Wrap { shape }).unwrap();
        assert!(
            toml.contains(&format!("shape = \"{}\"", shape.key())),
            "{toml}"
        );
        let back: Wrap = toml::from_str(&toml).unwrap();
        assert_eq!(back.shape, shape, "{toml}");
    }
    let keys: Vec<_> = ButtonShape::ALL.iter().map(|s| s.key()).collect();
    assert_eq!(keys, ["system", "50%", "25%", "10%", "0%"]);

    // Case and stray space in a hand-edited file are not the point.
    for (written, shape) in [(" 50% ", ButtonShape::Round), ("System", ButtonShape::Auto)] {
        assert_eq!(ButtonShape::from_key(written), Some(shape), "{written}");
    }

    // The names earlier versions wrote still read, so upgrading keeps
    // the corner the user chose.
    for (legacy, shape) in [
        ("auto", ButtonShape::Auto),
        ("round", ButtonShape::Round),
        ("slightlyround", ButtonShape::SlightlyRound),
        ("barelyround", ButtonShape::BarelyRound),
        ("square", ButtonShape::Square),
    ] {
        let back: Wrap = toml::from_str(&format!(r#"shape = "{legacy}""#)).unwrap();
        assert_eq!(back.shape, shape, "{legacy}");
    }

    // And the five are the whole vocabulary. A percentage looks like
    // a number the user could pick from and is not: anything but the
    // five is refused here, which is what leaves the shipped shape in
    // place rather than a corner this build cannot draw.
    for invented in ["37%", "60%", "12", "pill", ""] {
        assert!(
            ButtonShape::from_key(invented).is_none(),
            "{invented} was taken for a shape"
        );
        assert!(
            toml::from_str::<Wrap>(&format!(r#"shape = "{invented}""#)).is_err(),
            "{invented} was taken for a shape"
        );
    }
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
        ..Config::default()
    };
    c.validate_and_clamp();
    assert_eq!(c.button_corner_radius, 0.0);
    assert_eq!(c.significant_digits, MAX_SIGNIFICANT_DIGITS);
    assert_eq!(c.window_startup_width, MIN_WINDOW_DIM);
    assert_eq!(c.window_startup_height, MAX_WINDOW_DIM);
    assert_eq!(c.rand_decimals, MAX_RAND_DECIMALS);

    // A family name is the palette's, and a blank one is repaired to
    // the family that palette ships with rather than left for the
    // renderer to guess at.
    c.set_font("   ".to_string());
    assert_eq!(c.font(), ThemeKind::Cosmic.get().font);
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
/// does rather than a constructor only it can reach. Each palette
/// goes under its own id, which is how the file keys them.
fn toml_themes(themes: &[crate::theme::Theme]) -> crate::theme::ThemeTable {
    #[derive(serde::Serialize)]
    struct Wrap {
        themes: toml::value::Table,
    }
    #[derive(serde::Deserialize)]
    struct Read {
        themes: crate::theme::ThemeTable,
    }
    let mut table = toml::value::Table::new();
    for theme in themes {
        table.insert(
            theme.id.key().to_string(),
            toml::Value::try_from(theme).expect("serialise"),
        );
    }
    let text = toml::to_string_pretty(&Wrap { themes: table }).expect("serialise");
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
    mine.button_border_percent = 2.5;
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
    // retuned by hand: one `themes` table, each palette a sub-table
    // under its own id, and every part of a palette named by that id
    // rather than by its place in a list.
    for kind in ThemeKind::ALL {
        assert!(
            toml.contains(&format!("[themes.{}]", kind.key())),
            "{kind:?}\n{toml}"
        );
        assert!(
            toml.contains(&format!("[themes.{}.number]", kind.key())),
            "{kind:?}\n{toml}"
        );
    }
    // The id is the key of the entry and is not repeated inside it.
    assert!(!toml.contains("id = "), "{toml}");
    assert!(toml.contains("app_bg"), "{toml}");
    assert!(
        toml.contains(r#"display_name = "High Contrast Dark""#),
        "{toml}"
    );
    // The border is a percentage of the button's height, and the key
    // says so.
    assert!(toml.contains("button_border_percent"), "{toml}");
    assert!(!toml.contains("button_border_thickness"), "{toml}");
}

#[test]
fn a_file_that_lists_its_palettes_the_old_way_is_rewritten() {
    // Earlier versions wrote the palettes as a `[[themes]]` array,
    // each entry naming itself with an `id` and spelling its border
    // as `button_border_thickness`. Such a file has to load with what
    // its user tuned in it intact, and the save that follows writes
    // the table form — the migration is a start of the app, not a
    // hand-edit.
    let path = scratch_path("legacy-theme-list");
    std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    std::fs::write(
        &path,
        r##"
version = "0.2.2"
theme_kind = "Barbie"

[[themes]]
id = "Barbie"
display_name = "Not Barbie"
app_bg = "#0F0E0DFF"
button_border_thickness = 2.5

[themes.number]
fill = "#111111FF #222222FF #333333FF"
"##,
    )
    .expect("write");

    let cfg = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(cfg.theme().display_name, "Not Barbie");
    assert_eq!(cfg.theme().app_bg, rgba("#0F0E0DFF"));
    assert_eq!(cfg.theme().button_border_percent, 2.5);
    assert_eq!(cfg.theme().number.fill_row().hover, rgba("#222222FF"));

    cfg.save_at(&path).expect("save");
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(body.contains("[themes.Barbie]"), "{body}");
    assert!(body.contains("[themes.Barbie.number]"), "{body}");
    assert!(!body.contains("[[themes]]"), "{body}");
    assert!(!body.contains("button_border_thickness"), "{body}");

    // And it reads back as what it was.
    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.theme(), cfg.theme());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
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
fn each_toggle_starts_where_a_fresh_window_should_open() {
    let c = Config::default();
    // The memory register is off: empty until something is stored, it
    // would otherwise open as a row of the display saying `Memory:`.
    assert!(!c.show_memory);
    // The window size has always been remembered.
    assert!(c.save_window_size);
    // The history never has, so keeping it is the user's to ask for.
    assert!(!c.save_history);
    assert!(c.history.is_empty());
    // With both halves off there is no row under the display at all,
    // and either one alone is reason enough to draw it.
    assert!(!c.status_row_visible());
    assert!(Config {
        show_memory: true,
        ..Config::default()
    }
    .status_row_visible());
    assert!(Config {
        property_testing: true,
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
    let path = scratch_path("font-weight");
    let mut written = Config::default();
    written.set_font_weight(FontWeight::SemiBold);
    written.save_at(&path).expect("save");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.font_weight(), FontWeight::SemiBold);
    // Spelled readably in the file rather than as a number, and
    // inside the palette it belongs to.
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(body.contains("font_weight = \"semi_bold\""), "{body}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn the_font_belongs_to_the_palette_that_is_on_screen() {
    // Picking a family changes the palette being looked at and leaves
    // the other eighteen exactly as they were, so switching palettes
    // brings the family that palette was drawn for back with it.
    let mut c = Config {
        theme_kind: ThemeKind::Barbie,
        ..Config::default()
    };
    let cosmic_font = c.themes.font(ThemeKind::Cosmic).to_string();
    c.set_font("Trebuchet MS".to_string());
    c.set_font_weight(FontWeight::Bold);

    assert_eq!(c.font(), "Trebuchet MS");
    assert_eq!(c.font_weight(), FontWeight::Bold);
    assert_eq!(c.themes.font(ThemeKind::Cosmic), cosmic_font);
    assert_eq!(c.themes.font_weight(ThemeKind::Cosmic), FontWeight::Regular);

    c.theme_kind = ThemeKind::Cosmic;
    assert_eq!(c.font(), cosmic_font);

    // And both survive the file, under the palette they belong to.
    let path = scratch_path("theme-font");
    c.save_at(&path).expect("save");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.themes.font(ThemeKind::Barbie), "Trebuchet MS");
    assert_eq!(read.themes.font_weight(ThemeKind::Barbie), FontWeight::Bold);
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(body.contains("font = \"Trebuchet MS\""), "{body}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn the_button_shape_belongs_to_the_palette_that_is_on_screen() {
    // A corner is part of a look the way a colour or a face is, so
    // the shape travels with the palette: choosing one changes the
    // palette being looked at and leaves the other nineteen exactly
    // as they were.
    let mut c = Config {
        theme_kind: ThemeKind::Barbie,
        ..Config::default()
    };
    let cosmic_shape = c.themes.button_shape(ThemeKind::Cosmic);
    c.set_button_shape(ButtonShape::BarelyRound);

    assert_eq!(c.button_shape(), ButtonShape::BarelyRound);
    assert_eq!(c.themes.button_shape(ThemeKind::Cosmic), cosmic_shape);
    // And what the keypad is drawn to follows it.
    assert_eq!(
        c.effective_button_corner_radius(),
        ButtonShape::BarelyRound.resolved().unwrap().0
    );

    c.theme_kind = ThemeKind::Cosmic;
    assert_eq!(c.button_shape(), cosmic_shape);

    // It survives the file, under the palette it belongs to, and is
    // mirrored at the top of the file for the palette on screen.
    let path = scratch_path("theme-shape");
    c.theme_kind = ThemeKind::Barbie;
    c.save_at(&path).expect("save");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(
        read.themes.button_shape(ThemeKind::Barbie),
        ButtonShape::BarelyRound
    );
    assert_eq!(
        read.themes.button_shape(ThemeKind::Cosmic),
        ThemeKind::Cosmic.get().button_shape
    );
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(body.contains("button_shape = \"10%\""), "{body}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_file_written_before_the_shape_moved_keeps_the_shape_it_had() {
    // Until 0.2.7 the corner was one setting for the whole app, at
    // the top of the file, and a file that old has nothing else to
    // say about a shape. It is the shape that user was looking at, so
    // the palette they were looking at keeps it.
    let path = scratch_path("legacy-shape");
    std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    std::fs::write(&path, "theme_kind = \"Tokyo\"\nbutton_shape = \"round\"\n").expect("write");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.button_shape(), ButtonShape::Round);
    // Only that palette: the others keep the shape their preset
    // ships, which is the whole point of the move.
    assert_eq!(
        read.themes.button_shape(ThemeKind::Cosmic),
        ThemeKind::Cosmic.get().button_shape
    );

    read.save_at(&path).expect("save");
    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.button_shape(), ButtonShape::Round);

    // And a palette whose own entry names a shape keeps it: the key
    // at the top of the file is a mirror of what is on screen, not
    // an override of what the palette says.
    std::fs::write(
        &path,
        "theme_kind = \"Tokyo\"\nbutton_shape = \"round\"\n\n[themes.Tokyo]\nbutton_shape = \"square\"\n",
    )
    .expect("write");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.button_shape(), ButtonShape::Square);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_file_written_before_the_font_moved_keeps_the_font_it_had() {
    // Until 0.2.5 the family and the weight were one setting for the
    // whole app, at the top of the file, and a file that old has
    // nothing else to say about a face. It is the font that user was
    // looking at, so the palette they were looking at keeps it.
    let path = scratch_path("legacy-font");
    std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    std::fs::write(
        &path,
        "theme_kind = \"Tokyo\"\nfont = \"Trebuchet MS\"\nfont_weight = \"bold\"\n",
    )
    .expect("write");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.font(), "Trebuchet MS");
    assert_eq!(read.font_weight(), FontWeight::Bold);
    // Only that palette: the others keep the family their preset
    // ships, which is the whole point of the move.
    assert_eq!(
        read.themes.font(ThemeKind::Cosmic),
        ThemeKind::Cosmic.get().font
    );

    read.save_at(&path).expect("save");
    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.font(), "Trebuchet MS");
    assert_eq!(back.font_weight(), FontWeight::Bold);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn the_face_in_force_is_written_at_the_top_of_the_file() {
    // The face belongs to the palette and is carried in the palette's
    // own entry, but the file also says at the top which of the
    // twenty is on screen and what it is set in — so "what is this
    // window drawn in" is answered where `theme_kind` answers "which
    // palette", rather than in whichever of twenty tables happens to
    // be the live one.
    let path = scratch_path("face-in-force");
    let mut cfg = Config::load_or_create_default_at(&path).expect("create");
    let created = std::fs::read_to_string(&path).expect("read");
    let top_level = created.split("\n[").next().unwrap_or_default();
    assert!(top_level.contains("font = \"Open Sans\""), "{created}");
    assert!(top_level.contains("font_weight = \"regular\""), "{created}");

    // And it follows the settings panel: picking a family writes the
    // palette's entry and the pair at the top in the same save.
    cfg.set_font("Comfortaa".to_string());
    cfg.set_font_weight(FontWeight::Bold);
    cfg.save_at(&path).expect("save");
    let body = std::fs::read_to_string(&path).expect("read");
    let top_level = body.split("\n[").next().unwrap_or_default();
    assert!(top_level.contains("font = \"Comfortaa\""), "{body}");
    assert!(top_level.contains("font_weight = \"bold\""), "{body}");
    assert!(body.contains("[themes.Cosmic]"), "{body}");
    let back = Config::load_or_create_default_at(&path).expect("reload");
    assert_eq!(back.font(), "Comfortaa");
    assert_eq!(back.font_weight(), FontWeight::Bold);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_palette_that_names_its_own_face_outranks_the_pair_at_the_top() {
    // The pair at the top is a mirror of the palette in force, and a
    // stale one is what a hand-edit of the palette's own entry leaves
    // behind. The entry is where the face is edited, so it wins; the
    // mirror is rewritten from it on the next save.
    let path = scratch_path("face-hand-edit");
    std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    std::fs::write(
        &path,
        "theme_kind = \"Cosmic\"\nfont = \"Comfortaa\"\nfont_weight = \"bold\"\n\n\
         [themes.Cosmic]\nfont = \"Cantarell\"\nfont_weight = \"light\"\n",
    )
    .expect("write");
    let read = Config::load_or_create_default_at(&path).expect("load");
    assert_eq!(read.font(), "Cantarell");
    assert_eq!(read.font_weight(), FontWeight::Light);

    read.save_at(&path).expect("save");
    let body = std::fs::read_to_string(&path).expect("read");
    let top_level = body.split("\n[").next().unwrap_or_default();
    assert!(top_level.contains("font = \"Cantarell\""), "{body}");
    assert!(top_level.contains("font_weight = \"light\""), "{body}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn the_recommended_families_are_a_priority_order() {
    // The order is the fallback's: the first family on this list the
    // host actually has is the one a palette naming a font nobody
    // installed is drawn in.
    assert_eq!(RECOMMENDED_FONTS.first(), Some(&"SF Pro Display"));
    assert_eq!(RECOMMENDED_FONTS.last(), Some(&"zilverstone eYe/FS"));
    assert_eq!(RECOMMENDED_FONTS[2], "Adwaita Sans");

    // No repeats: a family listed twice would be a second chance at a
    // priority it already had.
    let mut seen = RECOMMENDED_FONTS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), before);

    assert!(is_recommended_font("Adwaita Sans"));
    assert!(!is_recommended_font("Nimbus Sans"));
    // The default is on the list, so a host that has it never has to
    // fall back past it.
    assert!(is_recommended_font(DEFAULT_FONT));
}

#[test]
fn every_palette_names_a_family_and_a_weight() {
    for kind in ThemeKind::ALL {
        let theme = kind.get();
        assert!(!theme.font.trim().is_empty(), "{}", theme.display_name);
        assert!(theme.font.len() <= MAX_FONT_NAME_LEN, "{}", theme.font);
        // A palette asks for a family the app would have reached for
        // on its own, so a host that has none of them falls back to a
        // family it does have rather than to another name it lacks.
        assert!(
            is_recommended_font(&theme.font),
            "{} asks for {}, which is not one of the recommended families",
            theme.display_name,
            theme.font
        );
    }
}
