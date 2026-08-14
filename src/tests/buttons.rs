use crate::config::{Config, Mode};
use crate::engine::item::{ConstKind, InputItem};
use crate::engine::Engine;
use crate::ui::buttons::*;

fn fresh() -> (Engine, UiState, Config) {
    (Engine::default(), UiState::default(), Config::default())
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
fn reciprocal_wraps_last_operand() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(4));
    apply_button(&mut e, &mut s, &c, Button::Reciprocal);
    assert_eq!(e.input.display_string(), "(1÷4)");
    // Pressing again unwraps.
    apply_button(&mut e, &mut s, &c, Button::Reciprocal);
    assert_eq!(e.input.display_string(), "4");
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
fn second_flips_sqrt_to_square() {
    let (mut e, mut s, c) = fresh();
    apply_button(&mut e, &mut s, &c, Button::Num(5));
    apply_button(&mut e, &mut s, &c, Button::Second);
    apply_button(&mut e, &mut s, &c, Button::Sqrt);
    assert_eq!(e.input.display_string(), "5^2");
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
