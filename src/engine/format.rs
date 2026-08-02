//! Render an f64 for the main display. Rounds to the configured
//! number of decimal digits, strips redundant zeroes, and switches to
//! scientific notation when the value is very large or very small.
//!
//! The rounding applies to the DISPLAYED value only – the evaluator
//! operates on full f64 precision throughout the computation.

/// Default digit count applied by the formatter and by Config.
pub const DEFAULT_ROUNDING_DECIMALS: u8 = 14;

/// Switch to scientific notation once the integer part is this large
/// in digit count. Corresponds to the spec's SCI_NOTATION_THRESHOLD.
pub const SCI_THRESHOLD_DIGITS: u8 = 15;

/// Format an f64 for the main display.
pub fn format_result(x: f64, rounding_decimals: u8) -> String {
    if x == 0.0 || x == -0.0 {
        return "0".to_string();
    }
    let abs = x.abs();

    // Small magnitudes: always scientific notation.
    if abs < 1e-4 {
        return format_sci(x, rounding_decimals);
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
            return format_sci(x, rounding_decimals);
        }
        // Fall through to fixed for values like 1000000000000001.
    }

    format_fixed(x, rounding_decimals)
}

/// Fixed-point rendering with trailing-zero trimming and leading-zero
/// preservation in the integer part (at least one digit kept).
fn format_fixed(x: f64, rounding_decimals: u8) -> String {
    let s = format!("{:.*}", rounding_decimals as usize, x);
    trim_number(&s).to_string()
}

/// Scientific-notation rendering: mantissa × 10^exp with the mantissa
/// trimmed and `e` + integer exponent appended. An exponent of zero
/// collapses to pure fixed-point.
fn format_sci(x: f64, rounding_decimals: u8) -> String {
    let abs = x.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = x / 10f64.powi(exp);
    let m_str = format!("{:.*}", rounding_decimals as usize, mantissa);
    let m_trim = trim_number(&m_str);
    if exp == 0 {
        m_trim.to_string()
    } else {
        format!("{}e{}", m_trim, exp)
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
    let int_final = if int_trimmed.is_empty() { "0" } else { int_trimmed };

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
