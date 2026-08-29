use crate::config::{Config, Mode};
use crate::engine::item::{ConstKind, InputItem};
use crate::engine::Engine;
use crate::ui::buttons::*;

/// A calculator on the Scientific keypad. Most of what the dispatcher
/// does is only reachable from there, and a Basic keypad — which is
/// what a first run opens on — drops those presses on purpose. The
/// handful of tests about that rule set the mode themselves.
fn fresh() -> (Engine, UiState, Config) {
    let config = Config {
        mode: Mode::Scientific,
        ..Config::default()
    };
    (Engine::default(), UiState::default(), config)
}

// --- digit entry ----------------------------------------------------

#[test]
fn digit_insertion_advances_cursor() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::Num(7));
    assert_eq!(e.input.display_string(), "37");
    assert_eq!(e.input.cursor(), 2);
    assert_eq!(s.clear_mode, ClearMode::Single);
}

#[test]
fn digit_entry_caps_at_15() {
    let (mut e, mut s, c) = fresh();
    for _ in 0..20 {
        apply_button(&mut e, &mut s, &c, Button::Num(9));
    }
    assert_eq!(e.input.items().len(), MAX_ENTRY_DIGITS);
}

#[test]
fn decimal_is_idempotent_within_a_run() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::Decimal);
    apply_button(&mut e, &mut s, &c, Button::Decimal);
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    assert_eq!(e.input.display_string(), "3.1");
}

// --- operator behaviour --------------------------------------------

#[test]
fn binop_after_trailing_operator_replaces_it() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    // Pressing another binop with no right operand replaces the
    // trailing operator – the user is correcting their mind on
    // which operation they want.
    apply_button(&mut e, &mut s, &c, Button::Sub);
    assert_eq!(e.input.display_string(), "5-");
}

#[test]
fn binop_on_empty_buffer_prepends_zero() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.display_string(), "0+");
}

#[test]
fn binop_after_left_paren_is_ignored() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    // Cursor parks between `(` and the auto-inserted `)`.
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.display_string(), "()");
}

#[test]
fn negate_wraps_operand_in_parens() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    apply_button(&mut e, &mut s, &c, Button::Negate);
    assert_eq!(e.input.display_string(), "(-4)");
    apply_button(&mut e, &mut s, &c, Button::Negate);
    assert_eq!(e.input.display_string(), "4");
}

// --- clear / backspace ---------------------------------------------

#[test]
fn clear_flips_from_single_to_all_clear() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    assert_eq!(s.clear_mode, ClearMode::Single);
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert_eq!(s.clear_mode, ClearMode::AllClear);
    assert!(e.input.is_empty());
}

#[test]
fn c_takes_back_the_last_operand_and_leaves_the_rest() {
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(1),
        Button::Num(2),
        Button::Add,
        Button::Num(3),
        Button::Num(4),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::Clear);
    // Only the operand the user was in the middle of goes; what it was
    // being added to stays on the line.
    assert_eq!(e.input.display_string(), "12+");
    assert_eq!(s.clear_mode, ClearMode::AllClear);
    // And with the key flipped, the next press takes the rest.
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert!(e.input.is_empty());
}

#[test]
fn c_on_an_operator_only_arms_the_all_clear() {
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(1), Button::Num(2), Button::Add] {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::Clear);
    // There is no operand to take back — the operator is one backspace
    // away — so the press is the key flipping to `AC` and nothing else.
    assert_eq!(e.input.display_string(), "12+");
    assert_eq!(s.clear_mode, ClearMode::AllClear);
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert!(e.input.is_empty());
}

#[test]
fn c_takes_a_whole_call_back_as_one_operand() {
    // An operand is not only a digit run: a bracketed group or a
    // function call is one thing to the user, so it is one press to
    // take back.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::Add,
        Button::Num(3),
        Button::Num(0),
        Button::Sin,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2+sin(30)");
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert_eq!(e.input.display_string(), "2+");
}

#[test]
fn c_after_an_equals_takes_the_result_back() {
    // The result is the only operand on the line, so one `C` clears
    // the display — and the caption goes with it, there being no
    // expression left for it to be the history of.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::Add, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5");
    assert_eq!(s.last_expression, "2+3");
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert!(e.input.is_empty());
    assert!(s.last_expression.is_empty());
}

#[test]
fn backspace_clears_flag_when_buffer_empties() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(9));
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert!(e.input.is_empty());
    assert_eq!(s.clear_mode, ClearMode::AllClear);
}

// --- unary wrapping ------------------------------------------------

#[test]
fn sqrt_wraps_trailing_digit() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(9));
    apply_button(&mut e, &mut s, &c, Button::Sqrt);
    assert_eq!(e.input.display_string(), "√(9)");
}

#[test]
fn sqrt_with_no_operand_inserts_matched_pair() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Sqrt);
    assert_eq!(e.input.display_string(), "√()");
    // Cursor parks between `(` and `)` so digits land inside.
    apply_button(&mut e, &mut s, &c, Button::Num(9));
    assert_eq!(e.input.display_string(), "√(9)");
}

#[test]
fn y_root_x_closes_its_bracket() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::YRootX);
    // The same matched pair √ and ∛ insert, rather than an opener left
    // hanging for the user to close by hand — plus the comma, which is
    // what gives the degree a slot to be typed into.
    assert_eq!(e.input.display_string(), "root(,)");
    // The radicand comes first whether or not there was one to close
    // over, so that is where the cursor starts.
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    assert_eq!(e.input.display_string(), "root(8,)");
}

#[test]
fn y_root_x_opens_its_radicand_first_and_its_degree_after() {
    // The order is the one the key has when there is already an
    // operand to close over: the radicand, then the degree. `)` moves
    // out to the degree, the way `logᵧ` moves out to its base. The
    // comma goes in up front either way: without it the press left
    // `root()` behind, `)` stepped clean out of the call, and the
    // degree could not be typed at all.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::YRootX);
    // Between the opener and the comma: the radicand, inside the
    // brackets the display draws after the sign.
    assert_eq!(e.input.cursor(), 1);
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    // And out to the degree, which is still empty.
    assert_eq!(e.input.cursor(), 3);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.input.display_string(), "root(8,3)");
    assert_eq!(e.evaluate().expect("cube root of 8").display, "2");
    // And `)` from the degree leaves the call for good, so what
    // follows is not swallowed by it.
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.display_string(), "root(8,3)+");
}

#[test]
fn closing_a_filled_radicand_leaves_the_root_call() {
    // With the degree already typed there is no empty slot to move
    // into, so `)` does what it does everywhere else: it closes the
    // call and puts the cursor past it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(1),
        Button::Num(6),
        Button::YRootX,
        Button::Num(4),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "root(16,4)");
    apply_button(&mut e, &mut s, &c, Button::CursorHome);
    for _ in 0..3 {
        apply_button(&mut e, &mut s, &c, Button::CursorRight);
    }
    // Cursor after the `6`, which is where the radicand's bracket is
    // drawn.
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    assert_eq!(e.input.cursor(), e.input.items().len());
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.display_string(), "root(16,4)+");
}

#[test]
fn y_root_x_closes_the_operand_it_wrapped_and_waits_for_the_degree() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    apply_button(&mut e, &mut s, &c, Button::YRootX);
    // The operand the user had already typed is the first argument and
    // nothing more, so the comma goes in with the bracket.
    assert_eq!(e.input.display_string(), "root(8,)");
    // Unlike √, the operand is not the whole argument list yet — the
    // degree still has to be typed, so the cursor waits after the
    // comma instead of past the closer.
    assert_eq!(e.input.cursor(), 3);
    // Which is what makes the degree reachable at all: before the
    // comma it ran onto the end of the first argument, giving
    // `root(84)` and no way to say the 4th root of 8.
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.input.display_string(), "root(8,3)");
    assert_eq!(e.evaluate().expect("cube root of 8").display, "2");
}

#[test]
fn squaring_nothing_at_all_squares_a_zero() {
    // `x²` is postfix and needs a base. On an empty buffer there is
    // none to be had, so the press starts the expression the way `×`
    // and `+` do — on a default `0` — rather than writing a `^2` with
    // nothing under it for the parser to reject.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Square);
    assert_eq!(e.input.ascii_expression(), "0^2");
    assert_eq!(e.evaluate().expect("evaluates").display, "0");

    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Cube);
    assert_eq!(e.input.ascii_expression(), "0^3");
}

#[test]
fn squaring_an_operand_still_raises_that_operand() {
    // The default base is only for the empty buffer — a typed operand
    // is squared as it always was.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Square);
    assert_eq!(e.input.ascii_expression(), "5^2");
    assert_eq!(e.evaluate().expect("evaluates").display, "25");
}

#[test]
fn the_keys_that_need_a_base_hold_still_without_one() {
    // An expression under way with nothing to attach to — a trailing
    // operator, an open bracket — is not an empty buffer, and the
    // default base is not for it: a `0` there would be a base the user
    // never typed, turning `5+` into `5+0²`. `EE` is in the same
    // position for a different reason: its `×10^` needs a mantissa to
    // multiply, and after a `+` there is none.
    for lead in [
        vec![Button::Num(5), Button::Add],
        vec![Button::LeftParen],
        vec![Button::Sin],
    ] {
        for key in [Button::Square, Button::Cube, Button::EE] {
            let (mut e, mut s, c) = fresh();
            for b in &lead {
                apply_button(&mut e, &mut s, &c, *b);
            }
            let before = e.input.ascii_expression();
            apply_button(&mut e, &mut s, &c, key);
            assert_eq!(
                e.input.ascii_expression(),
                before,
                "{key:?} moved the buffer on from {before:?}"
            );
        }
    }
}

#[test]
fn a_key_carrying_its_own_base_starts_a_value_anywhere() {
    // `10ˣ`, `2ˣ` and `𝑒ˣ` have their base in the key, so they ask
    // nothing of what came before and act wherever a value can start.
    for (key, from_empty, after_plus) in [
        (Button::TenPowX, "10^", "5+10^"),
        (Button::TwoPowX, "2^", "5+2^"),
        // Euler's number goes out as an ASCII `e` — the clipboard and
        // the tokenizer both read it as the constant with nothing but
        // a `^` behind it.
        (Button::EPowX, "e^", "5+e^"),
    ] {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, key);
        assert_eq!(e.input.ascii_expression(), from_empty);

        // After an operator the base goes in beside it. There is no
        // value to the operator's left to multiply, so no auto-mul is
        // inserted and the `+` stands as the user typed it.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, Button::Add);
        apply_button(&mut e, &mut s, &c, key);
        assert_eq!(e.input.ascii_expression(), after_plus);

        // And after an operand it is the auto-mul that joins them, so
        // the new base does not run onto the end of the old digits.
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, Button::Num(5));
        apply_button(&mut e, &mut s, &c, key);
        assert!(
            e.input.ascii_expression().starts_with("5*"),
            "{key:?} ran onto the operand: {}",
            e.input.ascii_expression()
        );
    }

    // `EE` is the odd one out: it multiplies a mantissa, and a `0`
    // mantissa would zero every exponent that followed, so from empty
    // it stays put.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::EE);
    assert!(e.input.is_empty(), "EE started an expression from nothing");
}

#[test]
fn log_y_opens_its_argument_first_and_its_base_after() {
    // The order is the one the key has when there is already an
    // operand to close over: the argument, then the base. `)` moves
    // out to the base under the log.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::LogY);
    assert_eq!(e.input.display_string(), "log(,)");
    // Between the comma and the closer: the argument, inside the
    // brackets.
    assert_eq!(e.input.cursor(), 2);
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    assert_eq!(e.input.display_string(), "log(,8)");
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    assert_eq!(e.input.cursor(), 1);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.input.display_string(), "log(2,8)");
    // And `)` from the base leaves the call for good, so what follows
    // is not swallowed by it.
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.display_string(), "log(2,8)+");
    assert_eq!(e.evaluate().expect("log base 2 of 8").display, "3");
}

#[test]
fn log_y_takes_the_operand_already_typed_as_its_argument() {
    // With an operand waiting there is nothing to type into the
    // argument, so the press lands straight in the base slot.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    apply_button(&mut e, &mut s, &c, Button::LogY);
    assert_eq!(e.input.display_string(), "log(,8)");
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.input.display_string(), "log(2,8)");
    assert_eq!(e.evaluate().expect("log base 2 of 8").display, "3");
}

#[test]
fn backspacing_out_of_the_base_slot_steps_into_the_argument() {
    // The base slot puts the cursor in front of the call's own comma,
    // so there is nothing of the base to its left to delete. The press
    // steps back into the argument, at its end, and only a call with
    // both slots empty comes off whole — otherwise a change of mind
    // would leave `,8` behind, which is not an expression at all.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(8));
    apply_button(&mut e, &mut s, &c, Button::LogY);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "log(,8)");
    // The end of the argument, inside the brackets.
    assert_eq!(e.input.cursor(), 3);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "log(,)");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "");

    // A call with both arguments filled keeps its comma however the
    // cursor reaches the start of one of them: removing it would run
    // `root(16,4)` together into `√(164)`, a different function
    // altogether. Only a cursor move gets there — the unwind never
    // does — and the press is dropped rather than guessing.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(1),
        Button::Num(6),
        Button::YRootX,
        Button::Num(4),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::CursorHome);
    apply_button(&mut e, &mut s, &c, Button::CursorRight);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "root(16,4)");
    assert_eq!(e.input.cursor(), 1);
}

#[test]
fn backspace_unwinds_a_call_the_way_it_was_filled() {
    // `logy 98 ) 71` reads log₇₁(98). Taking it apart runs the fill
    // backwards: the base first, then back inside the brackets for the
    // argument, and only then the call itself. It never comes off in
    // one press, and the comma never goes on its own — `log(7198)`
    // would be log base ten of a number the user never typed.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LogY,
        Button::Num(9),
        Button::Num(8),
        Button::RightParen,
        Button::Num(7),
        Button::Num(1),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "log(71,98)");
    assert_eq!(e.input.cursor(), e.input.items().len());

    // Back into the base, which is the slot that was filled last, and
    // then through it a digit at a time.
    let unwind = |e: &mut crate::engine::Engine, s: &mut crate::ui::UiState| {
        apply_button(e, s, &c, Button::Backspace);
        e.input.display_string()
    };
    assert_eq!(unwind(&mut e, &mut s), "log(71,98)");
    assert_eq!(e.input.cursor(), 3);
    assert_eq!(unwind(&mut e, &mut s), "log(7,98)");
    assert_eq!(unwind(&mut e, &mut s), "log(,98)");
    // The base is empty, so the next press steps back into the
    // argument, at its end, with the argument intact.
    assert_eq!(unwind(&mut e, &mut s), "log(,98)");
    assert_eq!(e.input.cursor(), 4);
    assert_eq!(unwind(&mut e, &mut s), "log(,9)");
    assert_eq!(unwind(&mut e, &mut s), "log(,)");
    // Nothing left in either slot: the call itself comes off.
    assert_eq!(unwind(&mut e, &mut s), "");
}

#[test]
fn backspace_never_turns_a_root_into_a_square_root() {
    // The degree of `³√(8)` is a whole argument of the call, so it
    // cannot be deleted out from under it: emptying it steps back into
    // the radicand rather than leaving a `√(83)` behind.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(8), Button::YRootX, Button::Num(3)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "root(8,3)");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "root(8,)");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    // Into the radicand, at its end, with the `8` still standing.
    assert_eq!(e.input.display_string(), "root(8,)");
    assert_eq!(e.input.cursor(), 2);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "root(,)");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "");
}

#[test]
fn reciprocal_wraps_last_operand() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    apply_button(&mut e, &mut s, &c, Button::Reciprocal);
    assert_eq!(e.input.display_string(), "(1÷4)");
    // Pressing again unwraps.
    apply_button(&mut e, &mut s, &c, Button::Reciprocal);
    assert_eq!(e.input.display_string(), "4");
}

// --- what the keys put on the display -------------------------------

/// The pieces the display draws for the current buffer, as
/// `(text, script depth)` pairs, with the cursor where the user left
/// it.
fn drawn(e: &Engine) -> Vec<(String, u8)> {
    crate::ui::display::render_expression(
        e.input.items(),
        e.input.cursor(),
        crate::locale::DecimalSeparator::Dot,
        None,
        None,
        crate::engine::script::Notation::Pretty,
    )
    .into_iter()
    .map(|seg| (seg.text, seg.script.depth))
    .collect()
}

/// The script depth of each piece the display draws, which is what
/// the three-level limit is counted in. Rendered without a cursor, so
/// the brackets a slot wears while it is being typed into do not
/// count as pieces of the expression.
fn drawn_depths(engine: &Engine) -> Vec<u8> {
    crate::ui::display::render_expression(
        engine.input.items(),
        crate::ui::display::NO_CURSOR,
        crate::locale::DecimalSeparator::Dot,
        None,
        None,
        crate::engine::script::Notation::Pretty,
    )
    .into_iter()
    .map(|seg| seg.script.depth)
    .collect()
}

#[test]
fn a_whole_call_can_be_typed_into_an_exponent() {
    // What the sized-script rendering buys at the keypad: press the
    // power key, then a function key, and the call goes up as a call.
    // Nothing here depends on the font having a raised `s`, an `i` or
    // a bracket, and none of it drops back to full size.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Sin,
        Button::Num(3),
        Button::Num(0),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^sin(30)");
    assert_eq!(
        drawn(&e),
        vec![
            ("2".to_string(), 0),
            ("sin(".to_string(), 1),
            ("30".to_string(), 1),
            (")".to_string(), 1),
        ]
    );

    // And a fractional exponent, which used to be the case the raised
    // brackets were reached for most often.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(1),
        Button::Decimal,
        Button::Num(5),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    // The cursor is still in the exponent, so it wears the brackets
    // that say the next digit lands up there rather than after the
    // power. See the slot-bracket tests in `tests::display`.
    assert_eq!(
        drawn(&e),
        vec![
            ("2".to_string(), 0),
            ("(".to_string(), 1),
            ("1.5".to_string(), 1),
            (")".to_string(), 1),
        ]
    );
}

#[test]
fn the_root_key_leaves_its_degree_slot_where_the_cursor_is() {
    // `16`, `ʸ√x`: the radicand is already typed, so the press lands
    // in the degree — and the empty brackets in front of the sign are
    // drawn dim, which is what says so.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(1), Button::Num(6), Button::YRootX] {
        apply_button(&mut e, &mut s, &c, b);
    }
    let segs = crate::ui::display::render_expression(
        e.input.items(),
        e.input.cursor(),
        crate::locale::DecimalSeparator::Dot,
        None,
        None,
        crate::engine::script::Notation::Pretty,
    );
    let slot = segs.iter().find(|seg| seg.text == "()").expect("the slot");
    assert!(!slot.active);
    assert!(slot.script.raise > 0.0);
    // Keying the degree fills it in, in front of the sign.
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    assert_eq!(
        drawn(&e),
        vec![
            ("(".to_string(), 1),
            ("4".to_string(), 1),
            (")".to_string(), 1),
            ("√(".to_string(), 0),
            ("16".to_string(), 0),
            (")".to_string(), 0),
        ]
    );
    assert_eq!(e.evaluate().expect("fourth root of 16").display, "2");
}

// --- second toggle -------------------------------------------------

#[test]
fn second_routes_sin_to_asin() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Second);
    assert!(s.second_mode);
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::Sin);
    // sin-1 is rendered as "sin-1(" by unary_func_name.
    assert!(e.input.display_string().contains("sin-1"));
    // Second is a sticky toggle — using a 2nd-mapped function does
    // not auto-clear it; only another `Second` press does.
    assert!(s.second_mode);
}

#[test]
fn second_flips_e_pow_x_to_y_pow_x() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Second);
    apply_button(&mut e, &mut s, &c, Button::EPowX);
    // Unshifted, 𝑒ˣ would have opened `5×𝑒^` and waited for the
    // exponent. Its second function reads the 5 as the exponent
    // instead and leaves the base to be typed.
    assert_eq!(e.input.display_string(), "^5");
}

// --- power shortcuts -----------------------------------------------

#[test]
fn square_appends_pow_two() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(7));
    apply_button(&mut e, &mut s, &c, Button::Square);
    assert_eq!(e.input.display_string(), "7^2");
}

#[test]
fn ten_pow_x_expands_to_ten_caret() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::TenPowX);
    assert_eq!(e.input.display_string(), "10^");
}

#[test]
fn y_pow_x_makes_the_typed_operand_the_exponent() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    // The caret goes in front of the 2, not after it, and the cursor
    // parks in front of the caret so the base lands where it opened.
    assert_eq!(e.input.display_string(), "^2");
    assert_eq!(e.input.cursor(), 0);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.input.display_string(), "3^2");
    assert_eq!(e.evaluate().expect("3 squared").display, "9");
}

#[test]
fn y_pow_x_is_x_pow_y_with_the_operands_swapped() {
    // The two keys are what tells them apart: same two operands in
    // the same order, opposite readings of which is the base.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::XPowY);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.evaluate().expect("2 cubed").display, "8");

    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.evaluate().expect("3 squared").display, "9");
}

#[test]
fn y_pow_x_swaps_only_the_operand_it_landed_on() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    // The `+` and the 5 stay exactly as typed; `^` binds the same two
    // neighbours it would have bound the other way round.
    assert_eq!(e.input.display_string(), "5+3^2");
    assert_eq!(e.evaluate().expect("5 + 3 squared").display, "14");
}

#[test]
fn y_pow_x_takes_a_bracketed_operand_whole() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(9));
    apply_button(&mut e, &mut s, &c, Button::Sqrt);
    assert_eq!(e.input.display_string(), "√(9)");
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    // The whole call becomes the exponent, not just its closing paren.
    assert_eq!(e.input.display_string(), "^√(9)");
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.input.display_string(), "2^√(9)");
    assert_eq!(e.evaluate().expect("2 cubed").display, "8");
}

#[test]
fn y_pow_x_raises_the_result_of_the_last_evaluation() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    // Like the wrapping functions, it acts on the result rather than
    // starting a fresh expression.
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert_eq!(e.input.display_string(), "^3");
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.evaluate().expect("2 cubed").display, "8");
}

#[test]
fn y_pow_x_left_without_its_base_reports_and_recovers() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    // `=` before the base is typed is an incomplete expression, the
    // same as any other. It reports rather than guessing a base, and
    // leaves the buffer alone so the press is not lost.
    apply_button(&mut e, &mut s, &c, Button::Equals);
    assert_eq!(s.error_message.as_deref(), Some("Undefined"));
    assert_eq!(e.input.display_string(), "^2");
    // The cursor is still waiting in the base slot, so typing one
    // picks the expression back up where the press left it.
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    assert_eq!(e.input.display_string(), "3^2");
    assert!(s.error_message.is_none());
    match apply_button(&mut e, &mut s, &c, Button::Equals) {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "9"),
        _ => panic!("expected Evaluated"),
    }
}

#[test]
fn y_pow_x_without_an_exponent_to_raise_is_a_noop() {
    // Nothing typed yet: unlike x², there is no sensible operand to
    // supply, since a `0` exponent would read `y^0` — 1 for every
    // base the user could go on to type.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert!(e.input.is_empty(), "yˣ started an expression from nothing");

    // Same after a trailing operator or an open bracket: the press
    // leaves no stray caret behind.
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert_eq!(e.input.display_string(), "5+");
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert_eq!(e.input.display_string(), "5+()");
}

#[test]
fn an_operator_after_y_pow_x_follows_the_whole_power() {
    // `2`, `yˣ`, `3` reads `3²` with the cursor still in the base slot
    // the press opened. An operator there is about the power, not
    // about the base: it used to land between the two and give
    // `3+^2` — a power with no base, and a `3` that had left the sum.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::YPowX, Button::Num(3), Button::Add] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "3^2+");
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    assert_eq!(e.evaluate().expect("3 squared plus 4").display, "13");

    // The same for the postfix keys, which would otherwise bind to the
    // base alone.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::YPowX,
        Button::Num(3),
        Button::Factorial,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "3^2!");

    // With the base still empty there is no value for an operator to
    // attach to, so the press is dropped as it always was.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::YPowX, Button::Add] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "^2");
}

#[test]
fn closing_a_bracket_in_an_exponent_closes_the_exponent() {
    // An exponent typed straight after the caret is a slot the
    // display draws brackets round, and `)` is how the user says they
    // are done with it. Nothing is written: `2^3` is already the
    // number they mean, and the press used to bracket the whole power
    // as `(2^3)` — the same value wearing a pair nobody asked for.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(3),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^3");
    assert_eq!(e.input.cursor(), 3);
    assert_eq!(e.evaluate().expect("two cubed").display, "8");

    // The keys with a base of their own read the same way.
    let (mut e, mut s, c) = fresh();
    for b in [Button::EPowX, Button::Num(8), Button::RightParen] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "e^8");

    // A whole tower is one slot closed, not a bracket round it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(3),
        Button::XPowY,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^3^2");

    // An operand that is not in an exponent is unaffected: the
    // brackets go round the number just typed, not the sum in front
    // of it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Add,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+(2)");
}

// --- equals + ans continuation -------------------------------------

#[test]
fn equals_sets_last_result() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
    match effect {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "5"),
        _ => panic!("expected Evaluated"),
    }
    assert_eq!(s.last_result, "5");
    assert!(s.just_evaluated);
}

#[test]
fn digit_after_equals_starts_fresh() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    apply_button(&mut e, &mut s, &c, Button::Num(9));
    assert_eq!(e.input.display_string(), "9");
    assert!(!s.just_evaluated);
}

#[test]
fn repeat_equals_replays_last_operator_and_operand() {
    let (mut e, mut s, c) = fresh();
    // 2 + 3 = 5
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    let r1 = apply_button(&mut e, &mut s, &c, Button::Equals);
    assert!(matches!(r1, ButtonEffect::Evaluated { ref result, .. } if result == "5"));
    // = → 5 + 3 = 8
    let r2 = apply_button(&mut e, &mut s, &c, Button::Equals);
    match r2 {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "8"),
        _ => panic!("expected Evaluated"),
    }
    // = → 8 + 3 = 11
    let r3 = apply_button(&mut e, &mut s, &c, Button::Equals);
    match r3 {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "11"),
        _ => panic!("expected Evaluated"),
    }
}

#[test]
fn operator_after_equals_continues_with_ans() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    apply_button(&mut e, &mut s, &c, Button::Mul);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    // 5 × 2 = 10.
    let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
    match effect {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "10"),
        _ => panic!("expected Evaluated"),
    }
}

// --- error message handling ----------------------------------------

#[test]
fn evaluation_error_sets_error_message() {
    let (mut e, mut s, c) = fresh();
    // Divide by zero is a guaranteed eval-time error.
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Div);
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    assert!(s.error_message.is_some());
}

#[test]
fn next_button_press_clears_error_message() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Div);
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    assert!(s.error_message.is_some());
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert!(s.error_message.is_none());
}

#[test]
fn second_button_does_not_dismiss_error_message() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Div);
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    apply_button(&mut e, &mut s, &c, Button::Second);
    assert!(s.error_message.is_some());
}

// --- basic-mode gating ---------------------------------------------

#[test]
fn scientific_button_ignored_in_basic_mode() {
    let c = Config {
        mode: Mode::Basic,
        ..Config::default()
    };
    let mut e = Engine::default();
    let mut s = UiState::default();
    apply_button(&mut e, &mut s, &c, Button::Sin);
    assert!(e.input.is_empty(), "Sin should no-op in Basic mode");
}

#[test]
fn a_scientific_key_the_user_put_on_the_basic_keypad_works_there() {
    // The gate exists so keyboard shortcuts for scientific functions
    // stay inert in Basic mode. A key the user placed on the Basic
    // keypad is a different matter: refusing it would make their own
    // layout look broken.
    let mut c = Config {
        mode: Mode::Basic,
        ..Config::default()
    };
    c.keypad.basic[0] = "sin backspace percent div".to_string();
    let mut e = Engine::default();
    let mut s = UiState::default();
    apply_button(&mut e, &mut s, &c, Button::Sin);
    assert_eq!(e.input.display_string(), "sin()");
    // Everything they did not place stays gated.
    let mut e = Engine::default();
    apply_button(&mut e, &mut s, &c, Button::Tan);
    assert!(e.input.is_empty(), "Tan is not on this keypad");
}

// --- percent / modulo ----------------------------------------------

#[test]
fn the_percent_key_covers_both_readings() {
    // One key, no separate `mod` cell: against a following operand the
    // tokenizer reads `%` as modulo, everywhere else as percent.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(7), Button::Percent, Button::Num(3)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "7%3");
    assert_eq!(e.evaluate().expect("evaluates").display, "1");

    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Num(0), Button::Percent] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.evaluate().expect("evaluates").display, "0.5");

    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::Num(0),
        Button::Num(0),
        Button::Add,
        Button::Num(1),
        Button::Num(0),
        Button::Percent,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.evaluate().expect("evaluates").display, "220");
}

#[test]
fn percentage_of_a_number_scales_it() {
    // 3.5% × 230 = 8.05. Nothing follows the `%`, so it reads as a
    // percentage.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(3),
        Button::Decimal,
        Button::Num(5),
        Button::Percent,
        Button::Mul,
        Button::Num(2),
        Button::Num(3),
        Button::Num(0),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "3.5%*230");
    assert_eq!(e.evaluate().expect("evaluates").display, "8.05");
}

#[test]
fn a_number_straight_after_the_percent_makes_it_modulo() {
    // 5%3.2 = 1.8. An operand follows the `%`, so the same key reads
    // as modulo — no second press, no separate `mod` key.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Percent,
        Button::Num(3),
        Button::Decimal,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "5%3.2");
    assert_eq!(e.evaluate().expect("evaluates").display, "1.8");
}

#[test]
fn modulo_by_a_negative_needs_no_extra_key() {
    // `±` parenthesises the operand, and `%` before a `(` is modulo,
    // so the negative right-hand side stays reachable from the keypad.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(7),
        Button::Percent,
        Button::Num(3),
        Button::Negate,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "7%(-3)");
    assert_eq!(e.evaluate().expect("evaluates").display, "1");
}

#[test]
fn percent_still_needs_something_to_apply_to() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Percent);
    assert!(e.input.is_empty());
}

// --- keypad presses arrive pre-resolved ----------------------------

#[test]
fn a_keypad_press_is_not_second_mapped_a_second_time() {
    // The keypad draws the armed table, so its cells already emit the
    // second function. Running that through the mapping again would
    // land back on the key the user is not looking at.
    let (mut e, mut s, c) = fresh();
    apply_resolved_button(&mut e, &mut s, &c, Button::Second);
    assert!(s.second_mode);
    apply_resolved_button(&mut e, &mut s, &c, Button::Asin);
    assert!(e.input.display_string().contains("sin-1"));
}

#[test]
fn a_keystroke_follows_the_users_own_second_table() {
    let (mut e, mut s, mut c) = fresh();
    c.keypad.scientific_second[3] = "factorial rand acos atan pi 1 2 3 add".to_string();
    apply_button(&mut e, &mut s, &c, Button::Second);
    assert_eq!(
        resolve_for_keyboard(&c, &s, Button::Sin),
        Button::Rand,
        "the sin cell now carries Rand in the second table"
    );
    apply_button(&mut e, &mut s, &c, Button::Sin);
    assert!(!e.input.display_string().contains("sin"));
}

// --- memory effects ------------------------------------------------

#[test]
fn memory_buttons_emit_effects() {
    let (mut e, mut s, c) = fresh();
    let eff = apply_button(&mut e, &mut s, &c, Button::MemAdd);
    assert_eq!(eff, ButtonEffect::MemoryStore(MemoryOp::Add));
    let eff = apply_button(&mut e, &mut s, &c, Button::MemRecall);
    assert_eq!(eff, ButtonEffect::MemoryRecall);
    let eff = apply_button(&mut e, &mut s, &c, Button::MemClear);
    assert_eq!(eff, ButtonEffect::MemoryClear);
}

// --- pi / constants ------------------------------------------------

#[test]
fn pi_inserts_constant() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Pi);
    assert_eq!(e.input.items(), &[InputItem::Constant(ConstKind::Pi)]);
}

// --- EE / scientific notation -------------------------------------

#[test]
fn ee_after_digit_inserts_times_ten_pow() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::EE);
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    assert_eq!(e.input.display_string(), "1×10^15");
}

#[test]
fn ee_on_empty_buffer_is_noop() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::EE);
    assert!(e.input.is_empty());
}

#[test]
fn ee_after_trailing_operator_is_noop() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::EE);
    assert_eq!(e.input.display_string(), "2+");
}

#[test]
fn equals_on_ten_pow_fifteen_roundtrips_via_scientific_notation() {
    let (mut e, mut s, c) = fresh();
    // 10 ^ 15 = 1e15. The result string `1e15` has to round-trip
    // back into the buffer as `1×10^15`, not the digit run `115`.
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::XPowY);
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
    match effect {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "1e15"),
        _ => panic!("expected Evaluated"),
    }
    assert_eq!(e.input.display_string(), "1×10^15");
}

#[test]
fn equals_on_negative_exponent_roundtrips_via_scientific_notation() {
    let (mut e, mut s, c) = fresh();
    // 1 ÷ 1000000 = 1e-6. The negative exponent must become `(-6)`
    // so the engine reads it as a single signed operand on the
    // next press of `=`.
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    apply_button(&mut e, &mut s, &c, Button::Div);
    apply_button(&mut e, &mut s, &c, Button::Num(1));
    for _ in 0..6 {
        apply_button(&mut e, &mut s, &c, Button::Num(0));
    }
    let effect = apply_button(&mut e, &mut s, &c, Button::Equals);
    match effect {
        ButtonEffect::Evaluated { result, .. } => assert_eq!(result, "1e-6"),
        _ => panic!("expected Evaluated"),
    }
    assert_eq!(e.input.display_string(), "1×10^(-6)");
}

// --- results carried on as operands ---------------------------------

/// Type a sequence of keys on a fresh scientific calculator and hand
/// back what it is left showing.
fn run(keys: &[Button]) -> (Engine, UiState, Config) {
    let (mut e, mut s, c) = fresh();
    for b in keys {
        apply_button(&mut e, &mut s, &c, *b);
    }
    (e, s, c)
}

#[test]
fn a_result_carried_on_keeps_the_precision_it_was_computed_at() {
    // The whole point: `1÷3` shows the fifteen digits that fit, but
    // what goes back into the buffer is the value they were rounded
    // from. Multiplying the digits gives 0.999999999999999;
    // multiplying the value gives back the 1 it came from.
    let (mut e, mut s, c) = run(&[Button::Num(1), Button::Div, Button::Num(3), Button::Equals]);
    assert_eq!(e.input.display_string(), "0.333333333333333");
    for b in [Button::Mul, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "1");
}

#[test]
fn the_digits_on_screen_are_still_the_ones_on_screen() {
    // Only evaluation reads the fuller value. Everything the user can
    // see or copy — the display, the ASCII expression, the history
    // entry — is what was drawn.
    let (e, _s, _c) = run(&[Button::Num(1), Button::Div, Button::Num(3), Button::Equals]);
    assert_eq!(e.input.ascii_expression(), "0.333333333333333");
    // The evaluator gets all eighteen digits the division was carried
    // to, written the way the tokenizer reads them back exactly.
    assert_eq!(
        e.input.ascii_expression_for_eval(),
        "333333333333333333e-18"
    );
}

#[test]
fn editing_a_result_makes_its_digits_the_whole_truth() {
    // Backspace over the result and the annotation goes with it: the
    // user is now looking at a number they typed, and it has to
    // evaluate as the number it looks like.
    let (mut e, mut s, c) = run(&[Button::Num(1), Button::Div, Button::Num(3), Button::Equals]);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "0.33333333333333");
    assert_eq!(
        e.input.ascii_expression_for_eval(),
        e.input.ascii_expression()
    );
    for b in [Button::Mul, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "0.99999999999999");
}

#[test]
fn a_negative_result_carries_on_with_its_sign() {
    // The result is written as `(-x)` when it lands in a non-empty
    // buffer and as `-x` when it does not; either way the value behind
    // it is the negative one.
    let (mut e, mut s, c) = run(&[
        Button::Num(1),
        Button::Div,
        Button::Num(3),
        Button::Negate,
        Button::Equals,
    ]);
    assert_eq!(e.input.display_string(), "-0.333333333333333");
    for b in [Button::Mul, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "-1");
}

#[test]
fn repeat_equals_keeps_the_result_it_repeats_on() {
    // `=` after `=` splices the last operator and operand onto the
    // result already in the buffer, which must not disturb the value
    // behind it.
    let (e, _s, _c) = run(&[
        Button::Num(1),
        Button::Div,
        Button::Num(3),
        Button::Equals,
        Button::Mul,
        Button::Num(3),
        Button::Equals,
        Button::Equals,
    ]);
    assert_eq!(e.input.display_string(), "3");
}

// --- parens --------------------------------------------------------

#[test]
fn parens_insert_literally() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    assert_eq!(e.input.display_string(), "(3)");
}

// --- auto-multiplication --------------------------------------------

#[test]
fn auto_mul_inserted_between_digit_and_left_paren() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    // The implicit `×` should now be a real backend token, not just
    // a synthetic frontend glyph.
    assert_eq!(e.input.display_string(), "5×()");
}

#[test]
fn no_auto_mul_between_digit_and_pi() {
    // Per spec, π attaches directly to a preceding digit run with
    // no synthetic ×; the engine still inserts an implicit
    // multiplication at evaluation time.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Pi);
    assert_eq!(e.input.display_string(), "5π");
}

#[test]
fn auto_mul_inserted_before_ten_pow_x_after_digit() {
    // The 10ˣ button used to glom its `1` onto the existing digit
    // run; the auto-mul backend pass should split them now.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::TenPowX);
    assert_eq!(e.input.display_string(), "5×10^");
}

#[test]
fn no_auto_mul_after_binary_operator() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    // Add ends the value chain so no `×` should be inserted before
    // the new paren group.
    assert_eq!(e.input.display_string(), "5+()");
}

#[test]
fn rand_repeat_replaces_only_the_random() {
    // Pre-load the buffer with `5+` so the second Rand press has
    // a preceding expression to preserve. The new Rand handler
    // deletes only the previous random's items, keeping `5+`.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    let prefix_len = e.input.items().len();
    apply_button(&mut e, &mut s, &c, Button::Rand);
    let after_first = e.input.items().len();
    assert!(after_first > prefix_len, "first rand should add items");
    apply_button(&mut e, &mut s, &c, Button::Rand);
    // The buffer must still start with the original `5+` items.
    assert!(e.input.items().len() > prefix_len);
    let head: Vec<_> = e.input.items().iter().take(prefix_len).collect();
    let original_head: Vec<_> = vec![
        InputItem::Digit('5'),
        InputItem::BinOp(crate::engine::item::BinOp::Add),
    ];
    assert_eq!(head.into_iter().cloned().collect::<Vec<_>>(), original_head);
}

#[test]
fn rand_repeat_dimming_covers_only_the_random() {
    // After two Rand presses, `random_range` should still reference
    // a non-trivial slice that lives inside the current buffer
    // (the just-inserted random), not the whole buffer.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Rand);
    apply_button(&mut e, &mut s, &c, Button::Rand);
    let (rs, re) = s.random_range.expect("random_range should be set");
    assert!(rs >= 2 && re <= e.input.items().len() && re > rs);
}

#[test]
fn digit_after_rand_clears_random_state() {
    // Any non-Rand mutating press must drop the inactive colouring
    // and the saved range so the random becomes a normal piece of
    // the expression the user is editing.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Rand);
    assert!(s.random_range.is_some());
    apply_button(&mut e, &mut s, &c, Button::Num(7));
    assert!(s.random_range.is_none());
    assert!(s.last_expression.is_empty());
}

#[test]
fn sin_after_equals_wraps_the_result() {
    // Before the fix, post-eval Sin cleared the buffer first and
    // then opened `sin(`. With Sin removed from `starts_new`, the
    // result that `evaluate_now` left in the buffer is wrapped.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    apply_button(&mut e, &mut s, &c, Button::Add);
    apply_button(&mut e, &mut s, &c, Button::Num(3));
    apply_button(&mut e, &mut s, &c, Button::Equals);
    let result = e.input.display_string();
    apply_button(&mut e, &mut s, &c, Button::Sin);
    let after = e.input.display_string();
    assert!(
        after.starts_with("sin(") && after.contains(&result) && after.ends_with(')'),
        "expected sin-wrapped result, got {after}"
    );
}

// --- regressions -----------------------------------------------------

#[test]
fn recalled_number_starts_a_new_operand() {
    // The per-item auto-mul helper saw a digit arriving after a digit,
    // called them one numeric run, and concatenated: with `5` typed,
    // recalling 42 produced `542`.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    insert_number_string(&mut e, "42");
    assert_eq!(e.input.display_string(), "5×42");
}

#[test]
fn recalled_negative_number_is_parenthesised() {
    // A bare leading `-` reads as subtraction, so `5` then a recall of
    // -3 evaluated as 5 - 3 = 2.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    insert_number_string(&mut e, "-3");
    assert_eq!(e.input.display_string(), "5×(-3)");
}

#[test]
fn recalled_number_into_empty_buffer_keeps_its_sign() {
    let (mut e, _s, _c) = fresh();
    insert_number_string(&mut e, "-3");
    assert_eq!(e.input.display_string(), "-3");
}

#[test]
fn rand_after_a_digit_does_not_glue_onto_it() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Rand);
    let shown = e.input.display_string();
    assert!(shown.starts_with("5×"), "got {shown}");
}

#[test]
fn mod_needs_a_left_operand() {
    // Mod inserted unconditionally while every other binary operator
    // guarded, so a press on an empty buffer left a stray operator.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Mod);
    assert!(e.input.is_empty());
}

#[test]
fn modulo_by_a_negative_is_expressible() {
    // Modulo and percent both serialised to `%`, so the tokenizer had
    // to guess which was meant from the following character: anything
    // after `mod` that was not a digit, paren or letter flipped the
    // whole expression to the percent reading, and a negative right
    // operand could not be written at all.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(7), Button::Mod, Button::Num(3), Button::Negate] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "7 mod (-3)");
    assert_eq!(e.evaluate().expect("evaluates").display, "1");
}

#[test]
fn mod_and_percent_stay_distinct_through_the_buffer() {
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(7), Button::Mod, Button::Num(3)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "7 mod 3");

    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(7), Button::Percent] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "7%");
}

#[test]
fn shipped_defaults_do_not_show_float_noise() {
    // The end-to-end path a user actually takes, at Config::default().
    for (keys, expected) in [
        (
            vec![
                Button::Num(8),
                Button::Decimal,
                Button::Num(2),
                Button::Add,
                Button::Num(8),
                Button::Decimal,
                Button::Num(2),
            ],
            "16.4",
        ),
        (
            vec![
                Button::Num(3),
                Button::Decimal,
                Button::Num(3),
                Button::Mul,
                Button::Num(3),
            ],
            "9.9",
        ),
        (
            vec![
                Button::Num(9),
                Button::Decimal,
                Button::Num(9),
                Button::Mul,
                Button::Num(9),
                Button::Decimal,
                Button::Num(9),
            ],
            "98.01",
        ),
    ] {
        let config = Config::default();
        let mut e = Engine::new(config.significant_digits);
        let mut s = UiState::default();
        for b in keys {
            apply_button(&mut e, &mut s, &config, b);
        }
        apply_button(&mut e, &mut s, &config, Button::Equals);
        assert_eq!(e.input.display_string(), expected);
    }
}

#[test]
fn home_and_end_move_the_cursor_to_the_extremes() {
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(1), Button::Num(2), Button::Num(3)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.cursor(), 3);
    apply_button(&mut e, &mut s, &c, Button::CursorHome);
    assert_eq!(e.input.cursor(), 0);
    apply_button(&mut e, &mut s, &c, Button::CursorEnd);
    assert_eq!(e.input.cursor(), 3);
}

// --- the three-level limit on nested scripts -------------------------

#[test]
fn x_pow_y_needs_a_base_to_raise() {
    // The `0` an empty display shows is not one the user typed, and
    // `0^y` is 0 for every exponent they could go on to key. So the
    // press does nothing at all rather than starting an expression on
    // a base nobody chose.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::XPowY);
    assert!(e.input.is_empty(), "xʸ started an expression from nothing");

    // A zero that *was* typed is an operand like any other.
    apply_button(&mut e, &mut s, &c, Button::Num(0));
    apply_button(&mut e, &mut s, &c, Button::XPowY);
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    assert_eq!(e.input.display_string(), "0^5");

    // And with no operand under the cursor the press still does
    // nothing, the way it does after an open bracket.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::LeftParen);
    apply_button(&mut e, &mut s, &c, Button::XPowY);
    assert_eq!(e.input.display_string(), "()");
}

#[test]
fn a_fourth_power_brackets_the_three_below_it() {
    // Three levels is as deep as the display goes, so the next power
    // takes what is there as its base instead of climbing: `2^2^2`
    // then `x²` is `(2^2^2)²`, which is 16² and reads at two levels.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(drawn_depths(&e), vec![0, 1, 2]);

    apply_button(&mut e, &mut s, &c, Button::Square);
    assert_eq!(e.input.display_string(), "(2^2^2)^2");
    assert_eq!(drawn_depths(&e), vec![0, 0, 1, 2, 0, 1]);
    assert_eq!(e.evaluate().expect("16 squared").display, "256");
}

#[test]
fn the_power_key_brackets_the_same_way_the_shortcut_does() {
    // `xʸ` and `x³` reach the limit through the same door, and a
    // fourth level is never drawn whichever key asked for it.
    for (key, expected) in [
        (Button::XPowY, "(2^2^2)^"),
        (Button::Cube, "(2^2^2)^3"),
        (Button::Square, "(2^2^2)^2"),
    ] {
        let (mut e, mut s, c) = fresh();
        for b in [
            Button::Num(2),
            Button::XPowY,
            Button::Num(2),
            Button::XPowY,
            Button::Num(2),
            key,
        ] {
            apply_button(&mut e, &mut s, &c, b);
        }
        assert_eq!(e.input.display_string(), expected);
        assert!(drawn_depths(&e).iter().all(|d| *d < MAX_SCRIPT_LEVELS));
    }

    // The bracketed power is an operand like any other, so the levels
    // start again from it: `(2^2^2)^2^2` is still three deep.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(2^2^2)^2^2");
    assert_eq!(drawn_depths(&e), vec![0, 0, 1, 2, 0, 1, 2]);
}

#[test]
fn a_two_argument_call_at_the_limit_takes_the_whole_power() {
    // `logᵧ` and `ʸ√x` write their slot a step off the line too, so
    // they hit the same limit — and they bring brackets of their own,
    // so the whole power goes inside the call rather than into a pair
    // added in front of it.
    let keys = [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(8),
    ];

    let (mut e, mut s, c) = fresh();
    for b in keys {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::LogY);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.input.display_string(), "log(2,2^2^8)");
    assert_eq!(drawn_depths(&e), vec![0, 1, 0, 0, 1, 2, 0]);

    let (mut e, mut s, c) = fresh();
    for b in keys {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::YRootX);
    apply_button(&mut e, &mut s, &c, Button::Num(2));
    assert_eq!(e.input.display_string(), "root(2^2^8,2)");
    assert!(drawn_depths(&e).iter().all(|d| *d < MAX_SCRIPT_LEVELS));

    // Below the limit they still take the operand the user is on, so
    // `2^8` then `logᵧ` is the log of the exponent, not of the power.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::XPowY, Button::Num(8), Button::LogY] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^log(,8)");
}

#[test]
fn bases_and_degrees_stop_at_three_levels_too() {
    // A base under a base under a log is the third level; a fourth
    // press has nowhere to put its slot and no brackets that would
    // help, so it does nothing and leaves the expression standing.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(8),
        Button::LogY,
        Button::Num(2),
        Button::LogY,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "log(log(2,2),8)");
    assert_eq!(drawn_depths(&e), vec![0, 1, 2, 1, 1, 1, 0, 0, 0]);
    let before = e.input.display_string();
    apply_button(&mut e, &mut s, &c, Button::LogY);
    assert_eq!(e.input.display_string(), before);

    // The same for a degree inside a degree.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(1),
        Button::Num(6),
        Button::YRootX,
        Button::Num(4),
        Button::YRootX,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "root(16,root(4,2))");
    assert!(drawn_depths(&e).iter().all(|d| *d < MAX_SCRIPT_LEVELS));
    let before = e.input.display_string();
    apply_button(&mut e, &mut s, &c, Button::YRootX);
    assert_eq!(e.input.display_string(), before);
}

#[test]
fn y_pow_x_at_the_limit_leaves_the_expression_alone() {
    // The one key brackets cannot make room for: the operand it
    // swaps goes *up* a level, and anything wrapped round it goes up
    // with it. So at the limit the press does nothing.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert_eq!(e.input.display_string(), "2^2^2");

    // One level down it works as it always has: the exponent already
    // typed becomes the exponent of a new base.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::XPowY, Button::Num(3), Button::YPowX] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^^3");
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    assert_eq!(e.input.display_string(), "2^4^3");
    assert_eq!(drawn_depths(&e), vec![0, 1, 2]);
}

#[test]
fn a_key_with_its_own_base_starts_again_on_the_line() {
    // `10ˣ`, `2ˣ`, `𝑒ˣ` and `EE` begin a new operand, and the `×` in
    // front of it ends whatever exponent the cursor was in — so they
    // are never the press that runs out of levels, they just carry on
    // from the line the power was written on.
    for (key, expected) in [
        (Button::TenPowX, "2^2^2×10^"),
        (Button::EE, "2^2^2×10^"),
        (Button::TwoPowX, "2^2^2×2^"),
    ] {
        let (mut e, mut s, c) = fresh();
        for b in [
            Button::Num(2),
            Button::XPowY,
            Button::Num(2),
            Button::XPowY,
            Button::Num(2),
            key,
        ] {
            apply_button(&mut e, &mut s, &c, b);
        }
        assert_eq!(e.input.display_string(), expected);
        assert!(drawn_depths(&e).iter().all(|d| *d < MAX_SCRIPT_LEVELS));
    }
}

// --- taking a press back, and what the keys leave behind -------------

#[test]
fn squaring_a_power_brackets_it_rather_than_stacking() {
    // `x²` is one operation on what is on screen, so the power under
    // it goes into brackets: `2^3` squared is `(2^3)²` = 64, where a
    // second caret would have meant `2^3^2` = 2⁹ = 512. `xʸ` is the
    // key for building a tower; this one squares the whole thing.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(3),
        Button::Square,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(2^3)^2");
    assert_eq!(e.evaluate().expect("eight squared").display, "64");

    // And again, one bracket per press.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::Square,
        Button::Square,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "((2^2)^2)^2");
    assert_eq!(drawn_depths(&e), vec![0, 0, 0, 1, 0, 1, 0, 1]);

    // A plain operand has no power to bracket, so nothing is added.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::Square, Button::Cube] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(2^2)^3");
}

#[test]
fn a_square_comes_off_in_one_press() {
    // Backspace takes the exponent back on its own and leaves the base
    // standing, the way it takes back any one press.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Square, Button::Backspace] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5");

    // `C` takes back the value, and `5²` is one value: base and
    // exponent go together.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Square, Button::Clear] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert!(e.input.is_empty());

    // The brackets a press adds are part of the press: backspace gives
    // the expression it was pressed on, not a bracketed one.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::Square,
        Button::Backspace,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^2");

    // And `C` on the same expression takes the whole `(2²)²`, brackets
    // and all, since that is the one value on screen.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::Square,
        Button::Clear,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert!(e.input.is_empty());

    // Mid-expression `C` takes the squared operand and leaves the rest
    // of the line where it is.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Add,
        Button::Num(2),
        Button::Square,
        Button::Clear,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+");

    // And nothing lands in the exponent afterwards: the key writes a
    // finished operation, so the next digit is a new factor with an
    // auto-multiplication in front of it. It used to run onto the end
    // of the exponent, turning `5²` into `5` to the twenty-fourth.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Square, Button::Num(4)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5^2×4");
    assert_eq!(e.evaluate().expect("twenty-five fours").display, "100");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "5^2");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "5");
}

#[test]
fn a_deletion_never_leaves_an_auto_multiplication_behind() {
    // The `×` in `5×(2)` is the calculator's — the user pressed `(`.
    // Taking the bracket group away takes the `×` with it, whether it
    // was backspace or `C` that did it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::LeftParen,
        Button::Num(2),
        Button::CursorRight,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5×(2)");
    apply_button(&mut e, &mut s, &c, Button::Clear);
    assert_eq!(e.input.display_string(), "5");

    // Backspace gets there one item at a time and ends up the same.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::LeftParen,
        Button::Num(2),
        Button::CursorRight,
        Button::Backspace,
        Button::Backspace,
        Button::Backspace,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5");
}

#[test]
fn backspace_retraces_the_cursor_the_press_moved() {
    // `yˣ` puts the caret in front of the operand and the cursor in
    // front of the caret, so there is nothing to the left of it to
    // delete. Backspace used to stick there with `^2` on screen and no
    // way to take it apart; it now takes back the press, and the
    // cursor lands where the press found it.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::YPowX, Button::Num(3)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "3^2");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "^2");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "2");
    assert_eq!(e.input.cursor(), 1);
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert!(e.input.is_empty());

    // Mid-expression it is the same move: the `+` in front of the
    // operand is not what the press wrote and is not what comes off.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Add,
        Button::Num(2),
        Button::YPowX,
        Button::Backspace,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+2");
    assert_eq!(e.input.cursor(), 3);
}

#[test]
fn backspace_steps_out_of_a_slot_that_is_still_empty() {
    // The slot `logᵧ` and `ʸ√x` park the cursor in has nothing of its
    // own to the left of the cursor, so the press steps back into the
    // argument the call closed over rather than deleting the comma —
    // which would have left `root(16)`, a call missing an argument.
    // The call itself only comes off once both slots are empty, which
    // is the one order the two keys ever unwind in.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(1),
        Button::Num(6),
        Button::YRootX,
        Button::Backspace,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "root(16,)");
    // The end of the radicand, which is where the comma is.
    assert_eq!(e.input.cursor(), 3);
    for _ in 0..3 {
        apply_button(&mut e, &mut s, &c, Button::Backspace);
    }
    assert!(e.input.is_empty());

    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(8), Button::LogY, Button::Backspace] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "log(,8)");
    assert_eq!(e.input.cursor(), 3);

    // From an empty display both keys open both slots, and both come
    // back off together once the argument is gone.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LogY,
        Button::Num(8),
        Button::Backspace,
        Button::Backspace,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert!(e.input.is_empty());
}

#[test]
fn y_pow_x_counts_the_whole_power_it_would_raise() {
    // The caret takes the operand *and* whatever is chained onto it up
    // a level: pressing `yˣ` on the `4` of `4^3^2` moves all three
    // levels, which would be a fourth. The press is refused, the same
    // as `xʸ` refuses one.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::YPowX,
        Button::Num(3),
        Button::YPowX,
        Button::Num(4),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "4^3^2");
    assert_eq!(drawn_depths(&e), vec![0, 1, 2]);

    let before = e.input.display_string();
    apply_button(&mut e, &mut s, &c, Button::YPowX);
    assert_eq!(e.input.display_string(), before);

    // Two levels still go up, which is what `2^2` then `yˣ` asks for.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(2),
        Button::YPowX,
        Button::Num(3),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^3^2");
    assert_eq!(drawn_depths(&e), vec![0, 1, 2]);
}

#[test]
fn a_closing_bracket_with_nothing_open_brackets_the_operand() {
    // `)` used to do nothing at all there. What the user is asking for
    // is the operand they just typed set apart, so that is what it
    // does: `5+2` then `)` is `5+(2)`.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Add,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+(2)");
    assert_eq!(e.input.cursor(), 5);

    // With nothing at all to bracket the press still does nothing,
    // rather than leaving an empty pair or a stray closer behind.
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::RightParen);
    assert!(e.input.is_empty());

    // A trailing operator has no right operand for the brackets to
    // close over, so the press takes it back and brackets the value
    // that is left: `5+` then `)` is `(5)`.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Add, Button::RightParen] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5)");

    // The same inside a bracket the user opened, which is where the
    // half-finished sum is most often left: `(5+` then `)` is `(5)`.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LeftParen,
        Button::Num(5),
        Button::Add,
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5)");

    // A bracket that is open is closed as it always was: the press
    // steps over the closer the `(` key put there.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LeftParen,
        Button::Num(2),
        Button::RightParen,
        Button::Num(3),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(2)×3");
}

#[test]
fn one_backspace_per_press_all_the_way_down() {
    // Two squares, two backspaces: the outer press wrote the brackets
    // and the second `^2`, the inner one the first, and each comes
    // back off on its own. A press reaching around an older one does
    // not disturb it.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Square, Button::Square] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5^2)^2");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "5^2");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert_eq!(e.input.display_string(), "5");
    apply_button(&mut e, &mut s, &c, Button::Backspace);
    assert!(e.input.is_empty());
}

// --- the fixed powers are finished operations -----------------------

#[test]
fn a_fixed_exponent_is_not_a_slot_anything_else_can_reach() {
    // `x²` writes one item, not a caret with a digit parked after it,
    // so nothing keyed next lands up in the exponent. A digit starts a
    // new factor, a decimal point starts a new number, and a constant
    // attaches as a factor of its own.
    let cases: [(&[Button], &str, &str); 3] = [
        (
            &[Button::Num(5), Button::Square, Button::Num(3)],
            "5^2×3",
            "75",
        ),
        (
            &[
                Button::Num(5),
                Button::Cube,
                Button::Decimal,
                Button::Num(5),
            ],
            "5^3×0.5",
            "62.5",
        ),
        (&[Button::Num(2), Button::Square, Button::Pi], "2^2×π", ""),
    ];
    for (presses, expected, value) in cases {
        let (mut e, mut s, c) = fresh();
        for b in presses {
            apply_button(&mut e, &mut s, &c, *b);
        }
        assert_eq!(e.input.display_string(), expected);
        if !value.is_empty() {
            assert_eq!(e.evaluate().expect("evaluates").display, value);
        }
    }
}

#[test]
fn a_fixed_exponent_goes_in_brackets_before_another_caret() {
    // `!` and `%` write themselves and nothing else — no brackets go
    // in around what they apply to — so after a square they read the
    // way the buffer spells it, `5^2`, with the postfix binding to
    // the exponent.
    for (key, expected) in [(Button::Factorial, "5^2!"), (Button::Percent, "5^2%")] {
        let (mut e, mut s, c) = fresh();
        for b in [Button::Num(5), Button::Square, key] {
            apply_button(&mut e, &mut s, &c, b);
        }
        assert_eq!(e.input.display_string(), expected);
    }

    // The caret keys do bracket, because without it they would raise
    // the `2` rather than the square — a different number, where a
    // postfix on the exponent is only a different reading of the same
    // gesture.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Square,
        Button::XPowY,
        Button::Num(3),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5^2)^3");
    assert_eq!(e.evaluate().expect("evaluates").display, "15625");

    // Squaring a square is the same case read from the other end.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Square, Button::Square] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5^2)^2");
    assert_eq!(e.evaluate().expect("evaluates").display, "625");
}

#[test]
fn a_factorial_still_belongs_in_a_caret_exponent() {
    // What the fixed powers are sealed *against* is not the factorial
    // itself: `xʸ` and `yˣ` leave the exponent open, and a `!` keyed
    // there is part of it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(3),
        Button::Factorial,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^3!");
    assert_eq!(e.evaluate().expect("two to the six").display, "64");
}

#[test]
fn squaring_a_swapped_power_squares_the_whole_power() {
    // `6`, `yˣ`, `3` reads `3⁶`, with the cursor still in the base
    // slot where the `3` was typed. `x²` is about the number on
    // screen, so it steps out of the slot first: `(3⁶)²`. Keyed in the
    // slot it wrote `3^2^6`, which raises the 3 to the sixty-fourth.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(6),
        Button::YPowX,
        Button::Num(3),
        Button::Square,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(3^6)^2");
    assert_eq!(e.evaluate().expect("729 squared").display, "531441");
}

#[test]
fn percent_and_factorial_write_no_brackets_of_their_own() {
    // Both keys write one item at the cursor. Keyed in an exponent
    // they stay in the exponent — `2^5%` is `2` raised to `5%` — and
    // nothing is moved or bracketed on their behalf.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(5),
        Button::Percent,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^5%");

    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::Num(3),
        Button::Factorial,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^3!");
    assert_eq!(e.evaluate().expect("two to the six").display, "64");

    // A bracket the user opened at the head of the exponent is
    // untouched too, as it always was.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::XPowY,
        Button::LeftParen,
        Button::Num(5),
        Button::Percent,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "2^(5%)");

    // On the line they are the postfix they have always been.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Num(0), Button::Percent] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.evaluate().expect("half").display, "0.5");
}

// --- the reported input defects ------------------------------------

#[test]
fn a_minus_where_a_value_begins_is_its_sign() {
    // On an empty display the `−` key used to supply a left operand
    // nobody typed: `−`, `6` read `0-6`. It is the sign of the number
    // about to be keyed.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Sub, Button::Num(6)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "-6");
    assert_eq!(e.evaluate().expect("minus six").display, "-6");

    // The other three still start on a `0`, which is what they need
    // and what the sign does not.
    for (key, expected) in [
        (Button::Add, "0+"),
        (Button::Mul, "0×"),
        (Button::Div, "0÷"),
    ] {
        let (mut e, mut s, c) = fresh();
        apply_button(&mut e, &mut s, &c, key);
        assert_eq!(e.input.display_string(), expected);
    }

    // A trailing operator is still a change of mind, and changing it
    // back to a sign leaves the sign.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Sub, Button::Add] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "0+");
}

#[test]
fn the_two_argument_calls_take_a_minus_in_every_slot() {
    // Each slot is a place a value begins, so a `−` keyed there is a
    // sign. Every one of these used to drop the press.
    for (presses, expected) in [
        (vec![Button::YRootX, Button::Sub], "root(-,)"),
        (
            vec![Button::Num(5), Button::YRootX, Button::Sub],
            "root(5,-)",
        ),
        (vec![Button::LogY, Button::Sub], "log(,-)"),
        (vec![Button::Num(5), Button::LogY, Button::Sub], "log(-,5)"),
    ] {
        let (mut e, mut s, c) = fresh();
        for b in presses {
            apply_button(&mut e, &mut s, &c, b);
        }
        assert_eq!(e.input.ascii_expression(), expected);
    }

    // And the value they close over keeps its own sign rather than
    // leaving it outside, where it negated the call instead.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Sub,
        Button::Num(4),
        Button::LogY,
        Button::Num(8),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "log(8,-4)");

    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Sub,
        Button::Num(5),
        Button::YRootX,
        Button::Num(3),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "root(-5,3)");

    // A binary minus is not a sign: the call closes over the right
    // operand alone.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(9),
        Button::Sub,
        Button::Num(4),
        Button::YRootX,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "9-root(4,2)");
}

#[test]
fn a_decimal_point_with_no_fraction_behind_it_goes() {
    // `5.` then `+` is `5+`: the point was started and left.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Decimal, Button::Add, Button::Num(6)] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+6");
    assert_eq!(e.evaluate().expect("eleven").display, "11");

    // `=` drops it too, rather than evaluating a half-typed number.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Decimal, Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(s.last_expression, "5");

    // Backspace is the one press that leaves it alone: deleting the
    // point is exactly what it is being asked to do.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Decimal, Button::Backspace] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5");

    // A point with digits behind it is a number, not a leftover.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(5), Button::Decimal, Button::Num(2), Button::Add] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5.2+");
}

#[test]
fn closing_a_bracket_takes_back_the_operator_left_hanging() {
    // The brackets go round a value, and a trailing operator is not
    // part of one: `(5+` then `)` is `(5)`.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LeftParen,
        Button::Num(5),
        Button::Add,
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(5)");
    assert_eq!(e.evaluate().expect("five").display, "5");

    // Modulo counts as an operator waiting for its right operand too.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(7), Button::Mod, Button::RightParen] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "(7)");

    // A value that is already closed is left alone: the press
    // brackets it as it always did.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::Add,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.display_string(), "5+(2)");
}

#[test]
fn an_opening_bracket_leaves_the_base_slot_it_finds_filled() {
    // A bracket after a finished base multiplies the power, so it
    // belongs after it: `5`, `yˣ`, `6`, `(` is `6⁵×()`. It used to
    // land in the slot and push the `6` out of it — `6×()⁵`.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::YPowX,
        Button::Num(6),
        Button::LeftParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "6^5*()");

    // With the slot still empty the bracket is how a base gets typed,
    // so the press stays where it is.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::YPowX,
        Button::LeftParen,
        Button::Num(2),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "(2)^5");
    assert_eq!(e.evaluate().expect("two to the fifth").display, "32");
}

#[test]
fn a_bracket_closed_inside_a_call_slot_finishes_the_slot() {
    // A bracket opened in the degree of a root is a formula written
    // *in* the degree, so closing it finishes the degree: the `+2`
    // then lands outside the call. It used to stay up there and give
    // `root(8,(2)+2)`.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(8),
        Button::YRootX,
        Button::LeftParen,
        Button::Num(2),
        Button::RightParen,
        Button::Add,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "root(8,(2))+2");

    // The `log_y` base reads the same way.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(8),
        Button::LogY,
        Button::LeftParen,
        Button::Num(2),
        Button::RightParen,
        Button::Add,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "log((2),8)+2");

    // The slot-to-slot walk the two keys exist for is untouched.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::LogY,
        Button::Num(8),
        Button::RightParen,
        Button::Num(2),
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "log(2,8)");
}

#[test]
fn closing_a_base_slot_steps_past_the_whole_power() {
    // `)` in the base slot `yˣ` opened closes it; the cursor lands
    // past the power, which is where anything keyed next belongs.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(5),
        Button::YPowX,
        Button::Num(6),
        Button::RightParen,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(e.input.ascii_expression(), "6^5");
    assert_eq!(e.input.cursor(), 3);
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert_eq!(e.input.ascii_expression(), "6^5+");
}

#[test]
fn the_caption_goes_as_soon_as_the_result_is_added_to() {
    // The caption is the expression that produced what is on the
    // display. Add to that result and it is no longer that
    // expression's answer.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::Add, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(s.last_expression, "2+3");
    apply_button(&mut e, &mut s, &c, Button::Add);
    assert!(s.last_expression.is_empty());
    assert!(s.last_expression_items.is_empty());

    // A press that leaves the buffer alone leaves the caption up.
    let (mut e, mut s, c) = fresh();
    for b in [Button::Num(2), Button::Add, Button::Num(3), Button::Equals] {
        apply_button(&mut e, &mut s, &c, b);
    }
    apply_button(&mut e, &mut s, &c, Button::CursorLeft);
    assert_eq!(s.last_expression, "2+3");

    // And a repeat `=` writes its own caption rather than clearing it.
    let (mut e, mut s, c) = fresh();
    for b in [
        Button::Num(2),
        Button::Add,
        Button::Num(3),
        Button::Equals,
        Button::Equals,
    ] {
        apply_button(&mut e, &mut s, &c, b);
    }
    assert_eq!(s.last_expression, "5+3");
}
