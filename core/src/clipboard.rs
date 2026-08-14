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

use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};

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
/// [`InputItem`] stream, or `None` when the string contains something
/// the buffer cannot represent faithfully.
///
/// Rejecting is the important part. This used to drop unrecognised
/// characters and accept whatever was left, which quietly turned a
/// paste into a *different expression*: `root(16,4)` lost its keyword
/// and its argument comma and became `(16.4)`, and `3pi` became `3`.
/// A paste that cannot be represented is now dropped whole, the same
/// way [`sanitize_paste`] already drops one containing a disallowed
/// character.
pub fn items_from_paste(s: &str) -> Option<Vec<InputItem>> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<InputItem> = Vec::new();
    // One flag per open paren: true when `,` separates arguments
    // rather than introducing a fractional part. Mirrors the same
    // stack in the engine tokenizer.
    let mut arg_sep_stack: Vec<bool> = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        // `sanitize_paste` only keeps spaces after a comma, where they
        // carry no meaning of their own.
        if c == ' ' {
            i += 1;
            continue;
        }

        // Scientific notation. `2e3` has to become `2×10^3`, not
        // `2 × 𝑒 × 3`: the buffer renders the Euler constant as `𝑒` but
        // serialises it back to a bare `e`, which the tokenizer's
        // number scanner then swallows as an exponent — so the display
        // said 2·e·3 (≈16.31) while the engine computed 2000.
        if let Some(len) = exponent_suffix_len(&chars, i, &out) {
            push_exponent(&mut out, &chars[i + 1..i + len]);
            i += len;
            continue;
        }

        if let Some((item, len)) = match_keyword(&chars, i) {
            let opens_paren = item_opens_paren(&item);
            let takes_arg_list = matches!(
                item,
                InputItem::BinaryFunc(_) | InputItem::UnaryFunc(UnaryFunc::Log)
            );
            out.push(item);
            i += len;
            // Function items render their own `(`; consume a literal
            // one that follows so we don't end up with two.
            if opens_paren {
                if chars.get(i) == Some(&'(') {
                    i += 1;
                }
                arg_sep_stack.push(takes_arg_list);
            }
            continue;
        }

        let item = match c {
            d @ '0'..='9' => InputItem::Digit(d),
            '.' => InputItem::DecimalPoint,
            ',' | ';' => {
                if arg_sep_stack.last().copied().unwrap_or(false) {
                    InputItem::Comma
                } else {
                    InputItem::DecimalPoint
                }
            }
            '+' => InputItem::BinOp(BinOp::Add),
            '-' => InputItem::BinOp(BinOp::Sub),
            '×' | '*' => InputItem::BinOp(BinOp::Mul),
            '÷' | '/' => InputItem::BinOp(BinOp::Div),
            '^' => InputItem::BinOp(BinOp::Pow),
            '%' => InputItem::Percent,
            '!' => InputItem::Factorial,
            '(' => {
                arg_sep_stack.push(false);
                InputItem::LeftParen
            }
            ')' => {
                arg_sep_stack.pop();
                InputItem::RightParen
            }
            '√' => InputItem::UnaryFunc(UnaryFunc::Sqrt),
            '∛' => InputItem::UnaryFunc(UnaryFunc::Cbrt),
            'π' => InputItem::Constant(ConstKind::Pi),
            '𝑒' | 'e' => InputItem::Constant(ConstKind::E),
            // Anything else would have to be dropped to continue, and
            // dropping changes the meaning of the expression.
            _ => return None,
        };
        // `√` and `∛` carry an implicit opener like the named
        // functions do, so a literal `(` after them is redundant.
        if matches!(c, '√' | '∛') {
            out.push(item);
            i += 1;
            if chars.get(i) == Some(&'(') {
                i += 1;
            }
            arg_sep_stack.push(false);
            continue;
        }
        out.push(item);
        i += 1;
    }

    Some(out)
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

/// Match a multi-char keyword at `i`. Longest-match wins, so `sinh-1`
/// is preferred over `sinh` and `log10` over `log`.
fn match_keyword(chars: &[char], i: usize) -> Option<(InputItem, usize)> {
    use UnaryFunc as U;
    const RULES: &[(&str, InputItem)] = &[
        ("sinh-1", InputItem::UnaryFunc(U::Asinh)),
        ("cosh-1", InputItem::UnaryFunc(U::Acosh)),
        ("tanh-1", InputItem::UnaryFunc(U::Atanh)),
        ("coth-1", InputItem::UnaryFunc(U::Acoth)),
        ("sin-1", InputItem::UnaryFunc(U::Asin)),
        ("cos-1", InputItem::UnaryFunc(U::Acos)),
        ("tan-1", InputItem::UnaryFunc(U::Atan)),
        ("cot-1", InputItem::UnaryFunc(U::Acot)),
        ("sinh", InputItem::UnaryFunc(U::Sinh)),
        ("cosh", InputItem::UnaryFunc(U::Cosh)),
        ("tanh", InputItem::UnaryFunc(U::Tanh)),
        ("coth", InputItem::UnaryFunc(U::Coth)),
        ("log10", InputItem::UnaryFunc(U::Log10)),
        ("log2", InputItem::UnaryFunc(U::Log2)),
        ("root", InputItem::BinaryFunc(BinaryFunc::Root)),
        ("sin", InputItem::UnaryFunc(U::Sin)),
        ("cos", InputItem::UnaryFunc(U::Cos)),
        ("tan", InputItem::UnaryFunc(U::Tan)),
        ("cot", InputItem::UnaryFunc(U::Cot)),
        ("mod", InputItem::Modulo),
        ("ln", InputItem::UnaryFunc(U::Ln)),
        ("log", InputItem::UnaryFunc(U::Log)),
        ("pi", InputItem::Constant(ConstKind::Pi)),
    ];
    for (name, item) in RULES {
        let len = name.chars().count();
        if i + len <= chars.len() && chars[i..i + len].iter().copied().eq(name.chars()) {
            return Some((item.clone(), len));
        }
    }
    None
}

/// True when the item renders its own opening paren.
fn item_opens_paren(item: &InputItem) -> bool {
    matches!(
        item,
        InputItem::UnaryFunc(_) | InputItem::BinaryFunc(_) | InputItem::LogN(_)
    )
}

/// Length of the `e[+-]?<digits>` run starting at `i`, when it should
/// be read as a decimal exponent rather than as Euler's number: the
/// item before it has to close a numeric literal, and at least one
/// digit has to follow. Returns `None` otherwise, so `2𝑒` and `𝑒3`
/// still mean multiplication by the constant.
fn exponent_suffix_len(chars: &[char], i: usize, out: &[InputItem]) -> Option<usize> {
    if !matches!(chars.get(i), Some('e') | Some('𝑒')) {
        return None;
    }
    if !matches!(
        out.last(),
        Some(InputItem::Digit(_)) | Some(InputItem::DecimalPoint)
    ) {
        return None;
    }
    let mut j = i + 1;
    if matches!(chars.get(j), Some('+') | Some('-')) {
        j += 1;
    }
    let digits_start = j;
    while matches!(chars.get(j), Some(d) if d.is_ascii_digit()) {
        j += 1;
    }
    if j == digits_start {
        return None;
    }
    Some(j - i)
}

/// Append `×10^<exponent>` for the sign-and-digits run in `suffix`.
/// A negative exponent is parenthesised so the engine reads it as one
/// signed operand.
fn push_exponent(out: &mut Vec<InputItem>, suffix: &[char]) {
    out.push(InputItem::BinOp(BinOp::Mul));
    out.push(InputItem::Digit('1'));
    out.push(InputItem::Digit('0'));
    out.push(InputItem::BinOp(BinOp::Pow));
    let negative = suffix.first() == Some(&'-');
    if negative {
        out.push(InputItem::LeftParen);
        out.push(InputItem::BinOp(BinOp::Sub));
    }
    for d in suffix.iter().filter(|c| c.is_ascii_digit()) {
        out.push(InputItem::Digit(*d));
    }
    if negative {
        out.push(InputItem::RightParen);
    }
}
