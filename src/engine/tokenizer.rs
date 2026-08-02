//! Tokenizer: ASCII expression string → Vec<Token>.
//! Accepts both '.' and ',' as decimal separators, supports scientific
//! notation embedded in numeric literals (e.g., 1e15, 1e-308), and
//! classifies '%' as modulo (when followed by a digit or '(') or as a
//! percent marker otherwise. Unknown characters produce a ParseError.

use crate::engine::errors::CalcError;
use crate::engine::item::{BinOp, BinaryFunc, ConstKind, UnaryFunc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    Num(f64),
    Op(BinOp),
    Mod,
    Percent,
    Factorial,
    LParen,
    RParen,
    Comma,
    UnaryFn(UnaryFunc),
    BinaryFn(BinaryFunc),
    /// log with a fixed base N baked in (e.g., log2, log6). The base
    /// is stored as f64 so the evaluator doesn't recompute it.
    LogN(f64),
    Const(ConstKind),
}

#[derive(Debug)]
pub struct TokenizeError;

impl From<TokenizeError> for CalcError {
    fn from(_: TokenizeError) -> Self {
        CalcError::Undefined
    }
}

/// Split `src` into tokens. Whitespace is skipped. Decimal commas are
/// treated as dots inside numeric literals. Returns Err on a character
/// that cannot be matched to any known token.
pub fn tokenize(src: &str) -> Result<Vec<Token>, TokenizeError> {
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut out: Vec<Token> = Vec::new();
    // Stack of "sep_mode" flags, one entry per open paren. When the
    // top is `true` we're inside a two-argument function call (root,
    // log-with-base) and `,` is an argument separator; otherwise `,`
    // is a decimal separator. `;` is always a separator regardless.
    let mut sep_mode_stack: Vec<bool> = Vec::new();

    while i < bytes.len() {
        let c = bytes[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        let in_sep_mode = sep_mode_stack.last().copied().unwrap_or(false);
        let at_num_start = c.is_ascii_digit()
            || c == '.'
            || (c == ',' && !in_sep_mode);
        if at_num_start {
            let allow_comma = !in_sep_mode;
            let (n, consumed) = parse_number(&bytes, i, allow_comma)?;
            out.push(Token::Num(n));
            i += consumed;
            continue;
        }

        match c {
            '+' => { out.push(Token::Op(BinOp::Add)); i += 1; continue; }
            '-' => { out.push(Token::Op(BinOp::Sub)); i += 1; continue; }
            '*' | '×' => { out.push(Token::Op(BinOp::Mul)); i += 1; continue; }
            '/' | '÷' => { out.push(Token::Op(BinOp::Div)); i += 1; continue; }
            '^' => { out.push(Token::Op(BinOp::Pow)); i += 1; continue; }
            '(' => {
                // Decide whether `,` inside this paren will be a
                // decimal point or an argument separator by looking at
                // the token that precedes the paren.
                //   • BinaryFn (root)           → always 2-arg, separator
                //   • UnaryFn(Log)              → may be 1 or 2 arg; use
                //                                 separator so log(3,2)
                //                                 parses as log-base;
                //                                 log(100) and log(-5)
                //                                 still work (no comma).
                //   • Everything else (LogN,
                //     sin/cos/…, grouping)      → 1-arg / grouping,
                //                                 comma is decimal so
                //                                 log6(279936,01) and
                //                                 tanh(14,5) work.
                let sep_mode = matches!(
                    out.last(),
                    Some(Token::BinaryFn(_)) | Some(Token::UnaryFn(UnaryFunc::Log))
                );
                out.push(Token::LParen);
                sep_mode_stack.push(sep_mode);
                i += 1;
                continue;
            }
            ')' => {
                out.push(Token::RParen);
                sep_mode_stack.pop();
                i += 1;
                continue;
            }
            '!' => { out.push(Token::Factorial); i += 1; continue; }
            '%' => {
                let next = next_non_space(&bytes, i + 1);
                let is_mod = match next {
                    Some(nc) => nc.is_ascii_digit() || nc == '(' || nc == 'π' || nc == '𝑒'
                        || nc == '.' || nc == ','
                        || nc.is_ascii_alphabetic(),
                    None => false,
                };
                out.push(if is_mod { Token::Mod } else { Token::Percent });
                i += 1;
                continue;
            }
            ',' | ';' => {
                // Argument separator. `,` reaches here only when the
                // enclosing paren is marked sep_mode (two-arg function);
                // `;` is always a separator.
                out.push(Token::Comma);
                i += 1;
                continue;
            }
            'π' => { out.push(Token::Const(ConstKind::Pi)); i += 1; continue; }
            '𝑒' => { out.push(Token::Const(ConstKind::E)); i += 1; continue; }
            '√' => { out.push(Token::UnaryFn(UnaryFunc::Sqrt)); i += 1; continue; }
            '∛' => { out.push(Token::UnaryFn(UnaryFunc::Cbrt)); i += 1; continue; }
            _ => {}
        }

        // Identifiers: function names and constants.
        if c.is_ascii_alphabetic() {
            let (tok, consumed) = parse_ident(&bytes, i)?;
            out.push(tok);
            i += consumed;
            continue;
        }

        // Unrecognised character.
        return Err(TokenizeError);
    }

    Ok(insert_implicit_mul(out))
}

/// Insert a Mul token between adjacent "value-producing" tokens so
/// expressions like `2π`, `3(x+1)`, or `(2+3)sqrt(4)` behave as if the
/// user had typed an explicit `×`. We conservatively only inject when
/// both sides are primary-like; operators, commas, and open parens as
/// the left side never trigger a multiplication insertion.
fn insert_implicit_mul(toks: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    for tok in toks {
        if let Some(prev) = out.last() {
            if produces_value(prev) && begins_value(&tok) {
                out.push(Token::Op(BinOp::Mul));
            }
        }
        out.push(tok);
    }
    out
}

/// Token class for the left side of an implicit-multiplication gap:
/// anything that produces a value (literal, constant, close paren,
/// postfix `!` / `%`).
fn produces_value(t: &Token) -> bool {
    matches!(
        t,
        Token::Num(_) | Token::Const(_) | Token::RParen | Token::Factorial | Token::Percent
    )
}

/// Token class for the right side of an implicit-multiplication gap:
/// anything that can start a primary expression.
fn begins_value(t: &Token) -> bool {
    matches!(
        t,
        Token::Num(_)
            | Token::Const(_)
            | Token::LParen
            | Token::UnaryFn(_)
            | Token::BinaryFn(_)
            | Token::LogN(_)
    )
}

/// Return the next non-whitespace character starting from index `j`.
fn next_non_space(bytes: &[char], mut j: usize) -> Option<char> {
    while j < bytes.len() && bytes[j].is_whitespace() {
        j += 1;
    }
    bytes.get(j).copied()
}

/// Parse a numeric literal starting at `i`. Accepts a leading digit or
/// decimal point, an optional fractional part (separator '.' or ','),
/// and an optional exponent ('e'/'E') when preceded by digits.
fn parse_number(bytes: &[char], i: usize, allow_comma: bool) -> Result<(f64, usize), TokenizeError> {
    let mut j = i;
    let start = i;
    let mut has_digits = false;
    // Integer part.
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
        has_digits = true;
    }
    // Fractional part. `.` is always a decimal separator; `,` only when
    // the caller says so (outside a function call).
    let is_frac_sep = |c: char| c == '.' || (allow_comma && c == ',');
    if j < bytes.len() && is_frac_sep(bytes[j]) {
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
            has_digits = true;
        }
    }
    if !has_digits {
        return Err(TokenizeError);
    }
    // Exponent part – only if a digit or '.' was consumed first AND the
    // letter is directly followed by an optional sign plus digits.
    if j < bytes.len() && (bytes[j] == 'e' || bytes[j] == 'E') {
        let mut k = j + 1;
        if k < bytes.len() && (bytes[k] == '+' || bytes[k] == '-') {
            k += 1;
        }
        let mut exp_digits = false;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
            exp_digits = true;
        }
        if exp_digits {
            j = k;
        }
    }
    // Build an f64. Normalise ',' → '.'.
    let raw: String = bytes[start..j].iter().map(|c| if *c == ',' { '.' } else { *c }).collect();
    let n: f64 = raw.parse().map_err(|_| TokenizeError)?;
    Ok((n, j - start))
}

/// Parse an identifier starting at `i` – longest match against the
/// known function/constant table. Also accepts the inverse-trig
/// syntax `sin-1`, `cos-1`, … by consuming the two extra characters
/// when they follow a trigonometric stem.
fn parse_ident(bytes: &[char], i: usize) -> Result<(Token, usize), TokenizeError> {
    let start = i;
    let mut j = i;
    while j < bytes.len() && (bytes[j].is_ascii_alphabetic()) {
        j += 1;
    }
    let name: String = bytes[start..j].iter().collect();
    let mut consumed = j - start;

    // Handle log<digits> forms such as log2, log10, log6.
    if name == "log" {
        let mut k = j;
        let dig_start = k;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
        }
        if k > dig_start {
            let digs: String = bytes[dig_start..k].iter().collect();
            let base: f64 = digs.parse().map_err(|_| TokenizeError)?;
            consumed = k - start;
            return Ok((Token::LogN(base), consumed));
        }
    }

    // Detect `sin-1`, `cos-1`, `tan-1`, `cot-1`, `sinh-1`, `cosh-1`, `tanh-1`, `coth-1`.
    let inverse_of = match name.as_str() {
        "sin" | "cos" | "tan" | "cot" | "ctg" | "sinh" | "cosh" | "tanh" | "coth" | "ctgh" => {
            let k = j;
            if k + 1 < bytes.len() && bytes[k] == '-' && bytes[k + 1] == '1' {
                consumed = (k + 2) - start;
                Some(name.as_str())
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(stem) = inverse_of {
        let f = match stem {
            "sin" => UnaryFunc::Asin,
            "cos" => UnaryFunc::Acos,
            "tan" => UnaryFunc::Atan,
            "cot" | "ctg" => UnaryFunc::Acot,
            "sinh" => UnaryFunc::Asinh,
            "cosh" => UnaryFunc::Acosh,
            "tanh" => UnaryFunc::Atanh,
            "coth" | "ctgh" => UnaryFunc::Acoth,
            _ => unreachable!(),
        };
        return Ok((Token::UnaryFn(f), consumed));
    }

    let tok = match name.as_str() {
        "sin" => Token::UnaryFn(UnaryFunc::Sin),
        "cos" => Token::UnaryFn(UnaryFunc::Cos),
        "tan" => Token::UnaryFn(UnaryFunc::Tan),
        "cot" | "ctg" => Token::UnaryFn(UnaryFunc::Cot),
        "sinh" => Token::UnaryFn(UnaryFunc::Sinh),
        "cosh" => Token::UnaryFn(UnaryFunc::Cosh),
        "tanh" => Token::UnaryFn(UnaryFunc::Tanh),
        "coth" | "ctgh" => Token::UnaryFn(UnaryFunc::Coth),
        "asin" | "arcsin" => Token::UnaryFn(UnaryFunc::Asin),
        "acos" | "arccos" => Token::UnaryFn(UnaryFunc::Acos),
        "atan" | "arctan" => Token::UnaryFn(UnaryFunc::Atan),
        "acot" | "arccot" => Token::UnaryFn(UnaryFunc::Acot),
        "asinh" | "arcsinh" => Token::UnaryFn(UnaryFunc::Asinh),
        "acosh" | "arccosh" => Token::UnaryFn(UnaryFunc::Acosh),
        "atanh" | "arctanh" => Token::UnaryFn(UnaryFunc::Atanh),
        "acoth" | "arccoth" => Token::UnaryFn(UnaryFunc::Acoth),
        "ln" => Token::UnaryFn(UnaryFunc::Ln),
        "log" => Token::UnaryFn(UnaryFunc::Log),
        "sqrt" => Token::UnaryFn(UnaryFunc::Sqrt),
        "cbrt" => Token::UnaryFn(UnaryFunc::Cbrt),
        "root" => Token::BinaryFn(BinaryFunc::Root),
        "pi" => Token::Const(ConstKind::Pi),
        "e" => Token::Const(ConstKind::E),
        _ => return Err(TokenizeError),
    };
    Ok((tok, consumed))
}
