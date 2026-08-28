//! Key names ⇄ calculator actions. The keypad layout in `config.toml`
//! names each cell with a short lowercase string; this module is the
//! single place that says which [`Button`] a name stands for and what
//! glyph the key wears.
//!
//! The table below is the source of truth in both directions: the
//! first entry for a given `Button` is its canonical name (the one
//! written back into the config file), and every later entry for the
//! same button is an accepted alias, so a user can write `x^2`, `x2`
//! or `square` and mean the same key.
//!
//! Names the table doesn't know are not an error: the cell is drawn
//! blank and a one-line warning goes to stderr, which beats refusing
//! to start over a typo in a hand-edited file.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::config::{Config, Mode};
use crate::engine::script::Shift;
use crate::layout::{canonical, LayoutKind};
use crate::ui::buttons::Button;

/// Labels that depend on live state rather than on the key alone.
#[derive(Debug, Clone, Copy)]
pub struct LabelContext {
    /// `AC` on an empty buffer, `C` once something has been typed.
    pub clear: &'static str,
    /// `.` or `,` per the configured decimal separator.
    pub decimal: &'static str,
    /// `DEG` or `RAD`, per the current angle mode.
    pub angle: &'static str,
}

impl Default for LabelContext {
    fn default() -> Self {
        Self {
            clear: "AC",
            decimal: ".",
            angle: "DEG",
        }
    }
}

/// Every name the config accepts, in `(name, button)` form. The first
/// row for each button carries its canonical name.
pub const KEY_NAMES: &[(&str, Button)] = &[
    // --- digits and basic entry ---
    ("0", Button::Num(0)),
    ("1", Button::Num(1)),
    ("2", Button::Num(2)),
    ("3", Button::Num(3)),
    ("4", Button::Num(4)),
    ("5", Button::Num(5)),
    ("6", Button::Num(6)),
    ("7", Button::Num(7)),
    ("8", Button::Num(8)),
    ("9", Button::Num(9)),
    ("decimal", Button::Decimal),
    (".", Button::Decimal),
    (",", Button::Decimal),
    ("negate", Button::Negate),
    ("+/-", Button::Negate),
    ("+/−", Button::Negate),
    ("±", Button::Negate),
    ("backspace", Button::Backspace),
    ("⌫", Button::Backspace),
    ("del", Button::Backspace),
    // --- control ---
    ("clear", Button::Clear),
    ("ac", Button::Clear),
    ("c", Button::Clear),
    ("equals", Button::Equals),
    ("=", Button::Equals),
    ("second", Button::Second),
    ("2nd", Button::Second),
    ("lparen", Button::LeftParen),
    ("(", Button::LeftParen),
    ("rparen", Button::RightParen),
    (")", Button::RightParen),
    ("left", Button::CursorLeft),
    ("cursorleft", Button::CursorLeft),
    ("←", Button::CursorLeft),
    ("right", Button::CursorRight),
    ("cursorright", Button::CursorRight),
    ("→", Button::CursorRight),
    ("home", Button::CursorHome),
    ("end", Button::CursorEnd),
    // --- operators ---
    ("add", Button::Add),
    ("+", Button::Add),
    ("sub", Button::Sub),
    ("-", Button::Sub),
    ("−", Button::Sub),
    ("mul", Button::Mul),
    ("*", Button::Mul),
    ("×", Button::Mul),
    ("div", Button::Div),
    ("/", Button::Div),
    ("÷", Button::Div),
    ("pow", Button::Pow),
    ("^", Button::Pow),
    ("mod", Button::Mod),
    ("percent", Button::Percent),
    ("%", Button::Percent),
    ("factorial", Button::Factorial),
    ("x!", Button::Factorial),
    ("!", Button::Factorial),
    ("ee", Button::EE),
    // --- unary functions ---
    ("sqrt", Button::Sqrt),
    ("√", Button::Sqrt),
    ("cbrt", Button::Cbrt),
    ("∛", Button::Cbrt),
    ("sin", Button::Sin),
    ("cos", Button::Cos),
    ("tan", Button::Tan),
    ("asin", Button::Asin),
    ("sin-1", Button::Asin),
    ("acos", Button::Acos),
    ("cos-1", Button::Acos),
    ("atan", Button::Atan),
    ("tan-1", Button::Atan),
    ("sinh", Button::Sinh),
    ("cosh", Button::Cosh),
    ("tanh", Button::Tanh),
    ("asinh", Button::Asinh),
    ("sinh-1", Button::Asinh),
    ("acosh", Button::Acosh),
    ("cosh-1", Button::Acosh),
    ("atanh", Button::Atanh),
    ("tanh-1", Button::Atanh),
    ("ln", Button::Ln),
    ("log", Button::Log10),
    ("log10", Button::Log10),
    ("log2", Button::Log2),
    // --- binary functions ---
    ("yrootx", Button::YRootX),
    ("root", Button::YRootX),
    ("logy", Button::LogY),
    ("logbase", Button::LogY),
    // --- power shortcuts ---
    ("square", Button::Square),
    ("x^2", Button::Square),
    ("x2", Button::Square),
    ("cube", Button::Cube),
    ("x^3", Button::Cube),
    ("x3", Button::Cube),
    ("xpowy", Button::XPowY),
    ("x^y", Button::XPowY),
    ("ypowx", Button::YPowX),
    ("y^x", Button::YPowX),
    ("tenpowx", Button::TenPowX),
    ("10^x", Button::TenPowX),
    ("twopowx", Button::TwoPowX),
    ("2^x", Button::TwoPowX),
    ("epowx", Button::EPowX),
    ("e^x", Button::EPowX),
    // --- constants ---
    ("pi", Button::Pi),
    ("π", Button::Pi),
    ("e", Button::Euler),
    ("euler", Button::Euler),
    ("𝑒", Button::Euler),
    // --- special ---
    ("reciprocal", Button::Reciprocal),
    ("1/x", Button::Reciprocal),
    ("inv", Button::Reciprocal),
    ("rand", Button::Rand),
    ("random", Button::Rand),
    // --- memory ---
    ("mc", Button::MemClear),
    ("mr", Button::MemRecall),
    ("m+", Button::MemAdd),
    ("m-", Button::MemSub),
    // --- panels and modes ---
    ("history", Button::ToggleHistoryPanel),
    ("settings", Button::ToggleSettingsPanel),
    ("mode", Button::ToggleMode),
    ("angle", Button::ToggleAngleMode),
    ("deg", Button::ToggleAngleMode),
    ("rad", Button::ToggleAngleMode),
];

/// Resolve one configured cell name to the action it triggers.
/// Returns `None` for a blank cell or a name the table doesn't know.
pub fn button_for_name(name: &str) -> Option<Button> {
    let wanted = canonical(name);
    if wanted.is_empty() {
        return None;
    }
    KEY_NAMES
        .iter()
        .find(|(n, _)| *n == wanted)
        .map(|(_, b)| *b)
}

/// Canonical config name for an action — the reverse direction, used
/// to look a keystroke up in the user's own layout.
pub fn name_for_button(button: Button) -> Option<&'static str> {
    KEY_NAMES
        .iter()
        .find(|(_, b)| *b == button)
        .map(|(n, _)| *n)
}

/// One piece of a key's face: the text, and where it sits relative to
/// the line the label is written on. The same vocabulary the display
/// uses for an expression — see [`crate::engine::script::Shift`] — so a
/// key and the thing it writes are drawn the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelPart {
    pub text: &'static str,
    pub shift: Shift,
}

impl LabelPart {
    /// Full size, on the line.
    pub const fn on_line(text: &'static str) -> Self {
        Self {
            text,
            shift: Shift::OnLine,
        }
    }

    /// An exponent: the `2` of `x²`, the `-1` of `sin⁻¹`.
    const fn raised(text: &'static str) -> Self {
        Self {
            text,
            shift: Shift::Up,
        }
    }

    /// A base: the `2` of `log₂`.
    const fn lowered(text: &'static str) -> Self {
        Self {
            text,
            shift: Shift::Down,
        }
    }

    /// A root degree, which is raised *and* written into the opening
    /// of the radical that follows it.
    const fn degree(text: &'static str) -> Self {
        Self {
            text,
            shift: Shift::Degree,
        }
    }
}

/// The pieces the key's face is drawn from.
///
/// The keypad draws a script the way the display does — the same
/// characters, smaller and moved off the line — rather than asking the
/// font for a raised or lowered glyph. Only a handful of characters
/// have one at all: `x²` and `log₂` come out of Unicode's superscript
/// and subscript blocks, but `xʸ`, `yˣ` and `ʸ√x` have to borrow
/// modifier letters and `logᵧ` a Greek subscript gamma, and those are
/// drawn by whichever face on the system happens to carry them. The
/// result was a keypad whose exponents sat at four different heights
/// and whose `logᵧ` wore a letter from another alphabet. Drawn from
/// pieces, every script on the keypad is the key's own font at 60% and
/// one step off the line, and they line up because they are placed
/// rather than found.
///
/// [`label_for`] stays the one-line spelling of the same face, for
/// measuring and for saying what a key reads as in one string.
pub fn label_parts(button: Button, ctx: LabelContext) -> Vec<LabelPart> {
    match button {
        // The inverse trigonometry, whose `-1` is an exponent.
        Button::Asin => vec![LabelPart::on_line("sin"), LabelPart::raised("-1")],
        Button::Acos => vec![LabelPart::on_line("cos"), LabelPart::raised("-1")],
        Button::Atan => vec![LabelPart::on_line("tan"), LabelPart::raised("-1")],
        Button::Asinh => vec![LabelPart::on_line("sinh"), LabelPart::raised("-1")],
        Button::Acosh => vec![LabelPart::on_line("cosh"), LabelPart::raised("-1")],
        Button::Atanh => vec![LabelPart::on_line("tanh"), LabelPart::raised("-1")],

        // Logarithms wear their base under them.
        Button::Log2 => vec![LabelPart::on_line("log"), LabelPart::lowered("2")],
        Button::LogY => vec![LabelPart::on_line("log"), LabelPart::lowered("y")],

        // Radicals wear their degree in the opening of the sign, and
        // the `x` they take under it — a bare radical says which
        // operation the key is but not what it does to what is on
        // screen, and the square and cube roots are the same key as
        // `ʸ√x` with the degree filled in.
        Button::Sqrt => vec![LabelPart::degree("2"), LabelPart::on_line("√x")],
        Button::Cbrt => vec![LabelPart::degree("3"), LabelPart::on_line("√x")],
        Button::YRootX => vec![LabelPart::degree("y"), LabelPart::on_line("√x")],

        // Powers wear their exponent above them.
        Button::Square => vec![LabelPart::on_line("x"), LabelPart::raised("2")],
        Button::Cube => vec![LabelPart::on_line("x"), LabelPart::raised("3")],
        Button::XPowY => vec![LabelPart::on_line("x"), LabelPart::raised("y")],
        Button::YPowX => vec![LabelPart::on_line("y"), LabelPart::raised("x")],
        Button::TenPowX => vec![LabelPart::on_line("10"), LabelPart::raised("x")],
        Button::TwoPowX => vec![LabelPart::on_line("2"), LabelPart::raised("x")],
        Button::EPowX => vec![LabelPart::on_line("𝑒"), LabelPart::raised("x")],

        // Everything else is one piece, written on the line.
        other => vec![LabelPart::on_line(label_for(other, ctx))],
    }
}

/// Glyph the key wears, as one line of text: what the key reads as,
/// and what its size on the button is measured from. The face the
/// keypad actually draws is [`label_parts`], which says where each
/// piece of it goes.
///
/// Static for almost every key; the three that track live state read
/// it from `ctx`.
pub fn label_for(button: Button, ctx: LabelContext) -> &'static str {
    match button {
        Button::Num(d) => match d {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            // Unreachable: the enum only ever carries a single digit.
            _ => "?",
        },
        Button::Decimal => ctx.decimal,
        // Spelt out rather than `±` so it reads the same way `1⁄x`
        // does — the minus is U+2212, which shares the `+` sign's
        // height and weight where a hyphen would sit higher and
        // lighter, and the slash is the fraction slash U+2044 the
        // percent sign is built from, which leans further over than
        // the ASCII one and so reads as a division rather than as a
        // separator.
        Button::Negate => "+⁄−",
        Button::Backspace => "⌫",

        Button::Clear => ctx.clear,
        Button::Equals => "=",
        Button::Second => "2nd",
        Button::LeftParen => "(",
        Button::RightParen => ")",
        Button::CursorLeft => "←",
        Button::CursorRight => "→",
        Button::CursorHome => "|←",
        Button::CursorEnd => "→|",

        Button::Add => "+",
        Button::Sub => "−",
        Button::Mul => "×",
        Button::Div => "÷",
        Button::Pow => "^",
        Button::Mod => "mod",
        Button::Percent => "%",
        Button::Factorial => "x!",
        Button::EE => "EE",

        // The radical with its degree in front of it, rather than the
        // single `√` and `∛` glyphs — the second of which a good many
        // fonts do not carry — and what the display draws for the
        // same keys.
        Button::Sqrt => "²√x",
        Button::Cbrt => "³√x",
        Button::Sin => "sin",
        Button::Cos => "cos",
        Button::Tan => "tan",
        Button::Asin => "sin⁻¹",
        Button::Acos => "cos⁻¹",
        Button::Atan => "tan⁻¹",
        Button::Sinh => "sinh",
        Button::Cosh => "cosh",
        Button::Tanh => "tanh",
        Button::Asinh => "sinh⁻¹",
        Button::Acosh => "cosh⁻¹",
        Button::Atanh => "tanh⁻¹",
        Button::Ln => "ln",
        Button::Log10 => "log",
        Button::Log2 => "log₂",

        Button::YRootX => "ʸ√x",
        Button::LogY => "logᵧ",

        Button::Square => "x²",
        Button::Cube => "x³",
        Button::XPowY => "xʸ",
        Button::YPowX => "yˣ",
        Button::TenPowX => "10ˣ",
        Button::TwoPowX => "2ˣ",
        Button::EPowX => "𝑒ˣ",

        Button::Pi => "π",
        Button::Euler => "𝑒",

        // Fraction slash, as on `+⁄−`: see [`Button::Negate`].
        Button::Reciprocal => "1⁄x",
        Button::Rand => "Rand",

        Button::MemClear => "MC",
        Button::MemRecall => "MR",
        Button::MemAdd => "M+",
        Button::MemSub => "M-",

        Button::ToggleHistoryPanel => "History",
        Button::ToggleSettingsPanel => "Settings",
        Button::ToggleMode => "Mode",
        Button::ToggleAngleMode => ctx.angle,
    }
}

/// Glyph for the configured decimal separator.
pub fn decimal_label(config: &Config) -> &'static str {
    match config.decimal_separator.to_char() {
        ',' => ",",
        _ => ".",
    }
}

/// Which of the four configured tables is on screen for `mode` with
/// the `2nd` toggle in the given state.
pub fn layout_kind(mode: Mode, second_mode: bool) -> LayoutKind {
    match (mode, second_mode) {
        (Mode::Basic, false) => LayoutKind::Basic,
        (Mode::Basic, true) => LayoutKind::BasicSecond,
        (Mode::Scientific, false) => LayoutKind::Scientific,
        (Mode::Scientific, true) => LayoutKind::ScientificSecond,
    }
}

/// Resolve a whole configured table into the actions to draw. Blank
/// and unknown cells come back as `None`, which the keypad renders as
/// an empty slot.
pub fn resolve_grid(config: &Config, kind: LayoutKind) -> Vec<Vec<Option<Button>>> {
    config
        .keypad
        .cells(kind)
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|name| {
                    let resolved = button_for_name(&name);
                    if resolved.is_none() && !name.is_empty() {
                        warn_unknown_key(&name);
                    }
                    resolved
                })
                .collect()
        })
        .collect()
}

/// Translate a keystroke through the user's own `2nd` mapping: find
/// where the key sits in the on-screen table and read the same cell
/// out of its second-function table. Returns `None` when the key is
/// not on the keypad at all (e.g. `Home`), leaving the caller to fall
/// back to the built-in inverse pairs.
pub fn second_of(config: &Config, mode: Mode, button: Button) -> Option<Button> {
    let name = name_for_button(button)?;
    let kind = layout_kind(mode, false);
    let (row, column) = config.keypad.position_of(kind, name)?;
    let second = config.keypad.name_at(kind.second(), row, column)?;
    button_for_name(&second)
}

/// Complain once per unknown name. A typo in the layout would
/// otherwise print on every frame.
fn warn_unknown_key(name: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut set) = seen.lock() {
        if set.insert(name.to_string()) {
            eprintln!("cosmic-calc: unknown keypad key {name:?} — leaving that cell empty");
        }
    }
}
