//! AST evaluator.
//!
//! Values are [`Decimal`]s, and the four operations a calculator is
//! mostly asked for — add, subtract, multiply, divide — are carried
//! out in base ten. That is what makes `0.1 + 0.2 - 0.3` exactly zero
//! instead of 5.55e-17: the numbers a person types have exact decimal
//! representations, so arithmetic on them has no binary rounding to
//! accumulate. The same goes for percent, modulo, whole-number powers
//! and small factorials, which are exact decimal operations too.
//!
//! Everything else — the trigonometry, the logarithms, the roots, a
//! power with a fractional exponent — has no decimal algorithm worth
//! writing, so it is handed to `f64` through [`Decimal::to_f64`] and
//! read back through [`from_float`]. The double comes back as the
//! shortest decimal that identifies it, so `√0.01` re-enters the
//! decimal world as `0.1` and the exactness of what follows it is
//! unaffected.
//!
//! IEEE edge cases and out-of-range decimals alike are translated into
//! the four calculator error states (Overflow, Underflow,
//! Indeterminate, Undefined).

use std::f64::consts::{E, PI};

use crate::engine::ast::Node;
use crate::engine::decimal::Decimal;
use crate::engine::errors::{classify, classify_decimal, CalcError};
use crate::engine::gamma::factorial;
use crate::engine::item::{BinOp, BinaryFunc, ConstKind, UnaryFunc};

/// Largest `n` whose factorial still fits the working precision
/// exactly. `22!` is 1124000727777607680000, eighteen significant
/// digits once its trailing zeros are set aside; `23!` needs
/// nineteen, so from there the gamma function takes over.
const MAX_EXACT_FACTORIAL: i64 = 22;

/// Bring an `f64` back into the decimal world, applying the same
/// range checks the binary path always did. The double is read as the
/// shortest decimal that identifies it — see [`Decimal::from_f64`].
fn from_float(x: f64) -> Result<Decimal, CalcError> {
    let checked = classify(x)?;
    Decimal::from_f64(checked).ok_or(CalcError::Undefined)
}

/// Run a binary operation in decimal, falling back to the `f64` form
/// if the decimal one leaves the range it can represent. The fallback
/// is unreachable for values the calculator itself can display, and is
/// here so that arithmetic never has an outcome other than a number or
/// a named error.
fn arithmetic(
    l: Decimal,
    r: Decimal,
    exact: impl Fn(Decimal, Decimal) -> Option<Decimal>,
    binary: impl Fn(f64, f64) -> f64,
) -> Result<Decimal, CalcError> {
    match exact(l, r) {
        Some(value) => classify_decimal(value),
        None => from_float(binary(l.to_f64(), r.to_f64())),
    }
}

/// `x / 100`, the percent operator. A shift of the decimal point, so
/// it is exact for every value: `200 + 10%` is 220 on the nose.
fn percent_scale(x: Decimal) -> Result<Decimal, CalcError> {
    x.scale_by_pow10(-2)
        .map(Ok)
        .unwrap_or_else(|| from_float(x.to_f64() / 100.0))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AngleMode {
    #[default]
    Deg,
    Rad,
}

/// Evaluate `node` under the given angle mode.
pub fn eval(node: &Node, mode: AngleMode) -> Result<Decimal, CalcError> {
    match node {
        Node::Num(v) => classify_decimal(*v),
        // The constants are irrational, so they enter as the decimal
        // nearest their double — which is what every calculator that
        // is not doing symbolic algebra works with.
        Node::Const(ConstKind::Pi) => from_float(PI),
        Node::Const(ConstKind::E) => from_float(E),
        Node::Neg(n) => Ok(-eval(n, mode)?),
        Node::Factorial(n) => {
            let v = eval(n, mode)?;
            decimal_factorial(v)
        }
        Node::Percent(n) => {
            // Standalone percent: x / 100.
            let v = eval(n, mode)?;
            percent_scale(v)
        }
        Node::Mod(a, b) => {
            let l = eval(a, mode)?;
            let r = eval_percent_as_scale(b, mode)?;
            if r.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            // Truncated remainder (C's `fmod`): the result takes the
            // sign of the dividend, so `-7 mod 3` is -1. Note this is
            // deliberately NOT a Euclidean remainder, which would give
            // 2. In decimal it is also exact — `5 mod 3.2` is 1.8, not
            // 1.7999999999999998.
            arithmetic(l, r, Decimal::checked_rem, |a, b| a % b)
        }
        Node::Bin(op, a, b) => eval_binary(*op, a, b, mode),
        Node::UnaryFn(f, x) => eval_unary_fn(*f, x, mode),
        Node::BinaryFn(BinaryFunc::LogBase, base, val) => {
            let b = eval(base, mode)?;
            let v = eval(val, mode)?;
            log_base(b.to_f64(), v.to_f64())
        }
        Node::BinaryFn(BinaryFunc::Root, x, n) => {
            let xv = eval(x, mode)?;
            let nv = eval(n, mode)?;
            nth_root(xv, nv)
        }
        Node::LogN(base, x) => {
            let v = eval(x, mode)?;
            log_base(base.to_f64(), v.to_f64())
        }
    }
}

/// log of `value` to `base`, both already in doubles: a ratio of two
/// natural logarithms, which is where decimals stop being any help.
fn log_base(base: f64, value: f64) -> Result<Decimal, CalcError> {
    // The argument first: it is the one the user keyed, and the one
    // the named errors are about. A bad base — zero, negative, or 1,
    // where the logarithm has no single value — stays the catch-all.
    if value == 0.0 {
        return Err(CalcError::ZeroLog);
    }
    if value < 0.0 {
        return Err(CalcError::NegativeLog);
    }
    if base == 1.0 {
        // Every power of 1 is 1, so the only argument with an answer
        // is 1 itself — and that one has every answer.
        return Err(CalcError::LogBaseOne);
    }
    if base <= 0.0 {
        return Err(CalcError::Undefined);
    }
    from_float(value.ln() / base.ln())
}

/// The check every single-argument logarithm makes on what it is
/// handed: zero and the negatives each answer with their own name, so
/// `ln(0)` and `ln(-1)` no longer read as the same problem.
fn log_argument(v: f64) -> Result<(), CalcError> {
    if v == 0.0 {
        return Err(CalcError::ZeroLog);
    }
    if v < 0.0 {
        return Err(CalcError::NegativeLog);
    }
    Ok(())
}

/// `x!`. Whole numbers up to [`MAX_EXACT_FACTORIAL`] are multiplied
/// out in decimal, so `20!` is every one of its digits rather than the
/// nearest double to it. Everything else — a bigger number, a
/// fractional one — goes to Γ.
fn decimal_factorial(x: Decimal) -> Result<Decimal, CalcError> {
    if let Some(n) = x.to_i64() {
        if (0..=MAX_EXACT_FACTORIAL).contains(&n) {
            let mut product = Decimal::ONE;
            for factor in 2..=n {
                match product.checked_mul(Decimal::from_i64(factor)) {
                    Some(next) => product = next,
                    None => return from_float(factorial(x.to_f64())?),
                }
            }
            return classify_decimal(product);
        }
    }
    from_float(factorial(x.to_f64())?)
}

/// Binary operator evaluation with context-aware percent handling on
/// the right-hand operand.
fn eval_binary(op: BinOp, a: &Node, b: &Node, mode: AngleMode) -> Result<Decimal, CalcError> {
    let l = eval(a, mode)?;
    if let Node::Percent(inner) = b {
        // `200 + 10%` is 220: the right operand is a proportion of the
        // left one rather than a value in its own right.
        let r = percent_scale(eval(inner, mode)?)?;
        return match op {
            BinOp::Add => {
                let share = arithmetic(l, r, Decimal::checked_mul, |a, b| a * b)?;
                arithmetic(l, share, Decimal::checked_add, |a, b| a + b)
            }
            BinOp::Sub => {
                let share = arithmetic(l, r, Decimal::checked_mul, |a, b| a * b)?;
                arithmetic(l, share, Decimal::checked_sub, |a, b| a - b)
            }
            BinOp::Mul => arithmetic(l, r, Decimal::checked_mul, |a, b| a * b),
            BinOp::Div => {
                if r.is_zero() {
                    return Err(CalcError::DivisionByZero);
                }
                arithmetic(l, r, Decimal::checked_div, |a, b| a / b)
            }
            BinOp::Pow => pow_checked(l, r),
        };
    }
    let r = eval(b, mode)?;
    match op {
        BinOp::Add => arithmetic(l, r, Decimal::checked_add, |a, b| a + b),
        BinOp::Sub => arithmetic(l, r, Decimal::checked_sub, |a, b| a - b),
        BinOp::Mul => arithmetic(l, r, Decimal::checked_mul, |a, b| a * b),
        BinOp::Div => {
            if l.is_zero() && r.is_zero() {
                return Err(CalcError::Indeterminate);
            }
            if r.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            arithmetic(l, r, Decimal::checked_div, |a, b| a / b)
        }
        BinOp::Pow => pow_checked(l, r),
    }
}

/// Evaluate `b`, unwrapping a top-level Percent as scale (x/100).
fn eval_percent_as_scale(b: &Node, mode: AngleMode) -> Result<Decimal, CalcError> {
    if let Node::Percent(inner) = b {
        return percent_scale(eval(inner, mode)?);
    }
    eval(b, mode)
}

/// x^y with calculator-specific edge cases: 0^0 undefined, negative
/// base with non-integer exponent undefined.
///
/// A whole-number exponent is multiplied out in decimal, so `1.1²` is
/// 1.21 rather than 1.2100000000000002, and `10^15` is the integer
/// with fifteen zeroes. A fractional one is a root in disguise and
/// goes to `powf`.
fn pow_checked(x: Decimal, y: Decimal) -> Result<Decimal, CalcError> {
    if x.is_zero() && y.is_zero() {
        return Err(CalcError::ZeroPowZero);
    }
    // `0^-n` is `1/0` with an exponent on it. Left to the arithmetic
    // it came back as Overflow — the reciprocal of zero is infinite —
    // which said the answer was too big rather than that there is
    // none.
    if x.is_zero() && y.is_negative() {
        return Err(CalcError::ZeroPowNegative);
    }
    if x.is_negative() && !y.is_integer() {
        return Err(CalcError::Undefined);
    }
    if let Some(exponent) = y.to_i64().and_then(|n| i32::try_from(n).ok()) {
        if let Some(value) = x.checked_powi(exponent) {
            return classify_decimal(value);
        }
    }
    let value = x.to_f64().powf(y.to_f64());
    from_float(value)
}

/// Y-th root of X: X^(1/Y). Handles negative bases with odd-integer
/// roots and reports Undefined for y = 0 or even roots of negatives.
fn nth_root(x: Decimal, y: Decimal) -> Result<Decimal, CalcError> {
    if y.is_zero() {
        return Err(CalcError::Undefined);
    }
    if x.is_zero() {
        return Ok(Decimal::ZERO);
    }
    let (xv, yv) = (x.to_f64(), y.to_f64());
    if x.is_negative() {
        if is_odd_integer(y) {
            return from_float(-(-xv).powf(1.0 / yv));
        }
        return Err(CalcError::NegativeEvenRoot);
    }
    from_float(xv.powf(1.0 / yv))
}

/// True when `y` is a whole number and that number is odd.
///
/// Asking the decimal rather than a double is what keeps
/// `root(-8, 1e30)` undefined: 1e30 has no `i64` to be odd or even in,
/// so it is neither, where a saturating cast would have made it
/// `i64::MAX` and therefore odd.
fn is_odd_integer(y: Decimal) -> bool {
    y.to_i64().map(|n| n % 2 != 0).unwrap_or(false)
}

/// True when `x` is an integer value. Shared with `gamma`, which needs
/// the same test to spot the poles of Γ at the negative integers.
pub(crate) fn is_integer(x: f64) -> bool {
    x.is_finite() && x.floor() == x
}

/// Unary-function dispatch. Every one of these is transcendental (or
/// a root), so the value goes out to `f64` here and the answer comes
/// back through [`from_float`]. DEG/RAD conversion happens inside.
fn eval_unary_fn(f: UnaryFunc, x: &Node, mode: AngleMode) -> Result<Decimal, CalcError> {
    let v = eval(x, mode)?.to_f64();
    from_float(eval_unary_f64(f, v, mode)?)
}

/// The `f64` half of [`eval_unary_fn`].
fn eval_unary_f64(f: UnaryFunc, v: f64, mode: AngleMode) -> Result<f64, CalcError> {
    match f {
        UnaryFunc::Sin => classify(trig_sin(v, mode)),
        UnaryFunc::Cos => classify(trig_cos(v, mode)),
        UnaryFunc::Tan => trig_tan(v, mode),
        UnaryFunc::Cot => trig_cot(v, mode),
        UnaryFunc::Asin => {
            if !(-1.0..=1.0).contains(&v) {
                return Err(CalcError::AsinDomain);
            }
            Ok(from_rad(v.asin(), mode))
        }
        UnaryFunc::Acos => {
            if !(-1.0..=1.0).contains(&v) {
                return Err(CalcError::AcosDomain);
            }
            Ok(from_rad(v.acos(), mode))
        }
        UnaryFunc::Atan => Ok(from_rad(v.atan(), mode)),
        UnaryFunc::Acot => {
            // acot(x) = π/2 - atan(x)
            let r = std::f64::consts::FRAC_PI_2 - v.atan();
            Ok(from_rad(r, mode))
        }
        UnaryFunc::Sinh => classify(v.sinh()),
        UnaryFunc::Cosh => classify(v.cosh()),
        UnaryFunc::Tanh => classify(v.tanh()),
        UnaryFunc::Coth => {
            if v == 0.0 {
                return Err(CalcError::HyperbolicCotangent);
            }
            classify(1.0 / v.tanh())
        }
        UnaryFunc::Asinh => classify(v.asinh()),
        UnaryFunc::Acosh => {
            if v < 1.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.acosh())
        }
        UnaryFunc::Atanh => {
            if v.abs() >= 1.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.atanh())
        }
        UnaryFunc::Acoth => {
            if v.abs() <= 1.0 {
                return Err(CalcError::Undefined);
            }
            classify(0.5 * ((v + 1.0) / (v - 1.0)).ln())
        }
        UnaryFunc::Ln => {
            log_argument(v)?;
            classify(v.ln())
        }
        UnaryFunc::Log | UnaryFunc::Log10 => {
            log_argument(v)?;
            classify(v.log10())
        }
        UnaryFunc::Log2 => {
            log_argument(v)?;
            classify(v.log2())
        }
        UnaryFunc::Sqrt => {
            // The square root is the even root the keypad reaches
            // for most, so it answers with the same name `ʸ√x` does.
            if v < 0.0 {
                return Err(CalcError::NegativeEvenRoot);
            }
            classify(v.sqrt())
        }
        UnaryFunc::Cbrt => classify(v.cbrt()),
    }
}

/// Convert angle argument from calculator mode to radians for libm trig.
/// In DEG mode the argument is reduced mod 360 first so that large
/// integer-degree arguments (e.g. `8!° = 40320°`) keep their precision
/// when multiplied by the irrational `PI/180`. Trig is 360°-periodic so
/// the reduction doesn't change the mathematical result.
fn to_rad(x: f64, mode: AngleMode) -> f64 {
    match mode {
        AngleMode::Deg => {
            let reduced = if x.is_finite() {
                x.rem_euclid(360.0)
            } else {
                x
            };
            reduced * PI / 180.0
        }
        AngleMode::Rad => x,
    }
}

/// Beyond this magnitude (`2^53`), adjacent f64 values can differ by
/// more than 1, so integer-multiple tests like `(x / 180.0).fract() ==
/// 0.0` always succeed regardless of whether `x` is mathematically on a
/// snap point. Skip the snap for such inputs so `cos(86!)` doesn't get
/// pinned to an exact zero just because every very-large f64 looks like
/// a multiple of 180.
const TRIG_SNAP_PRECISION_LIMIT: f64 = (1u64 << 53) as f64;

/// Convert inverse-trig result from radians back to calculator mode.
fn from_rad(x: f64, mode: AngleMode) -> f64 {
    match mode {
        AngleMode::Deg => x * 180.0 / PI,
        AngleMode::Rad => x,
    }
}

/// sin with DEG-mode snapping at the k·π multiples so `sin(π)` and
/// `sin(180°)` both return an exact zero instead of ~1e-16.
fn trig_sin(x: f64, mode: AngleMode) -> f64 {
    let r = to_rad(x, mode);
    let v = r.sin();
    snap_trig_zero(v, x, mode, TrigKind::Sin)
}

fn trig_cos(x: f64, mode: AngleMode) -> f64 {
    let r = to_rad(x, mode);
    let v = r.cos();
    snap_trig_zero(v, x, mode, TrigKind::Cos)
}

fn trig_tan(x: f64, mode: AngleMode) -> Result<f64, CalcError> {
    // tan is undefined where cos == 0 (odd multiples of π/2 in rad or 90° in deg).
    if is_tan_pole(x, mode) {
        return Err(CalcError::Tangent);
    }
    let r = to_rad(x, mode);
    // In DEG mode, snap tan(k·180°) to 0.
    let mut v = r.tan();
    v = snap_trig_zero(v, x, mode, TrigKind::Tan);
    classify(v)
}

fn trig_cot(x: f64, mode: AngleMode) -> Result<f64, CalcError> {
    if is_cot_pole(x, mode) {
        return Err(CalcError::Cotangent);
    }
    let r = to_rad(x, mode);
    let mut v = 1.0 / r.tan();
    // cot(π/2 + k·π) = 0; snap so the user sees an exact zero instead
    // of the ~6e-17 residual that comes out of `1.0 / tan(...)` near
    // the zero of cosine. Same symbolic-π trick used elsewhere.
    if x.is_finite() && x.abs() <= TRIG_SNAP_PRECISION_LIMIT {
        let on_zero = match mode {
            AngleMode::Deg => ((x - 90.0) / 180.0).fract() == 0.0,
            AngleMode::Rad => ((x - PI / 2.0) / PI).fract() == 0.0,
        };
        if on_zero {
            v = 0.0;
        }
    }
    classify(v)
}

#[derive(Copy, Clone)]
enum TrigKind {
    Sin,
    Cos,
    Tan,
}

/// Apply the epsilon trick: if the raw angle x lands on a zero of
/// the function exactly (k·180° for sin/tan, k·180° + 90° for cos in
/// DEG mode; k·π for sin/tan, k·π + π/2 for cos in RAD mode), return
/// an exact 0 instead of the small residual libm produces (≈1e-16).
/// For RAD mode we rely on the fact that π/π is exactly 1.0 in f64,
/// so `x/π` is an integer whenever the user wrote a symbolic
/// multiple of π (e.g. `π`, `2π`, `π÷6` does not snap because 1/6 is
/// not integer).
fn snap_trig_zero(v: f64, x: f64, mode: AngleMode, kind: TrigKind) -> f64 {
    if !x.is_finite() || x.abs() > TRIG_SNAP_PRECISION_LIMIT {
        return v;
    }
    match (mode, kind) {
        (AngleMode::Deg, TrigKind::Sin) | (AngleMode::Deg, TrigKind::Tan) => {
            if (x / 180.0).fract() == 0.0 {
                return 0.0;
            }
        }
        (AngleMode::Deg, TrigKind::Cos) => {
            if ((x - 90.0) / 180.0).fract() == 0.0 {
                return 0.0;
            }
        }
        (AngleMode::Rad, TrigKind::Sin) | (AngleMode::Rad, TrigKind::Tan) => {
            if (x / PI).fract() == 0.0 {
                return 0.0;
            }
        }
        (AngleMode::Rad, TrigKind::Cos) => {
            if ((x - PI / 2.0) / PI).fract() == 0.0 {
                return 0.0;
            }
        }
    }
    v
}

/// True when tan is at a pole given input x under the current mode.
/// In RAD mode this leans on the same f64 trick `snap_trig_zero` uses:
/// `(x - π/2) / π` is exactly an integer only when the user
/// constructed x out of the symbolic `π` constant (e.g. `π÷2`,
/// `3π÷2`); decimal literals that happen to land near π/2 still flow
/// through to libm's tan and produce the usual large finite value
/// rather than spuriously claiming an undefined result.
fn is_tan_pole(x: f64, mode: AngleMode) -> bool {
    if !x.is_finite() || x.abs() > TRIG_SNAP_PRECISION_LIMIT {
        return false;
    }
    match mode {
        AngleMode::Deg => ((x - 90.0) / 180.0).fract() == 0.0,
        AngleMode::Rad => ((x - PI / 2.0) / PI).fract() == 0.0,
    }
}

/// True when cot is at a pole (sin == 0 in mathematical terms). RAD
/// mode mirrors the symbolic-π trick described on `is_tan_pole`.
fn is_cot_pole(x: f64, mode: AngleMode) -> bool {
    if !x.is_finite() || x.abs() > TRIG_SNAP_PRECISION_LIMIT {
        return false;
    }
    match mode {
        AngleMode::Deg => (x / 180.0).fract() == 0.0,
        AngleMode::Rad => (x / PI).fract() == 0.0,
    }
}
