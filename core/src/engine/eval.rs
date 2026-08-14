//! AST evaluator. Performs all numeric computations in f64 and
//! translates IEEE edge cases into the four calculator error states
//! (Overflow, Underflow, Indeterminate, Undefined).

use std::f64::consts::{E, PI};

use crate::engine::ast::Node;
use crate::engine::errors::{CalcError, classify};
use crate::engine::gamma::factorial;
use crate::engine::item::{BinOp, BinaryFunc, ConstKind, UnaryFunc};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum AngleMode {
    #[default]
    Deg,
    Rad,
}

/// Evaluate `node` under the given angle mode.
pub fn eval(node: &Node, mode: AngleMode) -> Result<f64, CalcError> {
    match node {
        Node::Num(v) => classify(*v),
        Node::Const(ConstKind::Pi) => Ok(PI),
        Node::Const(ConstKind::E) => Ok(E),
        Node::Neg(n) => {
            let v = eval(n, mode)?;
            Ok(-v)
        }
        Node::Factorial(n) => {
            let v = eval(n, mode)?;
            factorial(v)
        }
        Node::Percent(n) => {
            // Standalone percent: x / 100.
            let v = eval(n, mode)?;
            classify(v / 100.0)
        }
        Node::Mod(a, b) => {
            let l = eval(a, mode)?;
            let r = eval_percent_as_scale(b, mode)?;
            if r == 0.0 {
                return Err(CalcError::Undefined);
            }
            // Truncated remainder (C's `fmod`): the result takes the
            // sign of the dividend, so `-7 mod 3` is -1. Note this is
            // deliberately NOT `f64::rem_euclid`, which would give 2.
            classify(l % r)
        }
        Node::Bin(op, a, b) => eval_binary(*op, a, b, mode),
        Node::UnaryFn(f, x) => eval_unary_fn(*f, x, mode),
        Node::BinaryFn(BinaryFunc::LogBase, base, val) => {
            let b = eval(base, mode)?;
            let v = eval(val, mode)?;
            if b <= 0.0 || b == 1.0 || v <= 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.ln() / b.ln())
        }
        Node::BinaryFn(BinaryFunc::Root, x, n) => {
            let xv = eval(x, mode)?;
            let nv = eval(n, mode)?;
            nth_root(xv, nv)
        }
        Node::LogN(base, x) => {
            let v = eval(x, mode)?;
            if *base <= 0.0 || *base == 1.0 || v <= 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.ln() / base.ln())
        }
    }
}

/// Binary operator evaluation with context-aware percent handling on
/// the right-hand operand.
fn eval_binary(op: BinOp, a: &Node, b: &Node, mode: AngleMode) -> Result<f64, CalcError> {
    let l = eval(a, mode)?;
    if let Node::Percent(inner) = b {
        let r = eval(inner, mode)?;
        return match op {
            BinOp::Add => classify(l + l * r / 100.0),
            BinOp::Sub => classify(l - l * r / 100.0),
            BinOp::Mul => classify(l * r / 100.0),
            BinOp::Div => {
                let s = r / 100.0;
                if s == 0.0 {
                    return Err(CalcError::Undefined);
                }
                classify(l / s)
            }
            BinOp::Pow => pow_checked(l, r / 100.0),
        };
    }
    let r = eval(b, mode)?;
    match op {
        BinOp::Add => classify(l + r),
        BinOp::Sub => classify(l - r),
        BinOp::Mul => classify(l * r),
        BinOp::Div => {
            if l == 0.0 && r == 0.0 {
                return Err(CalcError::Indeterminate);
            }
            if r == 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(l / r)
        }
        BinOp::Pow => pow_checked(l, r),
    }
}

/// Evaluate `b`, unwrapping a top-level Percent as scale (x/100).
fn eval_percent_as_scale(b: &Node, mode: AngleMode) -> Result<f64, CalcError> {
    if let Node::Percent(inner) = b {
        let v = eval(inner, mode)?;
        return classify(v / 100.0);
    }
    eval(b, mode)
}

/// x^y with calculator-specific edge cases: 0^0 undefined,
/// negative base with non-integer exponent undefined.
fn pow_checked(x: f64, y: f64) -> Result<f64, CalcError> {
    if x == 0.0 && y == 0.0 {
        return Err(CalcError::Undefined);
    }
    if x < 0.0 && !is_integer(y) {
        return Err(CalcError::Undefined);
    }
    let v = x.powf(y);
    classify(v)
}

/// Y-th root of X: X^(1/Y). Handles negative bases with odd-integer
/// roots and reports Undefined for y = 0 or even roots of negatives.
fn nth_root(x: f64, y: f64) -> Result<f64, CalcError> {
    if y == 0.0 {
        return Err(CalcError::Undefined);
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    if x < 0.0 {
        if is_odd_integer(y) {
            let v = -(-x).powf(1.0 / y);
            return classify(v);
        }
        return Err(CalcError::Undefined);
    }
    classify(x.powf(1.0 / y))
}

/// True when `y` is an integer this f64 can still tell apart from its
/// neighbours *and* that integer is odd.
///
/// Past 2^53 adjacent f64s differ by more than 1, so parity stops being
/// a meaningful property of the value – and a plain `y as i64` cast
/// saturates at `i64::MAX`, which is odd, so `root(-8, 1e30)` used to
/// take the odd branch and return -1 instead of Undefined.
fn is_odd_integer(y: f64) -> bool {
    if !is_integer(y) || y.abs() > TRIG_SNAP_PRECISION_LIMIT {
        return false;
    }
    (y as i64) % 2 != 0
}

/// True when `x` is an integer value. Shared with `gamma`, which needs
/// the same test to spot the poles of Γ at the negative integers.
pub(crate) fn is_integer(x: f64) -> bool {
    x.is_finite() && x.floor() == x
}

/// Unary-function dispatch. DEG/RAD conversion happens inside.
fn eval_unary_fn(f: UnaryFunc, x: &Node, mode: AngleMode) -> Result<f64, CalcError> {
    let v = eval(x, mode)?;
    match f {
        UnaryFunc::Sin => classify(trig_sin(v, mode)),
        UnaryFunc::Cos => classify(trig_cos(v, mode)),
        UnaryFunc::Tan => trig_tan(v, mode),
        UnaryFunc::Cot => trig_cot(v, mode),
        UnaryFunc::Asin => {
            if !(-1.0..=1.0).contains(&v) {
                return Err(CalcError::Undefined);
            }
            Ok(from_rad(v.asin(), mode))
        }
        UnaryFunc::Acos => {
            if !(-1.0..=1.0).contains(&v) {
                return Err(CalcError::Undefined);
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
                return Err(CalcError::Undefined);
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
            if v <= 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.ln())
        }
        UnaryFunc::Log | UnaryFunc::Log10 => {
            if v <= 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.log10())
        }
        UnaryFunc::Log2 => {
            if v <= 0.0 {
                return Err(CalcError::Undefined);
            }
            classify(v.log2())
        }
        UnaryFunc::Sqrt => {
            if v < 0.0 {
                return Err(CalcError::Undefined);
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
        return Err(CalcError::Undefined);
    }
    let r = to_rad(x, mode);
    // In DEG mode, snap tan(k·180°) to 0.
    let mut v = r.tan();
    v = snap_trig_zero(v, x, mode, TrigKind::Tan);
    classify(v)
}

fn trig_cot(x: f64, mode: AngleMode) -> Result<f64, CalcError> {
    if is_cot_pole(x, mode) {
        return Err(CalcError::Undefined);
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
