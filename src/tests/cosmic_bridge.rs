use crate::ui::cosmic_bridge::*;
use cosmic::cosmic_theme;

#[test]
fn override_from_cosmic_dark_default() {
    let theme = cosmic_theme::Theme::dark_default();
    let over = override_from_cosmic(&theme);
    assert!(over.is_dark);
    // The alpha of a normal background should be fully opaque.
    assert_eq!(over.window_bg.a, 255);
}

#[test]
fn override_from_cosmic_light_default() {
    let theme = cosmic_theme::Theme::light_default();
    let over = override_from_cosmic(&theme);
    assert!(!over.is_dark);
}
