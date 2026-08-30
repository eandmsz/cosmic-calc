//! Font-size geometry for the expression display.
//!
//! Pure arithmetic over window and slot dimensions: no widgets, no
//! application state. It lived in `app.rs`, which had grown past 1,200
//! lines around it, and it is much easier to reason about (and already
//! easier to test) on its own.

use crate::config::FontWeight;

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
    //
    // What decides how big the digits actually come out is the ratio
    // between the two numbers, not either on its own: the line height
    // is grown to fill the slot and the size follows it, so the
    // leftover is leading — blank space split evenly above and below
    // the digits. The original ladder left about three tenths of the
    // slot as leading, which read as padding around a readout that
    // had room to be bigger; these keep about one seventh of it,
    // which still clears an ascender and a descender and gives the
    // rest of the slot to the digits. Every tier holds the same
    // ratio, so the readout keeps its proportions as it steps down,
    // and the leading stays split evenly top and bottom.
    let (base_size, base_line) = match chars {
        0..=12 => (53.0, 62.0),
        13..=16 => (45.0, 52.0),
        17..=22 => (38.0, 44.0),
        23..=30 => (31.0, 36.0),
        _ => (26.0, 30.0),
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

/// How much wider than the regular face a weight draws, as a
/// multiplier on the per-character width estimate every fit here is
/// made with.
///
/// A heavier face carries more ink per glyph and a little more
/// advance with it — around a fifteenth between Regular and Bold on
/// the faces this was measured against. Without the allowance a
/// display fitted at the regular estimate ran past its window once
/// the user picked a Bold, and the long error messages lost their
/// last word off the right-hand edge.
pub fn char_width_factor(weight: FontWeight) -> f32 {
    const PER_STEP: f32 = 0.12;
    1.0 + (weight.value() as f32 - FontWeight::Regular.value() as f32) / 500.0 * PER_STEP
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
    let (mut size, mut line_h) =
        fit_display_text_to_width(width_units, available_width, size, line_h);
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
///
/// The caption's share is reserved whether or not there is a caption
/// to put in it. Handing the main line the whole column while the
/// caption is empty made it grow by two thirds, so the very first
/// number typed after a start (or an AC) came up in a much larger font
/// than the same number after an `=` — the split has to be the same
/// either way for the readout to keep one size.
pub fn display_line_budgets(display_height: f32, row_spacing: f32) -> (f32, f32) {
    let content = (display_height - row_spacing).max(1.0);
    let total_portions = MAIN_HEIGHT_PORTION as f32 + CAPTION_HEIGHT_PORTION as f32;
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

/// Text size the row under the display is drawn at — libcosmic's
/// caption preset, which both the property labels and the memory
/// register wear.
pub const STATUS_TEXT_SIZE: f32 = 12.0;

/// Gap between the pieces of that row, in logical pixels. The row
/// sets it as its widget spacing, and the fit below has to count the
/// same one.
pub const STATUS_SPACING: f32 = 8.0;

/// Ratio of character width to font size used for the row's fit: the
/// keypad's own estimate, which measures the shipped UI face's
/// property labels to within a few pixels at this size.
///
/// Being a shade wide is the safe direction here. Where the estimate
/// is wrong the register goes on a line of its own, and a register on
/// its own line is legible whether or not it would also have fitted
/// beside the labels.
const STATUS_CHAR_WIDTH_RATIO: f32 = crate::ui::keypad::LABEL_CHAR_WIDTH_RATIO;

/// Whether the property labels and the memory register still fit on
/// one line.
///
/// The two grow towards each other: a narrower window shortens the
/// space between them and a longer stored value lengthens the
/// register, so past some point the register would be drawn over the
/// `fibonacci` at the end of the labels. It goes to a line of its own
/// instead, and this is the question both the layout arithmetic and
/// the renderer ask so the height reserved and the height drawn
/// cannot disagree.
///
/// `units` is every piece of the row measured in character widths
/// (see `keypad::label_width_units`) and `gaps` how many spacings
/// stand between them. The character estimate runs deliberately wide,
/// so where it is wrong it is wrong towards the second line, which is
/// legible either way.
pub fn status_row_fits(units: f32, gaps: usize, available_width: f32) -> bool {
    let text = units * STATUS_TEXT_SIZE * STATUS_CHAR_WIDTH_RATIO;
    text + gaps as f32 * STATUS_SPACING <= available_width
}
