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
//!
//! The one thing that is dropped rather than refused is a letter that
//! is on the allow-list but starts no keyword — the list only admits
//! letters that appear in function names, so a leftover one is a stray
//! character. `l root(5, 4)` is `root(5, 4)`.

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

/// The whole paste decision in one place: what the clipboard delivered
/// in, the items to replace the buffer with out, or `None` when the
/// paste must be silently ignored.
///
/// `None` is returned for every rejection the spec lists:
///
/// * the clipboard held something that is not text, which the UI
///   surfaces as a `None` payload
/// * the text ran past [`MAX_PASTE_CHARS`], or held a character outside
///   the allow-list ([`sanitize_paste`])
/// * the text held something the buffer cannot represent faithfully
///   ([`items_from_paste`])
/// * the text was empty, or reduced to nothing
///
/// Having one entry point keeps the policy testable without standing up
/// a libcosmic application, which is where the not-text case used to
/// live and therefore go untested.
pub fn paste_items(payload: Option<&str>) -> Option<Vec<InputItem>> {
    let raw = payload?;
    let clean = sanitize_paste(raw)?;
    let items = items_from_paste(&clean)?;
    if items.is_empty() {
        return None;
    }
    Some(items)
}

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

    // Pass 2: char-level substitutions (case fold, unicode glyphs).
    // Runs before the exponent pass so a full-width `＋` is already an
    // ASCII sign by the time an exponent is looked for.
    let mut canonical = String::with_capacity(kept.len());
    for ch in kept.chars() {
        match substitute_char(ch) {
            Some(s) => canonical.push_str(s),
            None => canonical.push(ch),
        }
    }

    // Pass 3: settle what each ASCII `e` means while the spacing is
    // still intact — see `mark_euler_constants`.
    let eulered = mark_euler_constants(&canonical);

    // Pass 4: space handling – drop every ' ' except those preceded
    // by ','.
    let mut spaced = String::with_capacity(eulered.len());
    let mut prev: Option<char> = None;
    for ch in eulered.chars() {
        if ch == ' ' {
            if prev == Some(',') {
                spaced.push(' ');
            }
        } else {
            spaced.push(ch);
        }
        prev = Some(ch);
    }

    // Pass 5: function-name rewrites. Longer names first to avoid
    // `asinh` being eaten as `asin` + trailing `h`.
    let functional = rewrite_function_names(&spaced);

    Some(functional)
}

/// Rewrite each ASCII `e` that cannot be a decimal exponent into the
/// italic `𝑒`, which always means Euler's number.
///
/// An `e` is an exponent only when a mantissa sits directly before it
/// and an optional sign plus at least one digit sits directly after it,
/// with no space on either side. Whitespace is load-bearing here and
/// nowhere else, which is why this runs before spaces are dropped:
/// `2e8` is 2×10⁸, but `2e +2` is 2·𝑒 + 2, and once the space is gone
/// the two are indistinguishable.
fn mark_euler_constants(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        if c != 'e' {
            out.push(c);
            continue;
        }
        let mantissa_before = i > 0 && matches!(chars.get(i - 1), Some('0'..='9') | Some('.'));
        let mut j = i + 1;
        if matches!(chars.get(j), Some('+') | Some('-')) {
            j += 1;
        }
        let digits_after = matches!(chars.get(j), Some(d) if d.is_ascii_digit());
        // Only the ambiguous shape is rewritten: an `e` that looks like
        // it could continue a number but has no exponent after it. A
        // bare `e` is left alone, so the spec's `E` -> `e` fold still
        // holds and the tokenizer reads it as the constant either way.
        if mantissa_before && !digits_after {
            out.push('𝑒');
        } else {
            out.push('e');
        }
    }
    out
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
/// character. The exception is a stray allow-listed letter, which is
/// skipped — see the module docs.
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

        // `log<digits>(` carries its base in the name. Checked before
        // the keyword table so `log6` is not read as `log` followed by
        // a stray 6 — which turned `log6(279936)` into `log(6(279936)`
        // = 6.23 where the engine, handed the same text directly,
        // answers 7.
        if let Some((base, len)) = log_base_suffix_len(&chars, i) {
            out.push(InputItem::LogN(base));
            i += len;
            if chars.get(i) == Some(&'(') {
                i += 1;
            }
            arg_sep_stack.push(false);
            continue;
        }

        if let Some((item, len)) = match_keyword(&chars, i) {
            let takes_arg_list = matches!(
                item,
                InputItem::BinaryFunc(_) | InputItem::UnaryFunc(UnaryFunc::Log)
            );
            if item_opens_paren(&item) {
                i = open_function(
                    &mut out,
                    &chars,
                    i + len,
                    item,
                    takes_arg_list,
                    &mut arg_sep_stack,
                );
            } else {
                out.push(item);
                i += len;
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
            // A letter that starts no keyword is dropped. The
            // allow-list only admits letters that appear in function
            // names, so this is a stray character rather than a token
            // we failed to understand — `l root(5, 4)` is `root(5, 4)`.
            c if c.is_ascii_alphabetic() => {
                i += 1;
                continue;
            }
            // Anything else would have to be dropped to continue, and
            // dropping changes the meaning of the expression.
            _ => return None,
        };
        // `√` and `∛` carry an implicit opener like the named
        // functions do.
        if matches!(c, '√' | '∛') {
            i = open_function(&mut out, &chars, i + 1, item, false, &mut arg_sep_stack);
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
        'h' | 'H' | 'c' | 'C' | 't' | 'T' | 's' | 'S' | 'o' | 'O' | 'a' | 'A' | 'l' | 'L' | 'g'
        | 'G' | 'n' | 'N' | 'm' | 'M' | 'd' | 'D' | 'q' | 'Q' | 'r' | 'R' | 'b' | 'B' | 'p'
        | 'P' | 'i' | 'I' | 'e' | 'E' => true,
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
        'ℯ' | 'ｅ' | '𝐞' | '𝒆' | '𝓮' | '𝖾' | '𝗲' | '𝘦' | '𝙚' | '𝚎' => {
            "𝑒"
        }
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
        // `sin^-1` is the other spelling the README advertises for the
        // inverse functions. Folding it to `sin-1` here means the
        // keyword table below needs only the one form. Safe because a
        // `^-1` only reaches this rule directly after a function name;
        // `2^-1` keeps its caret and stays a power.
        ("sin^-1", "sin-1"),
        ("cos^-1", "cos-1"),
        ("tan^-1", "tan-1"),
        ("cot^-1", "cot-1"),
        ("ctg^-1", "ctg-1"),
        ("sinh^-1", "sinh-1"),
        ("cosh^-1", "cosh-1"),
        ("tanh^-1", "tanh-1"),
        ("coth^-1", "coth-1"),
        ("ctgh^-1", "ctgh-1"),
    ];
    let mut out = String::with_capacity(input.len());
    let bytes: &[u8] = input.as_bytes();
    let mut i = 0usize;
    'outer: while i < bytes.len() {
        // Multi-byte chars won't match any of the rules, so it's safe
        // to index by byte for the prefix check.
        for (from, to) in RULES {
            let from_bytes = from.as_bytes();
            if i + from_bytes.len() <= bytes.len() && &bytes[i..i + from_bytes.len()] == from_bytes
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
        ("ctgh-1", InputItem::UnaryFunc(U::Acoth)),
        ("sin-1", InputItem::UnaryFunc(U::Asin)),
        ("cos-1", InputItem::UnaryFunc(U::Acos)),
        ("tan-1", InputItem::UnaryFunc(U::Atan)),
        ("cot-1", InputItem::UnaryFunc(U::Acot)),
        ("ctg-1", InputItem::UnaryFunc(U::Acot)),
        ("sinh", InputItem::UnaryFunc(U::Sinh)),
        ("cosh", InputItem::UnaryFunc(U::Cosh)),
        ("tanh", InputItem::UnaryFunc(U::Tanh)),
        ("coth", InputItem::UnaryFunc(U::Coth)),
        ("ctgh", InputItem::UnaryFunc(U::Coth)),
        ("log10", InputItem::UnaryFunc(U::Log10)),
        ("log2", InputItem::UnaryFunc(U::Log2)),
        ("root", InputItem::BinaryFunc(BinaryFunc::Root)),
        ("sin", InputItem::UnaryFunc(U::Sin)),
        ("cos", InputItem::UnaryFunc(U::Cos)),
        ("tan", InputItem::UnaryFunc(U::Tan)),
        ("cot", InputItem::UnaryFunc(U::Cot)),
        ("ctg", InputItem::UnaryFunc(U::Cot)),
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

/// Match `log<digits>` at `i`, returning the base and the run length.
/// `log2` and `log10` have their own [`UnaryFunc`] variants and are
/// left to the keyword table; every other base becomes a
/// [`InputItem::LogN`], which is what the engine tokenizer produces for
/// the same text.
fn log_base_suffix_len(chars: &[char], i: usize) -> Option<(u32, usize)> {
    const LOG: [char; 3] = ['l', 'o', 'g'];
    if chars.get(i..i + LOG.len())? != LOG {
        return None;
    }
    let digits_start = i + LOG.len();
    let mut j = digits_start;
    while matches!(chars.get(j), Some(d) if d.is_ascii_digit()) {
        j += 1;
    }
    if j == digits_start {
        return None;
    }
    let base: u32 = chars[digits_start..j]
        .iter()
        .collect::<String>()
        .parse()
        .ok()?;
    if matches!(base, 2 | 10) {
        return None;
    }
    Some((base, j - i))
}

/// Push a function item and decide how its implicit `(` is closed.
///
/// A literal `(` in the source is consumed (the item renders its own)
/// and the group stays open until the matching `)`. Without one, the
/// function binds to just the operand that follows and the group is
/// closed immediately — matching what the engine does with the same
/// text, where `sqrt16-2` is `√(16) - 2` = 2. Treating the implicit
/// opener as running to the next `)` instead turned `(√16-2)!` into
/// `√(16-2)!`, quietly changing the expression.
///
/// Falls back to leaving the group open when what follows is not a
/// plain operand, so `√log3(2)` still reads as `√(log₃2)`.
fn open_function(
    out: &mut Vec<InputItem>,
    chars: &[char],
    mut i: usize,
    item: InputItem,
    takes_arg_list: bool,
    arg_sep_stack: &mut Vec<bool>,
) -> usize {
    out.push(item);
    if chars.get(i) == Some(&'(') {
        arg_sep_stack.push(takes_arg_list);
        return i + 1;
    }
    if let Some(next) = push_bare_operand(out, chars, i) {
        i = next;
    } else {
        arg_sep_stack.push(takes_arg_list);
    }
    i
}

/// Consume the single operand a parenthesis-less function applies to —
/// a numeric run or a constant, plus any postfix `!` / `%` — and close
/// the group after it. Returns `None` (having pushed nothing) when what
/// follows is not such an operand.
fn push_bare_operand(out: &mut Vec<InputItem>, chars: &[char], start: usize) -> Option<usize> {
    let mark = out.len();
    let mut i = start;
    let mut seen_digit = false;
    let mut seen_dot = false;
    while let Some(&c) = chars.get(i) {
        match c {
            '0'..='9' => {
                out.push(InputItem::Digit(c));
                seen_digit = true;
            }
            '.' if !seen_dot => {
                out.push(InputItem::DecimalPoint);
                seen_dot = true;
            }
            _ => break,
        }
        i += 1;
    }
    if !seen_digit {
        out.truncate(mark);
        i = start;
        match chars.get(i) {
            Some('π') => out.push(InputItem::Constant(ConstKind::Pi)),
            Some('𝑒') => out.push(InputItem::Constant(ConstKind::E)),
            _ => return None,
        }
        i += 1;
    }
    // `sqrt16!` is sqrt(16!) in the engine, so postfixes bind inside.
    while let Some(&c) = chars.get(i) {
        match c {
            '!' => out.push(InputItem::Factorial),
            '%' => out.push(InputItem::Percent),
            _ => break,
        }
        i += 1;
    }
    out.push(InputItem::RightParen);
    Some(i)
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
/// digit has to follow. Returns `None` otherwise, so `2e` and `e3`
/// still mean multiplication by the constant.
///
/// Only the plain ASCII `e` is eligible. The italic `𝑒` is the
/// calculator's symbol for Euler's number and always means the
/// constant, so `2𝑒3` is 2·𝑒·3 while `2e3` is 2000 — which is how the
/// two forms are written in the README's compatibility examples.
fn exponent_suffix_len(chars: &[char], i: usize, out: &[InputItem]) -> Option<usize> {
    if chars.get(i) != Some(&'e') {
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
