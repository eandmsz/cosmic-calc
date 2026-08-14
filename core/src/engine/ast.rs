//! AST produced by the parser and consumed by the evaluator.
//! Percent is represented as a tagged node so the evaluator can
//! apply its context-dependent semantics on the right-hand side of
//! a binary operator.

use crate::engine::item::{BinOp, BinaryFunc, ConstKind, UnaryFunc};

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Numeric literal.
    Num(f64),
    /// π or 𝑒.
    Const(ConstKind),
    /// Unary negation.
    Neg(Box<Node>),
    /// Binary arithmetic operator ('+', '-', '*', '/', '^').
    Bin(BinOp, Box<Node>, Box<Node>),
    /// Modulo (the `%` character when it acts as a binary operator).
    Mod(Box<Node>, Box<Node>),
    /// Postfix factorial `x!`.
    Factorial(Box<Node>),
    /// Postfix percent `x%`. Final semantics depend on context —
    /// see eval.rs for details.
    Percent(Box<Node>),
    /// Single-argument function application (sin, cos, sqrt, log, …).
    UnaryFn(UnaryFunc, Box<Node>),
    /// Two-argument function application (log(base,x), root(x,n)).
    BinaryFn(BinaryFunc, Box<Node>, Box<Node>),
    /// log with an integer base baked into the function name
    /// (log2, log6, log10, …).
    LogN(f64, Box<Node>),
}
