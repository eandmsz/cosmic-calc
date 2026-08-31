//! Which family the window is really drawn in.
//!
//! Every test here reads the host's own font database, so each one
//! either holds on any machine or says which machine it is about: a
//! build host with three fonts installed and a desktop with six
//! hundred must both pass.

use crate::config::{
    is_recommended_font, Config, FontWeight, MAX_FONT_NAME_LEN, RECOMMENDED_FONTS,
};
use crate::ui::font::{
    available_fonts, group_weight, is_installed, recommended_fallback, resolved_font,
    resolved_weight, weights_for,
};

/// A family the host has whose name survives the config's own
/// sanitising unchanged, so a test can assert on the name it set.
fn an_installed_family() -> &'static str {
    available_fonts()
        .iter()
        .find(|name| name.trim() == name.as_str() && name.chars().count() <= MAX_FONT_NAME_LEN)
        .expect("the host has at least one family with an ordinary name")
}

#[test]
fn is_installed_agrees_with_the_family_list() {
    for name in available_fonts().iter().take(50) {
        assert!(is_installed(name), "{name}");
    }
    assert!(!is_installed("No Such Family At All"));
    assert!(!is_installed(""));
}

#[test]
fn a_family_the_host_has_is_drawn_as_the_palette_asks() {
    let installed = an_installed_family();
    let mut config = Config::default();
    config.set_font(installed.to_string());
    config.set_font_weight(FontWeight::Bold);
    assert_eq!(resolved_font(&config), (installed, FontWeight::Bold));
}

#[test]
fn a_family_the_host_lacks_is_drawn_in_a_recommended_one() {
    let mut config = Config::default();
    config.set_font("No Such Family At All".to_string());
    config.set_font_weight(FontWeight::Black);
    let (family, weight) = resolved_font(&config);

    match recommended_fallback() {
        Some(fallback) => {
            assert_eq!(family, fallback);
            assert!(is_installed(family));
            assert!(is_recommended_font(family));
            // A substitution is drawn at the default weight: a Black
            // picked for one face says nothing about how another
            // should be set.
            assert_eq!(weight, FontWeight::default());
        }
        None => {
            // A machine with none of the recommended families is one
            // no list here can second-guess, so the name stands and
            // the renderer substitutes.
            assert_eq!(family, "No Such Family At All");
            assert_eq!(weight, FontWeight::Black);
        }
    }

    // Either way the file keeps the family the palette names, so
    // installing that font later is all it takes to get it.
    assert_eq!(config.font(), "No Such Family At All");
    assert_eq!(config.font_weight(), FontWeight::Black);
}

#[test]
fn the_fallback_is_the_first_recommended_family_the_host_has() {
    let expected = RECOMMENDED_FONTS.into_iter().find(|f| is_installed(f));
    assert_eq!(recommended_fallback(), expected);

    // Everything ahead of it on the list really is absent — the order
    // is a priority, not a preference.
    if let Some(chosen) = recommended_fallback() {
        for family in RECOMMENDED_FONTS.into_iter().take_while(|f| *f != chosen) {
            assert!(!is_installed(family), "{family}");
        }
    }
}

#[test]
fn switching_palettes_switches_the_family() {
    // The font travels with the palette, so what is drawn changes
    // when the palette does — without touching the family any other
    // palette asks for.
    let installed = an_installed_family();
    let mut config = Config::default();
    let started_on = config.theme_kind;
    config.set_font(installed.to_string());

    let other = crate::theme::ThemeKind::ALL
        .into_iter()
        .find(|kind| *kind != started_on)
        .expect("more than one palette");
    config.theme_kind = other;
    assert_eq!(config.font(), other.get().font);

    config.theme_kind = started_on;
    assert_eq!(config.font(), installed);
}

#[test]
fn a_group_without_a_weight_of_its_own_takes_the_palettes() {
    // Which is what all but two of the two hundred and eighty shipped
    // groups say. Nothing is named, so nothing is set: those labels
    // are drawn in the interface font, which is already the palette's
    // family at the palette's weight.
    let installed = an_installed_family();
    let mut config = Config::default();
    config.set_font(installed.to_string());
    assert_eq!(group_weight(&config, None), None);
}

#[test]
fn a_group_with_a_weight_gets_a_face_the_family_really_has() {
    // A group naming a weight is asking for a face, and a family that
    // has no Black is drawn at the nearest it does have rather than
    // at a weight the renderer would have to invent.
    let installed = an_installed_family();
    let mut config = Config::default();
    config.set_font(installed.to_string());

    let asked = FontWeight::Black;
    let drawn = group_weight(&config, Some(asked)).expect("the family is installed");
    assert_eq!(drawn, resolved_weight(installed, asked));
    assert!(weights_for(installed).contains(&drawn), "{drawn:?}");
}

#[test]
fn a_group_weight_is_dropped_with_the_family_it_was_chosen_for() {
    // The same rule the palette's own weight follows: a palette
    // standing in a recommended family is set at that family's
    // default throughout, because a Bold picked for one face says
    // nothing about how another should be set.
    let mut config = Config::default();
    config.set_font("No Such Family At All".to_string());
    let drawn = group_weight(&config, Some(FontWeight::Bold));

    match recommended_fallback() {
        // There is a substitute, so the group goes to it at its
        // default weight along with everything else on the keypad.
        Some(_) => assert_eq!(drawn, None),
        // And on a machine with no substitute to reach for, the name
        // stands and the group is set at whatever face the renderer
        // finds under it — which is what the interface font, carrying
        // the palette's own weight, is snapped to as well.
        None => assert_eq!(
            drawn,
            Some(resolved_weight("No Such Family At All", FontWeight::Bold))
        ),
    }
}
