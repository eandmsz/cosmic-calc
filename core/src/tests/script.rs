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
    // No superscript `×`, `!` or `^` exists, so a run holding one has
    // no raised form at all rather than a partial one.
    for flat in ["3×4", "2!", "2^2"] {
        assert_eq!(to_superscript(flat), None, "{flat:?} should not raise");
    }
}

#[test]
fn a_decimal_exponent_stays_raised() {
    // Both separators the display can be set to raise, so a fractional
    // exponent is not dropped back to full size over its point.
    assert_eq!(to_superscript("1.5").as_deref(), Some("¹·⁵"));
    assert_eq!(to_superscript("1,5").as_deref(), Some("¹ʼ⁵"));
    assert_eq!(to_superscript("0.25").as_deref(), Some("⁰·²⁵"));
    // And the raised separator is not the multiplication sign this
    // display uses, so `2¹·⁵` cannot be read as `2¹ × 5`.
    assert!(!to_superscript("1.5").unwrap().contains('×'));
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
fn the_exponent_span_covers_what_binds_to_it() {
    // Postfix `!` and `%` bind to the exponent, so they are part of
    // it: `2^2!` is 2^(2!), and a span stopping at the `2` would say
    // (2²)!.
    let mut factorial = digits("2");
    factorial.push(InputItem::Factorial);
    assert_eq!(exponent_span(&power(&factorial), 1), Some(4));

    let mut percent = digits("2");
    percent.push(InputItem::Percent);
    assert_eq!(exponent_span(&power(&percent), 1), Some(4));

    // A chained power is right-associative: `2^2^2` is 2^(2^2), so
    // the span reaches the end of the inner power too.
    let mut chained = digits("2");
    chained.push(InputItem::BinOp(BinOp::Pow));
    chained.extend(digits("2"));
    assert_eq!(exponent_span(&power(&chained), 1), Some(5));
}

#[test]
fn the_exponent_span_declines_what_is_not_there_yet() {
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
    // `root(` is the buffer's spelling of the radical, so the pretty
    // form wears the sign the square and cube roots already do.
    assert_eq!(
        pretty_display(&InputItem::BinaryFunc(BinaryFunc::Root)),
        "√("
    );
    // Everything else is spelled the same in both notations.
    for same in [
        InputItem::UnaryFunc(UnaryFunc::Sin),
        InputItem::UnaryFunc(UnaryFunc::Sqrt),
        InputItem::Constant(ConstKind::E),
        InputItem::Digit('4'),
        InputItem::BinOp(BinOp::Pow),
    ] {
        assert_eq!(pretty_display(&same), same.display());
    }
}

#[test]
fn a_power_reads_without_its_caret() {
    // What the display puts where the `^` was: the compact raise when
    // Unicode has every glyph, raised brackets when it does not.
    assert_eq!(raise("35"), "³⁵");
    assert_eq!(raise("-3"), "⁻³");
    assert_eq!(raise("(3+4)"), "⁽³⁺⁴⁾");
    // `π` has no superscript, and a bare `2π` would read as 2×π.
    assert_eq!(raise("π"), "⁽π⁾");
    assert_eq!(raise("2!"), "⁽2!⁾");
    // A decimal exponent raises whole — brackets are for what Unicode
    // has no raised glyph for, and the separators now have one.
    assert_eq!(raise("1.5"), "¹·⁵");
    // Neither form is ever a caret.
    for exponent in ["35", "-3", "(3+4)", "π", "2!", "1.5"] {
        assert!(!raise(exponent).contains('^'), "{exponent}");
    }
}

#[test]
fn a_log_base_reads_under_its_log() {
    // The mirror of `raise`: lowered outright where Unicode has the
    // glyphs, in lowered brackets where it does not.
    assert_eq!(lower("2"), "₂");
    assert_eq!(lower("10"), "₁₀");
    assert_eq!(lower("π"), "₍π₎");
    assert_eq!(lower("1.5"), "₍1.5₎");
    // The empty slot says a base is expected without claiming one.
    assert_eq!(EMPTY_BASE, "₍₎");
}

#[test]
fn the_argument_separator_finds_the_comma_that_ends_the_base() {
    let call = |items: Vec<InputItem>| {
        let mut all = vec![InputItem::BinaryFunc(BinaryFunc::LogBase)];
        all.extend(items);
        all
    };

    // `log(2,8)`: the base ends at the comma.
    let items = call(vec![
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ]);
    assert_eq!(argument_separator(&items, 0), Some(2));

    // An empty base slot is still a slot.
    let items = call(vec![
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ]);
    assert_eq!(argument_separator(&items, 0), Some(1));

    // A comma belonging to a nested call is not this one's.
    let items = call(vec![
        InputItem::BinaryFunc(BinaryFunc::Root),
        InputItem::Digit('9'),
        InputItem::Comma,
        InputItem::Digit('2'),
        InputItem::RightParen,
        InputItem::Comma,
        InputItem::Digit('8'),
        InputItem::RightParen,
    ]);
    assert_eq!(argument_separator(&items, 0), Some(6));

    // One argument and no comma is the log10 reading, not a base.
    let items = call(vec![InputItem::Digit('8'), InputItem::RightParen]);
    assert_eq!(argument_separator(&items, 0), None);

    // And an index that is not a log call at all.
    assert_eq!(argument_separator(&digits("12"), 0), None);
}

#[test]
fn raw_notation_is_not_pretty() {
    assert!(Notation::Pretty.is_pretty());
    assert!(!Notation::Raw.is_pretty());
    assert_eq!(Notation::default(), Notation::Pretty);
}
