//! Superscript / subscript rendering of an expression.
//!
//! The buffer stores `2 ^ 2` and `log2(`; a calculator display is
//! expected to show `2²` and `log₂(`. Everything needed to make that
//! swap lives here so the display renderer, the caption above it and
//! the history panel all reach the same conclusion about a given run
//! of items.
//!
//! Two renderings come out of that, and they part company over *how*
//! a glyph gets raised:
//!
//!   * The main display draws a script as ordinary characters at a
//!     smaller size, moved off the line. Nothing there depends on the
//!     font having a superscript for what is being raised, so an
//!     exponent can hold a decimal separator, a factorial, or a whole
//!     `sin(` call and still read as an exponent. That rendering is
//!     the display module's; what this module gives it is where each
//!     piece belongs — [`pretty_parts`] for the items whose form is a
//!     substitution, [`exponent_span`] and [`argument_separator`] for
//!     the runs that move.
//!   * A one-line rendering — a history row — has one font size to
//!     work with, so it borrows Unicode's superscript and subscript
//!     blocks through [`raise`] and [`lower`]. Those are
//!     all-or-nothing: a run raises only when every character of it
//!     has a raised form, and otherwise is written at full size inside
//!     raised brackets — `2⁽2!⁾` — rather than with a mix of sizes that
//!     would read as a different expression. It is the poorer of the
//!     two readings, which is why only the rows in the history panel
//!     are drawn from it: past two levels Unicode runs out and the
//!     brackets take over, so `2^2^2` comes back as `2⁽2²⁾`.
//!
//! Either way the `^` the buffer stores never reaches the pretty
//! display: it is what the raising is standing in for, and it is still
//! there in the raw form and in what the tokenizer is handed.
//!
//! [`exponent_span`] covers exactly what the parser treats as the
//! exponent — `power = postfix ('^' unary)?` — so `2^3π` raises only
//! the `3` (the `π` is a separate factor) while `2^2!` raises the `2!`
//! together, because the `!` belongs to the exponent and `2²!` would
//! read as `(2²)!`.
//!
//! The same rule runs the other way for the two-argument calls:
//! [`argument_separator`] finds the comma between the arguments, which
//! is where the base of a `log(base, value)` ends and where the degree
//! of a `root(value, degree)` starts. Both of those move out of the
//! brackets — the base under the `log`, the degree in front of the
//! radical — and a slot not typed yet shows as [`EMPTY_SLOT`], so
//! `log₍₎(8)` says which slot the next digit lands in.

use crate::engine::item::{unary_func_name, BinOp, BinaryFunc, InputItem, UnaryFunc};

/// How an expression is rendered for the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Notation {
    /// The text the clipboard carries and the tokenizer is handed:
    /// `root(2^2,6)`, `log2(8)`, `sin-1(1)`, `pi*e`. All of it ASCII,
    /// down to the spelled-out constants and the `sqrt(` the radical
    /// stands for, so what the settings panel's "Show ASCII
    /// expression" toggle draws is exactly what Copy would put on the
    /// clipboard.
    Raw,
    /// Exponents raised and log bases lowered: `⁶√(2²)`, `log₂(8)`,
    /// `sin⁻¹(1)`.
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

/// Where a piece of an item's pretty form sits relative to the line
/// the expression is written on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    /// Full size, on the line: the `log` of `log₂(`, its bracket.
    OnLine,
    /// One script step up: the `-1` of `sin⁻¹(`.
    Up,
    /// One script step down: the `2` of `log₂(`.
    Down,
    /// One step up, and written *into* the radical that follows it
    /// rather than beside it: the `3` of `³√(`. Where the step up ends
    /// is the display's business — see the root degree — but which
    /// pieces belong there has to be said here, since a cube root is
    /// one item and the display cannot tell its degree from its sign
    /// without being told.
    Degree,
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
        // Unicode has no superscript decimal separator, so the two the
        // display can be set to borrow the nearest raised glyph there
        // is: a middle dot for `.`, a modifier apostrophe for `,`.
        // Without them one separator dropped the whole exponent back to
        // full size — `2^1.5` read as `2⁽1.5⁾` — which is the case a
        // power key is most often reached for. The middle dot is
        // unambiguous here: this display spells multiplication `×`,
        // never `·`.
        '.' => '·',
        ',' => 'ʼ',
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
/// different expression, so the caller falls back to [`raise`].
pub fn to_superscript(s: &str) -> Option<String> {
    s.chars().map(superscript_char).collect()
}

/// What a power looks like without its `^` on a one-line display: the
/// exponent raised outright when every character of it has a
/// superscript, and written at full size inside raised brackets when
/// one of them does not.
///
/// The brackets are the point — `2⁽π⁾` says "2 to the π" where a bare
/// `2π` would say "2 times π", which is a different number. They are
/// only reached by exponents Unicode cannot raise: a constant, a
/// factorial, a decimal point, a function call. The main display,
/// which can shrink a glyph instead of swapping it, never needs them.
pub fn raise(exponent: &str) -> String {
    match to_superscript(exponent) {
        Some(raised) => raised,
        None => format!("{EXPONENT_OPEN}{exponent}{EXPONENT_CLOSE}"),
    }
}

const EXPONENT_OPEN: char = '⁽';
const EXPONENT_CLOSE: char = '⁾';

/// Lower a whole string, with the same all-or-nothing rule as
/// [`to_superscript`].
pub fn to_subscript(s: &str) -> Option<String> {
    s.chars().map(subscript_char).collect()
}

/// What the base of a `log(base, value)` call looks like under the
/// `log` on a one-line display: lowered outright when every character
/// of it has a subscript, and written at full size inside lowered
/// brackets when one of them does not. The mirror of [`raise`], and
/// for the same reason — a base of `π` written as a bare `logπ(x)`
/// would read as `log` times `π`.
pub fn lower(base: &str) -> String {
    match to_subscript(base) {
        Some(lowered) => lowered,
        None => format!("{BASE_OPEN}{base}{BASE_CLOSE}"),
    }
}

const BASE_OPEN: char = '₍';
const BASE_CLOSE: char = '₎';

/// The empty script slot: what a power shows before its exponent is
/// typed, and what a two-argument call shows in the slot it draws
/// outside its brackets — the `log_y` base, the root degree. Drawn
/// small and off the line like anything else in that slot, so
/// `log₍₎(8)` and `⁽⁾√(16)` say which slot the next digit lands in.
/// The display draws no cursor of its own, so without them the base
/// step would be invisible.
pub const EMPTY_SLOT: &str = "()";

/// The pieces an item is drawn as in pretty notation, each with where
/// it sits. Covers the items whose whole rendering is a substitution —
/// the log bases and the inverse functions; everything else (digits,
/// operators, parens) renders the same in both notations and comes
/// back as a single on-line piece.
///
/// The runs that have to be found rather than substituted are not
/// here: an exponent needs [`exponent_span`] and a two-argument call
/// needs [`argument_separator`], so both are the renderer's business.
pub fn pretty_parts(it: &InputItem) -> Vec<(String, Shift)> {
    match it {
        // `root(x,n)` wears the radical the square and cube roots do —
        // it is the same operation with the degree spelled out, and
        // `root` is the buffer's spelling, not a notation. The degree
        // itself is moved in front of the sign by the renderer, which
        // is the only place that can see where it ends.
        InputItem::BinaryFunc(BinaryFunc::Root) => vec![("√(".to_string(), Shift::OnLine)],
        // The cube root is the same radical with a degree of 3, and
        // written that way it needs nothing of the font but a `3` and
        // a `√`. `∛` is one glyph, and a font that lacks it (many do)
        // leaves the user with a box where the operation should be.
        // The buffer still holds `cbrt(`, and so does the ASCII form.
        InputItem::UnaryFunc(UnaryFunc::Cbrt) => vec![
            ("3".to_string(), Shift::Degree),
            ("√(".to_string(), Shift::OnLine),
        ],
        InputItem::UnaryFunc(UnaryFunc::Log2) => log_parts("2"),
        InputItem::UnaryFunc(UnaryFunc::Log10) => log_parts("10"),
        InputItem::LogN(n) => log_parts(&n.to_string()),
        // `sin-1`, `cosh-1`, … are stored with an ASCII `-1` suffix.
        InputItem::UnaryFunc(f) => match unary_func_name(*f).strip_suffix("-1") {
            Some(base) => vec![
                (base.to_string(), Shift::OnLine),
                ("-1".to_string(), Shift::Up),
                ("(".to_string(), Shift::OnLine),
            ],
            None => vec![(it.display(), Shift::OnLine)],
        },
        _ => vec![(it.display(), Shift::OnLine)],
    }
}

/// `log`, its base under it, and the bracket the argument opens.
fn log_parts(base: &str) -> Vec<(String, Shift)> {
    vec![
        ("log".to_string(), Shift::OnLine),
        (base.to_string(), Shift::Down),
        ("(".to_string(), Shift::OnLine),
    ]
}

/// Index of the comma separating the two arguments of the call opened
/// at `call_idx`. `None` when there is no top-level comma inside it —
/// a call the user is still typing the first argument of, or the
/// one-argument reading of `log(`, which is log10.
///
/// For a `log(base, value)` that comma is where the base ends and for
/// a `root(value, degree)` it is where the degree starts, which is
/// what the display needs to know to move either of them out of the
/// brackets, so both ends come from one walk of the items rather than
/// two guesses.
pub fn argument_separator(items: &[InputItem], call_idx: usize) -> Option<usize> {
    if !matches!(
        items.get(call_idx),
        Some(
            InputItem::LeftParen
                | InputItem::UnaryFunc(_)
                | InputItem::BinaryFunc(_)
                | InputItem::LogN(_)
        )
    ) {
        return None;
    }
    // The item at `call_idx` is (or carries) the opening bracket, so
    // the walk starts one level deep and ends when that level closes.
    let mut depth = 1usize;
    for (i, it) in items.iter().enumerate().skip(call_idx + 1) {
        match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => depth += 1,
            InputItem::RightParen => {
                depth -= 1;
                if depth == 0 {
                    return None;
                }
            }
            InputItem::Comma if depth == 1 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Index of the `)` that closes the paren-opening item at `opener` —
/// a bare `(`, or the bracket a function name carries. `None` while
/// the group is still open.
pub fn closing_paren(items: &[InputItem], opener: usize) -> Option<usize> {
    if !matches!(
        items.get(opener),
        Some(
            InputItem::LeftParen
                | InputItem::UnaryFunc(_)
                | InputItem::BinaryFunc(_)
                | InputItem::LogN(_)
        )
    ) {
        return None;
    }
    let mut depth = 1usize;
    for (j, it) in items.iter().enumerate().skip(opener + 1) {
        match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => depth += 1,
            InputItem::RightParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

/// How many script steps from the line each item is drawn at: 0 on the
/// line, 1 in an exponent or a base, 2 in the exponent of an exponent.
/// One entry per item, so a caller can ask about a position rather
/// than walk the expression itself.
///
/// The walk is the renderer's, and has to stay it: the three runs that
/// step are exactly the three it moves — the items an [`exponent_span`]
/// covers, the base of a `log_y` (up to its [`argument_separator`]) and
/// the degree of a root (from that comma to the call's closer). An item
/// the renderer steps over rather than draws — the `^`, the comma —
/// answers with the depth of the run it sits in, which is where
/// anything typed in its place would land.
///
/// What it is for is the limit on nesting: three levels is as deep as
/// the display goes before the text is too small to read, so the
/// keypad asks how deep a position already is before it writes another
/// script there.
pub fn script_depths(items: &[InputItem]) -> Vec<u8> {
    let mut depths = vec![0u8; items.len()];
    fill_depths(items, 0, items.len(), 0, &mut depths);
    depths
}

/// One run of [`script_depths`], `items[start..end]` drawn at `depth`.
fn fill_depths(items: &[InputItem], start: usize, end: usize, depth: u8, out: &mut [u8]) {
    let deeper = depth.saturating_add(1);
    let mut i = start;
    while i < end {
        out[i] = depth;
        match &items[i] {
            InputItem::BinOp(BinOp::Pow) => match exponent_span(items, i).map(|s| s.min(end)) {
                Some(span) if span > i + 1 => {
                    fill_depths(items, i + 1, span, deeper, out);
                    i = span;
                }
                _ => i += 1,
            },
            InputItem::BinaryFunc(BinaryFunc::LogBase) => {
                match argument_separator(items, i).filter(|comma| *comma < end) {
                    Some(comma) => {
                        fill_depths(items, i + 1, comma, deeper, out);
                        out[comma] = depth;
                        i = comma + 1;
                    }
                    None => i += 1,
                }
            }
            InputItem::BinaryFunc(BinaryFunc::Root) => {
                match (
                    argument_separator(items, i),
                    closing_paren(items, i).filter(|closer| *closer < end),
                ) {
                    (Some(comma), Some(closer)) => {
                        // The radicand stays on the line the call is
                        // written on; only the degree steps.
                        fill_depths(items, i + 1, comma, depth, out);
                        out[comma] = depth;
                        fill_depths(items, comma + 1, closer, deeper, out);
                        out[closer] = depth;
                        i = closer + 1;
                    }
                    _ => i += 1,
                }
            }
            _ => i += 1,
        }
    }
}

/// End (exclusive) of the exponent the `^` at `pow_idx` raises its
/// base to. `None` when there is no exponent to raise yet — nothing
/// after the caret, or a bracketed group the user has not closed.
///
/// The span follows the parser's `power = postfix ('^' unary)?` rule:
/// any leading signs, then one primary (a number, a constant or a
/// bracketed group), then the postfix `!` / `%` that bind to it, and
/// then — powers being right-associative — a chained `^` and whatever
/// *it* raises. So `2^3π` covers only the `3` (the `π` being a factor
/// of its own), while `2^2!` covers `2!` and `2^3^2` covers `3^2`:
/// raising less than that would read as `(2²)!` and `(2³)²`, which are
/// different numbers.
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
        // A power whose base is not typed yet — what `yˣ` leaves
        // behind when the operand it swapped was already an exponent.
        // The pending power is this one's exponent, so it goes up as
        // one: `2^^3` is `2` raised to (something raised to `3`).
        InputItem::BinOp(BinOp::Pow) => return Some(exponent_span(items, i).unwrap_or(i + 1)),
        // Any of these carries an opening bracket; the exponent runs
        // to the closer that matches it.
        InputItem::LeftParen
        | InputItem::UnaryFunc(_)
        | InputItem::BinaryFunc(_)
        | InputItem::LogN(_) => {
            let mut depth = 0usize;
            loop {
                match items.get(i) {
                    // A group the user is still inside: there is no
                    // exponent to close over yet.
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
    while matches!(
        items.get(i),
        Some(InputItem::Factorial | InputItem::Percent)
    ) {
        i += 1;
    }
    if matches!(items.get(i), Some(InputItem::BinOp(BinOp::Pow))) {
        // Right-associative: the chained power and its own exponent
        // are part of this one. A chain whose last caret has nothing
        // to raise yet still ends there — the empty slot belongs to
        // that caret, one step further up, and the part already typed
        // stays where it is instead of the whole chain dropping back
        // to the line: `2^2^` is `2` with a raised `2` waiting on its
        // own exponent, not two powers side by side.
        return Some(exponent_span(items, i).unwrap_or(i + 1));
    }
    Some(i)
}
