//! Expressions that must survive a paste, with the exact characters a
//! user would put on the clipboard.
//!
//! Each row pins both halves of the round trip: what the buffer renders
//! (so a paste can never show one expression while computing another)
//! and what the engine then returns. Several rows are deliberately
//! malformed or out of domain — "understood properly" there means a
//! definite error rather than a plausible wrong number.

use crate::clipboard::paste_items;
use crate::engine::{
    evaluate_expression, evaluate_to_string, AngleMode, InputBuffer, DEFAULT_SIGNIFICANT_DIGITS,
};

/// Paste `raw` and return `(rendered buffer, evaluated result)`.
fn paste(raw: &str) -> (String, String) {
    let items = paste_items(Some(raw)).unwrap_or_else(|| panic!("{raw:?} was rejected"));
    let mut buf = InputBuffer::new();
    buf.replace(items);
    let shown = buf.display_string();
    let value = evaluate_to_string(&buf.ascii_expression(), AngleMode::Deg, 15);
    (shown, value)
}

/// (pasted text, rendered buffer, evaluated result)
const CASES: &[(&str, &str, &str)] = &[
    // --- radicals binding to a single operand ------------------------
    // `√7 …` is √7 times what follows, not √(7 × …). arcsin(5) is
    // outside [-1, 1], so the whole thing is undefined — but for the
    // right reason now.
    (
        "√7 sin^-1(5)",
        "√(7)sin-1(5)",
        "Undefined sin\u{2212}1(x) must be between \u{2212}1 and 1",
    ),
    ("√3", "√(3)", "1.73205080756888"),
    ("∛27", "∛(27)", "3"),
    // A function after the radical has no single operand to bind to,
    // so the radical covers the rest: √(log₃2).
    ("√ log3(2)", "√(log3(2)", "0.794310867086343"),
    // `(√16-2)!` is (4-2)! = 2, not √(16-2)!. This value matches the
    // engine's own test for the same expression typed directly.
    (
        "(2^2×-2^2×3^(3+4)÷(-2)^2+(√16-2)!×5÷2+4)",
        "(2^2×-2^2×3^(3+4)÷(-2)^2+(√(16)-2)!×5÷2+4)",
        "-8739",
    ),
    // --- cotangent spellings -----------------------------------------
    ("ctg(0)", "cot(0)", "Undefined: Cotangent"),
    ("cot(0)", "cot(0)", "Undefined: Cotangent"),
    // --- powers, factorials, precedence ------------------------------
    ("-3^5!", "-3^5!", "-1.79701029991443e57"),
    ("2+3^(2+1)×4", "2+3^(2+1)×4", "110"),
    ("5555555", "5555555", "5555555"),
    // --- root and log with two arguments -----------------------------
    ("root(27, 6)", "root(27,6)", "1.73205080756888"),
    ("root(729, 3!)", "root(729,3!)", "3"),
    ("log(π, π^2)", "log(π,π^2)", "2"),
    // An even root of a negative has no real value.
    (
        "root(-1, 4)",
        "root(-1,4)",
        "Undefined: Negative number under even root",
    ),
    // `5. 6` is not an argument list: the space goes, leaving one
    // argument where root needs two.
    ("root(5. 6)", "root(5.6)", "Undefined"),
    // --- logarithms ---------------------------------------------------
    (
        "log2(-2)",
        "log2(-2)",
        "Undefined: Negative number inside logarithm",
    ),
    // Digit grouping by space survives, because the space is dropped.
    ("log2(65 535)", "log2(65535)", "15.9999779860527"),
    // --- modulo and percent ------------------------------------------
    ("10 mod 3", "10 mod 3", "1"),
    ("5-6%3×8+100%", "5-6%3×8+100%", "10"),
    ("2e8%3", "2×10^8%3", "2"),
    ("2e8%3 log(5)", "2×10^8%3log(5)", "1.39794000867204"),
    // --- scientific notation and the Euler constant -------------------
    // A decimal comma plus an exponent; `e` between digits is a power
    // of ten, so this renders as ×10^ rather than as the constant.
    ("-1,79701e57", "-1.79701×10^57", "-1.79701e57"),
    // The italic 𝑒 is always the constant: π⁵ × 𝑒².
    ("π×π×π×π×π×𝑒×𝑒", "π×π×π×π×π×𝑒×𝑒", "2261.19661825552"),
    // Both constants, a radical inside a logarithm, a factorial in an
    // exponent, a parenthesised divisor and a scientific-notation
    // literal in one expression. `2^2!` is 2^(2!) = 4, and sin is read
    // in degrees; `3e4` is a mantissa and an exponent, so it renders
    // as ×10^ rather than as the constant.
    (
        "𝑒×π×log(√2)+sin(2^2!)÷(4+5×3)×3e4",
        "𝑒×π×log(√(2))+sin(2^2!)÷(4+5×3)×3×10^4",
        "111.42715872663",
    ),
    // --- precision at the edge of the significant-digit budget --------
    (
        "1+0,00000000000001",
        "1+0.00000000000001",
        "1.00000000000001",
    ),
    ("1+0,0000000000001", "1+0.0000000000001", "1.0000000000001"),
];

#[test]
fn listed_expressions_paste_and_evaluate() {
    for (raw, expect_shown, expect_value) in CASES {
        let (shown, value) = paste(raw);
        assert_eq!(&shown, expect_shown, "buffer rendering of {raw:?}");
        assert_eq!(&value, expect_value, "evaluation of {raw:?}");
    }
}

#[test]
fn a_stray_letter_is_stripped() {
    // `l` is on the allow-list (it starts `ln` and `log`) but on its
    // own it is not a token, so it is dropped and the rest evaluates.
    // Characters off the allow-list still reject the whole paste.
    let (shown, value) = paste("l root(5, 4)");
    assert_eq!(shown, "root(5,4)");
    assert_eq!(value, "1.49534878122122");
    assert_eq!(paste_items(Some("root(5, 4)x")), None);
}

#[test]
fn long_mixed_expressions_round_trip() {
    // Two stress strings mixing Unicode operators, uppercase names,
    // spelled-out constants, implicit multiplication and modulo.
    //
    // They differ only in the space before `＋`, and that space decides
    // what the `e` in front of it means. Here nothing is attached after
    // the `e`, so it is the constant: `3π × 2𝑒 + 2×10⁸`.
    let (shown, value) = paste("2e8%3 sqrt(sin−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pi*2e ＋2e8%3");
    assert_eq!(
        shown,
        "2×10^8%3√(sin-1(1)+tan-1(1))∛(8)root(16,4)3π×2𝑒+2×10^8%3"
    );
    assert_eq!(value, "4764.69177326513");

    // With no space, `2e＋2` is a mantissa and a signed exponent, so the
    // `e` stays a power of ten.
    let (shown, value) = paste("sqrt(sin−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pi*2e＋2e8 mod3");
    assert_eq!(
        shown,
        "√(sin-1(1)+tan-1(1))∛(8)root(16,4)3π×2×10^2×10^8 mod 3"
    );
    // The product is 8760481940103.00054 at the working precision, and
    // what survives `mod 3` is its fractional tail — a residue of
    // where the arithmetic ran out of digits, not of the mathematics.
    // In binary that tail was 0.0009765625, which is 2⁻¹⁰ and just as
    // much an artifact; in decimal the digits that are left are
    // decimal ones.
    assert_eq!(value, "0.00054");
}

#[test]
fn the_inverse_caret_spelling_reaches_the_engine() {
    // `sin^-1` is the other spelling the README advertises. It works
    // both pasted and handed to the engine directly.
    assert_eq!(paste("sin^-1(1)").1, "90");
    assert_eq!(paste("cos^-1(1)").1, "0");
    assert_eq!(paste("tanh^-1(0)").1, "0");
    assert_eq!(evaluate_to_string("sin^-1(1)", AngleMode::Deg, 15), "90");
    // A caret after a number is still a power, not an inverse.
    assert_eq!(evaluate_to_string("2^-1", AngleMode::Deg, 15), "0.5");
    assert_eq!(paste("2^-1").1, "0.5");
}

#[test]
fn a_bare_radical_binds_tighter_than_a_following_operator() {
    // The engine and the paste path must agree on the same text.
    for raw in ["√16-2", "∛27+1", "√16!", "√4×3"] {
        let (_, pasted) = paste(raw);
        let direct = evaluate_to_string(
            &raw.replace('√', "sqrt").replace('∛', "cbrt"),
            AngleMode::Deg,
            15,
        );
        assert_eq!(pasted, direct, "paste and engine disagree on {raw:?}");
    }
}

#[test]
fn the_two_constant_formula_matches_to_the_last_representable_digit() {
    // Same expression as the table row above, checked against the full
    // value rather than the display string: 111,427158726630384.
    //
    // That expectation carries 18 significant digits. An f64 backs
    // under 16, and the display is budgeted at
    // DEFAULT_SIGNIFICANT_DIGITS (15) on top of that, so neither half
    // of the check can be a literal comparison against all 18. The
    // display stops at 111.42715872663 — rendered `111,42715872663` by
    // the display layer in a comma-decimal locale — and the raw value
    // is pinned to within one unit in the last place of the nearest
    // double to the expected number. It does not land exactly on that
    // double: every one of the ten-odd operations in the chain rounds,
    // and the accumulated drift moves the last bit.
    let items = paste_items(Some("𝑒×π×log(√2)+sin(2^2!)÷(4+5×3)×3e4"))
        .expect("the formula was rejected by the paste sanitiser");
    let mut buf = InputBuffer::new();
    buf.replace(items);

    let out = evaluate_expression(
        &buf.ascii_expression(),
        AngleMode::Deg,
        DEFAULT_SIGNIFICANT_DIGITS,
    )
    .expect("the formula did not evaluate");

    // Written out to all 18 digits on purpose: that the last two are
    // rounded away by the literal itself is half of what this test
    // documents, so clippy's "excessive precision" is the point.
    #[allow(clippy::excessive_precision)]
    let expected = 111.427158726630384_f64;
    let one_ulp = expected.next_up() - expected;
    let value = out.value.to_f64();
    assert!(
        (value - expected).abs() <= one_ulp,
        "{value} is more than one ulp from {expected}"
    );
    assert_eq!(out.display, "111.42715872663");
}
