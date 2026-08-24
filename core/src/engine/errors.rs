//! Error constants surfaced to the main display and the typed
//! CalcError enum returned by the evaluator.

use crate::engine::decimal::Decimal;

pub const ERR_OVERFLOW: &str = "Overflow";
pub const ERR_UNDERFLOW: &str = "Underflow";
pub const ERR_INDETERMINATE: &str = "Indeterminate";
pub const ERR_UNDEFINED: &str = "Undefined";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    Overflow,
    Underflow,
    Indeterminate,
    Undefined,
}

impl CalcError {
    /// Return the display string associated with this error.
    pub const fn as_str(self) -> &'static str {
        match self {
            CalcError::Overflow => ERR_OVERFLOW,
            CalcError::Underflow => ERR_UNDERFLOW,
            CalcError::Indeterminate => ERR_INDETERMINATE,
            CalcError::Undefined => ERR_UNDEFINED,
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
