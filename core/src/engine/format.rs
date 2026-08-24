//! Render a [`Decimal`] for the main display. Rounds to the configured
//! number of *significant* digits, strips redundant zeroes, and
//! switches to scientific notation when the value is very large or
//! very small.
//!
//! The rounding applies to the DISPLAYED value only – the evaluator
//! works at [`crate::engine::decimal::WORKING_DIGITS`] throughout, and
//! the difference between the two is what keeps the last digit of a
//! division out of sight.
//!
//! Significant digits, not digits after the decimal point: the value
//! carries a fixed number of digits wherever its decimal point is, so
//! one with an integer part has correspondingly fewer to spend on its
//! fraction. Rounding to a fixed count of decimals regardless of
//! magnitude prints the arithmetic's own tail instead of hiding it.

use crate::engine::decimal::Decimal;

/// Default significant-digit count applied by the formatter and by
/// Config. The working precision carries three more, which is what
/// makes the rounding of a non-terminating division invisible.
pub const DEFAULT_SIGNIFICANT_DIGITS: u8 = 15;

/// Switch to scientific notation once the integer part is this large
/// in digit count. Corresponds to the spec's SCI_NOTATION_THRESHOLD.
pub const SCI_THRESHOLD_DIGITS: u8 = 15;

/// Format a decimal for the main display.
///
/// Which form to use is decided from the value as computed, not from
/// the value as rounded: a whole number of sixteen digits is shown in
/// full (`1000000000000000 + 1` is not `1e15`), and one that rounding
/// would carry up to a power of ten keeps its positional form.
pub fn format_result(x: Decimal, significant_digits: u8) -> String {
    let budget = significant_digits.max(1) as u32;
    let Some(adjusted) = x.adjusted_exponent() else {
        return "0".to_string();
    };

    // Large magnitudes: scientific when the integer form would end in
    // trailing zeros (the sci form is then strictly shorter) or when
    // it would run past SCI_THRESHOLD_DIGITS digits with nothing to
    // compress. A canonical mantissa carries no trailing zeros, so a
    // positive exponent is exactly "the integer form ends in zeros".
    if adjusted >= SCI_THRESHOLD_DIGITS as i32 {
        // The fractional digits of a number this big are past what is
        // shown either way, so the comparison is between whole ones.
        let whole = x.round_to_digits((adjusted + 1) as u32);
        let integer_digits = whole.adjusted_exponent().map_or(1, |a| a + 1);
        if integer_digits > SCI_THRESHOLD_DIGITS as i32 + 1 || whole.exponent() > 0 {
            return format_sci(x.round_to_digits(budget));
        }
        return format_fixed(whole);
    }

    // Small magnitudes: always scientific notation. `adjusted` is the
    // place of the leading digit, so this is `abs < 1e-4`.
    if adjusted < -4 {
        return format_sci(x.round_to_digits(budget));
    }

    format_fixed(x.round_to_digits(budget))
}

/// Plain positional rendering: the mantissa's digits with the point
/// put back where the exponent says. Trailing zeros are already gone —
/// the canonical form has none — so nothing needs trimming.
fn format_fixed(x: Decimal) -> String {
    let sign = if x.is_negative() { "-" } else { "" };
    let digits = x.mantissa().unsigned_abs().to_string();
    let exponent = x.exponent();
    if exponent >= 0 {
        return format!("{sign}{digits}{}", "0".repeat(exponent as usize));
    }
    let places = (-exponent) as usize;
    if places < digits.len() {
        let split = digits.len() - places;
        format!("{sign}{}.{}", &digits[..split], &digits[split..])
    } else {
        // Leading zeros between the point and the first digit.
        let zeros = "0".repeat(places - digits.len());
        format!("{sign}0.{zeros}{digits}")
    }
}

/// Scientific-notation rendering: one digit before the point, the rest
/// after it, and `e` plus the exponent. An exponent of zero collapses
/// to pure fixed-point.
fn format_sci(x: Decimal) -> String {
    let Some(adjusted) = x.adjusted_exponent() else {
        return "0".to_string();
    };
    let sign = if x.is_negative() { "-" } else { "" };
    let digits = x.mantissa().unsigned_abs().to_string();
    let mantissa = if digits.len() == 1 {
        digits
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    if adjusted == 0 {
        format!("{sign}{mantissa}")
    } else {
        format!("{sign}{mantissa}e{adjusted}")
    }
}
