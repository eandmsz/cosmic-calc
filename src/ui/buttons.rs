//! Keypad button model. The [`Button`] enum enumerates every key on
//! the scientific keypad; the per-button behaviour rules from the
//! Phase-4 spec live in [`apply_button`]. All state lives in `Engine`
//! plus a small [`UiState`] struct, so the dispatcher is a pure
//! function the tests can exercise without spinning up a cosmic
//! event loop.
//!
//! Second-toggle handling lives here too – a [`Button::Second`] press
//! flips `UiState::second_mode`. A keypad cell already knows what it
//! does in either state (the keypad draws whichever of the user's two
//! tables is armed), so those presses arrive pre-resolved through
//! [`apply_resolved_button`]. A keystroke carries no such context, so
//! [`apply_button`] puts it through the same second-function mapping
//! the keypad is showing: the user's own `2nd` table where the key
//! appears on it, the built-in inverse pairs (`Sin` → `Asin`,
//! `Sqrt` → `Square`, …) otherwise.

use crate::config::{Config, Mode};
use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};
use crate::engine::script;
use crate::engine::{AngleMode, Decimal, Engine};
use crate::rng::rand_value;

/// Maximum number of significant digits the user is allowed to type
/// into a single numeric literal. The engine can *evaluate* wider
/// values (coming from π, from a long computation, etc.); this cap
/// only constrains the input-buffer entry path.
pub const MAX_ENTRY_DIGITS: usize = 15;

/// Every key the keypad and the keyboard can emit. The ordering is
/// purely alphabetical-by-category for readability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    // --- digits & basic input ---
    Num(u8),
    Decimal,
    Negate,
    Backspace,

    // --- control ---
    Clear,
    Equals,
    Second,
    LeftParen,
    RightParen,
    CursorLeft,
    CursorRight,
    /// Jump the cursor to the start / end of the expression. Bound to
    /// the Home and End keys; there is no keypad cell for either.
    CursorHome,
    CursorEnd,

    // --- operators ---
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    Percent,
    Factorial,
    /// `×10^` – exposed to the user as EE.
    EE,

    // --- unary functions ---
    Sqrt,
    Cbrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Ln,
    Log10,
    Log2,

    // --- binary functions ---
    /// `y√x` – opens `root(value, degree)` with the cursor in
    /// whichever of the two slots comes next. See [`open_root`].
    YRootX,
    /// `log_y(x)` – opens `log(base, value)` with the cursor in
    /// whichever of the two slots comes next. See [`open_log_base`].
    LogY,

    // --- power shortcuts ---
    Square,
    Cube,
    /// `x^y` – same behaviour as [`Button::Pow`] but bound to a
    /// dedicated keypad key for discoverability.
    XPowY,
    /// `y^x` – [`Button::XPowY`] with its operands the other way
    /// round: the operand already entered becomes the *exponent* and
    /// the base is what the user keys next.
    YPowX,
    TenPowX,
    TwoPowX,
    EPowX,

    // --- constants ---
    Pi,
    Euler,

    // --- special ---
    Reciprocal,
    Rand,

    // --- memory ---
    MemClear,
    MemRecall,
    MemAdd,
    MemSub,

    // --- panels & modes (side-effect-only buttons) ---
    ToggleHistoryPanel,
    ToggleSettingsPanel,
    ToggleMode,
    ToggleAngleMode,
}

/// Label shown on the Clear key: starts as `AC` on launch, flips to
/// `C` after a single character has been typed, flips back to `AC`
/// once the buffer is empty again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClearMode {
    #[default]
    AllClear,
    Single,
}

/// UI-layer state the dispatcher needs alongside [`Engine`]. Kept
/// outside of `AppModel` proper so the dispatch logic can be unit
/// tested without libcosmic.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Second toggle – when true, the next button that has an inverse
    /// will route to that inverse and flip this back to false.
    pub second_mode: bool,
    /// Whether the Clear button currently shows `AC` or `C`.
    pub clear_mode: ClearMode,
    /// Whether the left history side panel is visible.
    pub history_panel_open: bool,
    /// Whether the right settings side panel is visible.
    pub settings_panel_open: bool,
    /// Whether the copy/paste context menu is visible on the display.
    pub context_menu_open: bool,
    /// Formatted result of the most recent evaluation, kept around so
    /// `handle_post_eval` can still distinguish "continue with Ans" from
    /// a blank state. Not shown directly on the main window any more –
    /// the buffer itself is rewritten to hold the result after `=`.
    pub last_result: String,
    /// Numeric value behind `last_result`, used by `M+` / `M-` when the
    /// user stores the result of a just-evaluated expression without
    /// clearing the buffer first.
    pub last_result_value: Option<Decimal>,
    /// Previous evaluated expression (and any error message from the
    /// most recent `=`). Shown as a caption above the main display; the
    /// main display itself renders the buffer, which now holds the
    /// computed result after a successful evaluation.
    pub last_expression: String,
    /// Original items behind `last_expression`, captured at evaluation
    /// time so the user can click the caption to recall the expression
    /// back into the buffer without re-running the parse pipeline.
    /// Empty when the caption holds something other than an evaluated
    /// expression (e.g. the "Random number" hint or a recalled error).
    pub last_expression_items: Vec<InputItem>,
    /// Set to true immediately after `Equals` fires. The next input
    /// resets or extends the buffer depending on whether it starts a
    /// new expression or continues with the previous result.
    pub just_evaluated: bool,
    /// Captured "last operator + last operand" from the most recent
    /// successful evaluation. Used so a second `=` press treats the
    /// current result as the new first operand and replays the saved
    /// operator+operand: e.g. after `2+3=5`, pressing `=` again gives
    /// `5+3=8`. Cleared whenever the user starts a fresh expression.
    pub last_repeat: Option<(BinOp, Vec<InputItem>)>,
    /// Set when evaluation fails. The display layer renders this on
    /// the main line in place of the buffer so the user notices the
    /// problem; the buffer itself is preserved beneath. Any subsequent
    /// non-cosmetic button press clears the message and lets the user
    /// resume editing.
    pub error_message: Option<String>,
    /// Set by a `)` that left a script slot the cursor was written
    /// directly into — an exponent, the base slot `yˣ` opens. Nothing
    /// in the buffer records that step, because the press wrote
    /// nothing into the slot: the cursor is already past its last
    /// item and the only difference is that the user has said they
    /// are done with it. The display reads this to take the
    /// placeholder brackets down and the dispatcher to tell a
    /// finished power from one still being typed; the next press that
    /// touches the buffer clears it, and the brackets come back.
    pub script_slot_closed: bool,
    /// Half-open item-index range covering the most recent `Rand`
    /// insertion, when one is still live (i.e. no buffer-mutating press
    /// has happened since). The display layer dims only this range so
    /// any preceding expression keeps its normal active colour, and the
    /// Rand handler reuses the range to replace just the random on a
    /// repeated press instead of wiping the whole buffer.
    pub random_range: Option<(usize, usize)>,
}

/// Outcome of a button press. The caller (AppModel) uses this to
/// decide whether to push an entry to `History`, open a panel, or
/// perform other side effects that the dispatcher itself cannot do.
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonEffect {
    /// Nothing further for the caller to do – the engine and ui state
    /// already reflect the change.
    None,
    /// `=` fired and evaluation succeeded; the caller should record
    /// `(expression, result)` in history.
    Evaluated {
        expression: String,
        result: String,
        items: Vec<InputItem>,
    },
    /// The user asked to toggle the history side panel.
    ToggleHistoryPanel,
    /// The user asked to toggle the settings side panel.
    ToggleSettingsPanel,
    /// The user asked to switch between Basic and Scientific modes.
    ToggleMode,
    /// The user asked to switch between DEG and RAD.
    ToggleAngleMode,
    /// The user asked to recall the memory register. Caller inserts
    /// the formatted value (or does nothing if memory is empty).
    MemoryRecall,
    /// The user asked for `M+`, `M-`, or `MS` – caller evaluates the
    /// buffer and stores the result.
    MemoryStore(MemoryOp),
    /// The user asked to clear memory.
    MemoryClear,
}

/// Which memory mutation was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOp {
    Add,
    Sub,
}

/// Top-level dispatcher for a press that still needs the `2nd`
/// modifier applied — i.e. a keystroke, which carries no knowledge of
/// which table the keypad is currently drawing.
pub fn apply_button(
    engine: &mut Engine,
    state: &mut UiState,
    config: &Config,
    button: Button,
) -> ButtonEffect {
    let resolved = resolve_for_keyboard(config, state, button);
    apply_resolved_button(engine, state, config, resolved)
}

/// Dispatcher for a press whose meaning is already settled: the keypad
/// cell the user hit was drawn from the table that is armed, so it
/// emits the action it shows. Running such a press through
/// [`resolve_for_keyboard`] a second time would map it straight back
/// to the function the key is *not* displaying.
pub fn apply_resolved_button(
    engine: &mut Engine,
    state: &mut UiState,
    config: &Config,
    button: Button,
) -> ButtonEffect {
    // Scientific-only buttons are a no-op in Basic mode — unless the
    // user put one on the Basic keypad themselves, in which case
    // refusing it would make their own layout look broken.
    if config.mode == Mode::Basic && !available_in_basic(button) && !placed_in_basic(config, button)
    {
        return ButtonEffect::None;
    }

    // Any subsequent press dismisses a sticky error message – except
    // `Second`, which only flips the keypad's inverse mode and shouldn't
    // count as the user moving past the error. `Equals` is allowed to
    // clear here too: if it errors again, the eval path will re-arm the
    // message before this function returns.
    if !matches!(button, Button::Second) {
        state.error_message = None;
    }

    // Drop the "Random number" caption (and the inactive colouring it
    // implies) when the buffer is about to change to anything other
    // than a fresh random — Rand re-press is the only path that keeps
    // it armed.
    dismiss_rand_caption_if_buffer_changes(state, button);

    // A closed script slot stays closed only until the next press that
    // touches the line: anything typed there joins the slot again, so
    // the brackets that say which slot it is come back with it. What
    // *this* press makes of the slot it closed is decided below, from
    // the flag as it stood when the press arrived.
    let slot_was_closed = state.script_slot_closed;
    if !matches!(button, Button::Second | Button::RightParen) {
        state.script_slot_closed = false;
    }

    // Handle Equals's "continue with Ans" pre-step uniformly.
    handle_post_eval(engine, state, button);

    // A decimal separator with no fractional digits behind it is a
    // point the user started and moved on from: `5.` then `+` is `5+`.
    // Every press that ends the numeric run drops it — the digits and
    // the point itself do not, since those are the run continuing, and
    // backspace does not, since deleting the point is exactly what it
    // is being asked to do.
    if is_buffer_mutating_button(button)
        && !matches!(button, Button::Num(_) | Button::Decimal | Button::Backspace)
    {
        drop_empty_decimal(engine);
    }

    // An operator pressed while the cursor is parked in the base slot
    // `yˣ` opened belongs after the whole power. See
    // [`leave_pending_power_base`].
    let left_power_base = operates_on_a_whole_value(button) && leave_pending_power_base(engine);

    // A power the cursor has just come out of is a finished value,
    // and the keys that would otherwise reach back into its exponent
    // say so with brackets. See [`seals_the_power`].
    if seals_the_power(button, left_power_base, slot_was_closed) {
        seal_power_chain(engine);
    }

    // A digit keyed after a closed slot starts a new factor rather
    // than joining the exponent: `2`, `xʸ`, `3`, `)`, `5` is `2³×5`,
    // where the digit used to run onto the end of the exponent and
    // give `2` to the thirty-fifth. The `×` is the calculator's, so
    // it is drawn dim like every other one it fills in.
    if slot_was_closed && matches!(button, Button::Num(_) | Button::Decimal) {
        ensure_auto_mul_before_new_run(engine);
    }

    match button {
        Button::Num(d) if d <= 9 => {
            insert_digit(engine, d);
            state.clear_mode = ClearMode::Single;
            ButtonEffect::None
        }
        Button::Num(_) => ButtonEffect::None,
        Button::Decimal => {
            insert_decimal(engine);
            state.clear_mode = ClearMode::Single;
            ButtonEffect::None
        }
        Button::Negate => {
            toggle_negate(engine);
            ButtonEffect::None
        }
        Button::Backspace => {
            backspace_with_paren_match(engine);
            if engine.input.is_empty() {
                state.clear_mode = ClearMode::AllClear;
                state.last_result.clear();
                state.last_expression.clear();
                state.last_expression_items.clear();
            }
            ButtonEffect::None
        }
        Button::Clear => {
            match state.clear_mode {
                // `C` takes back the last thing entered rather than the
                // whole line: the operand the cursor sits after, with
                // the expression it was part of left standing. An
                // operator there is not an operand to take back, so the
                // press only arms the `AC` — it is one backspace away,
                // and clearing the line over it would be the opposite
                // of what the key is for. Either way the key flips to
                // `AC`, so the line is always two presses from empty.
                ClearMode::Single => {
                    clear_last_operand(engine);
                    state.clear_mode = ClearMode::AllClear;
                    if engine.input.is_empty() {
                        state.last_expression.clear();
                        state.last_expression_items.clear();
                    }
                }
                ClearMode::AllClear => {
                    engine.clear();
                    state.last_result.clear();
                    state.last_result_value = None;
                    state.last_expression.clear();
                    state.last_expression_items.clear();
                }
            }
            ButtonEffect::None
        }
        Button::Equals => {
            // Repeat-equals: when the previous press was also Equals
            // (just_evaluated is still true), splice the captured
            // operator + operand onto the current result so the user
            // can iterate (e.g. 2+3= 5, =, 8, =, 11).
            if state.just_evaluated {
                if let Some((op, operand)) = state.last_repeat.clone() {
                    engine.input.insert(InputItem::BinOp(op));
                    for it in operand {
                        engine.input.insert(it);
                    }
                }
            }
            evaluate_now(engine, state)
        }
        Button::Second => {
            state.second_mode = !state.second_mode;
            ButtonEffect::None
        }
        Button::LeftParen => {
            // Insert a matched pair and park the cursor between them so
            // the user can keep typing the body. This matches the
            // behaviour of most code editors and every modern
            // calculator app, and removes the footgun of dropped
            // closers that users reported in Phase-5 testing.
            insert_with_auto_mul(engine, InputItem::LeftParen);
            engine.input.insert(InputItem::RightParen);
            engine.input.move_cursor(crate::engine::CursorMove::Left);
            state.clear_mode = ClearMode::Single;
            ButtonEffect::None
        }
        Button::RightParen => {
            // An operator with nothing after it has no operand to be
            // closed over, and the `)` is the user saying they are
            // done: it goes, rather than being left dangling inside
            // the brackets. `(5+)` is `(5)`.
            drop_pending_operator(engine);
            match right_paren_target(engine) {
                Some(target) => {
                    engine.input.set_cursor(target);
                    // Where the press closed a slot rather than a
                    // bracket the cursor cannot say so on its own; see
                    // [`UiState::script_slot_closed`].
                    state.script_slot_closed = closes_a_script_slot(engine, target);
                    state.clear_mode = ClearMode::Single;
                }
                None => {
                    state.script_slot_closed = false;
                    if close_around_last_operand(engine) {
                        state.clear_mode = ClearMode::Single;
                    }
                }
            }
            ButtonEffect::None
        }
        Button::CursorLeft => {
            engine.input.move_cursor(crate::engine::CursorMove::Left);
            ButtonEffect::None
        }
        Button::CursorRight => {
            engine.input.move_cursor(crate::engine::CursorMove::Right);
            ButtonEffect::None
        }
        Button::CursorHome => {
            engine.input.move_cursor(crate::engine::CursorMove::Home);
            ButtonEffect::None
        }
        Button::CursorEnd => {
            engine.input.move_cursor(crate::engine::CursorMove::End);
            ButtonEffect::None
        }

        Button::Add => {
            replace_or_insert_binop(engine, BinOp::Add);
            ButtonEffect::None
        }
        Button::Sub => {
            replace_or_insert_binop(engine, BinOp::Sub);
            ButtonEffect::None
        }
        Button::Mul => {
            replace_or_insert_binop(engine, BinOp::Mul);
            ButtonEffect::None
        }
        Button::Div => {
            replace_or_insert_binop(engine, BinOp::Div);
            ButtonEffect::None
        }
        Button::Pow | Button::XPowY => {
            press_pow(engine);
            ButtonEffect::None
        }
        Button::Mod => {
            // Same guard as the binary operators: modulo needs a left
            // operand. Without it a press on an empty buffer left a
            // stray operator behind.
            if has_left_operand_at_cursor(engine) {
                engine.input.insert(InputItem::Modulo);
            }
            ButtonEffect::None
        }
        Button::Percent => {
            press_percent(engine);
            ButtonEffect::None
        }
        Button::Factorial => {
            // Empty buffer prepends `0` so the user can type `0!` from
            // scratch; otherwise the press only takes effect when there
            // is a value-producing token to the left of the cursor.
            //
            // The key writes a `!` and nothing else: no brackets go in
            // around what it applies to. In an exponent it stays in the
            // exponent — `2`, `xʸ`, `3`, `!` is `2` raised to `3!` —
            // and after a fixed exponent it reads the way the buffer
            // spells it, `5^2!`.
            if engine.input.is_empty() {
                engine.input.insert(InputItem::Digit('0'));
                engine.input.insert(InputItem::Factorial);
            } else if has_left_operand_at_cursor(engine) {
                engine.input.insert(InputItem::Factorial);
            }
            ButtonEffect::None
        }
        Button::EE => {
            // ×10^  expands to `×10^` literally so the user can type
            // the exponent next. Drops silently when the cursor isn't
            // on a value-producing token – there has to be a mantissa
            // for the EE to multiply against, and an empty buffer is no
            // exception: a default `0` there would be a mantissa that
            // zeroes whatever exponent follows it.
            if has_left_operand_at_cursor(engine) && new_run_depth(engine) < MAX_SCRIPT_DEPTH {
                engine.input.insert_all([
                    InputItem::BinOp(BinOp::Mul),
                    InputItem::Digit('1'),
                    InputItem::Digit('0'),
                    InputItem::BinOp(BinOp::Pow),
                ]);
            }
            ButtonEffect::None
        }

        Button::Sqrt => wrap_or_open_unary(engine, UnaryFunc::Sqrt),
        Button::Cbrt => wrap_or_open_unary(engine, UnaryFunc::Cbrt),
        Button::Sin => wrap_or_open_unary(engine, UnaryFunc::Sin),
        Button::Cos => wrap_or_open_unary(engine, UnaryFunc::Cos),
        Button::Tan => wrap_or_open_unary(engine, UnaryFunc::Tan),
        Button::Asin => wrap_or_open_unary(engine, UnaryFunc::Asin),
        Button::Acos => wrap_or_open_unary(engine, UnaryFunc::Acos),
        Button::Atan => wrap_or_open_unary(engine, UnaryFunc::Atan),
        Button::Sinh => wrap_or_open_unary(engine, UnaryFunc::Sinh),
        Button::Cosh => wrap_or_open_unary(engine, UnaryFunc::Cosh),
        Button::Tanh => wrap_or_open_unary(engine, UnaryFunc::Tanh),
        Button::Asinh => wrap_or_open_unary(engine, UnaryFunc::Asinh),
        Button::Acosh => wrap_or_open_unary(engine, UnaryFunc::Acosh),
        Button::Atanh => wrap_or_open_unary(engine, UnaryFunc::Atanh),
        Button::Ln => wrap_or_open_unary(engine, UnaryFunc::Ln),
        Button::Log10 => wrap_or_open_unary(engine, UnaryFunc::Log10),
        Button::Log2 => wrap_or_open_unary(engine, UnaryFunc::Log2),

        Button::YRootX => open_root(engine),
        Button::LogY => open_log_base(engine),

        Button::Square => raise_to(engine, 2),
        Button::Cube => raise_to(engine, 3),
        Button::YPowX => raise_over(engine),
        Button::TenPowX => open_power(engine, &[InputItem::Digit('1'), InputItem::Digit('0')]),
        Button::TwoPowX => open_power(engine, &[InputItem::Digit('2')]),
        Button::EPowX => open_power(engine, &[InputItem::Constant(ConstKind::E)]),

        Button::Pi => {
            insert_with_auto_mul(engine, InputItem::Constant(ConstKind::Pi));
            ButtonEffect::None
        }
        Button::Euler => {
            insert_with_auto_mul(engine, InputItem::Constant(ConstKind::E));
            ButtonEffect::None
        }

        Button::Reciprocal => wrap_reciprocal(engine),
        Button::Rand => {
            // A live previous random is signalled by `random_range`
            // (still set because no buffer-mutating press has run
            // since). Delete just that slice so a repeat-Rand replaces
            // only the random while any preceding expression survives.
            if state.last_expression == "Random number" {
                if let Some((s, e)) = state.random_range.take() {
                    engine.input.delete_range(s, e);
                }
            }
            let v = rand_value(
                config.rand_min_incl,
                config.rand_max_excl,
                config.rand_decimals,
            );
            let start = engine.input.cursor();
            insert_number_string(engine, &format_rand(v, config.rand_decimals));
            let end = engine.input.cursor();
            state.random_range = Some((start, end));
            state.clear_mode = ClearMode::Single;
            state.last_expression = "Random number".to_string();
            // The caption holds a literal hint, not an expression to
            // recall – clicking it should be a no-op.
            state.last_expression_items.clear();
            ButtonEffect::None
        }

        Button::MemClear => ButtonEffect::MemoryClear,
        Button::MemRecall => ButtonEffect::MemoryRecall,
        Button::MemAdd => ButtonEffect::MemoryStore(MemoryOp::Add),
        Button::MemSub => ButtonEffect::MemoryStore(MemoryOp::Sub),

        Button::ToggleHistoryPanel => ButtonEffect::ToggleHistoryPanel,
        Button::ToggleSettingsPanel => ButtonEffect::ToggleSettingsPanel,
        Button::ToggleMode => ButtonEffect::ToggleMode,
        Button::ToggleAngleMode => ButtonEffect::ToggleAngleMode,
    }
}

// ---------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------

/// Insert an explicit `BinOp(Mul)` if the cursor sits right after a
/// value-ender. Used by buttons whose first inserted item starts a
/// fresh value-run but is itself a digit/decimal that the per-item
/// `insert_with_auto_mul` helper would mistake for a continuation
/// (e.g. 10ˣ inserts a digit `1` after an existing digit run).
fn ensure_auto_mul_before_new_run(engine: &mut Engine) {
    let cur = engine.input.cursor();
    if cur == 0 {
        return;
    }
    let left = &engine.input.items()[cur - 1];
    let left_ends_value = matches!(
        left,
        InputItem::Constant(_)
            | InputItem::RightParen
            | InputItem::Factorial
            | InputItem::FixedPow(_)
            | InputItem::Digit(_)
            | InputItem::DecimalPoint
    );
    if left_ends_value {
        engine.input.insert(InputItem::AutoMul);
    }
}

/// Insert `item` at the cursor, prepending an explicit `BinOp(Mul)`
/// when the cursor sits right after a value-ender and the new item
/// begins a new value. Mirrors the auto-multiplication rule the
/// tokenizer applies at evaluation time, but materialises it as a
/// real buffer item so the user can backspace it, history records it,
/// and the display shows it like any other operator.
///
/// Constants (π, 𝑒) abutting a digit/decimal run are a deliberate
/// exception: per spec, `5π` reads as a single composite operand and
/// must NOT carry a synthetic `×`. Constants following anything else
/// (another constant, `)`, `!`) still produce the auto-mul marker.
fn insert_with_auto_mul(engine: &mut Engine, item: InputItem) {
    let cur = engine.input.cursor();
    if cur > 0 {
        let left = &engine.input.items()[cur - 1];
        let left_ends_value = matches!(
            left,
            InputItem::Constant(_)
                | InputItem::RightParen
                | InputItem::Factorial
                | InputItem::FixedPow(_)
                | InputItem::Digit(_)
                | InputItem::DecimalPoint
        );
        let same_run = matches!(left, InputItem::Digit(_) | InputItem::DecimalPoint)
            && matches!(item, InputItem::Digit(_) | InputItem::DecimalPoint);
        let constant_attaches_to_digits = matches!(item, InputItem::Constant(_))
            && matches!(left, InputItem::Digit(_) | InputItem::DecimalPoint);
        let item_begins_value = matches!(
            item,
            InputItem::Digit(_)
                | InputItem::DecimalPoint
                | InputItem::LeftParen
                | InputItem::UnaryFunc(_)
                | InputItem::BinaryFunc(_)
                | InputItem::LogN(_)
                | InputItem::Constant(_)
        );
        if left_ends_value && !same_run && !constant_attaches_to_digits && item_begins_value {
            engine.input.insert(InputItem::AutoMul);
        }
    }
    engine.input.insert(item);
}

/// Every button reachable in Basic mode. Anything NOT in this list is
/// scientific-only and silently no-ops while `mode == Basic`.
fn available_in_basic(b: Button) -> bool {
    matches!(
        b,
        Button::Num(_)
            | Button::Decimal
            | Button::Negate
            | Button::Backspace
            | Button::Clear
            | Button::Equals
            | Button::LeftParen
            | Button::RightParen
            | Button::CursorLeft
            | Button::CursorRight
            | Button::CursorHome
            | Button::CursorEnd
            | Button::Add
            | Button::Sub
            | Button::Mul
            | Button::Div
            | Button::Percent
            | Button::Reciprocal
            | Button::Square
            | Button::Sqrt
            | Button::Pi
            | Button::Rand
            | Button::MemClear
            | Button::MemRecall
            | Button::MemAdd
            | Button::MemSub
            | Button::ToggleHistoryPanel
            | Button::ToggleSettingsPanel
            | Button::ToggleMode
            | Button::ToggleAngleMode
    )
}

/// True when `button` sits somewhere in the user's Basic keypad.
fn placed_in_basic(config: &Config, button: Button) -> bool {
    crate::ui::keymap::name_for_button(button)
        .map(|name| config.keypad.basic_contains(name))
        .unwrap_or(false)
}

/// Translate `button` through the Second modifier. The user's own
/// second-function table wins — that is what the keypad is showing —
/// and the built-in inverse pairs cover keys their layout doesn't
/// mention. Returns the original button when Second is off OR the
/// button has no second function.
///
/// Public because the app layer needs the same answer to decide which
/// keypad cell to flash when the keystroke lands.
pub fn resolve_for_keyboard(config: &Config, state: &UiState, button: Button) -> Button {
    if !state.second_mode || matches!(button, Button::Second) {
        return button;
    }
    if let Some(mapped) = crate::ui::keymap::second_of(config, config.mode, button) {
        return mapped;
    }
    builtin_second(button)
}

/// The built-in inverse of a key, used where the configured layout has
/// nothing to say.
fn builtin_second(button: Button) -> Button {
    match button {
        Button::Sin => Button::Asin,
        Button::Cos => Button::Acos,
        Button::Tan => Button::Atan,
        Button::Asin => Button::Sin,
        Button::Acos => Button::Cos,
        Button::Atan => Button::Tan,
        Button::Sinh => Button::Asinh,
        Button::Cosh => Button::Acosh,
        Button::Tanh => Button::Atanh,
        Button::Asinh => Button::Sinh,
        Button::Acosh => Button::Cosh,
        Button::Atanh => Button::Tanh,
        Button::Sqrt => Button::Square,
        Button::Square => Button::Sqrt,
        Button::Cbrt => Button::Cube,
        Button::Cube => Button::Cbrt,
        Button::Log10 => Button::TenPowX,
        Button::TenPowX => Button::Log10,
        Button::Log2 => Button::TwoPowX,
        Button::TwoPowX => Button::Log2,
        Button::Ln => Button::EPowX,
        Button::EPowX => Button::Ln,
        // `y^x` is the inverse of `log_y(x)` in the order the two
        // keys read their operands: with `3` typed, `y^x` then `2`
        // gives 2³ = 8, and with `8` typed, `log_y` then `2` gives
        // log₂(8) = 3. `x^y` takes the same two the other way round,
        // so it is not the one to route to here even though it exists.
        Button::LogY => Button::YPowX,
        Button::Pow => Button::YRootX,
        Button::XPowY => Button::YRootX,
        Button::YRootX => Button::Pow,
        // The two power keys are each other's operand swap, which is
        // the only sense in which either inverts the other.
        Button::YPowX => Button::XPowY,
        other => other,
    }
}

/// Drop the "Random number" caption — and the all-segments-inactive
/// styling it triggers in the display — when the user is about to
/// modify the buffer. Rand re-press is intentionally the only path
/// that preserves the caption (the Rand handler then overwrites the
/// buffer with a fresh random). Buttons that don't touch the buffer
/// (cursor moves, mode toggles, panel toggles, MC/M+/M-) leave the
/// caption alone.
fn dismiss_rand_caption_if_buffer_changes(state: &mut UiState, button: Button) {
    if state.last_expression != "Random number" {
        return;
    }
    if matches!(button, Button::Rand) {
        return;
    }
    if !is_buffer_mutating_button(button) {
        return;
    }
    state.last_expression.clear();
    state.last_expression_items.clear();
    state.random_range = None;
}

/// Whitelist of presses that don't alter the input buffer. Keeping the
/// non-mutating set explicit means new buttons default to "mutating"
/// (the safe choice for the Random-caption rule and any future state
/// that needs to know when the buffer is about to change).
fn is_buffer_mutating_button(button: Button) -> bool {
    !matches!(
        button,
        Button::Second
            | Button::CursorLeft
            | Button::CursorRight
            | Button::CursorHome
            | Button::CursorEnd
            | Button::MemClear
            | Button::MemAdd
            | Button::MemSub
            | Button::ToggleHistoryPanel
            | Button::ToggleSettingsPanel
            | Button::ToggleMode
            | Button::ToggleAngleMode
    )
}

/// Handle the "press-after-=" transition. A fresh digit or function
/// starts a new expression; anything else continues with the previous
/// result, which `evaluate_now` has already written into the buffer.
fn handle_post_eval(engine: &mut Engine, state: &mut UiState, button: Button) {
    if !state.just_evaluated {
        return;
    }
    if matches!(button, Button::Equals) {
        // Equals after Equals is the "repeat" gesture – the Equals
        // handler reads `just_evaluated` itself to decide whether to
        // splice the saved operator+operand. Leave the flag set so it
        // can do that work; `evaluate_now` will reset it after the
        // splice fires.
        return;
    }
    // Unary-wrap functions (sin, cos, ln, √, …) deliberately stay OUT of
    // this list: per the spec, pressing one right after `=` should wrap
    // the result, so we want `evaluate_now`'s buffer (= the result) to
    // survive into `wrap_or_open_unary`.
    let starts_new = matches!(
        button,
        Button::Num(_)
            | Button::Decimal
            | Button::Pi
            | Button::Euler
            | Button::Rand
            | Button::TenPowX
            | Button::TwoPowX
            | Button::EPowX
            | Button::LeftParen
            | Button::Clear
    );
    if starts_new {
        engine.clear();
    }
    // The caption is the expression that produced what is on the
    // display. The moment the user adds to that result — an operator,
    // a digit, a wrapper, a backspace — the display stops being that
    // expression's answer, so the caption stops being true and comes
    // down. Presses that leave the buffer alone (a cursor move, a
    // panel toggle) leave it up.
    if is_buffer_mutating_button(button) {
        state.last_expression.clear();
        state.last_expression_items.clear();
    }
    // Everything else keeps the result that `evaluate_now` already
    // inserted into the buffer, so operators, postfixes, wrappers, and
    // memory ops naturally extend the current value.
    state.just_evaluated = false;
}

/// Count how many trailing items form the current numeric literal at
/// the cursor – used to enforce [`MAX_ENTRY_DIGITS`].
fn current_number_digit_count(engine: &Engine) -> usize {
    let items = engine.input.items();
    let cur = engine.input.cursor();
    let mut count = 0usize;
    let mut i = cur;
    while i > 0 {
        match items[i - 1] {
            InputItem::Digit(_) => {
                count += 1;
                i -= 1;
            }
            InputItem::DecimalPoint => {
                i -= 1;
            }
            _ => break,
        }
    }
    count
}

/// Insert a digit, respecting the 15-digit entry cap. When the
/// preceding run already has 15 digits we silently drop the key.
///
/// Strips a standalone leading `0`: if the current numeric run is exactly
/// `0` (no decimal point yet) the next non-zero digit replaces it, and a
/// pressed `0` no-ops so the buffer never shows `00`.
fn insert_digit(engine: &mut Engine, d: u8) {
    debug_assert!(d <= 9);
    if current_number_digit_count(engine) >= MAX_ENTRY_DIGITS {
        return;
    }
    if is_standalone_leading_zero(engine) {
        if d == 0 {
            return;
        }
        engine.input.delete_before();
    }
    let c = (b'0' + d) as char;
    insert_with_auto_mul(engine, InputItem::Digit(c));
}

/// Backspace handler that also removes the matching `RightParen` when
/// the deleted item opened a paren group (`(`, `sin(`, `log(`, …).
/// Without this, deleting `(` would leave a dangling `)` behind that
/// users have to clean up manually.
///
/// When the cursor sits immediately after a `RightParen` we don't
/// delete it – instead the cursor is moved one step left, "into" the
/// bracket pair. The display layer's cursor-inside-paren rule then
/// dims the closer automatically, giving the user a visible cue that
/// they're now editing inside the group.
///
/// Before any of that it undoes a jump: where a press moved the cursor
/// into a slot — the base `yˣ` opens, the base or degree of a
/// two-argument call — backspace takes that press back and puts the
/// cursor where the press found it. Deleting whatever happened to be
/// on the left there took the expression apart in an order it was
/// never built in, and in the `yˣ` case there was nothing on the left
/// at all, so the line could not be taken apart.
fn backspace_with_paren_match(engine: &mut Engine) {
    // A caret whose base slot the cursor is parked in — what `yˣ`
    // leaves behind — comes off first. There is nothing to the left of
    // the cursor to delete there, and the press that moved it in front
    // of the operand is the thing to take back, so backspace retraces
    // that move: the caret goes and the cursor lands back at the end
    // of the operand, exactly where it was before the press.
    if let Some((caret, resume)) = pending_base_caret(engine) {
        engine.input.delete_range(caret, caret + 1);
        engine.input.set_cursor(resume);
        return;
    }
    // The same for a two-argument call the cursor has reached the
    // start of a slot in: there is nothing of that slot to the left to
    // delete, and what the press takes back is the one that put the
    // cursor there.
    if let Some(step) = call_slot_backspace(engine) {
        match step {
            CallStep::Nothing => {}
            CallStep::Into(at) => engine.input.set_cursor(at),
            CallStep::TakeBackCall {
                opener,
                comma,
                closer,
            } => {
                let resume = opener;
                for idx in [closer, comma, opener] {
                    engine.input.delete_range(idx, idx + 1);
                }
                engine.input.set_cursor(resume);
            }
        }
        return;
    }
    // Past a two-argument call, the press steps into the slot drawn
    // outside its brackets — the degree in front of a radical, the
    // base under a `log`. That slot is the one filled last, so it is
    // the one the unwind starts from. Stepping inside the closer, as
    // the rule below does, lands in the bracketed argument instead:
    // right for a root, whose degree *is* at the closer, and wrong for
    // a `log`, whose base is drawn at the front.
    if let Some(at) = outer_slot_end_before(engine) {
        engine.input.set_cursor(at);
        return;
    }
    let cur = engine.input.cursor();
    if cur == 0 {
        return;
    }
    if matches!(engine.input.items()[cur - 1], InputItem::RightParen) {
        engine.input.move_cursor(crate::engine::CursorMove::Left);
        return;
    }
    let opener_idx = cur - 1;
    let opens_paren = matches!(
        engine.input.items()[opener_idx],
        InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_)
    );
    if opens_paren {
        // A call whose first argument is empty takes its comma with it.
        // That is the `log_y` base slot: the cursor sits between the
        // `log(` and the comma, so deleting the opener there would
        // otherwise leave the `,8` of `log(,8)` stranded. A call with
        // something in both arguments keeps its comma, since removing
        // it would run the two together into one number.
        let comma_idx = script::argument_separator(engine.input.items(), opener_idx)
            .filter(|comma| *comma == opener_idx + 1);
        let close_idx = script::closing_paren(engine.input.items(), opener_idx);
        // Highest index first, so the ones below it stay valid.
        for idx in [close_idx, comma_idx].iter().flatten().copied() {
            engine.input.items_mut().remove(idx);
        }
    }
    engine.input.delete_before();
}

/// The end of the slot a two-argument call draws outside its
/// brackets, when the cursor sits immediately past that call: the
/// comma for a `log_y` base, the closer for a root degree. `None`
/// when what the cursor is past is anything else.
fn outer_slot_end_before(engine: &Engine) -> Option<usize> {
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    if cursor == 0 || !matches!(items.get(cursor - 1), Some(InputItem::RightParen)) {
        return None;
    }
    let closer = cursor - 1;
    let opener = opener_for_closer(items, closer)?;
    let InputItem::BinaryFunc(kind) = items[opener] else {
        return None;
    };
    let comma = script::argument_separator(items, opener)?;
    Some(match kind {
        // The base is the first argument, so the slot ends at the
        // comma; the degree is the second, and ends at the closer.
        BinaryFunc::LogBase => comma,
        BinaryFunc::Root => closer,
    })
}

/// The caret of a power whose base is still to be typed, when the
/// cursor is sitting in that empty base slot: its index, and where the
/// cursor belongs once it is gone — the end of the operand the caret
/// raised, which is where the `yˣ` press found it.
///
/// Without this, backspace had nothing to its left to work on and the
/// expression could not be taken apart at all: `2`, `yˣ`, `3` then
/// backspace deleted the `3` and then stuck, with `^2` on screen and
/// the cursor in front of it.
fn pending_base_caret(engine: &Engine) -> Option<(usize, usize)> {
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    if !matches!(items.get(cursor), Some(InputItem::BinOp(BinOp::Pow))) {
        return None;
    }
    if cursor > 0 && items[cursor - 1].ends_operand() {
        // The caret has a base, so the cursor is between two operands
        // rather than in a slot: an ordinary backspace.
        return None;
    }
    let span = script::exponent_span(items, cursor).unwrap_or(items.len());
    // One item comes out in front of it, so the exponent ends a step
    // earlier than it does now.
    Some((cursor, span - 1))
}

/// What backspace does when the cursor has reached the start of one of
/// a two-argument call's slots.
enum CallStep {
    /// Take the whole call back — its opener, its comma and its
    /// closer. Only reached with both slots already empty, so there
    /// is nothing of the user's inside it to leave standing.
    TakeBackCall {
        opener: usize,
        comma: usize,
        closer: usize,
    },
    /// Step back into the other slot, at the end of what it holds.
    Into(usize),
    /// Nothing to take back here, and nothing safe to delete: the
    /// cursor is at the start of a slot that holds something, which
    /// only a cursor move can reach, and deleting the comma in front
    /// of it would run the two arguments together. The press is
    /// dropped.
    Nothing,
}

/// How backspace unwinds a `logᵧ` / `ʸ√x` call, when the cursor is at
/// the start of one of its two slots and so has nothing of that slot
/// to its left to delete. `None` when the cursor is somewhere else,
/// and then backspace deletes a character as it does anywhere.
///
/// The walk is the fill in reverse, and the fill has one order — the
/// bracketed argument first, then the slot drawn outside the brackets
/// (see [`open_root`]) — so the unwind has one order too:
///
///   * From past the call, into the outer slot. That is the ordinary
///     step-inside-the-closer rule, not this function's business.
///   * Through whatever the outer slot holds, a character per press.
///   * Then, with that slot empty, back into the bracketed argument,
///     at its end: `³√(8)` unwinds through the `3`, in past the
///     radicand's closer, and through the `8`. Deleting the comma
///     under the cursor instead would have turned `⁴√(16)` into
///     `√(164)` and `log₂(8)` into `log(28)` — a different function,
///     silently.
///   * Only with both slots empty does the call itself come off,
///     leaving the cursor where it stood.
///
/// A slot with something typed in it is never taken back from its own
/// start: the two arguments would run together into one number.
fn call_slot_backspace(engine: &Engine) -> Option<CallStep> {
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    // Innermost first: a call nested in another answers for itself.
    for opener in (0..cursor).rev() {
        if !matches!(items[opener], InputItem::BinaryFunc(_)) {
            continue;
        }
        let (Some(comma), Some(closer)) = (
            script::argument_separator(items, opener),
            script::closing_paren(items, opener),
        ) else {
            continue;
        };
        if cursor > closer {
            continue;
        }
        let take_back = CallStep::TakeBackCall {
            opener,
            comma,
            closer,
        };
        let in_first = cursor <= comma;
        let first_empty = comma == opener + 1;
        let second_empty = closer == comma + 1;
        let this_empty = if in_first { first_empty } else { second_empty };
        // Anywhere but the start of the slot there is a character of
        // it to delete, and backspace deletes it.
        let starts_here = cursor == if in_first { opener + 1 } else { comma + 1 };
        if !starts_here {
            return None;
        }
        return Some(if !this_empty {
            CallStep::Nothing
        } else if first_empty && second_empty {
            take_back
        } else {
            // The end of the slot filled before this one: the closer
            // when the cursor is in the first argument, the comma when
            // it is in the second. Either way that is the end of the
            // *other* slot, which under the one fill order is always
            // the one still holding something.
            CallStep::Into(if in_first { closer } else { comma })
        });
    }
    None
}

/// `)` with nothing open to close: the brackets go round the operand
/// just entered, which is what the key is being reached for — `5+2`
/// then `)` reads `5+(2)`. An unmatched opener earlier in the
/// expression is closed instead, and with no operand to wrap (an
/// empty display, a trailing operator) the press still does nothing.
/// Returns whether anything was written.
///
/// What the brackets go round is the whole power, not the exponent
/// that ends it: `2^3` then `)` is `(2^3)`, the number on screen, and
/// not the `2⁽³⁾` that bracketing the last operand alone would draw —
/// same value, but brackets round a part of it nobody asked to
/// separate. That is the same span `x²` puts in brackets when it runs
/// out of levels, and the same one `xʸ` and `yˣ` build.
fn close_around_last_operand(engine: &mut Engine) -> bool {
    let cursor = engine.input.cursor();
    if depth_between(engine.input.items(), 0, cursor) > 0 {
        engine.input.insert(InputItem::RightParen);
        return true;
    }
    let Some((start, end)) = power_chain_range(engine, cursor) else {
        return false;
    };
    engine.input.insert_at(start, InputItem::LeftParen);
    engine.input.insert_at(end + 1, InputItem::RightParen);
    true
}

/// True when the cursor sits immediately after a single `0` digit that
/// is not part of a longer numeric run (no preceding digit/decimal, no
/// following digit/decimal). Used to decide whether to replace it with
/// the next typed digit.
fn is_standalone_leading_zero(engine: &Engine) -> bool {
    let cur = engine.input.cursor();
    if cur == 0 {
        return false;
    }
    let items = engine.input.items();
    if !matches!(items[cur - 1], InputItem::Digit('0')) {
        return false;
    }
    let preceded_by_numeric = cur >= 2
        && matches!(
            items[cur - 2],
            InputItem::Digit(_) | InputItem::DecimalPoint
        );
    if preceded_by_numeric {
        return false;
    }
    let followed_by_numeric = items
        .get(cur)
        .map(|it| matches!(it, InputItem::Digit(_) | InputItem::DecimalPoint))
        .unwrap_or(false);
    !followed_by_numeric
}

/// Insert a decimal point if the current numeric run does not already
/// contain one. The engine tokeniser would treat `3..5` as an error,
/// so we silently drop a second `.` rather than let it through. When
/// the cursor isn't sitting on a digit run (empty buffer, or right
/// after an operator/paren) we prepend `0` so the press produces
/// `0.` instead of a bare `.` that would parse oddly.
fn insert_decimal(engine: &mut Engine) {
    let items = engine.input.items();
    let cur = engine.input.cursor();
    // Scan the current run backward until we hit a non-digit.
    let mut i = cur;
    while i > 0 {
        match items[i - 1] {
            InputItem::Digit(_) => i -= 1,
            InputItem::DecimalPoint => return,
            _ => break,
        }
    }
    if i == cur {
        // No digits to the left of the cursor → start with an explicit
        // zero so the buffer never holds a bare leading `.`. Goes
        // through the auto-mul helper so a value-ender on the left
        // (e.g. `5.`, `(2+3).`) properly gets a `×` between them.
        insert_with_auto_mul(engine, InputItem::Digit('0'));
    }
    engine.input.insert(InputItem::DecimalPoint);
}

/// Toggle the sign of the current operand by wrapping or unwrapping
/// it as `(-X)`. The outer parens keep the negation bound to its
/// operand under chained operators – `2×5±` becomes `2×(-5)`, not
/// `2×-5` (which works arithmetically but reads as ambiguous).
///
/// On an empty buffer the press is a no-op – without an operand to
/// flip there's nothing meaningful to do, and a stray `-` would be
/// confusing.
fn toggle_negate(engine: &mut Engine) {
    if engine.input.is_empty() {
        return;
    }
    // Without an operand to flip (e.g. cursor sits right after a
    // dangling operator) the press is a no-op rather than producing
    // a stray binary minus.
    let Some((start, end)) = engine.input.last_operand_range() else {
        return;
    };
    let items = engine.input.items();
    let already_wrapped = end >= start + 3
        && matches!(items[start], InputItem::LeftParen)
        && matches!(items[start + 1], InputItem::BinOp(BinOp::Sub))
        && matches!(items[end - 1], InputItem::RightParen);
    if already_wrapped {
        let cur = engine.input.cursor();
        {
            let v = engine.input.items_mut();
            v.remove(end - 1);
            v.remove(start + 1);
            v.remove(start);
        }
        engine.input.set_cursor(cur.saturating_sub(3));
    } else {
        engine.input.insert_at(start, InputItem::LeftParen);
        engine
            .input
            .insert_at(start + 1, InputItem::BinOp(BinOp::Sub));
        engine.input.insert_at(end + 2, InputItem::RightParen);
    }
}

/// Insert a binary operator. Four cases:
/// 1. Cursor sits immediately after another binop with no operand
///    between → replace that trailing operator with the new one (so
///    typing `5+` then `-` yields `5-`), then decide again from what
///    is left.
/// 2. `−` where a value may begin → the sign of the operand about to
///    be typed, written on its own. See [`value_may_start_here`].
/// 3. Buffer empty → prepend `0` so pressing `+` first gives `0+`.
/// 4. Cursor sits where there is no left operand at all (after a `(`,
///    function opener, etc.) → no action.
///
/// Otherwise the operator is inserted normally.
fn replace_or_insert_binop(engine: &mut Engine, op: BinOp) {
    let cur = engine.input.cursor();
    // A `−` straight after a caret is the sign of the exponent about
    // to be typed, not a change of mind about which operator was
    // wanted: `2`, `xʸ`, `−`, `3` is 2⁻³ and `5`, `EE`, `−`, `3` is
    // 5×10⁻³. Taking the caret back there instead left the base with
    // a subtraction hanging off it and no way to key a negative
    // exponent at all.
    let signs_the_exponent = op == BinOp::Sub
        && cur > 0
        && matches!(engine.input.items()[cur - 1], InputItem::BinOp(BinOp::Pow));
    if !signs_the_exponent
        && cur > 0
        && matches!(engine.input.items()[cur - 1], InputItem::BinOp(_))
    {
        // Replace the trailing binop with the new one. We pop and
        // push so any cursor bookkeeping in the engine stays
        // consistent with the standard insert path.
        engine.input.delete_before();
    }
    if has_left_operand_at_cursor(engine) {
        engine.input.insert(InputItem::BinOp(op));
        return;
    }
    // Nothing to the left to operate on. A `−` is then the sign of
    // what comes next rather than a subtraction — `−`, `6` is `-6`,
    // not the `0-6` a supplied left operand used to make of it, and
    // in the slots of a two-argument call it is the only way to key a
    // negative base, degree or argument at all.
    if op == BinOp::Sub && value_may_start_here(engine) {
        engine.input.insert(InputItem::BinOp(BinOp::Sub));
        return;
    }
    // The other three need a left operand, and an empty display is
    // the one place the calculator supplies it: `+` first gives `0+`.
    if engine.input.is_empty() {
        engine.input.insert(InputItem::Digit('0'));
        engine.input.insert(InputItem::BinOp(op));
    }
}

/// True when a value may begin at the cursor: the start of the
/// expression, or straight after something that opens one — a
/// bracket, a function's bracket, the comma between a call's two
/// arguments, or a caret, whose exponent is a value like any other
/// and so may be keyed with a sign in front of it.
fn value_may_start_here(engine: &Engine) -> bool {
    let cur = engine.input.cursor();
    if cur == 0 {
        return true;
    }
    matches!(
        engine.input.items()[cur - 1],
        InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_)
            | InputItem::Comma
            | InputItem::BinOp(BinOp::Pow)
    )
}

/// Take out a decimal separator with no fractional digits behind it.
/// `5.` then `+` is `5+`: the point was started and left, and a
/// number ending in one reads as unfinished wherever it is shown.
/// Nothing happens when the point has digits after it — the cursor is
/// then inside the number rather than at the end of it.
fn drop_empty_decimal(engine: &mut Engine) {
    let cur = engine.input.cursor();
    if cur == 0 || !matches!(engine.input.items()[cur - 1], InputItem::DecimalPoint) {
        return;
    }
    if matches!(engine.input.items().get(cur), Some(InputItem::Digit(_))) {
        return;
    }
    engine.input.delete_range(cur - 1, cur);
}

/// Take out the operator the cursor sits on when it is still waiting
/// for its right operand. What `)` does before closing anything: the
/// brackets go round a value, and a trailing `+` is not part of one.
fn drop_pending_operator(engine: &mut Engine) {
    let cur = engine.input.cursor();
    if cur == 0 {
        return;
    }
    if !matches!(
        engine.input.items()[cur - 1],
        InputItem::BinOp(_) | InputItem::AutoMul | InputItem::Modulo
    ) {
        return;
    }
    engine.input.delete_range(cur - 1, cur);
}

/// The one `%` key, covering both readings. There is no separate
/// `mod` cell: which one a press means is decided by what ends up
/// after it, exactly as the tokenizer already reads pasted text. `%`
/// with nothing following is a percentage (`3.5%×230` → 8.05, `50%` →
/// 0.5, `200+10%` → 220); `%` with an operand straight after it is
/// modulo (`5%3.2` → 1.8), including a parenthesised negative one
/// (`7%(-3)` → 1), which is what the `±` key produces.
///
/// Like the binary operators, a press with no operand to apply to is
/// dropped rather than leaving a stray token behind.
///
/// And, like them, it applies to the whole value rather than to the
/// exponent the cursor happens to be in: see [`leave_exponent`].
fn press_percent(engine: &mut Engine) {
    if !has_left_operand_at_cursor(engine) {
        return;
    }
    engine.input.insert(InputItem::Percent);
}

/// The caret whose exponent the cursor is written in directly — not
/// inside a bracket group of its own, and not in a call's slot.
/// `None` anywhere else.
///
/// Whether the exponent begins with a bracket the user opened is no
/// business of this question: the cursor is in that caret's exponent
/// either way. What the bracket does change is where a `)` keyed
/// there lands, which is [`right_paren_target`]'s to decide.
fn enclosing_exponent(items: &[InputItem], cursor: usize) -> Option<usize> {
    let mut depth = 0usize;
    for j in (0..cursor.min(items.len())).rev() {
        match items[j] {
            InputItem::RightParen => depth += 1,
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => {
                // An opener with nothing of ours closing it: the cursor
                // is inside that group, not out in the exponent.
                depth = depth.checked_sub(1)?;
            }
            InputItem::Comma if depth == 0 => return None,
            InputItem::BinOp(BinOp::Pow) if depth == 0 => {
                let span = script::exponent_span(items, j)?;
                if cursor > span {
                    return None;
                }
                return Some(j);
            }
            _ => {}
        }
    }
    None
}

/// Keys that attach to a value that is already finished — the binary
/// operators, modulo, `%`, `!`, `EE`, the two fixed powers and the
/// opening bracket. They are the presses that have to step out of a
/// power's base slot first: see [`leave_pending_power_base`].
///
/// `x²` and `x³` are among them because they square what is on
/// screen, not the piece of it the cursor happens to be parked in
/// front of. `6`, `yˣ`, `3` reads `3⁶`, and squaring that is
/// `(3⁶)²`; keyed in the base slot where the `3` was typed it used to
/// write `3^2^6`, which raises the `3` to the sixty-fourth.
///
/// `(` is among them for the same reason once the slot holds
/// something: a bracket opened after a finished base multiplies the
/// power, so `5`, `yˣ`, `6`, `(` is `6⁵×()` where it used to be
/// `6×()⁵` — the bracket landing *in* the slot and pushing the base
/// the user had just typed out of it. With the slot still empty there
/// is no base yet and the bracket is how one gets typed, so the press
/// stays where it is.
///
/// The digits are not among them, and neither are the functions:
/// those are how the slot gets filled in the first place.
fn operates_on_a_whole_value(button: Button) -> bool {
    matches!(
        button,
        Button::Add
            | Button::Sub
            | Button::Mul
            | Button::Div
            | Button::Mod
            | Button::Percent
            | Button::Factorial
            | Button::EE
            | Button::Square
            | Button::Cube
            | Button::LeftParen
    )
}

/// Keys that put brackets round a power before they write anything,
/// because what they would otherwise attach to is its exponent
/// rather than the power itself.
///
/// `!` and `%` write themselves and nothing else, so brackets are the
/// only way to say which of the two they apply to. In an exponent the
/// user is still filling they stay in it — `2`, `xʸ`, `3`, `!` is `2`
/// raised to `3!`, which is what the key is for — but a power the
/// cursor has just been moved out of is a finished value, and the
/// postfix belongs to the whole of it: `5`, `yˣ`, `9`, `!` is `(9⁵)!`
/// where it used to read back as `9` raised to `5!`.
///
/// The cursor comes out of a power two ways, and both count: a press
/// stepping out of the base slot `yˣ` opened, and a `)` the user
/// typed to close a script slot. After that `)` a caret joins them,
/// since the exponent it would otherwise add a level to is one the
/// user has said they are done with: `2`, `xʸ`, `3`, `)`, `xʸ` is
/// `(2³)^y` where a bare `2^3^y` would raise the `3`.
///
/// `x²` and `x³` are not here — [`raise_to`] already brackets the
/// power it squares, and a second pair round it would be one nobody
/// asked for.
fn seals_the_power(button: Button, left_power_base: bool, slot_was_closed: bool) -> bool {
    match button {
        Button::Factorial | Button::Percent => left_power_base || slot_was_closed,
        Button::Pow | Button::XPowY | Button::YPowX => slot_was_closed,
        _ => false,
    }
}

/// Put brackets round the whole power the cursor sits at the end of,
/// so what is keyed next reads as applying to the power rather than
/// to the exponent it ends in. The cursor ends up past the closer,
/// which is where the caller was going to write anyway.
///
/// Nothing happens where there is no power there: a bare operand
/// needs no brackets, and a pair round one would be a pair nobody
/// asked for.
fn seal_power_chain(engine: &mut Engine) {
    let cursor = engine.input.cursor();
    let Some((start, end)) = power_chain_range(engine, cursor) else {
        return;
    };
    if !bare_power(engine.input.items(), start, end, true) {
        return;
    }
    engine.input.insert_at(start, InputItem::LeftParen);
    engine.input.insert_at(end + 1, InputItem::RightParen);
}

/// Move the cursor out of the base slot a `yˣ` press opened, to the
/// end of the power it is the base of. `true` when it moved, which
/// is what tells the caller the press is now standing past a
/// finished power — see [`seals_the_power`].
///
/// `2`, `yˣ`, `3` reads `3²` with the cursor still between the `3` and
/// the caret, because that is where the base was typed. An operator
/// keyed there landed *inside* the power, between the base and its
/// exponent: `+` gave `3+^2`, which reads as a power with no base at
/// all and takes the `3` the user had just typed out of the sum. The
/// operator is about the whole `3²`, so the cursor goes past it first.
///
/// Only a caret that has a base already: with the slot still empty
/// there is no value for an operator to attach to, and the press is
/// dropped as it always was.
fn leave_pending_power_base(engine: &mut Engine) -> bool {
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    if !matches!(items.get(cursor), Some(InputItem::BinOp(BinOp::Pow))) {
        return false;
    }
    if cursor == 0 || !items[cursor - 1].ends_operand() {
        return false;
    }
    let end = script::exponent_span(items, cursor).unwrap_or(items.len());
    engine.input.set_cursor(end);
    true
}

/// True when the item directly to the left of the cursor closes a
/// value – i.e. there is something a binary operator can attach to.
/// Mirrors the engine tokenizer's `produces_value` predicate but
/// works on `InputItem` directly.
fn has_left_operand_at_cursor(engine: &Engine) -> bool {
    let cur = engine.input.cursor();
    cur > 0 && engine.input.items()[cur - 1].ends_operand()
}

/// How many levels of script the display draws before the text stops
/// being readable: the line itself and two steps off it, so `2^2^2` is
/// as deep as a power gets and `log₂` under a `log₂` under a `log` is
/// as deep as a base gets.
///
/// A key that would write past it puts what is already there in
/// brackets instead — pressing `x²` on `2^2^2` gives `(2^2^2)^2`,
/// which is the same number one level shallower. Where brackets cannot
/// help, the press does nothing rather than draw a script nobody can
/// read.
pub const MAX_SCRIPT_LEVELS: u8 = 3;

/// The deepest step a script may be written at: one less than the
/// number of levels, the first of which is the line itself.
const MAX_SCRIPT_DEPTH: u8 = MAX_SCRIPT_LEVELS - 1;

/// The script depth of the position at `at`: where the display would
/// draw a piece inserted there.
fn depth_at(items: &[InputItem], at: usize) -> u8 {
    let at = at.min(items.len());
    if at == 0 {
        return 0;
    }
    let depths = script::script_depths(items);
    match items[at - 1] {
        // The caret is drawn as nothing at all — what follows it is
        // the exponent, one step further off the line than the caret's
        // own run.
        InputItem::BinOp(BinOp::Pow) => depths[at - 1].saturating_add(1),
        _ => depths[at - 1],
    }
}

/// The depth a *new* value written at the cursor lands on. It is not
/// always [`depth_at`]: a value starting after an operand takes an
/// auto-multiplication in front of it, and an operator ends the
/// exponent it follows, so `2^2^2` then `10ˣ` writes its `10` back on
/// the line the chain started from rather than up in the exponent.
fn new_run_depth(engine: &Engine) -> u8 {
    let at = engine.input.cursor();
    match power_chain_range(engine, at) {
        Some((start, _)) => script::script_depths(engine.input.items())[start],
        None => depth_at(engine.input.items(), at),
    }
}

/// The whole power the operand ending at `at` is the tail of: all of
/// `2^2^2` rather than its last `2`. What goes into brackets when a
/// press has run out of levels, since it is the chain of carets that
/// put the position up there in the first place.
fn power_chain_range(engine: &Engine, at: usize) -> Option<(usize, usize)> {
    let (mut start, end) = engine.input.operand_range_ending_at(at)?;
    let items = engine.input.items();
    while start > 0 && matches!(items[start - 1], InputItem::BinOp(BinOp::Pow)) {
        // The caret raises the operand ending where it starts, so one
        // step left per caret walks the whole chain back to its base.
        match engine.input.operand_range_ending_at(start - 1) {
            Some((base, _)) => start = base,
            None => break,
        }
    }
    Some((start, end))
}

/// Make room for a script written at `at`, which lands one step deeper
/// than the position itself. `true` when there is room — after
/// wrapping the power chain ending there in brackets, if that is what
/// it took. `false` when even the brackets cannot bring it back inside
/// [`MAX_SCRIPT_LEVELS`], and the press has to be dropped.
fn fit_script_at(engine: &mut Engine, at: usize) -> bool {
    if depth_at(engine.input.items(), at) < MAX_SCRIPT_DEPTH {
        return true;
    }
    let Some((start, end)) = power_chain_range(engine, at) else {
        return false;
    };
    // Brackets only help when the chain started shallower than it
    // ended — they take it back to the line its base was written on.
    // A slot that was already that deep (a base under a base) stays
    // that deep whatever is put around it.
    if script::script_depths(engine.input.items())[start] >= MAX_SCRIPT_DEPTH {
        return false;
    }
    engine.input.insert_at(start, InputItem::LeftParen);
    engine.input.insert_at(end + 1, InputItem::RightParen);
    true
}

/// What a two-argument call — `logᵧ`, `ʸ√x` — closes over, and where
/// that leaves the slot it draws outside its brackets.
enum CallOperand {
    /// Wrap `start..end`; the call goes in front of it.
    Operand(usize, usize),
    /// Nothing to wrap: open an empty call and let the user type into
    /// it.
    Empty,
    /// No level left for the base or the degree. The press does
    /// nothing rather than write a slot nobody can read.
    TooDeep,
}

/// Widen an operand range to take in the `−` written in front of it,
/// when that `−` is the operand's sign rather than a subtraction —
/// nothing that ends a value stands before it.
///
/// It is what makes the two calls close over the number the user
/// keyed. `−`, `4`, `logᵧ` used to wrap the `4` alone and leave the
/// sign outside as `-log(,4)`, which negates the logarithm instead of
/// taking one of a negative number; the same press now reads
/// `log(,-4)`, and `−`, `5`, `ʸ√x` reads `root(-5,)`.
fn with_leading_sign(items: &[InputItem], start: usize) -> usize {
    if start == 0 || !matches!(items[start - 1], InputItem::BinOp(BinOp::Sub)) {
        return start;
    }
    if start >= 2 && items[start - 2].ends_operand() {
        // Something to subtract from: a binary minus, and the operand
        // is only what follows it.
        return start;
    }
    start - 1
}

/// Decide which of those a press gets.
///
/// The slot is written one step off the line the call sits on, so the
/// call has to sit somewhere with a level to spare. Where the operand
/// is the tail of a power that has used them all up, the call takes
/// the *whole* power instead of its tail — `2^2^2` then `logᵧ` reads
/// `log₍₎(2^2^2)`, with the base back on the line the power started
/// from. No brackets are added for it: a call brings its own, and they
/// go exactly where a reader would put them.
fn call_operand(engine: &Engine) -> CallOperand {
    let cursor = engine.input.cursor();
    let Some((start, end)) = engine
        .input
        .last_operand_range()
        .map(|(start, end)| (with_leading_sign(engine.input.items(), start), end))
    else {
        // Nothing to close over, so the call lands at the cursor.
        return if depth_at(engine.input.items(), cursor) < MAX_SCRIPT_DEPTH {
            CallOperand::Empty
        } else {
            CallOperand::TooDeep
        };
    };
    let depths = script::script_depths(engine.input.items());
    if depths[start] < MAX_SCRIPT_DEPTH {
        return CallOperand::Operand(start, end);
    }
    match power_chain_range(engine, cursor) {
        Some((chain, chain_end)) if depths[chain] < MAX_SCRIPT_DEPTH => {
            CallOperand::Operand(chain, chain_end)
        }
        _ => CallOperand::TooDeep,
    }
}

/// `xʸ` and the `^` key: raise what is already on screen to whatever
/// is typed next.
///
/// Unlike `+` it will not invent the operand it is missing. An empty
/// display shows a `0` nobody typed, and `0^y` is 0 for every exponent
/// the user could go on to key, so a press there does nothing at all —
/// the same as `%` and `EE`, and for the same reason `yˣ` refuses one.
/// A trailing operator is a change of mind about which operator was
/// wanted, so the caret replaces it, and the base is then whatever was
/// in front of it.
fn press_pow(engine: &mut Engine) {
    if engine.input.is_empty() {
        return;
    }
    let cursor = engine.input.cursor();
    let replacing = cursor > 0 && matches!(engine.input.items()[cursor - 1], InputItem::BinOp(_));
    let base_end = if replacing { cursor - 1 } else { cursor };
    if engine.input.operand_range_ending_at(base_end).is_none() {
        return;
    }
    if !fit_script_at(engine, base_end) {
        return;
    }
    if replacing {
        engine.input.delete_before();
    }
    // A fixed exponent is finished, so the caret raises the whole of
    // it rather than reaching into it: `5²` then `xʸ` is `(5²)^y`
    // where the bare `5^2^y` would have read as `5^(2^y)` and changed
    // what the square key wrote.
    seal_fixed_power(engine);
    engine.input.insert(InputItem::BinOp(BinOp::Pow));
}

/// Delete the operand the cursor sits after — the digits of a number,
/// a constant, a whole bracketed group or function call, along with
/// whatever postfix `!` / `%` hangs off it. What `C` takes back.
///
/// A cursor that is not sitting after an operand — one on an operator,
/// an open bracket, an empty buffer — has nothing to take back, and
/// the press leaves the expression alone.
fn clear_last_operand(engine: &mut Engine) {
    // A fixed exponent is part of the operand it hangs off, so `C`
    // takes the two together: `5²` goes back to nothing, where
    // backspace gives the `5` back on its own. The key is for taking
    // back the value, and `5²` is one value.
    //
    // It used to take back the press instead — the `x²` — which was
    // the same thing backspace does and left the base standing. That
    // was there because the press wrote a caret and a digit, and
    // deleting the digit alone left `5^` with an empty slot; the
    // exponent is one item now, so there is nothing to protect.
    if let Some((start, end)) = engine.input.last_operand_range() {
        engine.input.delete_range(start, end);
    }
}

/// Wrap the last operand in a unary function or, when there is no
/// operand, insert an open-function item plus its matching closer and
/// park the cursor between them so the user can type the argument.
fn wrap_or_open_unary(engine: &mut Engine, f: UnaryFunc) -> ButtonEffect {
    match engine.input.last_operand_range() {
        Some((start, end)) => {
            engine.input.insert_at(start, InputItem::UnaryFunc(f));
            // The insert above bumped `end` by 1; close paren goes at
            // (end + 1).
            engine.input.insert_at(end + 1, InputItem::RightParen);
            // Cursor is now past the closing paren; leave it there.
        }
        None => {
            insert_with_auto_mul(engine, InputItem::UnaryFunc(f));
            engine.input.insert(InputItem::RightParen);
            engine.input.move_cursor(crate::engine::CursorMove::Left);
        }
    }
    ButtonEffect::None
}

/// Open a `root(value, degree)` call — `ʸ√(x)` on the display — with
/// its comma and its closing bracket already in place, the way `√` and
/// `∛` put theirs in, and the cursor in whichever slot the user fills
/// first.
///
/// An operand already typed is the radicand: `16`, `ʸ√x` gives
/// `⁽⁾√(16)` with the cursor in the degree, so the next digit is the
/// degree. Without the comma the first argument stayed open and the
/// degree ran onto the end of it — `16`, `ʸ√x`, `4` read back as
/// `root(164)` — and with no comma key on the keypad the second
/// argument could not be reached at all.
///
/// From an empty operand the order is the same one, which is the
/// whole of why it is worth saying: the radicand first, then the
/// degree. `ʸ√x` parks the cursor in the radicand and `)` moves out
/// to the degree, so `ʸ√x`, `8`, `)`, `3` reads `³√(8)` — the same
/// two slots filled in the same order as when the `8` was already on
/// screen. Reading the empty case the other way round, degree first,
/// meant the key did one thing on a fresh display and another on a
/// typed one; the comma goes in up front either way, since the slot
/// has to exist before the display can show it empty and before `)`
/// has anywhere to move to.
fn open_root(engine: &mut Engine) -> ButtonEffect {
    let call = InputItem::BinaryFunc(BinaryFunc::Root);
    match call_operand(engine) {
        // Out of levels for another degree: see [`call_operand`].
        CallOperand::TooDeep => return ButtonEffect::None,
        CallOperand::Operand(start, end) => {
            engine.input.insert_at(start, call);
            // The insert above bumped `end` by 1; the comma closes the
            // radicand, the bracket closes the call, and the cursor
            // sits between them ready for the degree.
            engine.input.insert_at(end + 1, InputItem::Comma);
            engine.input.insert_at(end + 2, InputItem::RightParen);
            engine.input.set_cursor(end + 2);
        }
        CallOperand::Empty => {
            insert_with_auto_mul(engine, call);
            engine.input.insert(InputItem::Comma);
            engine.input.insert(InputItem::RightParen);
            // Back over the closer and the comma, into the radicand:
            // the first slot, and the one the press asks for first
            // whether or not there was anything to close over.
            engine.input.move_cursor(crate::engine::CursorMove::Left);
            engine.input.move_cursor(crate::engine::CursorMove::Left);
        }
    }
    ButtonEffect::None
}

/// Open a `log_y` call — `log(base, value)` in the buffer, `log₍₎(x)`
/// on the display — with the cursor in whichever slot the user is
/// going to fill first.
///
/// An operand already typed is the *value*: `8`, `logy` gives
/// `log₍₎(8)` with the cursor under the log, so the very next digit is
/// the base. That is the reading the key's own label has — `logᵧ` is
/// the log of what you have, to a base you are about to name.
///
/// From an empty operand the order is the same one: the argument
/// first, then the base. `logy` parks the cursor in the argument and
/// `)` moves out to the base, so `logy`, `8`, `)`, `2` reads
/// `log₂(8)` — the same two slots filled in the same order as when
/// the `8` was already on screen. Reading the empty case the other
/// way round, base first, meant the key did one thing on a fresh
/// display and another on a typed one. The comma is inserted up front
/// either way, so the slot exists (and the display can show it empty)
/// before anything is typed into it.
fn open_log_base(engine: &mut Engine) -> ButtonEffect {
    let call = InputItem::BinaryFunc(BinaryFunc::LogBase);
    match call_operand(engine) {
        // Out of levels for another base: see [`call_operand`].
        CallOperand::TooDeep => return ButtonEffect::None,
        CallOperand::Operand(start, end) => {
            engine.input.insert_at(start, call);
            // The comma goes straight after the opener, leaving the
            // base slot empty in front of the operand the call has
            // just taken as its argument.
            engine.input.insert_at(start + 1, InputItem::Comma);
            engine.input.insert_at(end + 2, InputItem::RightParen);
            engine.input.set_cursor(start + 1);
        }
        CallOperand::Empty => {
            insert_with_auto_mul(engine, call);
            engine.input.insert(InputItem::Comma);
            engine.input.insert(InputItem::RightParen);
            // Back over the closer only: between the comma and it is
            // the argument, which is the slot the press asks for
            // first whether or not there was anything to close over.
            engine.input.move_cursor(crate::engine::CursorMove::Left);
        }
    }
    ButtonEffect::None
}

/// Where `)` puts the cursor, or `None` when the press does nothing.
///
/// [`Button::LeftParen`] always inserts a matched closer, so a bare
/// `)` press never NEEDS to add a closer of its own: it either steps
/// over one that is already there or does nothing at all. That is what
/// keeps a stray `)` — typed past a closer the calculator had already
/// written — off the display.
///
/// The two-argument calls are the exception at both ends, because each
/// of them draws one argument outside its brackets: the `log_y` base
/// under the `log`, the root degree in front of the radical. The press
/// steps from one slot of such a call to the other while the one it is
/// stepping to is still empty, and leaves the call once there is
/// nothing left to fill. That is the whole gesture the two keys exist
/// for — `logy 2 ) 8` reads log₂(8), `yrootx 3 ) 8` reads ³√(8) — and
/// it is why the display draws the empty slot at all.
fn right_paren_target(engine: &Engine) -> Option<usize> {
    // In the slot a call draws outside its brackets, the bracket is
    // already closed on screen. While the bracketed argument is still
    // empty the press moves in to it — that is the order a call opened
    // from an empty display is filled in, the base or the degree first
    // — and once it holds something the press leaves the call.
    if let Some((opener, closer)) = call_with_cursor_in_outer_slot(engine) {
        return Some(empty_bracketed_argument(engine, opener).unwrap_or(closer + 1));
    }
    // In the base slot `yˣ` opened, with a base now in it: the press
    // closes the slot, and the cursor goes past the whole power,
    // which is where anything keyed next belongs.
    if let Some(end) = pending_base_slot_end(engine) {
        return Some(end);
    }
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    match items.get(cursor) {
        // The end of a bracketed argument: into the outer slot when it
        // is still waiting to be typed, past the closer when it is not.
        Some(InputItem::RightParen) => Some(match empty_log_base_before_closer(engine, cursor) {
            Some(slot) => slot,
            // Stepping over a closer that ends a call's outer slot
            // ends the call too: a bracket opened in the degree of a
            // root is a formula written *in* the degree, so closing
            // it finishes the degree. Without this the cursor stayed
            // in the slot and `8 ʸ√x ( 2 ) + 2` read as
            // `root(8,(2)+2)` — the `+2` up in the degree.
            // A base slot filled with a bracket group ends the same
            // way: see [`leave_base_slot_at`].
            None => leave_outer_slot_at(engine, cursor + 1)
                .or_else(|| leave_base_slot_at(engine, cursor + 1))
                .unwrap_or(cursor + 1),
        }),
        // A root's radicand ends at the call's comma — its degree is
        // drawn in front of the sign, so the `)` on screen there is the
        // one the comma stands for.
        Some(InputItem::Comma) => close_root_radicand(engine, cursor),
        // Nothing of the user's is open here. An exponent written
        // straight after the caret is still a slot the display draws
        // brackets round, so the press closes that: the cursor goes to
        // the end of what the caret raises and nothing is written.
        // `𝑒ˣ`, `8`, `)` is `e^8`, where bracketing the value it is
        // the whole of gave `(e^8)`.
        _ => {
            let caret = enclosing_exponent(items, cursor)?;
            // An exponent that begins with a bracket the user opened
            // has no slot brackets of its own to dismiss: theirs are
            // the pair on screen, and a `)` keyed past their closer
            // closes over the last operand the way it would anywhere
            // else rather than landing where it already is.
            if matches!(items.get(caret + 1), Some(InputItem::LeftParen)) {
                return None;
            }
            script::exponent_span(items, caret)
        }
    }
}

/// The end of the base slot a `yˣ` press opened, when the cursor is
/// in it and it now holds something: past the whole power the caret
/// builds. `None` anywhere else, including an empty slot — there is
/// no base yet, so there is nothing to close over.
fn pending_base_slot_end(engine: &Engine) -> Option<usize> {
    leave_base_slot_at(engine, engine.input.cursor())
}

/// The same rule asked about an explicit position rather than about
/// the cursor, so a `)` that has just stepped over a bracket of the
/// user's own carries on out of the power rather than stopping in
/// front of its caret. `5`, `yˣ`, `(`, `2`, `)` leaves the cursor
/// past the whole `(2)⁵`, exactly where `5`, `yˣ`, `2`, `)` leaves
/// it — otherwise the next digit landed between the base and the
/// caret and took the base slot for itself, `(2)×7⁵` for a `7` that
/// belonged after the power.
fn leave_base_slot_at(engine: &Engine, at: usize) -> Option<usize> {
    let items = engine.input.items();
    if !matches!(items.get(at), Some(InputItem::BinOp(BinOp::Pow))) {
        return None;
    }
    if at == 0 || !items[at - 1].ends_operand() {
        return None;
    }
    Some(script::exponent_span(items, at).unwrap_or(items.len()))
}

/// Where `)` goes from a position that is the *end* of a call's outer
/// slot — the comma a `log_y` base runs up to, the closer a root
/// degree runs up to. The same rule
/// [`call_with_cursor_in_outer_slot`] applies to a cursor sitting in
/// the slot, asked about an explicit position instead, so a press that
/// has just stepped over a bracket inside the slot carries on out of
/// the call rather than stopping there.
fn leave_outer_slot_at(engine: &Engine, at: usize) -> Option<usize> {
    let items = engine.input.items();
    for opener in 0..at {
        let InputItem::BinaryFunc(kind) = items[opener] else {
            continue;
        };
        let (Some(comma), Some(closer)) = (
            script::argument_separator(items, opener),
            script::closing_paren(items, opener),
        ) else {
            continue;
        };
        let (from, to) = match kind {
            BinaryFunc::LogBase => (opener + 1, comma),
            BinaryFunc::Root => (comma + 1, closer),
        };
        if at == to && from <= at && depth_between(items, from, at) == 0 {
            return Some(empty_bracketed_argument(engine, opener).unwrap_or(closer + 1));
        }
    }
    None
}

/// Whether a `)` that landed the cursor on `target` left a script
/// slot behind: the cursor is now at the end of an exponent, or past
/// a power whose base slot it was parked in.
///
/// What the press wrote does not come into it, only where it left the
/// cursor. A `)` that closed a bracket pair the user opened *inside*
/// the exponent ends the exponent too when that pair was the whole of
/// it: `𝑒ˣ`, `(`, `2`, `)` leaves the cursor past the last thing the
/// caret raises, exactly as the same keys without the bracket do, and
/// what is keyed next belongs to the power rather than to its
/// exponent.
///
/// See [`UiState::script_slot_closed`] for what reads it.
fn closes_a_script_slot(engine: &Engine, target: usize) -> bool {
    let items = engine.input.items();
    enclosing_exponent(items, target)
        .is_some_and(|caret| script::exponent_span(items, caret).is_some_and(|span| span == target))
}

/// The two-argument call whose outer slot — the `log_y` base, the root
/// degree — holds the cursor, as `(opener, closer)`. Innermost call
/// wins, so one nested in another answers for itself, and a cursor
/// inside a bracket group *within* the slot belongs to that group
/// rather than to the slot.
fn call_with_cursor_in_outer_slot(engine: &Engine) -> Option<(usize, usize)> {
    let items = engine.input.items();
    let cursor = engine.input.cursor();
    let mut found = None;
    for opener in 0..cursor {
        let InputItem::BinaryFunc(kind) = items[opener] else {
            continue;
        };
        let (Some(comma), Some(closer)) = (
            script::argument_separator(items, opener),
            script::closing_paren(items, opener),
        ) else {
            continue;
        };
        let (from, to) = match kind {
            // The base is written under the `log`, in front of the
            // bracket: the slot is the first argument.
            BinaryFunc::LogBase => (opener + 1, comma),
            // The degree is written in front of the radical: the slot
            // is the second.
            BinaryFunc::Root => (comma + 1, closer),
        };
        if (from..=to).contains(&cursor) && depth_between(items, from, cursor) == 0 {
            found = Some((opener, closer));
        }
    }
    found
}

/// Where the still-empty bracketed argument of the call at `opener`
/// starts — the `log_y` value, the root radicand — or `None` when it
/// already holds something. The mirror of
/// [`empty_log_base_before_closer`]: that one finds an empty slot from
/// inside the brackets, this one finds an empty bracket from inside
/// the slot.
fn empty_bracketed_argument(engine: &Engine, opener: usize) -> Option<usize> {
    let items = engine.input.items();
    let InputItem::BinaryFunc(kind) = items[opener] else {
        return None;
    };
    let comma = script::argument_separator(items, opener)?;
    let closer = script::closing_paren(items, opener)?;
    match kind {
        // The value is the second argument, between the comma and the
        // closer.
        BinaryFunc::LogBase => (closer == comma + 1).then_some(comma + 1),
        // The radicand is the first, between the opener and the comma.
        BinaryFunc::Root => (comma == opener + 1).then_some(opener + 1),
    }
}

/// Index the cursor moves to when `)` is pressed in front of the
/// closer at `cursor`, if that closer belongs to a `log_y` call whose
/// base has not been typed yet: the empty base slot. `None` when the
/// closer is anything else, or the base is already filled — then the
/// press steps over the closer as it always has.
fn empty_log_base_before_closer(engine: &Engine, cursor: usize) -> Option<usize> {
    let items = engine.input.items();
    let opener = opener_for_closer(items, cursor)?;
    if !matches!(items[opener], InputItem::BinaryFunc(BinaryFunc::LogBase)) {
        return None;
    }
    let comma = script::argument_separator(items, opener)?;
    (comma == opener + 1).then_some(opener + 1)
}

/// Index the cursor moves to when `)` closes the radicand of a root
/// call — the press lands on the call's comma, which is where the
/// closing bracket is drawn. Into the degree while it is still empty,
/// past the call when it has already been typed. `None` when the comma
/// belongs to something else.
fn close_root_radicand(engine: &Engine, comma: usize) -> Option<usize> {
    let items = engine.input.items();
    let opener = opener_for_comma(items, comma)?;
    if !matches!(items[opener], InputItem::BinaryFunc(BinaryFunc::Root)) {
        return None;
    }
    let closer = script::closing_paren(items, opener)?;
    Some(if closer == comma + 1 {
        comma + 1
    } else {
        closer + 1
    })
}

/// Net bracket depth over `items[from..to]`: zero when the position at
/// `to` sits at the level the run started on, and positive when it is
/// inside a group opened along the way.
fn depth_between(items: &[InputItem], from: usize, to: usize) -> i32 {
    items[from..to]
        .iter()
        .map(|it| match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => 1,
            InputItem::RightParen => -1,
            _ => 0,
        })
        .sum()
}

/// The call whose argument list the comma at `comma_idx` separates.
fn opener_for_comma(items: &[InputItem], comma_idx: usize) -> Option<usize> {
    let mut depth = 1usize;
    for j in (0..comma_idx).rev() {
        match items[j] {
            InputItem::RightParen => depth += 1,
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => {
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

/// The opening item matching the `RightParen` at `closer_idx`.
fn opener_for_closer(items: &[InputItem], closer_idx: usize) -> Option<usize> {
    if !matches!(items.get(closer_idx), Some(InputItem::RightParen)) {
        return None;
    }
    let mut depth = 1usize;
    for j in (0..closer_idx).rev() {
        match items[j] {
            InputItem::RightParen => depth += 1,
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => {
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

/// Wrap the last operand as `(1÷operand)`. The outer parens keep the
/// reciprocal isolated from any surrounding binary operators, so
/// chains like `2+5⁻¹×3` evaluate as `2+(1/5)×3` rather than
/// `2+1/(5×3)`. Pressing the button again on an operand that is
/// already wrapped this way unwraps it (toggle behaviour). Empty
/// buffer is a no-op – there's no operand to reciprocate.
fn wrap_reciprocal(engine: &mut Engine) -> ButtonEffect {
    if engine.input.is_empty() {
        return ButtonEffect::None;
    }
    if try_unwrap_reciprocal(engine) {
        return ButtonEffect::None;
    }
    // Same guard as negate: with no operand to wrap, do nothing
    // instead of inserting an empty `(1÷)` template.
    let Some((start, end)) = engine.input.last_operand_range() else {
        return ButtonEffect::None;
    };
    let prefix = [
        InputItem::LeftParen,
        InputItem::Digit('1'),
        InputItem::BinOp(BinOp::Div),
    ];
    for (i, item) in prefix.iter().enumerate() {
        engine.input.insert_at(start + i, item.clone());
    }
    engine
        .input
        .insert_at(end + prefix.len(), InputItem::RightParen);
    ButtonEffect::None
}

/// If the last operand is wrapped as `(1÷X)`, strip the wrapper.
/// Returns true when the unwrap was applied.
fn try_unwrap_reciprocal(engine: &mut Engine) -> bool {
    let Some((start, end)) = engine.input.last_operand_range() else {
        return false;
    };
    if end < start + 5 {
        return false;
    }
    let items = engine.input.items();
    let opens = matches!(items[start], InputItem::LeftParen)
        && matches!(items[start + 1], InputItem::Digit('1'))
        && matches!(items[start + 2], InputItem::BinOp(BinOp::Div));
    let closes = matches!(items[end - 1], InputItem::RightParen);
    if !(opens && closes) {
        return false;
    }
    let cur = engine.input.cursor();
    {
        let v = engine.input.items_mut();
        v.remove(end - 1);
        v.remove(start + 2);
        v.remove(start + 1);
        v.remove(start);
    }
    engine.input.set_cursor(cur.saturating_sub(4));
    true
}

/// Raise whatever is to the left of the cursor to a fixed power. Used
/// by `x²` and `x³`, which are postfix operations and so need a base.
///
/// What goes in is one item — [`InputItem::FixedPow`] — and that is
/// the whole point of the key. `x²` is a finished operation, not a
/// caret with a `2` parked in the slot after it, so the exponent is
/// not something anything keyed next can reach: a digit after `5²`
/// starts a new factor and takes an auto-multiplication in front of
/// it (`5²×3`), where writing the caret and the digit separately gave
/// `5²³`.
///
/// An empty buffer is the one place the base is supplied rather than
/// required: a press there starts the expression on a `0`, the same
/// default the binary operators start one on. Before that the press
/// wrote a `^2` with nothing under it, which the parser rejected.
///
/// Anywhere else without a base — a trailing operator, an open bracket
/// — the press does nothing at all. A `0` there is a base the user did
/// not type and did not mean: `5+` then `x²` read back as `5+0²`.
fn raise_to(engine: &mut Engine, exponent: u8) -> ButtonEffect {
    if engine.input.is_empty() {
        engine.input.insert(InputItem::Digit('0'));
    } else if !has_left_operand_at_cursor(engine) {
        return ButtonEffect::None;
    }
    let at = engine.input.cursor();
    // A power under the key goes into brackets rather than gaining
    // another level: `2^2` then `x²` is `(2^2)²`, and again is
    // `((2^2)^2)²`. Squaring is one operation on what is on screen —
    // `xʸ` is the key for building a tower — and the brackets say so
    // where a second caret would have said `2^(2^2)`, a different
    // number. What is bracketed is the whole power, since that is
    // what is being squared. A fixed exponent already there is the
    // same case read from the other end: `5²` then `x²` is `(5²)²`,
    // because `5²²` is `5` to the twenty-second.
    let operand = engine.input.operand_range_ending_at(at);
    let chain = power_chain_range(engine, at);
    let wrap = matches!(
        (operand, chain),
        (Some((o, _)), Some((c, _)))
            if c < o || bare_power(engine.input.items(), o, at, false)
    );
    let base = chain.map_or(at, |(start, _)| start);
    let depths = script::script_depths(engine.input.items());
    let base_depth = if wrap {
        depths[base]
    } else {
        depth_at(engine.input.items(), at)
    };
    if base_depth >= MAX_SCRIPT_DEPTH {
        return ButtonEffect::None;
    }
    let head = if wrap {
        let (start, end) = chain.expect("a chain is what made the wrap");
        engine.input.insert_at(start, InputItem::LeftParen);
        engine.input.insert_at(end + 1, InputItem::RightParen);
        Some((start, start + 1))
    } else {
        None
    };
    // The closing bracket belongs to the press as much as the exponent
    // does, so it is part of what one backspace takes back.
    let tail = engine.input.cursor() - usize::from(wrap);
    engine.input.insert(InputItem::FixedPow(exponent));
    engine
        .input
        .mark_atomic(head, (tail, engine.input.cursor()));
    ButtonEffect::None
}

/// True when `items[start..end]` writes a power at its own bracket
/// level — nothing standing between it and whatever is keyed next.
/// `carets` says whether a `^` counts as one of those too, or only the
/// fixed exponent `x²` and `x³` write.
///
/// The two differ over what may follow. A caret's exponent is a slot
/// the user is still filling, so a `!` keyed there belongs in it —
/// `2^3!` is `2` raised to `3!`, which is what `xʸ` and `yˣ` are for.
/// A fixed exponent is finished, so nothing may reach into it, and
/// since the buffer spells it `^2` the only way to say that is with
/// brackets: `5²` then `!` is `(5²)!`, where a bare `5^2!` would have
/// read as `5` raised to `2!`.
fn bare_power(items: &[InputItem], start: usize, end: usize, carets: bool) -> bool {
    let mut depth = 0i32;
    for it in &items[start..end.min(items.len())] {
        match it {
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => depth += 1,
            InputItem::RightParen => depth -= 1,
            InputItem::FixedPow(_) if depth == 0 => return true,
            InputItem::BinOp(BinOp::Pow) if depth == 0 && carets => return true,
            _ => {}
        }
    }
    false
}

/// Put brackets round the operand at the cursor when it carries a
/// fixed exponent that nothing separates from what is about to be
/// keyed. See [`bare_power`]; the cursor ends up past the closer, so
/// the caller writes into the same place it would have anyway.
fn seal_fixed_power(engine: &mut Engine) {
    let cursor = engine.input.cursor();
    let Some((start, end)) = engine.input.operand_range_ending_at(cursor) else {
        return;
    };
    if !bare_power(engine.input.items(), start, end, false) {
        return;
    }
    engine.input.insert_at(start, InputItem::LeftParen);
    engine.input.insert_at(end + 1, InputItem::RightParen);
}

/// `yˣ` — [`raise_to`]'s mirror, and the one thing that keeps the key
/// from being a second `xʸ`: the operand already on screen becomes the
/// *exponent*, and the base is what the user keys next. `2`, `yˣ`, `3`
/// reads back as `3^2`, where `2`, `xʸ`, `3` reads back as `2^3`.
///
/// The `^` therefore goes in *front* of the operand rather than after
/// it, and the cursor parks in front of the `^` so the base lands in
/// the slot it opened. Mid-expression the swap stays local — `5+2`
/// then `yˣ` then `3` is `5+3^2`, the same two neighbours `^` would
/// have bound either way round.
///
/// With nothing to raise — an empty buffer, a trailing operator, an
/// open bracket — the press does nothing. `x²` treats an empty buffer
/// as the one place to supply the missing operand, but the operand
/// missing here is the exponent, and a `0` there would read `y^0`,
/// which is 1 whatever base the user goes on to type.
fn raise_over(engine: &mut Engine) -> ButtonEffect {
    // The sign goes up with the number it belongs to: `−`, `2`, `yˣ`,
    // `9` is `9^-2`, where lifting the `2` alone left the `−` on the
    // line as `-9^2` — the negative of nine squared rather than nine
    // to the minus two. See [`with_leading_sign`].
    let Some(start) = engine
        .input
        .last_operand_range()
        .map(|(start, _)| with_leading_sign(engine.input.items(), start))
    else {
        return ButtonEffect::None;
    };
    // This is the one key brackets cannot make room for: the operand
    // does not stay where it is, it goes up a level, and anything
    // inside it goes up with it — brackets included. So a press that
    // would push it past the limit does nothing.
    //
    // What goes up is not just the operand. The caret raises what the
    // parser gives it, which is the operand *and* any power chained
    // onto it — press `yˣ` on the `4` of `4^3^2` and all three levels
    // move up together. So the count is taken on the expression the
    // press would produce rather than on the operand alone.
    let mut probe = engine.input.items().to_vec();
    probe.insert(start, InputItem::BinOp(BinOp::Pow));
    let raised = script::exponent_span(&probe, start).unwrap_or(probe.len());
    let depths = script::script_depths(&probe);
    if depths[start..raised].iter().any(|d| *d > MAX_SCRIPT_DEPTH) {
        return ButtonEffect::None;
    }
    engine.input.insert_at(start, InputItem::BinOp(BinOp::Pow));
    engine.input.set_cursor(start);
    ButtonEffect::None
}

/// `10ˣ`, `2ˣ` and `𝑒ˣ`: a base of their own, and a power left waiting
/// for its exponent.
///
/// They ask nothing of what came before, because the base is in the
/// key — so unlike `x²` and `EE` they act wherever a value can start,
/// which is everywhere: an empty buffer, an open bracket, and after an
/// operator, where `5+` becomes `5+10⁽⁾` with the `+` left as the user
/// typed it.
///
/// The auto-mul goes in first so `5` then `10ˣ` becomes `5×10^` rather
/// than glomming the leading `1` onto the existing digit run. After an
/// operator there is no value to its left to multiply, so none is
/// inserted and the operator stands.
fn open_power(engine: &mut Engine, base: &[InputItem]) -> ButtonEffect {
    // The base is the key's own, so there is no operand to bracket:
    // where the exponent it opens would have nowhere left to go, the
    // press does nothing.
    if new_run_depth(engine) >= MAX_SCRIPT_DEPTH {
        return ButtonEffect::None;
    }
    ensure_auto_mul_before_new_run(engine);
    for item in base {
        engine.input.insert(item.clone());
    }
    engine.input.insert(InputItem::BinOp(BinOp::Pow));
    ButtonEffect::None
}

/// Evaluate the current buffer and emit a `ButtonEffect::Evaluated`
/// so the caller can record history. Empty-buffer presses are a
/// no-op.
///
/// On success the buffer is rewritten with the result so the main
/// display shows the computed value, and the just-typed expression is
/// stashed in `state.last_expression` to be shown as a caption above.
/// On error the buffer is left alone – the user's typed input stays
/// visible so they can correct it – and `last_expression` carries the
/// error text instead.
fn evaluate_now(engine: &mut Engine, state: &mut UiState) -> ButtonEffect {
    if engine.input.is_empty() {
        return ButtonEffect::None;
    }
    let expression = engine.input.display_string();
    let original_items: Vec<InputItem> = engine.input.items().to_vec();
    let repeat = extract_repeat(engine.input.items());
    match engine.evaluate() {
        Ok(out) => {
            state.last_expression = expression.clone();
            state.last_expression_items = original_items.clone();
            state.last_result = out.display.clone();
            state.last_result_value = Some(out.value);
            // Always overwrite — including with `None` for bare-value
            // expressions like `6 =`. Otherwise a stale repeat from an
            // earlier evaluation (e.g. `2*3=`) would still be spliced
            // by the next `=`, even after C/AC and a fresh entry.
            state.last_repeat = repeat;
            engine.clear();
            insert_exact_value(engine, &out.display, out.value);
            state.just_evaluated = true;
            state.clear_mode = ClearMode::Single;
            ButtonEffect::Evaluated {
                expression,
                result: out.display,
                items: original_items,
            }
        }
        Err(e) => {
            let msg = e.as_str().to_string();
            state.last_expression = expression.clone();
            state.last_expression_items = original_items.clone();
            state.last_result = msg.clone();
            state.last_result_value = None;
            state.just_evaluated = false;
            state.error_message = Some(msg.clone());
            ButtonEffect::Evaluated {
                expression,
                result: msg,
                items: original_items,
            }
        }
    }
}

/// Walk the input items right-to-left at depth 0 and find the last
/// top-level binary operator. Returns that operator together with all
/// items that came after it (the right operand of the trailing
/// binary expression). Returns `None` when no top-level binary op
/// exists – e.g. the user typed a single number.
fn extract_repeat(items: &[InputItem]) -> Option<(BinOp, Vec<InputItem>)> {
    let mut depth: i32 = 0;
    let mut op_idx: Option<usize> = None;
    for i in (0..items.len()).rev() {
        match items[i] {
            InputItem::RightParen => depth += 1,
            InputItem::LeftParen
            | InputItem::UnaryFunc(_)
            | InputItem::BinaryFunc(_)
            | InputItem::LogN(_) => depth -= 1,
            InputItem::BinOp(_) | InputItem::AutoMul if depth == 0 => {
                op_idx = Some(i);
                break;
            }
            _ => {}
        }
    }
    let i = op_idx?;
    let op = match items[i] {
        InputItem::BinOp(op) => op,
        InputItem::AutoMul => BinOp::Mul,
        _ => return None,
    };
    let operand: Vec<InputItem> = items[i + 1..].to_vec();
    if operand.is_empty() {
        return None;
    }
    Some((op, operand))
}

/// Insert a formatted numeric string (as produced by `rand_value`, a
/// memory recall, or an `Ans` recall) into the buffer at the cursor.
/// Handles the optional leading `-`, per-digit characters, and a
/// single `.`.
///
/// The value always starts a *new* operand. Without that, the per-item
/// auto-multiplication helper saw a digit landing after a digit,
/// concluded they were the same numeric run, and concatenated: with `5`
/// in the buffer, recalling a memory of 42 produced `542`, and pressing
/// Rand produced `50.123…`.
///
/// Returns the half-open item range the value itself landed in, which
/// is what [`insert_exact_value`] hangs the unrounded value off.
pub fn insert_number_string(engine: &mut Engine, s: &str) -> (usize, usize) {
    ensure_auto_mul_before_new_run(engine);
    // Any `×` the line above inserted belongs to the expression, not
    // to the value, so the range starts after it.
    let start = engine.input.cursor();
    // A leading `-` on an empty buffer is just the sign of the value;
    // anywhere else a bare `-` would read as subtraction, so the value
    // is parenthesised the way the negate key does it.
    if let Some(rest) = s.strip_prefix('-') {
        if !engine.input.is_empty() {
            engine.input.insert(InputItem::LeftParen);
            engine.input.insert(InputItem::BinOp(BinOp::Sub));
            insert_number_body(engine, rest);
            engine.input.insert(InputItem::RightParen);
            return (start, engine.input.cursor());
        }
    }
    insert_number_body(engine, s);
    (start, engine.input.cursor())
}

/// Insert a value the calculator computed: its digits as the display
/// rounds them, plus the annotation that lets evaluation read the
/// unrounded `value` back out of them.
///
/// This is what keeps a result usable as an operand. `1÷3=` shows
/// 0.333333333333333 either way, but multiplying those fifteen digits
/// by three gives 0.999999999999999, while multiplying the eighteen
/// the division was computed to gives the 1 the user is expecting.
/// The exactness lasts only as long as the digits are untouched — edit
/// them and the buffer drops the annotation, because then they really
/// are the number.
pub fn insert_exact_value(engine: &mut Engine, shown: &str, value: Decimal) {
    let (start, end) = insert_number_string(engine, shown);
    engine.input.mark_exact(start, end, value);
}

/// Digit-by-digit insertion shared by [`insert_number_string`] and its
/// parenthesised-negative path.
fn insert_number_body(engine: &mut Engine, s: &str) {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' => engine.input.insert(InputItem::BinOp(BinOp::Sub)),
            '.' => insert_with_auto_mul(engine, InputItem::DecimalPoint),
            d if d.is_ascii_digit() => insert_with_auto_mul(engine, InputItem::Digit(d)),
            'e' | 'E' => {
                // Expand scientific notation `e±NN` into the equivalent
                // `×10^(...)` so the buffer stays purely in primitive
                // items. Negative exponents are wrapped in parens so
                // the engine reads them as a single signed operand.
                engine.input.insert(InputItem::BinOp(BinOp::Mul));
                engine.input.insert(InputItem::Digit('1'));
                engine.input.insert(InputItem::Digit('0'));
                engine.input.insert(InputItem::BinOp(BinOp::Pow));
                let negative = matches!(chars.peek(), Some('-'));
                if negative {
                    chars.next();
                }
                if matches!(chars.peek(), Some('+')) {
                    chars.next();
                }
                if negative {
                    engine.input.insert(InputItem::LeftParen);
                    engine.input.insert(InputItem::BinOp(BinOp::Sub));
                }
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        engine.input.insert(InputItem::Digit(d));
                        chars.next();
                    } else {
                        break;
                    }
                }
                if negative {
                    engine.input.insert(InputItem::RightParen);
                }
            }
            _ => {}
        }
    }
}

/// Render an f64 into the minimal textual form the input buffer
/// accepts (fixed-point). Duplicate of `AppModel::format_rand` but
/// available here so the dispatcher is self-contained.
pub fn format_rand(value: f64, decimals: u8) -> String {
    if decimals == 0 {
        format!("{value:.0}")
    } else {
        format!("{:.*}", decimals as usize, value)
    }
}

/// Utility: take the current angle-mode and flip it.
pub fn toggled_angle_mode(mode: AngleMode) -> AngleMode {
    match mode {
        AngleMode::Deg => AngleMode::Rad,
        AngleMode::Rad => AngleMode::Deg,
    }
}

/// Utility: flip Basic ↔ Scientific.
pub fn toggled_layout(mode: Mode) -> Mode {
    match mode {
        Mode::Basic => Mode::Scientific,
        Mode::Scientific => Mode::Basic,
    }
}
