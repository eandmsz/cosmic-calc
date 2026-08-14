//! Render an f64 for the main display. Rounds to the configured number
//! of *significant* digits, strips redundant zeroes, and switches to
//! scientific notation when the value is very large or very small.
//!
//! The rounding applies to the DISPLAYED value only – the evaluator
//! operates on full f64 precision throughout the computation.
//!
//! Significant digits, not digits after the decimal point: an f64 backs
//! roughly 15–17 significant digits in total, so a value with an
//! integer part has correspondingly fewer to spend on its fraction.
//! Rounding to a fixed count of decimals regardless of magnitude prints
//! the binary representation error instead of hiding it – `8.2 + 8.2`
//! renders as `16.399999999999999` at 15 decimals but as `16.4` at 15
//! significant digits.

use crate::engine::errors::{ERR_OVERFLOW, ERR_UNDEFINED};

/// Default significant-digit count applied by the formatter and by
/// Config. An f64 carries 15.95 decimal digits; 15 is the largest count
/// that is reliable across the whole range.
pub const DEFAULT_SIGNIFICANT_DIGITS: u8 = 15;

/// Switch to scientific notation once the integer part is this large
/// in digit count. Corresponds to the spec's SCI_NOTATION_THRESHOLD.
pub const SCI_THRESHOLD_DIGITS: u8 = 15;

/// Upper bound on the digits we ask `{:.*}` for. The fixed-point path
/// only runs for magnitudes at or above 1e-4, so the real ceiling is
/// well under this; the clamp just keeps a hand-edited config from
/// requesting an absurd format width.
const MAX_FRACTION_DIGITS: i32 = 30;

/// Format an f64 for the main display.
pub fn format_result(x: f64, significant_digits: u8) -> String {
    // `classify` normally turns these into errors long before the
    // formatter sees them, but `Memory` accumulates raw f64s and can
    // reach them without a second pass through the evaluator.
    if x.is_nan() {
        return ERR_UNDEFINED.to_string();
    }
    if x.is_infinite() {
        return ERR_OVERFLOW.to_string();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let abs = x.abs();

    // Small magnitudes: always scientific notation.
    if abs < 1e-4 {
        return format_sci(x, significant_digits);
    }

    // Large magnitudes: use scientific when the raw integer form would
    // end in trailing zeros (the sci form is then strictly shorter) or
    // when the integer part would exceed SCI_THRESHOLD_DIGITS digits
    // without any possible compression.
    let threshold = 10f64.powi(SCI_THRESHOLD_DIGITS as i32);
    if abs >= threshold {
        let int_str = format!("{:.0}", abs);
        let len = int_str.len();
        let trailing_zeros = int_str.chars().rev().take_while(|c| *c == '0').count();
        if len > SCI_THRESHOLD_DIGITS as usize + 1 || trailing_zeros > 0 {
            return format_sci(x, significant_digits);
        }
        // Fall through to fixed for values like 1000000000000001.
    }

    format_fixed(x, significant_digits)
}

/// Fixed-point rendering with trailing-zero trimming and leading-zero
/// preservation in the integer part (at least one digit kept).
fn format_fixed(x: f64, significant_digits: u8) -> String {
    let decimals = fraction_digits_for(x, significant_digits);
    let s = format!("{:.*}", decimals, x);
    trim_number(&s)
}

/// How many digits after the point still fall inside the
/// significant-digit budget at this magnitude. A value in the hundreds
/// spends three digits before the point, so it keeps `budget - 3`
/// after it; a value below 1 gets the leading zeros back.
fn fraction_digits_for(x: f64, significant_digits: u8) -> usize {
    let budget = significant_digits.max(1) as i32;
    let exp = decimal_exponent(x.abs());
    (budget - 1 - exp).clamp(0, MAX_FRACTION_DIGITS) as usize
}

/// `floor(log10(abs))`, corrected at the powers of ten. `log10` alone
/// is not guaranteed to land exactly on an integer for values like
/// `1000.0`, and being one out there would cost (or invent) a whole
/// digit of displayed precision.
fn decimal_exponent(abs: f64) -> i32 {
    if abs == 0.0 || !abs.is_finite() {
        return 0;
    }
    let mut exp = abs.log10().floor() as i32;
    if 10f64.powi(exp + 1) <= abs {
        exp += 1;
    } else if 10f64.powi(exp) > abs {
        exp -= 1;
    }
    exp
}

/// Scientific-notation rendering: mantissa × 10^exp with the mantissa
/// trimmed and `e` + integer exponent appended. An exponent of zero
/// collapses to pure fixed-point.
///
/// Built on Rust's `{:e}`, which is correctly rounded across the whole
/// f64 range. Recovering the mantissa as `x / 10f64.powi(exp)` instead
/// divides by zero once the exponent drops below about -308, which
/// rendered subnormals as the literal string `infe-314`.
fn format_sci(x: f64, significant_digits: u8) -> String {
    // One digit sits before the point, so the rest of the budget goes
    // after it.
    let decimals = significant_digits.max(1) as usize - 1;
    let raw = format!("{:.*e}", decimals, x);
    let Some((mantissa, exp)) = raw.split_once('e') else {
        return trim_number(&raw);
    };
    let m = trim_number(mantissa);
    if exp == "0" {
        m
    } else {
        format!("{m}e{exp}")
    }
}

/// Trim trailing zeros from the fractional part and a dangling decimal
/// point, and strip leading zeros from the integer part (keeping at
/// least one digit there).
fn trim_number(s: &str) -> String {
    let (sign, body) = if let Some(rest) = s.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", s)
    };

    let (int_part, frac_part) = match body.find('.') {
        Some(idx) => (&body[..idx], Some(&body[idx + 1..])),
        None => (body, None),
    };

    let int_trimmed = int_part.trim_start_matches('0');
    let int_final = if int_trimmed.is_empty() {
        "0"
    } else {
        int_trimmed
    };

    match frac_part {
        Some(frac) => {
            let frac_trimmed = frac.trim_end_matches('0');
            if frac_trimmed.is_empty() {
                format!("{}{}", sign, int_final)
            } else {
                format!("{}{}.{}", sign, int_final, frac_trimmed)
            }
        }
        None => format!("{}{}", sign, int_final),
    }
}
