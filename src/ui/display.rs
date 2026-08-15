//! Display renderer. Walks the input buffer and produces a list of
//! [`DisplaySegment`]s that the app layer arranges as a row of text
//! widgets, one per segment, so individual pieces of the rendered
//! expression can be coloured independently.
//!
//! On top of the raw [`InputItem::display`] we apply three spec-mandated
//! touches:
//!
//!   * thousands separators inside numeric runs (integer part only)
//!   * configurable decimal glyph (`.` or `,` per locale / user
//!     override)
//!   * synthetic auto-multiplication glyphs between adjacent operand-end
//!     and operand-start items, rendered inactive so the user can tell
//!     they were inserted by the renderer rather than the buffer
//!   * closing parens whose matching opener sits to the left of the
//!     cursor (cursor is currently inside that paren group) are flagged
//!     inactive so the user can tell at a glance which closer they're
//!     about to step over.
//!
//! Sub/superscript layout, inverse-trig glyph swaps, etc. belong to a
//! richer pipeline that Phase-7 leaves as future work. The history
//! panel uses [`format_result`] (further down) which keeps separators
//! out – `format_result` already hands us a canonical string.

use crate::engine::item::InputItem;
use crate::locale::DecimalSeparator;

/// One piece of the rendered display. Multiple segments line up
/// horizontally; the `active` flag tells the app layer whether to use
/// the full `text_active` colour or the dim `text_inactive` variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySegment {
    pub text: String,
    pub active: bool,
}

impl DisplaySegment {
    fn active(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            active: true,
        }
    }
    pub(crate) fn inactive(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            active: false,
        }
    }
}

/// Render the input buffer as a list of [`DisplaySegment`]s. Numeric
/// runs are coalesced into a single segment with thousands separators
/// applied, auto-multiplication is inserted between adjacent operands,
/// and closing parens that the cursor currently sits inside are flagged
/// inactive. Pass `cursor = items.len()` when the cursor is unknown
/// (e.g. in tests that don't care about the inactive-paren rule).
///
/// `inactive_range` flags an item-index half-open range whose segments
/// should additionally render in the inactive colour — the Rand handler
/// uses it to dim only the just-inserted random number, so the rest of
/// the surrounding expression keeps its normal active colour.
pub fn render_expression(
    items: &[InputItem],
    cursor: usize,
    decimal: DecimalSeparator,
    thousands_glyph: Option<char>,
    inactive_range: Option<(usize, usize)>,
) -> Vec<DisplaySegment> {
    let matching_open = compute_matching_openers(items);
    let decimal_glyph = decimal.to_char();
    let mut segments: Vec<DisplaySegment> = Vec::new();
    let mut prev_value_end = false;

    let mut i = 0;
    while i < items.len() {
        let here = &items[i];
        let begins_value_here = item_begins_value(here);
        // A Constant abutting another value-producing item normally
        // suppresses the auto-mul glyph (so `5π` reads cleanly), but
        // when the left side is itself a non-digit value-producer
        // (Constant, Factorial, Percent, `)`) the glyph IS shown so
        // sequences like `π·π` or `5)·π` aren't ambiguous.
        let constant_after_non_digit = prev_value_end
            && i > 0
            && matches!(here, InputItem::Constant(_))
            && !matches!(items[i - 1], InputItem::Digit(_) | InputItem::DecimalPoint);

        if prev_value_end && (begins_value_here || constant_after_non_digit) {
            segments.push(DisplaySegment::inactive("×"));
        }

        let item_start = i;
        match here {
            InputItem::Digit(_) | InputItem::DecimalPoint => {
                let (run, consumed) = extract_numeric_run(&items[i..]);
                let mut s = String::new();
                write_formatted_number(&mut s, &run, decimal_glyph, thousands_glyph);
                segments.push(DisplaySegment::active(s));
                i += consumed;
                prev_value_end = true;
            }
            InputItem::RightParen => {
                let active = match matching_open[i] {
                    Some(opener_idx) => !(cursor > opener_idx && cursor <= i),
                    None => true,
                };
                segments.push(DisplaySegment {
                    text: ")".to_string(),
                    active,
                });
                i += 1;
                prev_value_end = true;
            }
            InputItem::AutoMul => {
                // The buffer materialises auto-multiplication as a real
                // item so backspace, history, and ASCII export all see
                // it. Render it dimmed so the user can tell at a glance
                // that the calculator inserted it on their behalf.
                segments.push(DisplaySegment::inactive("×"));
                i += 1;
                prev_value_end = false;
            }
            other => {
                segments.push(DisplaySegment::active(other.display()));
                i += 1;
                prev_value_end = item_produces_value(other);
            }
        }

        // If this item-derived segment overlaps the inactive range,
        // dim it. Synthetic auto-mul glyphs inserted above don't
        // correspond to a buffer item, so they keep their own
        // (already-inactive) colouring untouched.
        if let Some((rs, re)) = inactive_range {
            if item_start < re && rs < i {
                if let Some(last) = segments.last_mut() {
                    last.active = false;
                }
            }
        }
    }
    segments
}

/// Convenience for tests and callers that only need the flat textual
/// rendering – concatenates every segment's text in order.
pub fn render_expression_string(
    items: &[InputItem],
    decimal: DecimalSeparator,
    thousands_glyph: Option<char>,
) -> String {
    let segs = render_expression(items, items.len(), decimal, thousands_glyph, None);
    let mut s = String::new();
    for seg in segs {
        s.push_str(&seg.text);
    }
    s
}

/// For each item index, record the index of its matching opener when
/// the item is a `RightParen`. Unmatched closers map to `None`. Used to
/// answer "is the cursor inside the bracket pair this `)` closes?".
fn compute_matching_openers(items: &[InputItem]) -> Vec<Option<usize>> {
    let mut out: Vec<Option<usize>> = vec![None; items.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => stack.push(i),
            InputItem::RightParen => {
                if let Some(l) = stack.pop() {
                    out[i] = Some(l);
                }
            }
            _ => {}
        }
    }
    out
}

/// Same predicates the engine tokenizer uses for implicit
/// multiplication, lifted to the InputItem level. Anything that ends a
/// value on the left side counts; anything that starts a value on the
/// right side counts.
///
/// Notes:
///   * Constants (π, 𝑒) DO end a value, so a constant followed by an
///     operand renders an inactive `×` glyph (e.g. `π × 5`).
///   * `Percent` deliberately does NOT end a value: per spec, anything
///     after `%` should attach without an auto-multiplication marker.
fn item_produces_value(it: &InputItem) -> bool {
    matches!(
        it,
        InputItem::Constant(_) | InputItem::RightParen | InputItem::Factorial
    )
}

fn item_begins_value(it: &InputItem) -> bool {
    // Constants (π, 𝑒) deliberately omitted on the right side: per
    // spec they attach to the preceding digit run with no auto-mul
    // glyph between them, even though the engine still inserts an
    // implicit `×` token at evaluation time.
    matches!(
        it,
        InputItem::Digit(_)
            | InputItem::DecimalPoint
            | InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_)
    )
}

/// Collect the longest prefix of `items` that forms a single numeric
/// run (digits + at most one `.`). Returns `(run_as_string,
/// consumed_count)`.
fn extract_numeric_run(items: &[InputItem]) -> (String, usize) {
    let mut s = String::new();
    let mut seen_dot = false;
    let mut n = 0;
    for it in items {
        match it {
            InputItem::Digit(c) => {
                s.push(*c);
                n += 1;
            }
            InputItem::DecimalPoint if !seen_dot => {
                s.push('.');
                seen_dot = true;
                n += 1;
            }
            _ => break,
        }
    }
    (s, n)
}

/// Push `run` into `out`, inserting `thousands` every 3 digits of the
/// integer part and replacing the raw `.` with `decimal`. Runs that
/// start with `.` have no integer part and are emitted unchanged.
/// `thousands` is `None` when the user disables digit grouping.
fn write_formatted_number(out: &mut String, run: &str, decimal: char, thousands: Option<char>) {
    let dot_pos = run.find('.');
    let (int_part, frac_part) = match dot_pos {
        Some(p) => (&run[..p], Some(&run[p + 1..])),
        None => (run, None),
    };
    if int_part.is_empty() {
        out.push(decimal);
        if let Some(frac) = frac_part {
            out.push_str(frac);
        }
        return;
    }
    match thousands {
        Some(sep) => write_with_thousands(out, int_part, sep),
        None => out.push_str(int_part),
    }
    if let Some(frac) = frac_part {
        out.push(decimal);
        out.push_str(frac);
    }
}

/// Append `digits` to `out` with `sep` every 3 characters from the
/// right. `digits` is always ASCII (it comes from a numeric run), so
/// slicing by byte index is safe.
fn write_with_thousands(out: &mut String, digits: &str, sep: char) {
    let len = digits.len();
    if len <= 3 {
        out.push_str(digits);
        return;
    }
    // The first group is whatever does not divide evenly into threes.
    let first_group = len % 3;
    if first_group > 0 {
        out.push_str(&digits[..first_group]);
    }
    for (n, start) in (first_group..len).step_by(3).enumerate() {
        // Separate every group after the first thing written.
        if n > 0 || first_group > 0 {
            out.push(sep);
        }
        out.push_str(&digits[start..start + 3]);
    }
}
