use crate::ui::display_metrics::{
    available_display_width, display_line_budgets, fit_display_text, fit_display_text_to_width,
};
use crate::ui::keypad::{label_width_units, LABEL_CHAR_WIDTH_RATIO};

#[test]
fn display_line_budgets_keep_caption_at_sixty_percent_of_main() {
    let (caption_h, main_h) = display_line_budgets(200.0, 8.0, true);
    assert!(caption_h > 0.0 && main_h > 0.0);
    assert!(
        (caption_h / main_h - 0.6).abs() < 0.01,
        "caption_h={caption_h} main_h={main_h}"
    );
}

#[test]
fn display_line_budgets_give_main_full_height_without_caption() {
    let (caption_h, main_h) = display_line_budgets(150.0, 8.0, false);
    assert_eq!(caption_h, 0.0);
    assert!((main_h - 150.0).abs() < f32::EPSILON);
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
