//! Font-size geometry for the expression display.
//!
//! Pure arithmetic over window and slot dimensions: no widgets, no
//! application state. It lived in `app.rs`, which had grown past 1,200
//! lines around it, and it is much easier to reason about (and already
//! easier to test) on its own.

/// Pick a (font size, line height) pair for the main display so a
/// long expression shrinks rather than wrapping. The thresholds are
/// tuned to the default 300px window: the title1 preset (35sp) fits
/// roughly 12 chars, so once the rendered string exceeds that we step
/// down through the libcosmic title2..title4 sizes and finally cap at
/// the body size for genuinely long inputs. Line heights mirror the
/// libcosmic preset ratios (~1.49) so vertical spacing tracks the
/// font size cleanly.
pub fn scale_main_text_size(chars: usize, window_width: f32, display_height: f32) -> (f32, f32) {
    // Each tier was bumped a few points over the original libcosmic
    // title1..title4 ladder so the answer reads more like a calculator
    // result than a generic heading. The tallest tier in particular sits
    // closer to the "huge digit" feel of dedicated calc apps without
    // overflowing the 300px reference window.
    let (base_size, base_line) = match chars {
        0..=12 => (44.0, 62.0),
        13..=16 => (36.0, 52.0),
        17..=22 => (30.0, 44.0),
        23..=30 => (24.0, 36.0),
        _ => (20.0, 30.0),
    };
    let factor = window_width_scale_factor(window_width)
        * display_height_scale_factor(display_height, chars);
    (base_size * factor, base_line * factor)
}

/// Same idea for the caption above the main display – starts at the
/// default caption size (10sp) and shrinks slightly for very long
/// previously-evaluated expressions.
pub fn scale_caption_text_size(chars: usize, window_width: f32, display_height: f32) -> (f32, f32) {
    let (base_size, base_line) = match chars {
        0..=24 => (10.0, 14.0),
        25..=40 => (9.0, 13.0),
        _ => (8.0, 12.0),
    };
    let factor = window_width_scale_factor(window_width)
        * display_height_scale_factor(display_height, chars.saturating_add(8));
    (base_size * factor, base_line * factor)
}

/// Map a window width (logical pixels) to a font-size multiplier so the
/// main display and caption grow as the user enlarges the window. The
/// reference width is 480px (the default startup size) – at that point
/// the multiplier is 1.0; bigger windows scale up to a 2.0 cap and
/// smaller windows scale down to 0.7 so the layout doesn't collapse.
fn window_width_scale_factor(window_width: f32) -> f32 {
    const REFERENCE_WIDTH: f32 = 480.0;
    let raw = window_width / REFERENCE_WIDTH;
    raw.clamp(0.7, 2.0)
}

/// Horizontal space for right-aligned display text after column padding.
pub fn available_display_width(window_width: f32, edge_spacing: f32) -> f32 {
    (window_width - 2.0 * edge_spacing).max(1.0)
}

/// Fit display text to both the slot height and available width.
pub fn fit_display_text(
    width_units: f32,
    available_width: f32,
    max_line_h: f32,
    size: f32,
    line_h: f32,
) -> (f32, f32) {
    let (mut size, mut line_h) = fit_display_text_to_width(width_units, available_width, size, line_h);
    if max_line_h > 1.0 && line_h > 0.0 && line_h < max_line_h {
        let grow = max_line_h / line_h;
        size *= grow;
        line_h = max_line_h;
        (size, line_h) = fit_display_text_to_width(width_units, available_width, size, line_h);
    }
    fit_text_to_line_height(max_line_h, &mut size, &mut line_h);
    (size, line_h)
}

/// Shrink `(size, line_h)` when the rendered string would extend past
/// `available_width`. Uses the same width-per-glyph estimate as the
/// keypad so tall, narrow windows do not clip after height-based scaling.
pub fn fit_display_text_to_width(
    width_units: f32,
    available_width: f32,
    size: f32,
    line_h: f32,
) -> (f32, f32) {
    if width_units <= 0.0 || size <= 0.0 {
        return (size, line_h);
    }
    let estimated = width_units * size * crate::ui::keypad::LABEL_CHAR_WIDTH_RATIO;
    if estimated <= available_width {
        return (size, line_h);
    }
    let scale = available_width / estimated;
    (size * scale, line_h * scale)
}

/// Grow display fonts when the flexible display slice is taller than
/// the reference layout, but cap the boost for long expressions so
/// they still fit vertically.
fn display_height_scale_factor(display_height: f32, chars: usize) -> f32 {
    const REFERENCE_HEIGHT: f32 = 72.0;
    let raw = display_height / REFERENCE_HEIGHT;
    let cap = match chars {
        0..=12 => 2.2,
        13..=22 => 1.8,
        23..=30 => 1.5,
        _ => 1.2,
    };
    // Do not shrink below 1.0 when the window is short — per-slot fitting
    // handles vertical overflow instead of scaling text away.
    raw.clamp(1.0, cap)
}

/// Last-expression slot is always 60% of the main readout slot height.
const CAPTION_TO_MAIN_HEIGHT_RATIO: f32 = 0.6;
const MAIN_HEIGHT_PORTION: u16 = 100;
const CAPTION_HEIGHT_PORTION: u16 =
    (MAIN_HEIGHT_PORTION as f32 * CAPTION_TO_MAIN_HEIGHT_RATIO) as u16;

/// Split the fixed display column into caption and main line heights.
pub fn display_line_budgets(
    display_height: f32,
    row_spacing: f32,
    has_caption: bool,
) -> (f32, f32) {
    if !has_caption {
        return (0.0, display_height.max(1.0));
    }
    let content = (display_height - row_spacing).max(1.0);
    let total_portions =
        MAIN_HEIGHT_PORTION as f32 + CAPTION_HEIGHT_PORTION as f32;
    let main_h = content * MAIN_HEIGHT_PORTION as f32 / total_portions;
    let caption_h = content * CAPTION_HEIGHT_PORTION as f32 / total_portions;
    (caption_h, main_h)
}

/// Clamp font metrics to a single line slot.
fn fit_text_to_line_height(max_line_h: f32, size: &mut f32, line_h: &mut f32) {
    if max_line_h <= 1.0 {
        *line_h = 1.0;
        *size = 1.0;
        return;
    }
    if *line_h > max_line_h {
        let scale = max_line_h / *line_h;
        *line_h *= scale;
        *size *= scale;
    }
    *line_h = (*line_h).min(max_line_h);
    *size = (*size).min(*line_h);
}
