use crate::color::rgba;
use crate::theme::*;

#[test]
fn cosmic_preset_has_expected_colours() {
    let t = ThemeKind::Cosmic.get();
    assert_eq!(t.app_bg, rgba("#1B1B1BFF"));
    assert_eq!(t.basicop.normal.background, rgba("#61CDDCFF"));
}

#[test]
fn a_group_can_still_carry_nine_colours_of_its_own() {
    // The grid the tables are written in must not become the limit of
    // what a palette can say. Nine distinct colours — a fill, a label
    // and a border for each of the three states — go in, and all nine
    // come back out, so a theme that wants its label to change under
    // the pointer or its outline to darken on a press can have it.
    let group = ButtonColors::grid(
        //               resting            hover              pressed
        StateColors::new(rgba("#010101FF"), rgba("#040404FF"), rgba("#070707FF")), // fill
        StateColors::new(rgba("#020202FF"), rgba("#050505FF"), rgba("#080808FF")), // label
        StateColors::new(rgba("#030303FF"), rgba("#060606FF"), rgba("#090909FF")), // border
    );
    let seen = [
        group.normal.background,
        group.normal.text,
        group.normal.border,
        group.hover.background,
        group.hover.text,
        group.hover.border,
        group.pressed.background,
        group.pressed.text,
        group.pressed.border,
    ];
    let wanted = [
        "#010101FF",
        "#020202FF",
        "#030303FF",
        "#040404FF",
        "#050505FF",
        "#060606FF",
        "#070707FF",
        "#080808FF",
        "#090909FF",
    ];
    for (got, want) in seen.iter().zip(wanted) {
        assert_eq!(*got, rgba(want));
    }

    // And it survives being put in a palette, which is where a hand
    // -written theme would put it.
    let mut t = ThemeKind::Cosmic.get();
    t.science = group;
    assert_eq!(t.science.hover.text, rgba("#050505FF"));
    assert_ne!(t.science.normal.text, t.science.pressed.text);
    assert_ne!(t.science.normal.border, t.science.hover.border);
}

#[test]
fn a_grid_reads_the_way_the_tables_are_written() {
    // Rows are fill, label, border; columns are resting, hover,
    // pressed. Transpose the two and every shipped palette would
    // hover the wrong way round with nothing else to notice it.
    let fill = StateColors::new(rgba("#111111FF"), rgba("#222222FF"), rgba("#333333FF"));
    let label = StateColors::flat(rgba("#444444FF"));
    let border = StateColors::flat(rgba("#555555FF"));
    let c = ButtonColors::grid(fill, label, border);

    assert_eq!(c.normal.background, fill.resting);
    assert_eq!(c.hover.background, fill.hover);
    assert_eq!(c.pressed.background, fill.pressed);
    for face in [c.normal, c.hover, c.pressed] {
        assert_eq!(face.text, label.resting);
        assert_eq!(face.border, border.resting);
    }

    // And the rows read back out as they went in, which is what the
    // config file is written from.
    assert_eq!(c.fill_row(), fill);
    assert_eq!(c.label_row(), label);
    assert_eq!(c.border_row(), border);
}

#[test]
fn every_preset_spells_out_all_three_states() {
    // The point of the table is that nothing is derived, so every
    // group has to carry a colour of its own for each state rather
    // than leaving one to be worked out. A palette built from the old
    // formulas moved on hover and on press; one that does not is a
    // group somebody forgot to fill in.
    for kind in ThemeKind::ALL {
        let t = kind.get();
        assert!(!t.display_name.is_empty());
        for (group, colors) in groups(&t) {
            let where_ = format!("{}/{group}", t.display_name);
            // Redmond Light's white keys are the one exception, and
            // they are that way in the tables because they were that
            // way before them: the old hover formula lifted a colour
            // toward white, and a key already white had nowhere to go.
            if colors.normal.background != rgba("#FFFFFFFF") {
                assert_ne!(
                    colors.normal.background, colors.hover.background,
                    "{where_} does not answer the pointer"
                );
            }
            assert_ne!(
                colors.normal.background, colors.pressed.background,
                "{where_} does not answer a press"
            );
            // Every face is opaque in the shipped themes; the alpha
            // channel is there for a theme that wants to use it.
            for face in [colors.normal, colors.hover, colors.pressed] {
                assert_eq!(face.background.a, 0xFF, "{where_}");
                assert_eq!(face.text.a, 0xFF, "{where_}");
            }
        }
    }
}

#[test]
fn the_new_slots_start_out_looking_like_the_ones_they_left() {
    // Splitting a category out of another gives a theme somewhere to
    // mark those keys; until one does, nothing on screen may move.
    // `AC`/`C` and backspace left the top row, the brackets left it
    // too, and percent, `1/x`, `rand` and the trig keys left science.
    for kind in ThemeKind::ALL {
        let t = kind.get();
        let name = &t.display_name;
        assert_eq!(t.delete, t.toprow, "{name}");
        assert_eq!(t.bracket, t.toprow, "{name}");
        for (group, colors) in [
            ("percent", t.percent),
            ("reciprocal", t.reciprocal),
            ("trig", t.trig),
            ("rand", t.rand),
        ] {
            assert_eq!(colors, t.science, "{name}/{group}");
        }
    }
}

#[test]
fn every_preset_asks_for_a_border_the_renderer_can_draw() {
    // A border is opt-in per theme. Most palettes leave it at zero,
    // and zero is no border at all whatever height it is asked
    // about; the ones that do ask get a whole pixel of it at the
    // sizes a button is actually drawn at, however thin the setting.
    // No palette may ask for one so heavy it swallows the label.
    let mut with_a_border = 0;
    for kind in ThemeKind::ALL {
        let t = kind.get();
        let name = &t.display_name;
        assert!(
            (0.0..=MAX_BORDER_THICKNESS).contains(&t.button_border_thickness),
            "{name} asks for {}",
            t.button_border_thickness
        );
        if t.button_border_thickness == 0.0 {
            assert_eq!(t.border_width(80.0), 0.0, "{name}");
            continue;
        }
        with_a_border += 1;
        for height in [20.0, 80.0, 300.0] {
            let w = t.border_width(height);
            assert!(w >= 1.0, "{name} at {height} gave {w}");
            assert!(w <= height * MAX_BORDER_THICKNESS / 100.0, "{name} {w}");
        }
    }
    // Cupertino Dark and Cyberpunk carry one; the rest do not. A
    // count rather than a list, so turning one on or off in a
    // palette is a one-line change here rather than a hunt.
    assert_eq!(with_a_border, 2);
}

#[test]
fn a_border_is_a_whole_pixel_that_follows_the_button() {
    let mut t = ThemeKind::Cosmic.get();
    t.button_border_thickness = 4.0;
    // Four per cent of the button, rounded to a pixel it can be drawn
    // in: bigger buttons wear a proportionally bigger outline, and
    // every width is whole so the line stays crisp rather than
    // smearing across two pixels.
    assert_eq!(t.border_width(100.0), 4.0);
    assert_eq!(t.border_width(50.0), 2.0);
    assert_eq!(t.border_width(30.0), 1.0);
    for height in 1..400 {
        let w = t.border_width(height as f32);
        assert_eq!(w, w.round(), "{height} gave {w}");
    }

    // A theme that asks for a border always gets at least a pixel of
    // it, however small the button.
    t.button_border_thickness = 0.5;
    assert_eq!(t.border_width(20.0), 1.0);

    // And no thickness can swallow the label.
    t.button_border_thickness = 500.0;
    assert_eq!(t.border_width(80.0), 80.0 * MAX_BORDER_THICKNESS / 100.0);
}

#[test]
fn all_presets_enumerate_in_order() {
    let names: Vec<_> = ThemeKind::ALL
        .iter()
        .map(|k| k.get().display_name)
        .collect();
    assert_eq!(names[0], "Cupertino Dark");
    // The two-word spelling: the palette is "HighContrast", light or
    // dark, rather than a contrast that is high.
    assert_eq!(names[4], "HighContrast Dark");
    assert_eq!(names[5], "HighContrast Light");
    assert_eq!(names[6], "Cosmic");
    assert_eq!(names[names.len() - 1], "Flat Green Light");
    assert_eq!(names.len(), ThemeKind::ALL.len());
    // Each name is its own, so no two buttons in the settings panel
    // read the same.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len());
    // And each palette knows which preset it is.
    for kind in ThemeKind::ALL {
        assert_eq!(kind.get().id, kind);
    }
}

#[test]
fn a_fresh_config_starts_on_cosmic() {
    // Where a palette sits in the settings list and which one a fresh
    // `config.toml` starts on are separate questions: reordering the
    // enum moves the first, and must not quietly move the second.
    assert_eq!(ThemeKind::default(), ThemeKind::Cosmic);
}

#[test]
fn a_theme_is_stored_and_read_back_by_name() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrap {
        kind: ThemeKind,
    }
    for kind in ThemeKind::ALL {
        let s = toml::to_string(&Wrap { kind }).unwrap();
        assert!(s.contains(kind.key()), "{s}");
        let back: Wrap = toml::from_str(&s).unwrap();
        assert_eq!(back.kind, kind);
    }
}

#[test]
fn a_theme_the_build_does_not_know_falls_back() {
    #[derive(serde::Deserialize)]
    struct Wrap {
        kind: ThemeKind,
    }
    // `Custom` is the palette earlier versions let a user hand-edit,
    // and a file that still names it has to keep loading — with every
    // other setting in it intact — rather than failing the parse.
    let back: Wrap = toml::from_str(r#"kind = "Custom""#).unwrap();
    assert_eq!(back.kind, ThemeKind::default());
    let back: Wrap = toml::from_str(r#"kind = "no such theme""#).unwrap();
    assert_eq!(back.kind, ThemeKind::default());
    assert_eq!(ThemeKind::from_key("Custom"), None);
    assert_eq!(ThemeKind::from_key("Cosmic"), Some(ThemeKind::Cosmic));
}

// ---------------------------------------------------------------------
// The `config.toml` theme table
// ---------------------------------------------------------------------

/// Deserialize a `themes` list on its own, the way `Config` carries it.
fn table_from(toml_src: &str) -> ThemeTable {
    #[derive(Default, serde::Deserialize)]
    #[serde(default)]
    struct Wrap {
        themes: ThemeTable,
    }
    toml::from_str::<Wrap>(toml_src)
        .expect("a themes list must never fail a load")
        .themes
}

#[test]
fn the_table_round_trips_every_palette() {
    #[derive(serde::Serialize)]
    struct Wrap {
        themes: ThemeTable,
    }
    let table = ThemeTable::default();
    let text = toml::to_string_pretty(&Wrap {
        themes: table.clone(),
    })
    .unwrap();
    assert_eq!(table_from(&text), table);
}

#[test]
fn what_the_file_says_is_what_gets_painted() {
    // The whole point of carrying the palettes in the file: an edit
    // there reaches the window without a rebuild.
    let table = table_from(
        r##"
        [[themes]]
        id = "Texas"
        display_name = "My Texas"
        app_bg = "#010203FF"
        button_border_thickness = 3.5

        [themes.number]
        fill = ["#111111FF", "#222222FF", "#333333FF"]
        label = ["#444444FF", "#555555FF", "#666666FF"]
        "##,
    );
    let t = table.get(ThemeKind::Texas);
    assert_eq!(t.display_name, "My Texas");
    assert_eq!(table.display_name(ThemeKind::Texas), "My Texas");
    assert_eq!(t.app_bg, rgba("#010203FF"));
    assert_eq!(t.button_border_thickness, 3.5);
    assert_eq!(t.number.fill_row().hover, rgba("#222222FF"));
    assert_eq!(t.number.label_row().pressed, rgba("#666666FF"));
    // A row the file left out keeps the shipped colours.
    assert_eq!(
        t.number.border_row(),
        ThemeKind::Texas.get().number.border_row()
    );
    // And a palette the file did not mention is the shipped one.
    assert_eq!(table.get(ThemeKind::Tokyo), ThemeKind::Tokyo.get());
}

#[test]
fn nothing_a_file_can_say_costs_the_user_their_settings() {
    // Every value here is wrong in a different way — a colour that is
    // not hex, a colour that is a number, a row that is a string, a
    // row with too few entries, a thickness that is text, a name that
    // is a table. None of them may fail the load, and each falls back
    // to the shipped value on its own rather than taking the rest of
    // the palette with it.
    let table = table_from(
        r##"
        [[themes]]
        id = "Tokyo"
        display_name = "Tokyo Nights"
        app_bg = "not a colour"
        display_bg = 42
        accent = "#00FF00"
        button_border_thickness = "thick"

        [themes.science]
        fill = true
        label = ["#ABCDEFFF"]
        border = ["#GGGGGGGG", "#123456FF", true]
        "##,
    );
    let t = table.get(ThemeKind::Tokyo);
    let shipped = ThemeKind::Tokyo.get();
    assert_eq!(t.display_name, "Tokyo Nights");
    assert_eq!(t.app_bg, shipped.app_bg);
    assert_eq!(t.display_bg, shipped.display_bg);
    // Six digits is a colour; the alpha channel defaults to opaque.
    assert_eq!(t.accent, rgba("#00FF00FF"));
    assert_eq!(t.button_border_thickness, shipped.button_border_thickness);
    // A row that is neither a line nor a list keeps all three
    // shipped colours...
    assert_eq!(t.science.fill_row(), shipped.science.fill_row());
    // ...a short one fills in only the slots it gave...
    assert_eq!(t.science.label_row().resting, rgba("#ABCDEFFF"));
    assert_eq!(
        t.science.label_row().hover,
        shipped.science.label_row().hover
    );
    // ...and a list with unusable entries repairs those slots alone.
    assert_eq!(
        t.science.border_row().resting,
        shipped.science.border_row().resting
    );
    assert_eq!(t.science.border_row().hover, rgba("#123456FF"));
    assert_eq!(
        t.science.border_row().pressed,
        shipped.science.border_row().pressed
    );
}

#[test]
fn a_table_always_comes_out_whole_and_in_order() {
    // A palette named twice keeps its first entry, one the build does
    // not have is dropped, and every one the file left out is added
    // back — so the rest of the app can count on all nineteen being
    // there, in the order the settings panel offers them.
    let table = table_from(
        r##"
        [[themes]]
        id = "Barbie"
        display_name = "First"

        [[themes]]
        id = "Barbie"
        display_name = "Second"

        [[themes]]
        id = "Custom"

        [[themes]]
        display_name = "no id at all"
        "##,
    );
    let ids: Vec<_> = ThemeKind::ALL.iter().map(|k| table.get(*k).id).collect();
    assert_eq!(ids, ThemeKind::ALL.to_vec());
    assert_eq!(table.display_name(ThemeKind::Barbie), "First");
    assert_eq!(table, {
        let mut expected = table.clone();
        expected.normalize();
        expected
    });
}

#[test]
fn a_name_that_would_break_its_button_is_repaired() {
    // The name is drawn on a button and nowhere else, so what it must
    // survive is a layout: no control characters, no invisible
    // formatting codepoints that let it render as something other
    // than what it says, and no length that would take the panel
    // over.
    let table = table_from(
        r##"
        [[themes]]
        id = "Texas"
        display_name = "  Lone\tStar\n  "

        [[themes]]
        id = "Tokyo"
        display_name = "Tok\u202Eyo\u200B"

        [[themes]]
        id = "Barbie"
        display_name = "   "

        [[themes]]
        id = "Plastic"
        display_name = "PlasticPlasticPlasticPlasticPlasticPlastic"
        "##,
    );
    assert_eq!(table.display_name(ThemeKind::Texas), "LoneStar");
    assert_eq!(table.display_name(ThemeKind::Tokyo), "Tokyo");
    // Nothing usable left, so the shipped name stands.
    assert_eq!(
        table.display_name(ThemeKind::Barbie),
        ThemeKind::Barbie.get().display_name
    );
    assert_eq!(
        table.display_name(ThemeKind::Plastic).chars().count(),
        MAX_DISPLAY_NAME_LEN
    );
}

#[test]
fn a_border_the_file_asks_too_much_of_is_clamped() {
    let table = table_from(
        r##"
        [[themes]]
        id = "Texas"
        button_border_thickness = 900.0

        [[themes]]
        id = "Tokyo"
        button_border_thickness = -4.0

        [[themes]]
        id = "Barbie"
        button_border_thickness = 2
        "##,
    );
    assert_eq!(
        table.get(ThemeKind::Texas).button_border_thickness,
        MAX_BORDER_THICKNESS
    );
    assert_eq!(table.get(ThemeKind::Tokyo).button_border_thickness, 0.0);
    // An integer is a thickness too — TOML tells the two apart and
    // the user should not have to.
    assert_eq!(table.get(ThemeKind::Barbie).button_border_thickness, 2.0);
}

#[test]
fn the_cosmic_override_takes_the_desktop_at_its_word() {
    // Every colour comes from the desktop's own component tables, so
    // the keys hover the way the rest of the desktop hovers and an
    // accent key wears the accent's own text colour — which is where
    // the contrast used to go, with the window's text lifted onto a
    // bright fill it had nothing to spare against.
    let component = CosmicComponent {
        base: rgba("#505050FF"),
        hover: rgba("#606060FF"),
        pressed: rgba("#404040FF"),
        text: rgba("#FFFFFFFF"),
        border: rgba("#707070FF"),
    };
    let surface = CosmicComponent {
        base: rgba("#303030FF"),
        hover: rgba("#3A3A3AFF"),
        pressed: rgba("#282828FF"),
        text: rgba("#FFFFFFFF"),
        border: rgba("#404040FF"),
    };
    let accent = CosmicComponent {
        base: rgba("#00FF00FF"),
        hover: rgba("#40FF40FF"),
        pressed: rgba("#00C000FF"),
        text: rgba("#000000FF"),
        border: rgba("#00FF00FF"),
    };
    let over = CosmicOverride {
        window_bg: rgba("#101010FF"),
        container_bg: rgba("#202020FF"),
        interface_text: rgba("#FFFFFFFF"),
        interface_text_dim: rgba("#FFFFFF80"),
        component,
        surface_component: surface,
        accent,
    };
    let t = apply_cosmic_override(ThemeKind::Cosmic.get(), over);

    assert_eq!(t.app_bg, over.window_bg);
    assert_eq!(t.display_bg, over.window_bg);
    assert_eq!(t.sidepanel_bg, over.container_bg);
    assert_eq!(t.text_inactive, over.interface_text_dim);
    // The switches and sliders take the desktop's accent.
    assert_eq!(t.accent, accent.base);
    // The palette it overlays is still the one it started from.
    assert_eq!(t.id, ThemeKind::Cosmic);
    assert_eq!(t.display_name, ThemeKind::Cosmic.get().display_name);

    assert_eq!(t.science, component.colors());
    assert_eq!(t.delete, component.colors());
    assert_eq!(t.trig, component.colors());
    assert_eq!(t.bracket, component.colors());
    assert_eq!(t.number, surface.colors());
    assert_eq!(t.equals, accent.colors());
    // Nothing is invented: each state is the colour the desktop
    // published for it.
    assert_eq!(t.equals.hover.background, accent.hover);
    assert_eq!(t.equals.pressed.background, accent.pressed);
    assert_eq!(t.equals.normal.text, accent.text);
    // Every group the window draws is covered by the overlay, so no
    // key is left wearing a colour from the preset underneath.
    for (group, colors) in groups(&t) {
        assert!(
            [component.colors(), surface.colors(), accent.colors()].contains(&colors),
            "{group} was left behind by the overlay"
        );
    }
}

/// Every button group of a palette, named, so a test can walk them.
fn groups(t: &Theme) -> [(&'static str, ButtonColors); 14] {
    [
        ("science", t.science),
        ("second", t.second),
        ("toprow", t.toprow),
        ("delete", t.delete),
        ("bracket", t.bracket),
        ("basicop", t.basicop),
        ("equals", t.equals),
        ("percent", t.percent),
        ("reciprocal", t.reciprocal),
        ("trig", t.trig),
        ("rand", t.rand),
        ("negate", t.negate),
        ("decimal", t.decimal),
        ("number", t.number),
    ]
}
