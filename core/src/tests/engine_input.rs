use crate::engine::input::*;
use crate::engine::item::{ConstKind, InputItem, UnaryFunc};

fn buf(seq: &[InputItem]) -> InputBuffer {
    let mut b = InputBuffer::new();
    b.replace(seq.to_vec());
    b
}

#[test]
fn last_operand_range_finds_digit_run() {
    let b = buf(&[
        InputItem::Digit('1'),
        InputItem::BinOp(crate::engine::item::BinOp::Add),
        InputItem::Digit('2'),
        InputItem::Digit('3'),
        InputItem::DecimalPoint,
        InputItem::Digit('4'),
    ]);
    // cursor is at the end (6). Range covers indices 2..6.
    assert_eq!(b.last_operand_range(), Some((2, 6)));
}

#[test]
fn last_operand_range_skips_postfix() {
    let b = buf(&[InputItem::Digit('5'), InputItem::Factorial]);
    assert_eq!(b.last_operand_range(), Some((0, 2)));
}

#[test]
fn last_operand_range_matches_paren_group() {
    // (1+2) – closed grouping, cursor at end.
    let b = buf(&[
        InputItem::LeftParen,
        InputItem::Digit('1'),
        InputItem::BinOp(crate::engine::item::BinOp::Add),
        InputItem::Digit('2'),
        InputItem::RightParen,
    ]);
    assert_eq!(b.last_operand_range(), Some((0, 5)));
}

#[test]
fn last_operand_range_matches_function_group() {
    // sqrt(9) – opener is a UnaryFunc with implicit `(`.
    let b = buf(&[
        InputItem::UnaryFunc(UnaryFunc::Sqrt),
        InputItem::Digit('9'),
        InputItem::RightParen,
    ]);
    assert_eq!(b.last_operand_range(), Some((0, 3)));
}

#[test]
fn last_operand_range_returns_constant_alone() {
    let b = buf(&[InputItem::Constant(ConstKind::Pi)]);
    assert_eq!(b.last_operand_range(), Some((0, 1)));
}

#[test]
fn last_operand_range_returns_none_on_operator() {
    let b = buf(&[
        InputItem::Digit('3'),
        InputItem::BinOp(crate::engine::item::BinOp::Mul),
    ]);
    assert_eq!(b.last_operand_range(), None);
}

#[test]
fn last_operand_range_returns_none_for_empty() {
    let b = InputBuffer::new();
    assert_eq!(b.last_operand_range(), None);
}

#[test]
fn insert_at_shifts_cursor_when_before() {
    let mut b = buf(&[InputItem::Digit('1'), InputItem::Digit('2')]);
    // cursor currently at end = 2. Insert at 0 shifts cursor to 3.
    b.insert_at(0, InputItem::Digit('0'));
    assert_eq!(b.items().len(), 3);
    assert_eq!(b.cursor(), 3);
    assert_eq!(b.items()[0], InputItem::Digit('0'));
}

#[test]
fn insert_at_after_cursor_does_not_move_it() {
    let mut b = buf(&[InputItem::Digit('1'), InputItem::Digit('2')]);
    b.set_cursor(0);
    b.insert_at(2, InputItem::Digit('3'));
    assert_eq!(b.cursor(), 0);
    assert_eq!(b.items().last(), Some(&InputItem::Digit('3')));
}

#[test]
fn insert_all_appends_sequence() {
    let mut b = InputBuffer::new();
    b.insert_all([InputItem::Digit('1'), InputItem::Digit('2')]);
    assert_eq!(b.items().len(), 2);
    assert_eq!(b.cursor(), 2);
}

#[test]
fn euler_between_digits_round_trips_as_the_constant() {
    // Reachable by typing `35`, moving the cursor left and pressing
    // the 𝑒 key. Serialising the constant as a bare `e` produced
    // "3e5", which the tokenizer's number scanner read as 300000
    // while the display showed 3𝑒5.
    let mut buf = InputBuffer::new();
    buf.replace(vec![
        InputItem::Digit('3'),
        InputItem::Constant(ConstKind::E),
        InputItem::Digit('5'),
    ]);
    assert_eq!(buf.display_string(), "3𝑒5");
    let shown = crate::engine::evaluate_to_string(
        &buf.ascii_expression(),
        crate::engine::AngleMode::Deg,
        15,
    );
    // 3 · e · 5 = 15e ≈ 40.7742274268857
    assert!(shown.starts_with("40.7742274268857"), "got {shown}");
}

// --- exact values behind the digits on screen -----------------------

/// A buffer holding the digits of `shown` with `value` recorded as
/// what they were rounded from, cursor at the end.
fn with_exact(shown: &str, value: &str) -> InputBuffer {
    let value = crate::engine::decimal::Decimal::parse(value).expect(value);
    let mut b = InputBuffer::new();
    for c in shown.chars() {
        b.insert(match c {
            '.' => InputItem::DecimalPoint,
            d => InputItem::Digit(d),
        });
    }
    b.mark_exact(0, b.items().len(), value);
    b
}

#[test]
fn an_exact_run_evaluates_at_full_precision_and_displays_as_written() {
    let b = with_exact("0.333333333333333", "0.333333333333333333");
    // What the user sees, and what a copy gets.
    assert_eq!(b.ascii_expression(), "0.333333333333333");
    // What the evaluator is handed: all eighteen digits, in the
    // mantissa-and-exponent form the tokenizer reads back exactly.
    assert_eq!(b.ascii_expression_for_eval(), "333333333333333333e-18");
}

#[test]
fn a_run_the_user_types_into_is_the_number_it_looks_like() {
    let mut b = with_exact("5", "5.00000000000000012");
    b.insert(InputItem::Digit('7'));
    assert_eq!(b.ascii_expression_for_eval(), "57");
}

#[test]
fn a_run_the_user_backspaces_into_is_the_number_it_looks_like() {
    let mut b = with_exact("0.5", "0.500000000000000012");
    b.delete_before();
    assert_eq!(b.ascii_expression_for_eval(), "0.");
}

#[test]
fn an_operator_after_a_run_leaves_it_exact() {
    // The whole point of keeping the value: what follows the result is
    // an operator, and the result has to survive it.
    let mut b = with_exact("0.333333333333333", "0.333333333333333333");
    b.insert(InputItem::BinOp(crate::engine::item::BinOp::Mul));
    b.insert(InputItem::Digit('3'));
    assert_eq!(b.ascii_expression_for_eval(), "333333333333333333e-18*3");
}

#[test]
fn work_in_front_of_a_run_shifts_it_rather_than_dropping_it() {
    let mut b = with_exact("2", "2.00000000000000004");
    b.insert_at(0, InputItem::LeftParen);
    b.insert_at(1, InputItem::BinOp(crate::engine::item::BinOp::Sub));
    b.insert_at(b.items().len(), InputItem::RightParen);
    assert_eq!(b.ascii_expression(), "(-2)");
    assert_eq!(b.ascii_expression_for_eval(), "(-200000000000000004e-17)");
}

#[test]
fn a_negative_value_keeps_its_brackets_in_the_spliced_form() {
    let b = with_exact("-0.333333333333333", "-0.333333333333333333");
    assert_eq!(b.ascii_expression_for_eval(), "(-333333333333333333e-18)");
}

#[test]
fn extreme_magnitudes_splice_as_exponents_rather_than_runs_of_zeroes() {
    let b = with_exact("1e-300", "1e-300");
    assert_eq!(b.ascii_expression_for_eval(), "1e-300");
    let b = with_exact("1e300", "1e300");
    assert_eq!(b.ascii_expression_for_eval(), "1e300");
}

#[test]
fn a_spliced_literal_reads_back_as_the_value_it_came_from() {
    // The splice is only worth anything if the tokenizer gets the
    // value back intact, so check the round trip rather than trusting
    // the spelling.
    for literal in [
        "0.333333333333333333",
        "-0.333333333333333333",
        "1e-300",
        "1e300",
        "2.00000000000000004",
    ] {
        let b = with_exact("0", literal);
        let spliced = b.ascii_expression_for_eval();
        let reparsed = crate::engine::evaluate_expression(
            &spliced,
            crate::engine::AngleMode::Deg,
            crate::engine::DEFAULT_SIGNIFICANT_DIGITS,
        )
        .expect("the spliced literal evaluates");
        assert_eq!(
            reparsed.value,
            crate::engine::decimal::Decimal::parse(literal).unwrap(),
            "{literal} spliced as {spliced}"
        );
    }
}

#[test]
fn clearing_and_replacing_forget_what_the_digits_stood_for() {
    let mut b = with_exact("5", "5.00000000000000012");
    b.forget_exact();
    assert_eq!(b.ascii_expression_for_eval(), "5");

    let mut b = with_exact("5", "5.00000000000000012");
    b.replace(vec![InputItem::Digit('5')]);
    assert_eq!(b.ascii_expression_for_eval(), "5");

    let mut b = with_exact("5", "5.00000000000000012");
    b.items_mut().push(InputItem::Digit('1'));
    assert_eq!(b.ascii_expression_for_eval(), "51");
}
