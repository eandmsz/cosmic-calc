use crate::config::{ButtonShape, Config};
use crate::ui::keymap::{label_parts, LabelContext, LabelPart};
use crate::ui::keypad::{
    button_cell_width, keypad_metrics, label_font_size, label_width_units, min_window_size,
    parts_width_units,
};

#[test]
fn min_window_size_keeps_font_legible() {
    let config = Config::default();
    let (min_w, min_h) = min_window_size(&config);
    // Sanity: should be a usable, non-tiny rect.
    assert!((360.0..=800.0).contains(&min_w), "min_w = {}", min_w);
    assert!((360.0..=800.0).contains(&min_h), "min_h = {}", min_h);
    let m = keypad_metrics(min_h, &config);
    let edge = crate::ui::keypad::keypad_metrics(min_h, &config).spacing;
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

#[test]
fn a_script_on_a_key_face_costs_less_width_than_a_full_character() {
    // The keypad draws its scripts at 60%, so a face has to be
    // measured piece by piece: sized as though the raised `2` of `x²`
    // were a second full-size character, the label would be shrunk to
    // fit a width it never needed.
    let ctx = LabelContext::default();
    let square = parts_width_units(&label_parts(crate::ui::buttons::Button::Square, ctx));
    assert!(
        square < label_width_units("x2"),
        "x² measured as wide as x2: {square}"
    );
    assert!(
        square > label_width_units("x"),
        "x² measured no wider than x"
    );

    // A face with nothing off the line measures exactly as the string
    // it is.
    let plain = [LabelPart::on_line("sin")];
    assert!((parts_width_units(&plain) - label_width_units("sin")).abs() < 1e-6);
}

#[test]
fn a_pinned_minimum_width_wins_over_the_computed_one() {
    let auto = Config::default();
    let (computed_w, computed_h) = min_window_size(&auto);

    // A width in the config file is the floor, whether it is narrower
    // than what the keypad would ask for or wider. The height is not
    // pinned by it — only the width is configurable.
    for pinned in [200u32, 1000] {
        let config = Config {
            min_window_width: pinned,
            ..Config::default()
        };
        let (w, h) = min_window_size(&config);
        assert_eq!(w, pinned as f32, "pinned {pinned}");
        assert_eq!(h, computed_h, "pinned {pinned} moved the height");
    }

    // Zero is the "work it out" value, and is what a config file with
    // no opinion carries.
    assert_eq!(auto.min_window_width, crate::config::AUTO_MIN_WINDOW_WIDTH);
    let (w, _) = min_window_size(&Config {
        min_window_width: crate::config::AUTO_MIN_WINDOW_WIDTH,
        ..Config::default()
    });
    assert_eq!(w, computed_w);
}
