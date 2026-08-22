use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};
use crate::engine::script::*;

/// Digits (and decimal points) as buffer items.
fn digits(s: &str) -> Vec<InputItem> {
    s.chars()
        .map(|c| match c {
            '.' => InputItem::DecimalPoint,
            d if d.is_ascii_digit() => InputItem::Digit(d),
            _ => unreachable!("test helper only handles digit/decimal"),
        })
        .collect()
}

/// `2 ^ <exponent>`, with the `^` always at index 1.
fn power(exponent: &[InputItem]) -> Vec<InputItem> {
    let mut items = digits("2");
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend_from_slice(exponent);
    items
}

#[test]
fn raising_is_all_or_nothing() {
    assert_eq!(to_superscript("-12").as_deref(), Some("⁻¹²"));
    assert_eq!(to_superscript("(3+4)").as_deref(), Some("⁽³⁺⁴⁾"));
    // No superscript `×`, `!`, `.` or `^` exists, so a run holding one
    // has no raised form at all rather than a partial one.
    for flat in ["3×4", "2!", "1.5", "2^2"] {
        assert_eq!(to_superscript(flat), None, "{flat:?} should not raise");
    }
}

#[test]
fn lowering_is_all_or_nothing() {
    assert_eq!(to_subscript("10").as_deref(), Some("₁₀"));
    assert_eq!(to_subscript("x"), None);
}

#[test]
fn the_exponent_span_covers_one_operand() {
    // A digit run, and nothing past it.
    let items = power(&digits("25"));
    assert_eq!(exponent_span(&items, 1), Some(4));

    // Leading signs belong to the exponent (the parser reads it as a
    // `unary`).
    let mut signed = vec![InputItem::BinOp(BinOp::Sub)];
    signed.extend(digits("3"));
    let items = power(&signed);
    assert_eq!(exponent_span(&items, 1), Some(4));

    // A bracketed group runs to its matching closer, nested pairs
    // included.
    let mut group = vec![InputItem::LeftParen, InputItem::LeftParen];
    group.extend(digits("3"));
    group.push(InputItem::RightParen);
    group.push(InputItem::RightParen);
    let items = power(&group);
    assert_eq!(exponent_span(&items, 1), Some(7));

    // A constant is one item, and what follows it is a separate
    // factor: `2^π5` is (2^π)×5.
    let mut constant = vec![InputItem::Constant(ConstKind::Pi)];
    constant.extend(digits("5"));
    let items = power(&constant);
    assert_eq!(exponent_span(&items, 1), Some(3));
}

#[test]
fn the_exponent_span_declines_what_it_cannot_raise() {
    // Postfix `!` and `%` bind to the exponent, and neither has a
    // raised form.
    let mut factorial = digits("2");
    factorial.push(InputItem::Factorial);
    assert_eq!(exponent_span(&power(&factorial), 1), None);

    let mut percent = digits("2");
    percent.push(InputItem::Percent);
    assert_eq!(exponent_span(&power(&percent), 1), None);

    // A chained power is right-associative: `2^2^2` is 2^(2^2), so
    // raising the first exponent alone would say (2²)².
    let mut chained = digits("2");
    chained.push(InputItem::BinOp(BinOp::Pow));
    chained.extend(digits("2"));
    assert_eq!(exponent_span(&power(&chained), 1), None);

    // An unclosed group has no end to stop at.
    let mut unclosed = vec![InputItem::LeftParen];
    unclosed.extend(digits("3"));
    assert_eq!(exponent_span(&power(&unclosed), 1), None);

    // A trailing `^`, and an operator where an operand should be.
    assert_eq!(exponent_span(&power(&[]), 1), None);
    assert_eq!(
        exponent_span(&power(&[InputItem::BinOp(BinOp::Mul)]), 1),
        None
    );

    // Asking about an index that is not a `^` at all.
    assert_eq!(exponent_span(&digits("12"), 0), None);
}

#[test]
fn log_bases_and_inverses_get_their_pretty_spelling() {
    assert_eq!(pretty_display(&InputItem::LogN(7)), "log₇(");
    assert_eq!(pretty_display(&InputItem::LogN(128)), "log₁₂₈(");
    assert_eq!(
        pretty_display(&InputItem::UnaryFunc(UnaryFunc::Log2)),
        "log₂("
    );
    assert_eq!(
        pretty_display(&InputItem::UnaryFunc(UnaryFunc::Log10)),
        "log₁₀("
    );
    assert_eq!(
        pretty_display(&InputItem::UnaryFunc(UnaryFunc::Acosh)),
        "cosh⁻¹("
    );
    // Everything else is spelled the same in both notations.
    for same in [
        InputItem::UnaryFunc(UnaryFunc::Sin),
        InputItem::UnaryFunc(UnaryFunc::Sqrt),
        InputItem::BinaryFunc(BinaryFunc::Root),
        InputItem::Constant(ConstKind::E),
        InputItem::Digit('4'),
        InputItem::BinOp(BinOp::Pow),
    ] {
        assert_eq!(pretty_display(&same), same.display());
    }
}

#[test]
fn raw_notation_is_not_pretty() {
    assert!(Notation::Pretty.is_pretty());
    assert!(!Notation::Raw.is_pretty());
    assert_eq!(Notation::default(), Notation::Pretty);
}
