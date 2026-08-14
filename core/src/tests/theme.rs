use crate::theme::*;
use crate::color::Rgba;

#[test]
fn cosmic_preset_has_expected_colours() {
    let t = ThemeKind::Cosmic.get();
    assert_eq!(t.app_bg, Rgba::from_hex(0x1b_1b_1b_FF));
    assert_eq!(t.basicop_button, Rgba::from_hex(0x61_cd_dc_FF));
}

#[test]
fn text_inactive_is_30_percent_alpha() {
    let t = ThemeKind::Cosmic.get();
    assert_eq!(t.text_inactive().a, 77);
    assert_eq!(t.text_inactive().r, t.text_active.r);
}

#[test]
fn all_presets_enumerate_in_order() {
    let names: Vec<_> = ThemeKind::all().iter().map(|k| k.display_name()).collect();
    assert_eq!(names[0], "Cosmic");
    assert_eq!(names[7], "Custom");
}

#[test]
fn apply_cosmic_override_dark_derives_number_button() {
    let base = ThemeKind::Cosmic.get();
    let over = CosmicOverride {
        window_bg: Rgba::from_hex(0x10_10_10_FF),
        container_bg: Rgba::from_hex(0x20_20_20_FF),
        interface_text: Rgba::from_hex(0xFF_FF_FF_FF),
        component_tint: Rgba::from_hex(0x50_50_50_FF),
        accent: Rgba::from_hex(0x00_FF_00_FF),
        is_dark: true,
    };
    let t = apply_cosmic_override(base, over);
    assert_eq!(t.app_bg, over.window_bg);
    assert_eq!(t.science_button, over.component_tint);
    assert_eq!(t.equals_button, over.accent);
    // 0x50 * 0.8 ≈ 0x40 – darker than the component tint.
    assert!(t.number_button.r < over.component_tint.r);
}

#[test]
fn apply_cosmic_override_light_lightens_number_button() {
    // Same test but `is_dark=false` – number/decimal buttons
    // should come out *lighter* than the component tint.
    let base = ThemeKind::Cosmic.get();
    let over = CosmicOverride {
        window_bg: Rgba::from_hex(0xF0_F0_F0_FF),
        container_bg: Rgba::from_hex(0xE8_E8_E8_FF),
        interface_text: Rgba::from_hex(0x00_00_00_FF),
        component_tint: Rgba::from_hex(0x80_80_80_FF),
        accent: Rgba::from_hex(0x00_67_C0_FF),
        is_dark: false,
    };
    let t = apply_cosmic_override(base, over);
    assert!(t.number_button.r > over.component_tint.r);
    assert_eq!(t.decimal_button, t.number_button);
}
