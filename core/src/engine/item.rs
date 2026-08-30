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
    /// Binary modulo. Kept distinct from [`InputItem::Percent`] all the
    /// way down to the ASCII form, which is the word `mod`. Both used
    /// to serialise to `%`, leaving the tokenizer to guess which was
    /// meant from the following character – so `7 mod -3` silently
    /// became `7% - 3`, and modulo by a negative was inexpressible.
    Modulo,
    Factorial,
    /// A power the calculator wrote whole: the `²` of `x²`, the `³`
    /// of `x³`. One item rather than a caret and a digit, because the
    /// key writes one finished operation — the exponent is not a slot
    /// left open for the user to type into, and nothing keyed after
    /// it should land up there.
    FixedPow(u8),
    UnaryFunc(UnaryFunc),
    BinaryFunc(BinaryFunc),
    /// log with an arbitrary positive-integer base typed as log<N>
    LogN(u32),
    Constant(ConstKind),
    LeftParen,
    RightParen,
    Comma,
}

impl InputItem {
    /// True for an item that closes a value: what a binary operator
    /// can attach to, what a power can raise, what `C` takes back.
    /// Mirrors the tokenizer's `produces_value`, at the item level.
    pub fn ends_operand(&self) -> bool {
        matches!(
            self,
            InputItem::Digit(_)
                | InputItem::DecimalPoint
                | InputItem::Constant(_)
                | InputItem::RightParen
                | InputItem::Factorial
                | InputItem::Percent
                | InputItem::FixedPow(_)
        )
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
            InputItem::Modulo => " mod ".to_string(),
            InputItem::Factorial => "!".to_string(),
            InputItem::FixedPow(n) => format!("^{}", n),
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

/// The tokenizer's spelling of one item: what the clipboard carries,
/// what evaluation is handed, and what the "Show ASCII expression"
/// toggle draws. All of it is ASCII, which is the point — `π` is
/// `pi`, `×` is `*`, the radical is `sqrt(`.
///
/// `prev` is the character already written in front of this item, and
/// only Euler's number needs it: a bare `e` behind a digit run is read
/// back as that number's exponent (`3e5` is three hundred thousand,
/// not `3·𝑒·5`), so there the constant is written `*e`. The
/// multiplication is the one the tokenizer inserts there anyway, so
/// spelling it out changes nothing but the ambiguity.
pub fn ascii_text(it: &InputItem, prev: Option<char>) -> String {
    match it {
        InputItem::Digit(c) => c.to_string(),
        InputItem::DecimalPoint => ".".to_string(),
        InputItem::BinOp(BinOp::Add) => "+".to_string(),
        InputItem::BinOp(BinOp::Sub) => "-".to_string(),
        InputItem::BinOp(BinOp::Mul) | InputItem::AutoMul => "*".to_string(),
        InputItem::BinOp(BinOp::Div) => "/".to_string(),
        InputItem::BinOp(BinOp::Pow) => "^".to_string(),
        InputItem::Percent => "%".to_string(),
        InputItem::Modulo => " mod ".to_string(),
        InputItem::Factorial => "!".to_string(),
        InputItem::FixedPow(n) => format!("^{}", n),
        InputItem::UnaryFunc(UnaryFunc::Sqrt) => "sqrt(".to_string(),
        InputItem::UnaryFunc(UnaryFunc::Cbrt) => "cbrt(".to_string(),
        InputItem::UnaryFunc(f) => format!("{}(", unary_func_name(*f)),
        InputItem::BinaryFunc(BinaryFunc::LogBase) => "log(".to_string(),
        InputItem::BinaryFunc(BinaryFunc::Root) => "root(".to_string(),
        InputItem::LogN(n) => format!("log{}(", n),
        InputItem::Constant(ConstKind::Pi) => "pi".to_string(),
        InputItem::Constant(ConstKind::E) => match prev {
            Some(c) if c.is_ascii_digit() || c == '.' => "*e".to_string(),
            _ => "e".to_string(),
        },
        InputItem::LeftParen => "(".to_string(),
        InputItem::RightParen => ")".to_string(),
        InputItem::Comma => ",".to_string(),
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
