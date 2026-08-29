//! Error constants surfaced to the main display and the typed
//! CalcError enum returned by the evaluator.

use crate::engine::decimal::Decimal;

pub const ERR_OVERFLOW: &str = "Overflow";
pub const ERR_UNDERFLOW: &str = "Underflow";
pub const ERR_INDETERMINATE: &str = "Indeterminate";
pub const ERR_UNDEFINED: &str = "Undefined";

/// The named undefined cases, spelled out on the display.
///
/// A bare "Undefined" says the expression has no value but not which
/// part of it is the problem, and these are the ones a user actually
/// keys by accident: they name the operation and the operand that
/// made it undefined, so the fix is visible without re-deriving it
/// from the expression.
///
/// A zero is written `0` rather than spelled out — it is the digit
/// the user pressed, and the display it lands on is a calculator's.
pub const ERR_NEGATIVE_EVEN_ROOT: &str = "Undefined: Negative number under even root";
pub const ERR_NEGATIVE_LOG: &str = "Undefined: Negative number inside logarithm";
pub const ERR_ZERO_LOG: &str = "Undefined: 0 inside logarithm";
pub const ERR_LOG_BASE_ONE: &str = "Undefined: Logarithm base cannot be 1";
pub const ERR_ZERO_POW_ZERO: &str = "Undefined: 0 raised to 0 power";
pub const ERR_ZERO_POW_NEGATIVE: &str = "Undefined: 0 raised to negative power";
pub const ERR_DIVISION_BY_ZERO: &str = "Undefined: Division by 0";
pub const ERR_TANGENT: &str = "Undefined: Tangent";
pub const ERR_COTANGENT: &str = "Undefined: Cotangent";
pub const ERR_HYPERBOLIC_COTANGENT: &str = "Undefined: Hyperbolic cotangent";
/// The two inverse-trig domains. Written with U+2212, the minus sign
/// the keypad and the display use — escaped rather than typed so it
/// cannot be mistaken here for the ASCII hyphen it looks like.
pub const ERR_ASIN_DOMAIN: &str = "Undefined sin\u{2212}1(x) must be between \u{2212}1 and 1";
pub const ERR_ACOS_DOMAIN: &str = "Undefined cos\u{2212}1(x) must be between \u{2212}1 and 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    Overflow,
    Underflow,
    Indeterminate,
    /// No named case fits: the catch-all, still spelled "Undefined".
    Undefined,
    /// An even root of a negative number — `√(-4)`, `root(-8, 4)`.
    NegativeEvenRoot,
    /// A logarithm of a negative number.
    NegativeLog,
    /// A logarithm of zero.
    ZeroLog,
    /// A logarithm to base 1, which every positive argument but 1
    /// has no answer for and 1 itself has every answer for.
    LogBaseOne,
    /// `0^0`.
    ZeroPowZero,
    /// `0` raised to a negative power, which is a division by zero
    /// wearing an exponent. It used to report Overflow, which said
    /// the answer was too big rather than that there was none.
    ZeroPowNegative,
    /// A division (or a modulo) whose divisor is zero.
    DivisionByZero,
    /// `tan` at one of its poles.
    Tangent,
    /// `cot` at one of its poles.
    Cotangent,
    /// `coth` at its pole, which is the one place it has: zero.
    HyperbolicCotangent,
    /// `sin⁻¹` outside [−1, 1].
    AsinDomain,
    /// `cos⁻¹` outside [−1, 1].
    AcosDomain,
}

impl CalcError {
    /// Return the display string associated with this error.
    pub const fn as_str(self) -> &'static str {
        match self {
            CalcError::Overflow => ERR_OVERFLOW,
            CalcError::Underflow => ERR_UNDERFLOW,
            CalcError::Indeterminate => ERR_INDETERMINATE,
            CalcError::Undefined => ERR_UNDEFINED,
            CalcError::NegativeEvenRoot => ERR_NEGATIVE_EVEN_ROOT,
            CalcError::NegativeLog => ERR_NEGATIVE_LOG,
            CalcError::ZeroLog => ERR_ZERO_LOG,
            CalcError::LogBaseOne => ERR_LOG_BASE_ONE,
            CalcError::ZeroPowZero => ERR_ZERO_POW_ZERO,
            CalcError::ZeroPowNegative => ERR_ZERO_POW_NEGATIVE,
            CalcError::DivisionByZero => ERR_DIVISION_BY_ZERO,
            CalcError::Tangent => ERR_TANGENT,
            CalcError::Cotangent => ERR_COTANGENT,
            CalcError::HyperbolicCotangent => ERR_HYPERBOLIC_COTANGENT,
            CalcError::AsinDomain => ERR_ASIN_DOMAIN,
            CalcError::AcosDomain => ERR_ACOS_DOMAIN,
        }
    }
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Threshold below which a finite, non-zero result is treated as
/// Underflow per spec ("below 1e-308").
pub const UNDERFLOW_THRESHOLD: f64 = 1e-308;

/// Convert an f64 result to Result<f64, CalcError>. NaN is reported as
/// Undefined (Indeterminate is reserved for 0/0, handled explicitly
/// by the Div evaluator). ±∞ becomes Overflow and subnormals below
/// 1e-308 become Underflow.
pub fn classify(x: f64) -> Result<f64, CalcError> {
    if x.is_nan() {
        return Err(CalcError::Undefined);
    }
    if x.is_infinite() {
        return Err(CalcError::Overflow);
    }
    if x != 0.0 && x.abs() < UNDERFLOW_THRESHOLD {
        return Err(CalcError::Underflow);
    }
    Ok(x)
}

/// The same range check for a decimal result. The bounds are the
/// binary ones — an f64 is still what the display and the rest of the
/// app hand around — so a decimal too large to be a double reports
/// Overflow exactly where the arithmetic used to, and one too small
/// reports Underflow.
pub fn classify_decimal(x: Decimal) -> Result<Decimal, CalcError> {
    let Some(adjusted) = x.adjusted_exponent() else {
        return Ok(x); // zero
    };
    if adjusted < -308 {
        return Err(CalcError::Underflow);
    }
    // 1e308 and up may or may not still be a double; ask.
    if adjusted > 308 || (adjusted == 308 && x.to_f64().is_infinite()) {
        return Err(CalcError::Overflow);
    }
    Ok(x)
}
