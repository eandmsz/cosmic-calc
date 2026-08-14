use crate::config::{ButtonShape, Config};
use crate::ui::keypad::{button_cell_width, keypad_metrics, label_font_size, min_window_size};

#[test]
fn min_window_size_keeps_font_legible() {
    let config = Config::default();
    let (min_w, min_h) = min_window_size(&config);
    // Sanity: should be a usable, non-tiny rect.
    assert!((360.0..=800.0).contains(&min_w), "min_w = {}", min_w);
    assert!((360.0..=800.0).contains(&min_h), "min_h = {}", min_h);
    let m = keypad_metrics(min_h, &config);
    let edge = crate::ui::keypad::effective_spacing(min_h, &config);
    let cell_w = button_cell_width(min_w, 9, m.spacing, edge);
    let font = label_font_size(m.button_height, cell_w, "cosh⁻¹");
    assert!(
        font >= m.button_height * 0.22,
        "font = {} should respect min height ratio",
        font
    );
    assert!(
        font <= m.button_height * 0.36,
        "font = {} should respect max height ratio",
        font
    );
}

#[test]
fn label_font_scales_with_height_for_short_labels() {
    let font = label_font_size(40.0, 120.0, "8");
    assert!((font - 40.0 * (14.0 / 44.0)).abs() < 0.5);
}

#[test]
fn label_font_caps_by_width_for_long_labels() {
    let tall_narrow = label_font_size(80.0, 28.0, "sin⁻¹");
    let wide = label_font_size(80.0, 120.0, "sin⁻¹");
    assert!(tall_narrow < wide);
    assert!(tall_narrow <= 80.0 * 0.36);
}

#[test]
fn round_metrics_solve_62_percent() {
    let config = Config {
        button_shape: ButtonShape::Round,
        ..Config::default()
    };
    let m = keypad_metrics(1000.0, &config);
    // Round: 5h + 4*(h/8) == window*0.62 → h*5.5 == 620 → h≈112.7
    let total = 5.0 * m.button_height + 4.0 * m.spacing;
    assert!((total - 620.0).abs() < 0.001);
    assert!((m.spacing - m.button_height * 0.125).abs() < 0.001);
    assert!((m.radius - m.button_height * 0.5).abs() < 0.001);
}

#[test]
fn slightly_round_metrics_solve_62_percent() {
    let config = Config {
        button_shape: ButtonShape::SlightlyRound,
        ..Config::default()
    };
    let m = keypad_metrics(1000.0, &config);
    // SlightlyRound: 5h + 4*(h/16) == window*0.62 → h*5.25 == 620.
    let total = 5.0 * m.button_height + 4.0 * m.spacing;
    assert!((total - 620.0).abs() < 0.001);
    assert!((m.radius - m.button_height * 0.25).abs() < 0.001);
    assert!((m.spacing - m.radius * 0.25).abs() < 0.001);
}

#[test]
fn metrics_grow_with_window() {
    let config = Config {
        button_shape: ButtonShape::Round,
        ..Config::default()
    };
    let small = keypad_metrics(800.0, &config);
    let large = keypad_metrics(1600.0, &config);
    assert!(large.button_height > small.button_height);
}
