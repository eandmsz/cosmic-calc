//! Input-layer item types. Every item carries a kind that dictates
//! how many operands it consumes: 0 for leaves (digits, constants,
//! parens), 1 for unary prefix/postfix ops, 2 for binary operators
//! and binary functions. The tokenizer and parser use this to build
//! the AST.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryFunc {
    Sin,
    Cos,
    Tan,
    Cot,
    Asin,
    Acos,
    Atan,
    Acot,
    Sinh,
    Cosh,
    Tanh,
    Coth,
    Asinh,
    Acosh,
    Atanh,
    Acoth,
    Ln,
    Log,
    Log2,
    Log10,
    Sqrt,
    Cbrt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryFunc {
    /// log(base, value)
    LogBase,
    /// root(value, n) – the n-th root of value
    Root,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstKind {
    Pi,
    E,
}

/// A single input item stored in the input buffer. Every variant maps
/// to exactly one logical token; digits are separate so the cursor
/// can sit between them.
#[derive(Debug, Clone, PartialEq)]
pub enum InputItem {
    Digit(char),
    DecimalPoint,
    BinOp(BinOp),
    /// `×` materialised by the auto-multiplication path (e.g. `5(` →
    /// `5×(`). Behaves identically to `BinOp(Mul)` everywhere except
    /// the display layer, which renders it dimmed so the user can tell
    /// at a glance which operators they typed and which the calculator
    /// inserted on their behalf.
    AutoMul,
    Percent,
    Factorial,
    UnaryFunc(UnaryFunc),
    BinaryFunc(BinaryFunc),
    /// log with an arbitrary positive-integer base typed as log<N>
    LogN(u32),
    Constant(ConstKind),
    LeftParen,
    RightParen,
    Comma,
}

/// Arity (operand count) of an item. Numbers, constants and structural
/// tokens are classified as leaves (0). The tokenizer does not rely on
/// this directly; it is used by the state machine for input validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Leaf,
    Unary,
    Binary,
    Structural,
}

impl InputItem {
    /// Classify the operand count of this item.
    pub fn arity(&self) -> Arity {
        match self {
            InputItem::Digit(_) | InputItem::DecimalPoint | InputItem::Constant(_) => Arity::Leaf,
            InputItem::BinOp(_) | InputItem::AutoMul => Arity::Binary,
            InputItem::Percent | InputItem::Factorial => Arity::Unary,
            InputItem::UnaryFunc(_) | InputItem::LogN(_) => Arity::Unary,
            InputItem::BinaryFunc(_) => Arity::Binary,
            InputItem::LeftParen | InputItem::RightParen | InputItem::Comma => Arity::Structural,
        }
    }

    /// Return the ASCII/Unicode display glyph for this item.
    pub fn display(&self) -> String {
        match self {
            InputItem::Digit(c) => c.to_string(),
            InputItem::DecimalPoint => ".".to_string(),
            InputItem::BinOp(BinOp::Add) => "+".to_string(),
            InputItem::BinOp(BinOp::Sub) => "-".to_string(),
            InputItem::BinOp(BinOp::Mul) => "×".to_string(),
            InputItem::AutoMul => "×".to_string(),
            InputItem::BinOp(BinOp::Div) => "÷".to_string(),
            InputItem::BinOp(BinOp::Pow) => "^".to_string(),
            InputItem::Percent => "%".to_string(),
            InputItem::Factorial => "!".to_string(),
            InputItem::UnaryFunc(f) => format!("{}(", unary_func_name(*f)),
            InputItem::BinaryFunc(BinaryFunc::LogBase) => "log(".to_string(),
            InputItem::BinaryFunc(BinaryFunc::Root) => "root(".to_string(),
            InputItem::LogN(n) => format!("log{}(", n),
            InputItem::Constant(ConstKind::Pi) => "π".to_string(),
            InputItem::Constant(ConstKind::E) => "𝑒".to_string(),
            InputItem::LeftParen => "(".to_string(),
            InputItem::RightParen => ")".to_string(),
            InputItem::Comma => ",".to_string(),
        }
    }
}

/// Human-readable ASCII name for a unary function.
pub fn unary_func_name(f: UnaryFunc) -> &'static str {
    match f {
        UnaryFunc::Sin => "sin",
        UnaryFunc::Cos => "cos",
        UnaryFunc::Tan => "tan",
        UnaryFunc::Cot => "cot",
        UnaryFunc::Asin => "sin-1",
        UnaryFunc::Acos => "cos-1",
        UnaryFunc::Atan => "tan-1",
        UnaryFunc::Acot => "cot-1",
        UnaryFunc::Sinh => "sinh",
        UnaryFunc::Cosh => "cosh",
        UnaryFunc::Tanh => "tanh",
        UnaryFunc::Coth => "coth",
        UnaryFunc::Asinh => "sinh-1",
        UnaryFunc::Acosh => "cosh-1",
        UnaryFunc::Atanh => "tanh-1",
        UnaryFunc::Acoth => "coth-1",
        UnaryFunc::Ln => "ln",
        UnaryFunc::Log => "log",
        UnaryFunc::Log2 => "log2",
        UnaryFunc::Log10 => "log10",
        UnaryFunc::Sqrt => "√",
        UnaryFunc::Cbrt => "∛",
    }
}
