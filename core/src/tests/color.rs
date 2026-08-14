use crate::color::*;
use serde::{Deserialize, Serialize};

#[test]
fn from_hex_round_trip() {
    let c = Rgba::from_hex(0x12_34_56_78);
    assert_eq!(c, Rgba { r: 0x12, g: 0x34, b: 0x56, a: 0x78 });
}

#[test]
fn rgba_serializes_as_hex_string() {
    #[derive(Serialize, Deserialize)]
    struct Wrap {
        color: Rgba,
    }
    let c = Rgba::from_hex(0x28_31_33_FF);
    let s = toml::to_string(&Wrap { color: c }).unwrap();
    assert!(s.contains("#283133FF"), "{s}");
    let back: Wrap = toml::from_str(&s).unwrap();
    assert_eq!(back.color, c);
}

#[test]
fn rgba_deserializes_legacy_table() {
    let c: Rgba = toml::from_str("r = 1\ng = 2\nb = 3\na = 255").unwrap();
    assert_eq!(c, Rgba { r: 1, g: 2, b: 3, a: 255 });
}

#[test]
fn parse_hex_str_accepts_six_and_eight_digits() {
    assert_eq!(
        Rgba::parse_hex_str("#AABBCC").unwrap(),
        Rgba::from_hex(0xAA_BB_CC_FF)
    );
    assert_eq!(
        Rgba::parse_hex_str("11223344").unwrap(),
        Rgba::from_hex(0x11_22_33_44)
    );
}

#[test]
fn inactive_sets_30_percent_alpha() {
    let c = Rgba::from_hex(0xff_ff_ff_ff).inactive();
    assert_eq!(c.a, 77);
}

#[test]
fn hover_lightens_dark_pigment() {
    // #202020FF – a dark grey, plenty of headroom in V.
    let base = Rgba::from_hex(0x20_20_20_FF);
    let hov = hover(base);
    // V should increase, hue shift should be near 0 (headroom).
    let (r0, g0, b0, _) = base.to_f32();
    let (r1, g1, b1, _) = hov.to_f32();
    let v0 = rgb_to_hsv(r0, g0, b0).v;
    let v1 = rgb_to_hsv(r1, g1, b1).v;
    assert!(v1 > v0, "hover should lighten dark base");
}

#[test]
fn hover_shifts_hue_when_clipped() {
    // Pure saturated red at full V – no room to lighten.
    let base = Rgba::from_hex(0xFF_00_00_FF);
    let hov = hover(base);
    assert_ne!(base, hov, "fully-clipped base should shift hue");
    // The result should lean slightly toward orange/yellow.
    assert!(hov.g > base.g);
}

#[test]
fn hover_preserves_alpha() {
    let base = Rgba { r: 0x30, g: 0x60, b: 0x90, a: 0xAB };
    let hov = hover(base);
    assert_eq!(hov.a, 0xAB);
}

#[test]
fn scaled_darkens_and_lightens() {
    let c = Rgba::from_hex(0x80_80_80_FF);
    let darker = c.scaled(0.8);
    let lighter = c.scaled(1.2);
    assert!(darker.r < c.r);
    assert!(lighter.r > c.r);
}

#[test]
fn hover_shift_scales_with_saturation() {
    // Two bases at the same V=1 but different saturations. The
    // saturated red should shift more toward yellow than the
    // near-grey pastel, per the formula's `scale * hsv.s` term.
    let saturated = Rgba::from_hex(0xFF_00_00_FF);
    let pastel = Rgba::from_hex(0xFF_E0_E0_FF);
    let h_sat = {
        let (r, g, b, _) = hover(saturated).to_f32();
        rgb_to_hsv(r, g, b).h
    };
    let h_pastel = {
        let (r, g, b, _) = hover(pastel).to_f32();
        rgb_to_hsv(r, g, b).h
    };
    assert!(
        h_sat > h_pastel,
        "saturated red should shift further toward yellow (got sat={h_sat} pastel={h_pastel})"
    );
}

#[test]
fn hover_white_is_still_white() {
    // v=1, s=0 → no headroom to lighten, no saturation to spend.
    let base = Rgba::from_hex(0xFF_FF_FF_FF);
    assert_eq!(hover(base), base);
}
