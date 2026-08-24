//! Fixed-precision decimal arithmetic: the number the calculator adds,
//! subtracts, multiplies and divides.
//!
//! A value is `mantissa × 10^exponent`, the mantissa an `i128` kept to
//! at most [`WORKING_DIGITS`] significant digits and the exponent an
//! `i32`. That is base ten, so the numbers a person types have exact
//! representations: `0.1` is one tenth here, not the binary fraction
//! nearest to it, and `0.1 + 0.2 - 0.3` is zero rather than 5.55e-17.
//! The display shows fifteen digits, so the three extra ones are guard
//! digits — room for the rounding of a division to stay out of sight,
//! which is what lets `1÷3×3` come back as 1.
//!
//! Everything a decimal cannot do — the trigonometry, the logarithms,
//! the roots and the non-integer powers — goes out to `f64` and comes
//! back through [`Decimal::from_f64`], which reads the double as the
//! shortest decimal that identifies it. `√0.01` therefore returns to
//! the decimal world as `0.1`, not as `0.1000000000000000055…`, and
//! the exactness of what follows it is unaffected.
//!
//! Every operation is `checked_*` and answers `None` rather than
//! wrapping or panicking, so a caller can fall back to `f64`. For
//! values inside the calculator's own range that fallback is
//! unreachable — the bounds below keep the widest intermediate a
//! factor of ten inside `i128` — but a fallback that cannot be hit is
//! cheaper than a proof that it cannot.

/// Significant digits kept in a mantissa. Three more than the display
/// shows, which is what makes the last rounding invisible.
pub const WORKING_DIGITS: u32 = 18;

/// Widest gap in decimal scale across which two values can still
/// affect each other's rounded sum. Beyond it the smaller one lands
/// past every digit the larger one keeps — including the digit the
/// rounding looks at — so it can be dropped. Bounding the gap is also
/// what bounds the intermediate: aligning two 18-digit mantissas
/// across 19 places of scale needs 37 digits, and an `i128` holds 38.
const NEGLIGIBLE_SCALE_GAP: i32 = WORKING_DIGITS as i32 + 1;

/// Widest exponent a literal can name. Past this the value is out of
/// range whatever its digits are, and clamping keeps the arithmetic
/// that follows well inside `i32`.
const EXPONENT_LIMIT: i64 = 1_000_000_000;

/// Digits read from a literal before the rest is dropped. Two past the
/// working precision, so the rounding to it has a digit to look at and
/// one to spare.
const PARSE_DIGITS: u32 = WORKING_DIGITS + 2;

/// Powers of ten that fit an `i128`, indexed by exponent.
const POW10: [i128; 39] = {
    let mut table = [1i128; 39];
    let mut i = 1;
    while i < 39 {
        table[i] = table[i - 1] * 10;
        i += 1;
    }
    table
};

/// `10^n`, or `None` when that is wider than an `i128`.
fn pow10(n: u32) -> Option<i128> {
    POW10.get(n as usize).copied()
}

/// Number of decimal digits in `|m|`; zero has one.
fn digit_count(m: i128) -> u32 {
    let m = m.unsigned_abs();
    let mut digits = 1;
    let mut bound = 10u128;
    while m >= bound {
        digits += 1;
        match bound.checked_mul(10) {
            Some(next) => bound = next,
            None => break,
        }
    }
    digits
}

/// A decimal number: `mantissa × 10^exponent`. The default is zero.
///
/// Constructed only through the functions below, all of which restore
/// the invariant that the mantissa has at most [`WORKING_DIGITS`]
/// digits and carries no trailing zeros. That canonical form makes
/// equality and ordering straightforward and keeps the intermediates
/// of the next operation as small as they can be.
#[derive(Debug, Clone, Copy, Default)]
pub struct Decimal {
    mantissa: i128,
    exponent: i32,
}

impl Decimal {
    pub const ZERO: Decimal = Decimal {
        mantissa: 0,
        exponent: 0,
    };
    pub const ONE: Decimal = Decimal {
        mantissa: 1,
        exponent: 0,
    };

    /// Build from a raw mantissa and exponent, rounding to
    /// [`WORKING_DIGITS`] significant digits.
    pub fn new(mantissa: i128, exponent: i32) -> Decimal {
        Decimal::with_digits(mantissa, exponent, WORKING_DIGITS)
    }

    /// As [`Decimal::new`] but to an explicit digit budget, rounding
    /// half away from zero — the way a calculator rounds, and in one
    /// step, so a long tail cannot round twice and land a digit out.
    fn with_digits(mut mantissa: i128, mut exponent: i32, digits: u32) -> Decimal {
        if mantissa == 0 {
            return Decimal::ZERO;
        }
        let digits = digits.max(1);
        let have = digit_count(mantissa);
        if have > digits {
            let drop = have - digits;
            // `drop < 39` because `have` counts the digits of an i128.
            let scale = pow10(drop).unwrap_or(i128::MAX);
            let remainder = (mantissa % scale).abs();
            mantissa /= scale;
            // Powers of ten above 1 are even, so half is exact.
            if remainder >= scale / 2 {
                mantissa += mantissa.signum();
            }
            exponent = exponent.saturating_add(drop as i32);
            // Rounding up can carry into a new digit (99 → 100); the
            // one it gains is a zero, so dropping it is exact.
            if digit_count(mantissa) > digits {
                mantissa /= 10;
                exponent = exponent.saturating_add(1);
            }
        }
        while mantissa != 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            exponent = exponent.saturating_add(1);
        }
        if mantissa == 0 {
            return Decimal::ZERO;
        }
        Decimal { mantissa, exponent }
    }

    /// Whole number as a decimal.
    pub fn from_i64(v: i64) -> Decimal {
        Decimal::new(v as i128, 0)
    }

    pub fn mantissa(self) -> i128 {
        self.mantissa
    }

    pub fn exponent(self) -> i32 {
        self.exponent
    }

    /// Number of significant digits the mantissa carries.
    pub fn digits(self) -> u32 {
        digit_count(self.mantissa)
    }

    /// Exponent the value would have written as `d.ddd × 10^n`: the
    /// place of its leading digit. `None` for zero, which has none.
    pub fn adjusted_exponent(self) -> Option<i32> {
        if self.is_zero() {
            return None;
        }
        Some(self.exponent + self.digits() as i32 - 1)
    }

    pub fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    pub fn is_negative(self) -> bool {
        self.mantissa < 0
    }

    pub fn signum(self) -> i32 {
        self.mantissa.signum() as i32
    }

    pub fn abs(self) -> Decimal {
        Decimal {
            mantissa: self.mantissa.abs(),
            exponent: self.exponent,
        }
    }

    /// True when the value has no fractional part.
    pub fn is_integer(self) -> bool {
        self.is_zero() || self.exponent >= 0
    }

    /// The value with its fractional digits dropped, towards zero.
    pub fn trunc(self) -> Decimal {
        if self.exponent >= 0 || self.is_zero() {
            return self;
        }
        let drop = (-self.exponent) as u32;
        match pow10(drop) {
            Some(scale) => Decimal::new(self.mantissa / scale, 0),
            // More fractional places than an i128 has digits: all of
            // them are fractional, so the integer part is zero.
            None => Decimal::ZERO,
        }
    }

    /// The value as an `i64`, or `None` when it is fractional or too
    /// large — the parity and integer-exponent questions the evaluator
    /// asks, answered only where the answer means something.
    pub fn to_i64(self) -> Option<i64> {
        if self.is_zero() {
            return Some(0);
        }
        if self.exponent < 0 {
            let scale = pow10((-self.exponent) as u32)?;
            if self.mantissa % scale != 0 {
                return None;
            }
            return i64::try_from(self.mantissa / scale).ok();
        }
        let scale = pow10(u32::try_from(self.exponent).ok()?)?;
        i64::try_from(self.mantissa.checked_mul(scale)?).ok()
    }

    /// Multiply by a power of ten — a shift of the decimal point, so
    /// no digit of the mantissa moves and nothing is rounded. `None`
    /// only if the exponent would leave `i32`.
    pub fn scale_by_pow10(self, places: i32) -> Option<Decimal> {
        if self.is_zero() {
            return Some(Decimal::ZERO);
        }
        Some(Decimal {
            mantissa: self.mantissa,
            exponent: self.exponent.checked_add(places)?,
        })
    }

    /// Round to `digits` significant digits — the display formatter's
    /// one way in to the rounding.
    pub fn round_to_digits(self, digits: u32) -> Decimal {
        Decimal::with_digits(self.mantissa, self.exponent, digits)
    }

    // -----------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------

    pub fn checked_add(self, other: Decimal) -> Option<Decimal> {
        if self.is_zero() {
            return Some(other);
        }
        if other.is_zero() {
            return Some(self);
        }
        let (a, b, exponent) = align(self, other)?;
        Some(Decimal::new(a.checked_add(b)?, exponent))
    }

    pub fn checked_sub(self, other: Decimal) -> Option<Decimal> {
        self.checked_add(-other)
    }

    pub fn checked_mul(self, other: Decimal) -> Option<Decimal> {
        if self.is_zero() || other.is_zero() {
            return Some(Decimal::ZERO);
        }
        let mantissa = self.mantissa.checked_mul(other.mantissa)?;
        let exponent = self.exponent.checked_add(other.exponent)?;
        Some(Decimal::new(mantissa, exponent))
    }

    /// Division to the working precision: exact when the quotient
    /// terminates inside it (`1÷8` is `0.125`, not `0.12499…`) and
    /// rounded when it does not (`1÷3` keeps eighteen threes). `None`
    /// when the divisor is zero — the evaluator has its own answer for
    /// that.
    pub fn checked_div(self, other: Decimal) -> Option<Decimal> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Decimal::ZERO);
        }
        // Scale the numerator so the truncated quotient carries two
        // digits more than the working precision, leaving the rounding
        // in `new` a digit to look at and one to spare.
        let wanted = PARSE_DIGITS as i32;
        let shift = wanted + other.digits() as i32 - self.digits() as i32;
        let (numerator, exponent) = if shift > 0 {
            (
                self.mantissa
                    .checked_mul(pow10(u32::try_from(shift).ok()?)?)?,
                self.exponent.checked_sub(shift)?,
            )
        } else {
            (self.mantissa, self.exponent)
        };
        Some(Decimal::new(
            numerator / other.mantissa,
            exponent.checked_sub(other.exponent)?,
        ))
    }

    /// Truncated remainder, matching C's `fmod`: the result takes the
    /// sign of the dividend. `None` when the divisor is zero or the
    /// quotient is too large to take an exact integer part of, which
    /// is past where the answer means anything anyway.
    pub fn checked_rem(self, other: Decimal) -> Option<Decimal> {
        let quotient = self.checked_div(other)?.trunc();
        // A quotient wide enough to have been rounded cannot give an
        // exact remainder.
        quotient.to_i64()?;
        self.checked_sub(quotient.checked_mul(other)?)
    }

    /// Raised to a whole-number power, by squaring. `None` when an
    /// intermediate leaves the range, so the caller can fall back to
    /// the `f64` path.
    pub fn checked_powi(self, exponent: i32) -> Option<Decimal> {
        if exponent < 0 {
            let positive = self.checked_powi(exponent.checked_neg()?)?;
            return Decimal::ONE.checked_div(positive);
        }
        let mut result = Decimal::ONE;
        let mut base = self;
        let mut n = exponent as u32;
        while n > 0 {
            if n & 1 == 1 {
                result = result.checked_mul(base)?;
            }
            n >>= 1;
            if n > 0 {
                base = base.checked_mul(base)?;
            }
        }
        Some(result)
    }

    // -----------------------------------------------------------------
    // Conversion
    // -----------------------------------------------------------------

    /// The `f64` nearest this value. Goes through the decimal literal
    /// so the conversion is the correctly-rounded one Rust's parser
    /// performs, rather than a multiplication that would round twice
    /// and overflow past 1e308.
    pub fn to_f64(self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        self.to_literal().parse().unwrap_or(f64::NAN)
    }

    /// Read a double as the shortest decimal that identifies it, which
    /// is the decimal it was standing in for. `0.1_f64` comes back as
    /// one tenth, and a root that landed exactly on `0.5` comes back
    /// as one half rather than as its binary expansion. `None` for NaN
    /// and the infinities, which are not decimals.
    pub fn from_f64(value: f64) -> Option<Decimal> {
        if !value.is_finite() {
            return None;
        }
        if value == 0.0 {
            return Some(Decimal::ZERO);
        }
        // `{:e}` is the shortest round-tripping form, and unlike `{}`
        // it stays short for very small values: `1e-300` rather than
        // three hundred literal zeroes.
        Decimal::parse(&format!("{value:e}"))
    }

    /// Parse a decimal literal: digits, an optional `.` fraction and
    /// an optional `e` exponent. `None` when the text is not one.
    ///
    /// A literal with more digits than the working precision is
    /// rounded to it, so pasting a forty-digit number is not an error.
    pub fn parse(text: &str) -> Option<Decimal> {
        let text = text.trim();
        let (text, negative) = match text.strip_prefix('-') {
            Some(rest) => (rest, true),
            None => (text.strip_prefix('+').unwrap_or(text), false),
        };
        let (digits, exponent_text) = match text.find(['e', 'E']) {
            Some(at) => (&text[..at], Some(&text[at + 1..])),
            None => (text, None),
        };
        let (integer, fraction) = match digits.find('.') {
            Some(at) => (&digits[..at], &digits[at + 1..]),
            None => (digits, ""),
        };
        if integer.is_empty() && fraction.is_empty() {
            return None;
        }
        if !integer.is_empty() && !integer.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if !fraction.is_empty() && !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }

        // The digits, with the point's place remembered rather than
        // stored: `12.34` is 1234 × 10^-2.
        let all: Vec<u8> = integer.bytes().chain(fraction.bytes()).collect();
        let first_significant = all.iter().position(|b| *b != b'0');
        let Some(first) = first_significant else {
            return Some(Decimal::ZERO);
        };
        // Read only what can matter. Everything past it is rounded
        // away anyway, and reading a long paste in full would overflow
        // the mantissa.
        let end = (first + PARSE_DIGITS as usize).min(all.len());
        let mut mantissa: i128 = 0;
        for byte in &all[first..end] {
            mantissa = mantissa * 10 + (byte - b'0') as i128;
        }
        // Digits dropped off the right count as scale, not as value.
        let dropped = (all.len() - end) as i32;
        let mut exponent = dropped - fraction.len() as i32;

        if let Some(text) = exponent_text {
            if text.is_empty() {
                return None;
            }
            // Clamped rather than refused: `1e999999999` is a number
            // this cannot hold, but it is still a number, and the
            // range check has a name for it (Overflow) that a parse
            // failure does not.
            let extra: i64 = text.parse().ok()?;
            exponent = exponent.saturating_add(extra.clamp(-EXPONENT_LIMIT, EXPONENT_LIMIT) as i32);
        }
        if negative {
            mantissa = -mantissa;
        }
        Some(Decimal::new(mantissa, exponent))
    }

    /// The value written out in full: every digit the mantissa holds,
    /// in plain or exponential form depending on scale. Round-trips
    /// through [`Decimal::parse`] exactly, which is what the input
    /// buffer's exact-value annotation rests on.
    pub fn to_literal(self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        if self.exponent == 0 {
            return self.mantissa.to_string();
        }
        let sign = if self.is_negative() { "-" } else { "" };
        let digits = self.mantissa.unsigned_abs();
        format!("{sign}{digits}e{}", self.exponent)
    }
}

/// Bring two values onto one exponent so their mantissas can be added,
/// dropping whichever is too small in scale to reach the other's last
/// kept digit. Returns `(mantissa_a, mantissa_b, exponent)`.
fn align(a: Decimal, b: Decimal) -> Option<(i128, i128, i32)> {
    let (adjusted_a, adjusted_b) = (a.adjusted_exponent()?, b.adjusted_exponent()?);
    if adjusted_a - adjusted_b > NEGLIGIBLE_SCALE_GAP {
        return Some((a.mantissa, 0, a.exponent));
    }
    if adjusted_b - adjusted_a > NEGLIGIBLE_SCALE_GAP {
        return Some((0, b.mantissa, b.exponent));
    }
    let exponent = a.exponent.min(b.exponent);
    let scale_a = pow10(u32::try_from(a.exponent - exponent).ok()?)?;
    let scale_b = pow10(u32::try_from(b.exponent - exponent).ok()?)?;
    Some((
        a.mantissa.checked_mul(scale_a)?,
        b.mantissa.checked_mul(scale_b)?,
        exponent,
    ))
}

// ---------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------

impl std::ops::Neg for Decimal {
    type Output = Decimal;

    fn neg(self) -> Decimal {
        Decimal {
            mantissa: -self.mantissa,
            exponent: self.exponent,
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        // Canonical form: equal values have equal parts.
        self.mantissa == other.mantissa && self.exponent == other.exponent
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let (sign_a, sign_b) = (self.signum(), other.signum());
        if sign_a != sign_b {
            return sign_a.cmp(&sign_b);
        }
        if sign_a == 0 {
            return Ordering::Equal;
        }
        // Same sign, neither zero: the leading digit's place decides
        // unless both sit at the same one.
        let by_scale = self
            .adjusted_exponent()
            .cmp(&other.adjusted_exponent())
            .then_with(|| match align(*self, *other) {
                Some((a, b, _)) => a.abs().cmp(&b.abs()),
                // Unreachable once the scales agree.
                None => Ordering::Equal,
            });
        if self.is_negative() {
            by_scale.reverse()
        } else {
            by_scale
        }
    }
}

impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_literal())
    }
}
