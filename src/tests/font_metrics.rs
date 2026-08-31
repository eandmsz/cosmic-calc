use crate::ui::font_metrics::{baseline_drop, label_nudge, offset_for};

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

#[test]
fn a_family_the_host_does_not_have_still_offers_a_weight() {
    use crate::config::FontWeight;
    use crate::ui::font::{resolved_weight, weights_for};

    // The settings panel draws a button per weight, so the list can
    // never be empty — a name nothing on the machine answers to gets
    // the one weight the renderer would fall back to anyway.
    const MISSING: &str = "No Such Family At All";
    assert_eq!(weights_for(MISSING), [FontWeight::Regular]);
    // And a weight that family has no face for resolves to the
    // nearest it does, so nothing is drawn in a face it does not
    // ship.
    assert_eq!(
        resolved_weight(MISSING, FontWeight::Black),
        FontWeight::Regular
    );

    // Every family the host does have offers its weights lightest
    // first, and answers for each of them with itself.
    for family in crate::ui::font::available_fonts() {
        let weights = weights_for(family);
        assert!(!weights.is_empty(), "{family}");
        assert!(weights.windows(2).all(|pair| pair[0] < pair[1]), "{family}");
        for weight in weights {
            assert_eq!(resolved_weight(family, *weight), *weight, "{family}");
        }
    }
}

// --- keeping a row of pieces on one baseline -------------------------

#[test]
fn a_piece_drawn_from_one_face_needs_no_correction() {
    // Nothing to correct: the band the piece stands on is the family's
    // own, which is the band every other piece in the row stands on.
    let alone = baseline_drop(0.80, 0.20);
    assert_eq!(alone - baseline_drop(0.80, 0.20), 0.0);
}

#[test]
fn a_taller_fallback_face_pushes_a_piece_down_and_is_pulled_back_up() {
    // `√` from a face that reserves more room above the baseline: the
    // line is stood on the taller ascent, so the baseline — and with
    // it the `(` sharing the piece — drops. The correction is the
    // distance back up.
    let alone = baseline_drop(0.80, 0.20);
    let mixed = baseline_drop(1.10, 0.20);
    assert!(mixed > alone, "{mixed} vs {alone}");
    let correction = alone - mixed;
    assert!(correction < 0.0, "{correction}");
    assert!((correction + 0.15).abs() < 1e-6, "{correction}");
}

#[test]
fn a_deeper_fallback_face_moves_a_piece_the_other_way() {
    // Descent is the other half of the band, and a fallback face that
    // hangs further below the line raises the baseline instead. The
    // correction follows it rather than assuming one direction.
    let alone = baseline_drop(0.80, 0.20);
    let mixed = baseline_drop(0.80, 0.40);
    assert!(mixed < alone, "{mixed} vs {alone}");
    assert!(alone - mixed > 0.0);
}

#[test]
fn a_fallback_face_inside_the_family_band_changes_nothing() {
    // The band is a max over the faces on the line, so a fallback that
    // is shorter and shallower than the family adds nothing to it and
    // the piece stays exactly where it was.
    let alone = baseline_drop(0.80, 0.20);
    let mixed = baseline_drop(0.80_f32.max(0.70), 0.20_f32.max(0.15));
    assert_eq!(alone, mixed);
}

#[test]
fn the_measurement_is_free_of_the_font_size_until_it_is_applied() {
    // `baseline_nudge` is em measured against the size the piece is
    // drawn at, so the same family corrects a caption and a readout by
    // proportionally the same amount. Nothing to assert against a real
    // font here — a machine with no fonts at all must simply not move
    // anything.
    use crate::ui::font_metrics::baseline_nudge;
    assert_eq!(baseline_nudge("Nimbus Sans", "", 40.0), 0.0);
    assert_eq!(baseline_nudge("Nimbus Sans", "5", 0.0), 0.0);
    assert_eq!(baseline_nudge("Nimbus Sans", "5", f32::NAN), 0.0);
}

// --- keeping a root degree out of its radical ------------------------

#[test]
fn a_degree_that_already_stands_clear_is_left_where_it_is() {
    use crate::ui::font_metrics::degree_climb;

    // The sign reaches 20 up in the columns the degree covers and the
    // degree's baseline is at 30: there is daylight between them, and
    // a degree that is not in the way is not moved.
    assert_eq!(degree_climb(20.0, 30.0, 2.0, 10.0), 0.0);
    // Exactly the clearance apart is still clear.
    assert_eq!(degree_climb(20.0, 22.0, 2.0, 10.0), 0.0);
}

#[test]
fn a_degree_resting_on_the_sign_climbs_off_it() {
    use crate::ui::font_metrics::degree_climb;

    // Foot of the degree at 18, top of the stroke under it at 20: it
    // is in the stroke by 2, and comes up by that plus the clearance.
    assert_eq!(degree_climb(20.0, 18.0, 2.0, 10.0), 4.0);
    // And by no more than it is short of — the degree belongs in the
    // opening of the sign, not above the whole radical.
    assert!(degree_climb(20.0, 19.5, 2.0, 10.0) < degree_climb(20.0, 18.0, 2.0, 10.0));
}

#[test]
fn a_degree_never_climbs_out_of_its_own_line() {
    use crate::ui::font_metrics::degree_climb;

    // A face whose opening reaches higher than there is room to climb
    // gets as much of the lift as the line box holds and no more:
    // going further would trade a degree touching its sign for one
    // reaching into the row above it.
    assert_eq!(degree_climb(100.0, 0.0, 2.0, 10.0), 10.0);
    // No room at all is a legal answer, and asks for nothing.
    assert_eq!(degree_climb(100.0, 0.0, 2.0, 0.0), 0.0);
    assert_eq!(degree_climb(100.0, 0.0, 2.0, -5.0), 0.0);
}

#[test]
fn nothing_a_broken_measurement_says_moves_a_degree() {
    use crate::ui::font_metrics::degree_climb;

    assert_eq!(degree_climb(f32::NAN, 0.0, 2.0, 10.0), 0.0);
    assert_eq!(degree_climb(20.0, 0.0, 2.0, f32::INFINITY), 0.0);
}

#[test]
fn the_radical_is_measured_at_its_opening_rather_than_its_bar() {
    use crate::ui::display::ROOT_DEGREE_OVERLAP;
    use crate::ui::font_metrics::radical_band_top;

    // The degree covers the left-hand sliver of the sign, so what it
    // has to clear is the short stroke there — the bar is away to the
    // right of it and does not come into the answer. Whatever the
    // host has installed, that is somewhere above the baseline and
    // below the top of the sign.
    for family in crate::ui::font::available_fonts().iter().take(40) {
        let Some(top) = radical_band_top(family, ROOT_DEGREE_OVERLAP) else {
            continue;
        };
        assert!(top > 0.0 && top < 2.0, "{family}: {top}");
        // A wider band can only find more of the sign, never less.
        let wider = radical_band_top(family, ROOT_DEGREE_OVERLAP * 2.0).unwrap_or(top);
        assert!(wider >= top - 1e-6, "{family}: {wider} vs {top}");
    }
    // A band with nothing in it is nothing measured, not a zero.
    assert_eq!(radical_band_top("DejaVu Sans", 0.0), None);
    assert_eq!(radical_band_top("DejaVu Sans", f32::NAN), None);
}

#[test]
fn the_lift_is_the_same_for_every_piece_of_one_degree() {
    use crate::ui::display::ROOT_DEGREE_OVERLAP;
    use crate::ui::font_metrics::root_degree_climb;

    // Nothing about the measurement depends on which characters the
    // degree is spelled with — the signature has no text in it — and
    // that is what keeps a `12` on one line: every piece of a degree
    // is placed on its own, so a lift read off one piece's ink would
    // stand the two digits at different heights. What it does depend
    // on is the size, and it scales with it.
    for family in crate::ui::font::available_fonts().iter().take(40) {
        let small = root_degree_climb(family, ROOT_DEGREE_OVERLAP, 0.2, 31.0, 15.9, 26.5);
        let large = root_degree_climb(family, ROOT_DEGREE_OVERLAP, 0.2, 62.0, 31.8, 53.0);
        assert!(small >= 0.0 && large >= 0.0, "{family}");
        assert!(large >= small - 1e-6, "{family}: {large} vs {small}");
        // And it stays a nudge: a degree is moved within its own line,
        // never by a whole line.
        assert!(large < 31.8, "{family}: {large}");
    }
}

#[test]
fn a_degree_is_not_moved_by_a_font_that_cannot_be_read() {
    use crate::ui::display::ROOT_DEGREE_OVERLAP;
    use crate::ui::font_metrics::root_degree_climb;

    // Every degenerate way the geometry can arrive asks for no move
    // rather than for a NaN of one.
    let climb = |band, step, line_h, size, radical| {
        root_degree_climb("DejaVu Sans", band, step, line_h, size, radical)
    };
    assert_eq!(climb(f32::NAN, 0.2, 62.0, 31.8, 53.0), 0.0);
    assert_eq!(climb(ROOT_DEGREE_OVERLAP, f32::NAN, 62.0, 31.8, 53.0), 0.0);
    assert_eq!(climb(ROOT_DEGREE_OVERLAP, 0.2, f32::NAN, 31.8, 53.0), 0.0);
    assert_eq!(climb(ROOT_DEGREE_OVERLAP, 0.2, 62.0, 0.0, 53.0), 0.0);
    assert_eq!(climb(ROOT_DEGREE_OVERLAP, 0.2, 62.0, 31.8, f32::NAN), 0.0);
}
