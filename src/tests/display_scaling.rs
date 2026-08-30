use crate::ui::app::segment_padding;
use crate::ui::display_metrics::{
    available_display_width, display_line_budgets, fit_display_text, fit_display_text_to_width,
    scale_main_text_size,
};
use crate::ui::keypad::{label_width_units, LABEL_CHAR_WIDTH_RATIO};

#[test]
fn display_line_budgets_keep_caption_at_sixty_percent_of_main() {
    let (caption_h, main_h) = display_line_budgets(200.0, 8.0);
    assert!(caption_h > 0.0 && main_h > 0.0);
    assert!(
        (caption_h / main_h - 0.6).abs() < 0.01,
        "caption_h={caption_h} main_h={main_h}"
    );
}

#[test]
fn display_line_budgets_reserve_the_caption_slot_when_it_is_empty() {
    // The caption slot is held open even with nothing in it, so the
    // main readout gets the same height before and after the first `=`.
    let (_, main_h) = display_line_budgets(150.0, 8.0);
    assert!(main_h < 150.0, "main_h={main_h}");
}

#[test]
fn main_display_font_does_not_grow_when_the_caption_is_empty() {
    // Regression: with the caption empty the main readout used to be
    // handed the whole display column, so the first number typed came
    // up far larger than the same number does after an `=` fills the
    // caption. Both paths now size against the shared slot.
    let display_budget = 150.0;
    let row_spacing = 8.0;
    let available = available_display_width(480.0, row_spacing);
    let sized = |slot_h: f32| {
        let (size, line_h) = scale_main_text_size(3, 480.0, slot_h);
        fit_display_text(3.0, available, slot_h, size, line_h).0
    };

    let (_, main_slot_h) = display_line_budgets(display_budget, row_spacing);
    let shared_slot = sized(main_slot_h);
    let whole_column = sized(display_budget);
    assert!(
        shared_slot < whole_column,
        "shared={shared_slot} whole_column={whole_column}"
    );
}

#[test]
fn fit_display_text_grows_to_fill_tall_slot_after_width_shrink() {
    let (fitted_size, fitted_line_h) = fit_display_text(1.0, 200.0, 120.0, 44.0, 62.0);
    assert!(
        (fitted_line_h - 120.0).abs() < 0.5,
        "line_h={fitted_line_h}"
    );
    assert!(fitted_size > 44.0);
}

#[test]
fn fit_display_text_shrinks_when_tall_window_boosts_font() {
    // Tall narrow window: height scaling can push 12 digits past the width.
    let size = 44.0_f32 * 0.7 * 2.2;
    let line_h = 62.0_f32 * 0.7 * 2.2;
    let units = 12.0;
    let available = available_display_width(320.0, 8.0);
    let (fitted_size, _) = fit_display_text_to_width(units, available, size, line_h);
    assert!(fitted_size < size);
    let estimated = units * fitted_size * LABEL_CHAR_WIDTH_RATIO;
    assert!(
        estimated <= available + 0.5,
        "estimated {estimated} available {available}"
    );
}

#[test]
fn fit_display_text_leaves_size_when_it_already_fits() {
    let (size, line_h) = fit_display_text_to_width(4.0, 400.0, 30.0, 42.0);
    assert!((size - 30.0).abs() < f32::EPSILON);
    assert!((line_h - 42.0).abs() < f32::EPSILON);
}

#[test]
fn expression_width_units_counts_wide_glyphs() {
    assert!(label_width_units("sin⁻¹") > label_width_units("8"));
}

// --- where a piece is placed on the line -----------------------------

#[test]
fn a_script_is_padded_on_the_side_it_moves_away_from() {
    // The box centres its text, so a piece rises by half of whatever
    // is padded underneath it: an exponent a tenth of the line above
    // the middle gets a fifth of the line under it, and a base the
    // same over it.
    let raised = segment_padding(0.1, 0.0, 50.0);
    assert_eq!((raised.top, raised.bottom), (0.0, 10.0));
    let lowered = segment_padding(-0.1, 0.0, 50.0);
    assert_eq!((lowered.top, lowered.bottom), (10.0, 0.0));
}

#[test]
fn a_slide_costs_the_row_nothing() {
    // What makes a root degree overlap the radical rather than push it
    // along: the two horizontal paddings cancel, so the box is exactly
    // as wide as its text and only the ink inside it moves.
    let padding = segment_padding(0.1, 4.0, 50.0);
    assert_eq!(padding.left, 4.0);
    assert_eq!(padding.right, -4.0);
    assert_eq!(padding.left + padding.right, 0.0);
    // And a piece that is not sliding is not moved sideways at all.
    let still = segment_padding(0.1, 0.0, 50.0);
    assert_eq!((still.left, still.right), (0.0, 0.0));
}

#[test]
fn the_readout_keeps_only_as_much_leading_as_it_needs() {
    // The size-to-line ratio is what decides how big the digits come
    // out: the line height is grown to fill the slot and the size
    // follows it, so whatever the ratio leaves over is blank space
    // split evenly above and below. Every tier holds the same ratio,
    // so stepping down for a longer expression does not also change
    // how much of the slot the digits get.
    let ratios: Vec<f32> = [4, 14, 20, 26, 40]
        .iter()
        .map(|chars| {
            let (size, line) = scale_main_text_size(*chars, 480.0, 72.0);
            size / line
        })
        .collect();
    for ratio in &ratios {
        assert!(
            (0.84..=0.88).contains(ratio),
            "a tier leaves {:.0}% of its slot as leading",
            (1.0 - ratio) * 100.0
        );
    }
    let spread = ratios
        .iter()
        .fold(0.0f32, |acc, r| acc.max((r - ratios[0]).abs()));
    assert!(spread < 0.02, "the tiers disagree about their leading");
}

// --- the row under the display ---------------------------------------

#[test]
fn the_memory_register_is_named_rather_than_lettered() {
    use crate::ui::app::memory_readout;

    // A bare `M` is a letter; the row says what it is holding.
    assert_eq!(memory_readout(""), "Memory:");
    // And the space between the word and the number is a no-break
    // one, so wrapping the row can never put the two on different
    // lines.
    assert_eq!(memory_readout("1 234.5"), "Memory:\u{00A0}1 234.5");
}

#[test]
fn a_long_memory_value_moves_under_the_property_labels() {
    use crate::ui::app::{memory_readout, status_row_lines};

    // Wide enough for both, and the register sits beside the labels.
    let register = memory_readout("42");
    assert_eq!(status_row_lines(true, Some(&register), 500.0), 1);

    // The two grow towards each other from either end: fifteen digits
    // of stored value, or a window dragged in, and the register would
    // be drawn over the `fibonacci` at the end of the row. It goes
    // under it instead.
    let long = memory_readout("123 456 789 012 345");
    assert_eq!(status_row_lines(true, Some(&long), 500.0), 2);
    assert_eq!(status_row_lines(true, Some(&register), 300.0), 2);

    // Either half on its own always fits: there is nothing for it to
    // collide with.
    assert_eq!(status_row_lines(false, Some(&long), 100.0), 1);
    assert_eq!(status_row_lines(true, None, 100.0), 1);
    // And with both switched off the row is not drawn at all.
    assert_eq!(status_row_lines(false, None, 400.0), 0);
}

#[test]
fn a_heavier_face_is_fitted_to_a_narrower_window() {
    use crate::config::FontWeight;
    use crate::ui::display_metrics::char_width_factor;

    // The regular face is what the per-character estimate is made
    // against, so it is the one that costs nothing.
    assert_eq!(char_width_factor(FontWeight::Regular), 1.0);
    // Heavier draws wider, lighter narrower, and the steps are in
    // order.
    assert!(char_width_factor(FontWeight::Bold) > 1.0);
    assert!(char_width_factor(FontWeight::Thin) < 1.0);
    let mut previous = 0.0;
    for weight in FontWeight::ALL {
        let factor = char_width_factor(weight);
        assert!(factor > previous, "{weight:?}");
        previous = factor;
    }
    // A tenth or so across the whole range: enough to keep a Bold
    // display inside its window, not so much that it is drawn small.
    assert!(char_width_factor(FontWeight::Black) < 1.2);
}
