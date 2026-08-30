use crate::ui::cosmic_bridge::*;
use cosmic::cosmic_theme;

#[test]
fn a_desktop_palette_comes_across_whole() {
    for theme in [
        cosmic_theme::Theme::dark_default(),
        cosmic_theme::Theme::light_default(),
    ] {
        let over = override_from_cosmic(&theme);
        // The surfaces are opaque.
        assert_eq!(over.window_bg.a, 255);
        assert_eq!(over.container_bg.a, 255);
        // And each component arrives with a state of its own for
        // every state the calculator draws, so nothing downstream has
        // to invent one. A desktop that drew its hover the same as
        // its base would be a desktop with no hover, which neither of
        // the two shipped palettes is.
        for component in [over.component, over.surface_component, over.accent] {
            assert_ne!(component.base, component.hover);
            assert_ne!(component.base, component.pressed);
        }
    }
}

#[test]
fn an_accent_key_takes_the_accent_s_own_text_colour() {
    // This is the contrast fix: the window's text colour is picked to
    // read against the window, and lifting it onto a bright accent
    // fill left it with nothing to spare. The desktop publishes the
    // colour that belongs on its accent, so that is the one used.
    let theme = cosmic_theme::Theme::dark_default();
    let over = override_from_cosmic(&theme);
    assert_ne!(over.accent.text, over.interface_text);
}

#[test]
fn the_dim_text_is_dimmer_than_the_text_it_comes_from() {
    let over = override_from_cosmic(&cosmic_theme::Theme::dark_default());
    assert_eq!(over.interface_text_dim.r, over.interface_text.r);
    assert!(over.interface_text_dim.a < over.interface_text.a);
}
