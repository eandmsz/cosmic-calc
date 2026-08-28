use crate::ui::font_metrics::{label_nudge, offset_for};

#[test]
fn a_typical_sans_face_pushes_text_slightly_down() {
    // Ascent 0.80 em, descent 0.20 em, cap height 0.70 em: the cap band
    // sits above the middle of the ascent/descent band, so the label
    // has to move down to look centred.
    let nudge = offset_for(0.80, 0.20, 0.35);
    assert!(nudge > 0.0, "{nudge}");
    assert!((nudge - 0.05).abs() < 1e-6, "{nudge}");
}

#[test]
fn a_deep_ascender_lifts_the_label_instead() {
    // A face that reserves more room above the cap line (accents,
    // symbol fonts) centres its band higher above the baseline, so the
    // same cap height now sits too low and the correction flips. This
    // is the difference between two families — and between the UI font
    // and the fallback a glyph like ⌫ is drawn from.
    let shallow = offset_for(0.80, 0.20, 0.35);
    let deep = offset_for(1.05, 0.30, 0.35);
    assert!(deep < shallow, "deep {deep} vs shallow {shallow}");
    assert!(
        deep < 0.0 && shallow > 0.0,
        "deep {deep}, shallow {shallow}"
    );
}

#[test]
fn a_glyph_centred_on_the_math_axis_moves_up() {
    // `×` and `÷` sit on the math axis, below the middle of the cap
    // band, so their correction points the other way.
    let nudge = offset_for(0.90, 0.25, 0.25);
    assert!(nudge < 0.0, "{nudge}");
}

#[test]
fn a_symmetric_band_needs_no_correction() {
    assert_eq!(offset_for(0.6, 0.6, 0.0), 0.0);
}

#[test]
fn degenerate_input_is_never_a_nudge() {
    // No label, no size, no font: the label stays exactly where the
    // container put it rather than jumping by a NaN.
    assert_eq!(label_nudge("Adwaita Sans", "", 20.0), 0.0);
    assert_eq!(label_nudge("Adwaita Sans", "7", 0.0), 0.0);
    assert_eq!(label_nudge("Adwaita Sans", "7", f32::NAN), 0.0);
    assert!(label_nudge("No Such Family At All", "7", 20.0).is_finite());
}

#[test]
fn a_measured_nudge_stays_inside_the_button() {
    // Whatever the host has installed, the correction is a fraction of
    // the font size — it can never throw the label out of its cell.
    for label in ["7", "⌫", "sin⁻¹", "+⁄−", "Rand"] {
        let nudge = label_nudge("Adwaita Sans", label, 20.0);
        assert!(nudge.abs() <= 20.0 * 0.25 + 1e-6, "{label}: {nudge}");
    }
}
