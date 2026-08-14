use crate::engine::input::*;
use crate::engine::item::{ConstKind, InputItem, UnaryFunc};

fn buf(seq: &[InputItem]) -> InputBuffer {
    let mut b = InputBuffer::new();
    for it in seq {
        b.push(it.clone());
    }
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
