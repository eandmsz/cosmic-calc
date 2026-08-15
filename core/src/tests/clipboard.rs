use crate::clipboard::*;
use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};

#[test]
fn copy_empty_buffer_yields_zero() {
    assert_eq!(copy_text_for(""), "0");
}

#[test]
fn copy_roundtrips_non_empty() {
    assert_eq!(copy_text_for("1+2"), "1+2");
}

#[test]
fn paste_rejects_disallowed_char() {
    assert_eq!(sanitize_paste("1+2+wat"), None);
}

#[test]
fn paste_rejects_over_255_chars() {
    let s = "1".repeat(256);
    assert_eq!(sanitize_paste(&s), None);
}

#[test]
fn paste_accepts_exactly_255_chars() {
    let s = "1".repeat(255);
    assert_eq!(sanitize_paste(&s), Some(s));
}

#[test]
fn paste_normalises_operators() {
    // multiplication variants, division variants, plus, minus,
    // percent glyphs.
    let raw = "1×2÷3＋4－5﹪";
    let out = sanitize_paste(raw).unwrap();
    assert_eq!(out, "1×2÷3+4-5%");
}

#[test]
fn paste_normalises_parens() {
    let raw = "{1+[2*3]}";
    let out = sanitize_paste(raw).unwrap();
    assert_eq!(out, "(1+(2×3))");
}

#[test]
fn paste_case_folds_letters() {
    let raw = "SIN(0)";
    let out = sanitize_paste(raw).unwrap();
    assert_eq!(out, "sin(0)");
}

#[test]
fn paste_rewrites_asin_to_sin_minus_one() {
    let out = sanitize_paste("asin(1)").unwrap();
    assert_eq!(out, "sin-1(1)");
}

#[test]
fn paste_rewrites_asinh_before_asin() {
    let out = sanitize_paste("asinh(1)").unwrap();
    assert_eq!(out, "sinh-1(1)");
}

#[test]
fn paste_rewrites_sqrt_and_cbrt() {
    assert_eq!(sanitize_paste("sqrt(4)").unwrap(), "√(4)");
    assert_eq!(sanitize_paste("cbrt(8)").unwrap(), "∛(8)");
}

#[test]
fn paste_keeps_mod_spelled_out() {
    // `mod` used to be rewritten to `%`, which then had to be told
    // apart from the percent postfix by peeking at the next character.
    // It now survives as its own token all the way to `InputItem::Modulo`.
    assert_eq!(sanitize_paste("5 mod 3").unwrap(), "5mod3");
    let items = items_from_paste("5mod3").expect("representable");
    assert!(matches!(items[1], InputItem::Modulo));
}

#[test]
fn paste_drops_spaces_except_after_comma() {
    let out = sanitize_paste("root(9, 2)").unwrap();
    // Space after ',' preserved; the one inside `root( 9` dropped.
    assert_eq!(out, "root(9, 2)");
}

#[test]
fn paste_pi_variants_normalise() {
    let out = sanitize_paste("𝜋+𝝅").unwrap();
    assert_eq!(out, "π+π");
}

#[test]
fn paste_e_variants_normalise() {
    let out = sanitize_paste("ℯ*𝐞").unwrap();
    assert_eq!(out, "𝑒×𝑒");
}

#[test]
fn items_from_paste_builds_digits_and_ops() {
    let items = items_from_paste("1+2").expect("representable");
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], InputItem::Digit('1')));
    assert!(matches!(items[1], InputItem::BinOp(BinOp::Add)));
    assert!(matches!(items[2], InputItem::Digit('2')));
}

#[test]
fn items_from_paste_collapses_function_paren() {
    // Canonical post-sanitise form of `sin(0)`.
    let items = items_from_paste("sin(0)").expect("representable");
    assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Sin)));
    assert!(matches!(items[1], InputItem::Digit('0')));
    assert!(matches!(items[2], InputItem::RightParen));
}

#[test]
fn items_from_paste_handles_sin_minus_one() {
    let items = items_from_paste("sin-1(1)").expect("representable");
    assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Asin)));
    assert!(matches!(items[1], InputItem::Digit('1')));
    assert!(matches!(items[2], InputItem::RightParen));
}

#[test]
fn items_from_paste_recognises_pi_and_e() {
    let items = items_from_paste("π+𝑒").expect("representable");
    assert!(matches!(items[0], InputItem::Constant(ConstKind::Pi)));
    assert!(matches!(items[1], InputItem::BinOp(BinOp::Add)));
    assert!(matches!(items[2], InputItem::Constant(ConstKind::E)));
}

#[test]
fn paste_rejects_what_it_cannot_represent() {
    // Anything off the allow-list drops the whole paste; keeping the
    // representable remainder would substitute a different expression.
    // `x` never reaches this stage — `sanitize_paste` refuses it first.
    assert_eq!(sanitize_paste("xyz"), None);
    assert_eq!(sanitize_paste("2+wat"), None);
    // Letters that *are* on the list but start no keyword are stray
    // characters rather than unrepresentable ones, and get dropped.
    assert!(items_from_paste("2+3").is_some());
}

#[test]
fn paste_keeps_root_and_its_argument_separator() {
    // `root(16,4)` used to lose the keyword character by character and
    // read the comma as a decimal point, yielding `(16.4)` = 16.4.
    let items = items_from_paste("root(16,4)").expect("representable");
    assert!(matches!(items[0], InputItem::BinaryFunc(BinaryFunc::Root)));
    assert!(items.contains(&InputItem::Comma));
    assert_eq!(
        crate::engine::evaluate_to_string(&buffer_ascii(&items), crate::engine::AngleMode::Deg, 15),
        "2"
    );
}

#[test]
fn paste_keeps_spelled_out_pi() {
    // `3pi` used to drop both letters and evaluate to 3.
    let items = items_from_paste("3pi").expect("representable");
    assert!(matches!(items[1], InputItem::Constant(ConstKind::Pi)));
}

#[test]
fn paste_expands_scientific_notation_instead_of_faking_euler() {
    // `2e3` produced [2, 𝑒, 3]: the display read "2·e·3" (≈16.31) but
    // the ASCII round-trip gave the tokenizer "2e3" = 2000.
    let items = items_from_paste("2e3").expect("representable");
    assert_eq!(
        items,
        vec![
            InputItem::Digit('2'),
            InputItem::BinOp(BinOp::Mul),
            InputItem::Digit('1'),
            InputItem::Digit('0'),
            InputItem::BinOp(BinOp::Pow),
            InputItem::Digit('3'),
        ]
    );
    // A bare `e` with no exponent digits is still Euler's number.
    let items = items_from_paste("2𝑒").expect("representable");
    assert!(matches!(
        items.last(),
        Some(InputItem::Constant(ConstKind::E))
    ));
}

#[test]
fn paste_negative_exponent_is_parenthesised() {
    let items = items_from_paste("1e-4").expect("representable");
    assert!(items.contains(&InputItem::LeftParen));
    assert_eq!(
        crate::engine::evaluate_to_string(&buffer_ascii(&items), crate::engine::AngleMode::Deg, 15),
        "0.0001"
    );
}

/// Render an item stream the way the input buffer would, so a paste can
/// be handed straight to the engine.
fn buffer_ascii(items: &[InputItem]) -> String {
    let mut buf = crate::engine::InputBuffer::new();
    buf.replace(items.to_vec());
    buf.ascii_expression()
}

#[test]
fn italic_euler_is_never_an_exponent() {
    // `𝑒` is the calculator's symbol for Euler's number; only the
    // plain ASCII `e` between digits introduces an exponent.
    let items = items_from_paste("2𝑒3").expect("representable");
    assert_eq!(
        items,
        vec![
            InputItem::Digit('2'),
            InputItem::Constant(ConstKind::E),
            InputItem::Digit('3'),
        ]
    );
    // 2 · e · 3 ≈ 16.31, not 2000.
    let shown =
        crate::engine::evaluate_to_string(&buffer_ascii(&items), crate::engine::AngleMode::Deg, 15);
    assert!(shown.starts_with("16.309690970754"), "got {shown}");
}

#[test]
fn paste_keeps_an_arbitrary_log_base() {
    // `log6(279936)` used to read as `log` + a stray `6`, producing
    // `log(6(279936)` = 6.23 where the engine, handed the same text
    // directly, answers 7.
    let items = items_from_paste("log6(279936)").expect("representable");
    assert!(matches!(items[0], InputItem::LogN(6)));
    assert_eq!(
        crate::engine::evaluate_to_string(&buffer_ascii(&items), crate::engine::AngleMode::Deg, 15),
        "7"
    );
    // log2 and log10 keep their dedicated variants.
    let items = items_from_paste("log2(8)").expect("representable");
    assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Log2)));
    let items = items_from_paste("log10(1000)").expect("representable");
    assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Log10)));
    // A bare `log` is still log base 10.
    let items = items_from_paste("log(100)").expect("representable");
    assert!(matches!(items[0], InputItem::UnaryFunc(UnaryFunc::Log)));
}
