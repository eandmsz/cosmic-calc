//! Clipboard input/output pipeline. Two halves:
//!
//! * **Copy** – turn the engine's current buffer into an ASCII string
//!   suitable for the system clipboard. Empty buffer is emitted as
//!   `"0"` so the user never ends up with a blank paste target.
//! * **Paste** – sanitise an incoming clipboard string against the
//!   spec's whitelist, normalise Unicode variants into the canonical
//!   forms the engine understands, then convert the result into a
//!   stream of [`InputItem`]s ready to replace the buffer.
//!
//! All of the heavy lifting lives in pure helpers here so the UI layer
//! just dispatches `Message::Clipboard(Copy/Paste)` and awaits the
//! `Task` result.
//!
//! The paste pipeline is conservative: any single disallowed character
//! or a length beyond 255 characters causes the whole paste to be
//! silently dropped. We also reject non-text clipboard payloads simply
//! by using `iced::clipboard::read()` which returns `None` in that
//! case.

use crate::engine::item::{BinOp, ConstKind, InputItem, UnaryFunc};

/// Outbound clipboard operation dispatched by the UI after a user
/// action. The `update` handler turns each variant into a libcosmic
/// `Task` that talks to the real system clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp {
    /// Snapshot the engine's ASCII buffer and copy it to the clipboard.
    /// Empty buffer surfaces as `"0"`.
    Copy,
    /// Request the current clipboard contents. The delivered string is
    /// sanitised; invalid payloads are silently dropped.
    Paste,
}

/// Maximum number of characters we'll accept from the clipboard. Beyond
/// this the whole paste is dropped.
pub const MAX_PASTE_CHARS: usize = 255;

/// Render the engine buffer into the textual form that goes on the
/// clipboard. Empty buffer is surfaced as `"0"` per spec.
pub fn copy_text_for(ascii_expression: &str) -> String {
    if ascii_expression.is_empty() {
        "0".to_string()
    } else {
        ascii_expression.to_string()
    }
}

/// Sanitise a clipboard string against the spec's whitelist and
/// character-substitution rules. Returns `None` when the paste should
/// be silently dropped (non-ASCII character, length > 255, or a char
/// not on the allow-list).
pub fn sanitize_paste(raw: &str) -> Option<String> {
    // Pass 1: length cap + whitelist check.
    let mut kept = String::new();
    for (i, ch) in raw.chars().enumerate() {
        if i >= MAX_PASTE_CHARS {
            return None;
        }
        if !is_allowed(ch) {
            return None;
        }
        kept.push(ch);
    }

    // Pass 2: space handling – drop every ' ' except those preceded
    // by ','.
    let mut spaced = String::with_capacity(kept.len());
    let mut prev: Option<char> = None;
    for ch in kept.chars() {
        if ch == ' ' {
            if prev == Some(',') {
                spaced.push(' ');
            }
        } else {
            spaced.push(ch);
        }
        prev = Some(ch);
    }

    // Pass 3: char-level substitutions (case fold, unicode glyphs).
    let mut canonical = String::with_capacity(spaced.len());
    for ch in spaced.chars() {
        match substitute_char(ch) {
            Some(s) => canonical.push_str(s),
            None => canonical.push(ch),
        }
    }

    // Pass 4: function-name rewrites. Longer names first to avoid
    // `asinh` being eaten as `asin` + trailing `h`.
    let functional = rewrite_function_names(&canonical);

    Some(functional)
}

/// Translate a sanitised paste string into the engine's
/// [`InputItem`] stream. Unrecognised characters are silently dropped
/// so that an adversarial clipboard can't blow up the buffer.
pub fn items_from_paste(s: &str) -> Vec<InputItem> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<InputItem> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if let Some((func, len)) = match_function_keyword(&chars, i) {
            out.push(InputItem::UnaryFunc(func));
            // `UnaryFunc` renders its own `(`; if a literal `(` follows
            // in the paste stream, consume it so we don't end up with
            // double-open parens.
            let after = i + len;
            if chars.get(after) == Some(&'(') {
                i = after + 1;
            } else {
                i = after;
            }
            continue;
        }
        match chars[i] {
            c @ '0'..='9' => out.push(InputItem::Digit(c)),
            '.' | ',' => out.push(InputItem::DecimalPoint),
            '+' => out.push(InputItem::BinOp(BinOp::Add)),
            '-' => out.push(InputItem::BinOp(BinOp::Sub)),
            '×' | '*' => out.push(InputItem::BinOp(BinOp::Mul)),
            '÷' | '/' => out.push(InputItem::BinOp(BinOp::Div)),
            '^' => out.push(InputItem::BinOp(BinOp::Pow)),
            '%' => out.push(InputItem::Percent),
            '!' => out.push(InputItem::Factorial),
            '(' => out.push(InputItem::LeftParen),
            ')' => out.push(InputItem::RightParen),
            '√' => out.push(InputItem::UnaryFunc(UnaryFunc::Sqrt)),
            '∛' => out.push(InputItem::UnaryFunc(UnaryFunc::Cbrt)),
            'π' => out.push(InputItem::Constant(ConstKind::Pi)),
            '𝑒' | 'e' | 'E' => out.push(InputItem::Constant(ConstKind::E)),
            _ => {}
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------

/// Everything the spec's whitelist allows, including Unicode variants
/// and the letters that can appear inside function names.
fn is_allowed(ch: char) -> bool {
    matches!(
        ch,
        '0'..='9'
            | ','
            | '.'
            | '('
            | ')'
            | '{'
            | '}'
            | '['
            | ']'
            | '!'
            | ' '
            | '^'
            | '√'
            | '∛'
            // Function-name letters (both cases).
            | 'a'..='z'
            | 'A'..='Z'
            // Pi variants.
            | 'π' | '𝛑' | '𝜋' | '𝝅' | '𝝿'
            // Euler-e variants.
            | 'ℯ' | 'ｅ' | '𝐞' | '𝑒' | '𝒆' | '𝓮' | '𝖾' | '𝗲' | '𝘦' | '𝙚' | '𝚎'
            // Division glyphs.
            | '/' | '／' | '∕' | '➗' | '÷'
            // Multiplication glyphs.
            | '＊' | '*' | '﹡' | '×' | '⋅' | '✕' | '✖'
            // Plus glyphs.
            | '🞡' | '🞢' | '🞣' | '🞤' | '🞥' | '🞦' | '🞧' | '✚' | '＋' | '+' | '﹢'
            // Minus glyphs.
            | '－' | '−' | '-' | '﹣' | '˗'
            // Percent glyphs.
            | '⁒' | '％' | '%' | '﹪'
    ) && is_letter_on_allowlist(ch)
}

/// Limit the a-z whitelist to letters that actually appear in the
/// spec's allowed identifier set, so random ASCII text like `xyz` is
/// rejected straight away.
fn is_letter_on_allowlist(ch: char) -> bool {
    match ch {
        c if !c.is_ascii_alphabetic() => true,
        'h' | 'H' | 'c' | 'C' | 't' | 'T' | 's' | 'S' | 'o' | 'O' | 'a' | 'A' | 'l' | 'L'
        | 'g' | 'G' | 'n' | 'N' | 'm' | 'M' | 'd' | 'D' | 'q' | 'Q' | 'r' | 'R' | 'b' | 'B'
        | 'p' | 'P' | 'i' | 'I' | 'e' | 'E' => true,
        _ => false,
    }
}

/// Map a character to its canonical replacement. `None` means "pass
/// the character through unchanged" – the caller copies it verbatim.
fn substitute_char(ch: char) -> Option<&'static str> {
    Some(match ch {
        // Case-fold letters.
        'H' => "h",
        'C' => "c",
        'T' => "t",
        'S' => "s",
        'O' => "o",
        'A' => "a",
        'L' => "l",
        'G' => "g",
        'N' => "n",
        'M' => "m",
        'D' => "d",
        'Q' => "q",
        'R' => "r",
        'B' => "b",
        'P' => "p",
        'I' => "i",
        'E' => "e",
        // Paren variants.
        '{' | '[' => "(",
        '}' | ']' => ")",
        // Division.
        '/' | '／' | '∕' | '➗' => "÷",
        // Multiplication.
        '＊' | '*' | '﹡' | '⋅' | '✕' | '✖' => "×",
        // Plus.
        '🞡' | '🞢' | '🞣' | '🞤' | '🞥' | '🞦' | '🞧' | '✚' | '＋' | '﹢' => "+",
        // Minus.
        '－' | '−' | '﹣' | '˗' => "-",
        // Percent.
        '％' | '⁒' | '﹪' => "%",
        // Pi.
        '𝛑' | '𝜋' | '𝝅' | '𝝿' => "π",
        // Euler e.
        'ℯ' | 'ｅ' | '𝐞' | '𝒆' | '𝓮' | '𝖾' | '𝗲' | '𝘦' | '𝙚' | '𝚎' => "𝑒",
        // Everything else: copy verbatim.
        _ => return None,
    })
}

/// Replace standard function names with the engine's canonical forms.
/// Order matters: `asinh` must be matched before `asin`, `log2` before
/// `log`, etc.
fn rewrite_function_names(input: &str) -> String {
    // Table sorted longest-first inside each name-family so greedy
    // substitution doesn't accidentally chop `asinh` into `asin`+`h`.
    const RULES: &[(&str, &str)] = &[
        ("asinh", "sinh-1"),
        ("acosh", "cosh-1"),
        ("atanh", "tanh-1"),
        ("asin", "sin-1"),
        ("acos", "cos-1"),
        ("atan", "tan-1"),
        ("sqrt", "√"),
        ("cbrt", "∛"),
        ("mod", "%"),
    ];
    let mut out = String::with_capacity(input.len());
    let bytes: &[u8] = input.as_bytes();
    let mut i = 0usize;
    'outer: while i < bytes.len() {
        // Multi-byte chars won't match any of the rules, so it's safe
        // to index by byte for the prefix check.
        for (from, to) in RULES {
            let from_bytes = from.as_bytes();
            if i + from_bytes.len() <= bytes.len()
                && &bytes[i..i + from_bytes.len()] == from_bytes
            {
                out.push_str(to);
                i += from_bytes.len();
                continue 'outer;
            }
        }
        // Not a keyword boundary – copy the next char as-is.
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Match a multi-char function name that should become a UnaryFunc
/// item. Longest-match wins.
fn match_function_keyword(chars: &[char], i: usize) -> Option<(UnaryFunc, usize)> {
    const RULES: &[(&str, UnaryFunc)] = &[
        ("sinh-1", UnaryFunc::Asinh),
        ("cosh-1", UnaryFunc::Acosh),
        ("tanh-1", UnaryFunc::Atanh),
        ("sin-1", UnaryFunc::Asin),
        ("cos-1", UnaryFunc::Acos),
        ("tan-1", UnaryFunc::Atan),
        ("sinh", UnaryFunc::Sinh),
        ("cosh", UnaryFunc::Cosh),
        ("tanh", UnaryFunc::Tanh),
        ("log10", UnaryFunc::Log10),
        ("log2", UnaryFunc::Log2),
        ("sin", UnaryFunc::Sin),
        ("cos", UnaryFunc::Cos),
        ("tan", UnaryFunc::Tan),
        ("ln", UnaryFunc::Ln),
        ("log", UnaryFunc::Log),
    ];
    for (name, func) in RULES {
        let name_chars: Vec<char> = name.chars().collect();
        if i + name_chars.len() <= chars.len()
            && chars[i..i + name_chars.len()] == name_chars[..]
        {
            return Some((*func, name_chars.len()));
        }
    }
    None
}
