//! Superscript / subscript rendering of an expression.
//!
//! The buffer stores `2 ^ 2` and `log2(`; a calculator display is
//! expected to show `2²` and `log₂(`. Everything needed to make that
//! swap lives here so the display renderer, the caption above it and
//! the history panel all reach the same conclusion about a given run
//! of items.
//!
//! Two rules keep the pretty form honest:
//!
//!   * A glyph is only ever raised or lowered when Unicode has a real
//!     superscript / subscript for it. There is no `×` or `!` in the
//!     superscript block, so an exponent containing one is left in its
//!     `^` form rather than rendered with a mix of sizes that would
//!     read as a different expression.
//!   * [`exponent_span`] covers exactly what the parser treats as the
//!     exponent — `power = postfix ('^' unary)?` — so `2^3π` raises
//!     only the `3` (the `π` is a separate factor) while `2^2!` is
//!     left alone (the `!` belongs to the exponent, and `2²!` would
//!     read as `(2²)!`).

use crate::engine::item::{unary_func_name, BinOp, InputItem, UnaryFunc};

/// How an expression is rendered for the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Notation {
    /// Exactly what the buffer holds: `root(2^2,6)`, `log2(8)`,
    /// `sin-1(1)`. Reachable through the settings panel's debug
    /// toggle, and what the tokenizer sees either way.
    Raw,
    /// Exponents raised and log bases lowered: `root(2²,6)`,
    /// `log₂(8)`, `sin⁻¹(1)`.
    #[default]
    Pretty,
}

impl Notation {
    /// True when items should be rendered in their raised / lowered
    /// form.
    pub fn is_pretty(self) -> bool {
        self == Notation::Pretty
    }
}

/// Superscript form of one character, or `None` when Unicode has no
/// superscript for it.
pub fn superscript_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' => '⁻',
        '(' => '⁽',
        ')' => '⁾',
        _ => return None,
    })
}

/// Subscript form of one character, or `None` when Unicode has no
/// subscript for it.
pub fn subscript_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '(' => '₍',
        ')' => '₎',
        _ => return None,
    })
}

/// Raise a whole string. `None` as soon as one character has no
/// superscript form — a partially raised exponent would read as a
/// different expression, so the caller falls back to `^`.
pub fn to_superscript(s: &str) -> Option<String> {
    s.chars().map(superscript_char).collect()
}

/// Lower a whole string, with the same all-or-nothing rule as
/// [`to_superscript`].
pub fn to_subscript(s: &str) -> Option<String> {
    s.chars().map(subscript_char).collect()
}

/// Pretty glyphs for the items whose whole rendering is a substitution
/// — the log bases and the inverse functions. Everything else (digits,
/// operators, parens) renders the same in both notations, and the
/// exponent after a `^` needs its span, so it goes through
/// [`exponent_span`] in the renderer instead.
pub fn pretty_display(it: &InputItem) -> String {
    match it {
        InputItem::UnaryFunc(UnaryFunc::Log2) => "log₂(".to_string(),
        InputItem::UnaryFunc(UnaryFunc::Log10) => "log₁₀(".to_string(),
        InputItem::LogN(n) => {
            let digits = n.to_string();
            match to_subscript(&digits) {
                Some(sub) => format!("log{sub}("),
                None => it.display(),
            }
        }
        // `sin-1`, `cosh-1`, … are stored with an ASCII `-1` suffix.
        InputItem::UnaryFunc(f) => match unary_func_name(*f).strip_suffix("-1") {
            Some(base) => format!("{base}⁻¹("),
            None => it.display(),
        },
        _ => it.display(),
    }
}

/// End (exclusive) of the exponent the `^` at `pow_idx` raises its
/// base to, or `None` when that exponent has no superscript form.
///
/// The span follows the parser's `power = postfix ('^' unary)?` rule:
/// any leading signs, then one primary (a number, a constant or a
/// bracketed group), and nothing else. A postfix `!` / `%` or a
/// chained `^` is part of the exponent but cannot be raised, so those
/// return `None` and leave the whole run in its `^` form.
pub fn exponent_span(items: &[InputItem], pow_idx: usize) -> Option<usize> {
    if !matches!(items.get(pow_idx), Some(InputItem::BinOp(BinOp::Pow))) {
        return None;
    }
    let mut i = pow_idx + 1;
    while matches!(
        items.get(i),
        Some(InputItem::BinOp(BinOp::Sub) | InputItem::BinOp(BinOp::Add))
    ) {
        i += 1;
    }
    match items.get(i)? {
        InputItem::Digit(_) | InputItem::DecimalPoint => {
            while matches!(
                items.get(i),
                Some(InputItem::Digit(_) | InputItem::DecimalPoint)
            ) {
                i += 1;
            }
        }
        InputItem::Constant(_) => i += 1,
        // Any of these carries an opening bracket; the exponent runs
        // to the closer that matches it.
        InputItem::LeftParen
        | InputItem::UnaryFunc(_)
        | InputItem::BinaryFunc(_)
        | InputItem::LogN(_) => {
            let mut depth = 0usize;
            loop {
                match items.get(i) {
                    // Unclosed group: the exponent runs to the end of
                    // the buffer, so there is nothing to raise it over.
                    None => return None,
                    Some(
                        InputItem::LeftParen
                        | InputItem::UnaryFunc(_)
                        | InputItem::BinaryFunc(_)
                        | InputItem::LogN(_),
                    ) => depth += 1,
                    Some(InputItem::RightParen) => depth -= 1,
                    Some(_) => {}
                }
                i += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        _ => return None,
    }
    if matches!(
        items.get(i),
        Some(InputItem::Factorial | InputItem::Percent | InputItem::BinOp(BinOp::Pow))
    ) {
        return None;
    }
    Some(i)
}
