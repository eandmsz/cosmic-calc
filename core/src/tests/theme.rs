use crate::color::rgba;
use crate::theme::*;

#[test]
fn cosmic_preset_has_expected_colours() {
    let t = ThemeKind::Cosmic.get();
    assert_eq!(t.app_bg, rgba("#1B1B1BFF"));
    assert_eq!(t.basicop.normal.background, rgba("#61CDDCFF"));
}

#[test]
fn a_key_spreads_its_label_and_border_over_all_three_states() {
    // What the palette tables are written in, pinned: the three
    // fills land in the order they are named, and the one label and
    // the one border go on all three of them. Swap two of the fills
    // and every shipped palette would hover the wrong way round with
    // nothing else to notice it.
    let key = KeyColors {
        fill: rgba("#111111FF"),
        fill_hover: rgba("#222222FF"),
        fill_pressed: rgba("#333333FF"),
        label: rgba("#444444FF"),
        border: rgba("#555555FF"),
    };
    let c = ButtonColors::spread(key);
    assert_eq!(c.normal.background, key.fill);
    assert_eq!(c.hover.background, key.fill_hover);
    assert_eq!(c.pressed.background, key.fill_pressed);
    for face in [c.normal, c.hover, c.pressed] {
        assert_eq!(face.text, key.label);
        assert_eq!(face.border, key.border);
    }

    // And it is the long form spelled out, not a separate rule.
    assert_eq!(
        c,
        ButtonColors::new(
            ButtonFace::new(key.fill, key.label, key.border),
            ButtonFace::new(key.fill_hover, key.label, key.border),
            ButtonFace::new(key.fill_pressed, key.label, key.border),
        )
    );
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
        let name = t.name.clone();
        assert!(!name.is_empty());
        for (group, colors) in groups(&t) {
            let where_ = format!("{name}/{group}");
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
fn the_delete_keys_start_out_looking_like_the_top_row() {
    // `AC`/`C` and backspace have a group of their own so a theme can
    // mark them, but no shipped theme does yet, so nothing about the
    // window changes for having split them out.
    for kind in ThemeKind::ALL {
        let t = kind.get();
        assert_eq!(t.delete, t.toprow, "{}", t.name);
    }
}

#[test]
fn every_preset_ships_without_borders() {
    // A border is opt-in per theme, so the shipped look is unchanged
    // — and a thickness of zero is no border at all whatever height
    // it is asked about.
    for kind in ThemeKind::ALL {
        let t = kind.get();
        assert_eq!(t.button_border_thickness, 0.0, "{}", t.name);
        assert_eq!(t.border_width(80.0), 0.0, "{}", t.name);
    }
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
    let names: Vec<_> = ThemeKind::all().iter().map(|k| k.display_name()).collect();
    assert_eq!(names[0], "Cupertino Dark");
    assert_eq!(names[6], "Cosmic");
    assert_eq!(names[7], "Texas");
    assert_eq!(names[names.len() - 1], "Flat Green Light");
    assert_eq!(names.len(), ThemeKind::ALL.len());
    // Each name is its own, and each is the name the palette carries.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len());
    for kind in ThemeKind::ALL {
        assert_eq!(kind.get().name, kind.display_name());
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

    assert_eq!(t.science, component.colors());
    assert_eq!(t.delete, component.colors());
    assert_eq!(t.number, surface.colors());
    assert_eq!(t.equals, accent.colors());
    // Nothing is invented: each state is the colour the desktop
    // published for it.
    assert_eq!(t.equals.hover.background, accent.hover);
    assert_eq!(t.equals.pressed.background, accent.pressed);
    assert_eq!(t.equals.normal.text, accent.text);
}

/// Every button group of a palette, named, so a test can walk them.
fn groups(t: &Theme) -> [(&'static str, ButtonColors); 9] {
    [
        ("science", t.science),
        ("second", t.second),
        ("toprow", t.toprow),
        ("delete", t.delete),
        ("basicop", t.basicop),
        ("equals", t.equals),
        ("negate", t.negate),
        ("decimal", t.decimal),
        ("number", t.number),
    ]
}
