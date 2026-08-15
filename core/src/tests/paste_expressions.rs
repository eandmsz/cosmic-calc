//! Expressions that must survive a paste, with the exact characters a
//! user would put on the clipboard.
//!
//! Each row pins both halves of the round trip: what the buffer renders
//! (so a paste can never show one expression while computing another)
//! and what the engine then returns. Several rows are deliberately
//! malformed or out of domain — "understood properly" there means a
//! definite error rather than a plausible wrong number.

use crate::clipboard::paste_items;
use crate::engine::{evaluate_to_string, AngleMode, InputBuffer};

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
    ("√7 sin^-1(5)", "√(7)sin-1(5)", "Undefined"),
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
    ("ctg(0)", "cot(0)", "Undefined"),
    ("cot(0)", "cot(0)", "Undefined"),
    // --- powers, factorials, precedence ------------------------------
    ("-3^5!", "-3^5!", "-1.79701029991443e57"),
    ("2+3^(2+1)×4", "2+3^(2+1)×4", "110"),
    ("5555555", "5555555", "5555555"),
    // --- root and log with two arguments -----------------------------
    ("root(27, 6)", "root(27,6)", "1.73205080756888"),
    ("root(729, 3!)", "root(729,3!)", "3"),
    ("log(π, π^2)", "log(π,π^2)", "2"),
    // An even root of a negative has no real value.
    ("root(-1, 4)", "root(-1,4)", "Undefined"),
    // `5. 6` is not an argument list: the space goes, leaving one
    // argument where root needs two.
    ("root(5. 6)", "root(5.6)", "Undefined"),
    // --- logarithms ---------------------------------------------------
    ("log2(-2)", "log2(-2)", "Undefined"),
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
fn a_stray_letter_rejects_the_paste() {
    // `l` is on the allow-list (it starts `ln` and `log`) but on its
    // own it is not a token. Keeping the rest would mean pasting
    // `root(5, 4)` when the clipboard said something else, so the whole
    // paste is dropped instead.
    assert_eq!(paste_items(Some("l root(5, 4)")), None);
}

#[test]
fn long_mixed_expressions_round_trip() {
    // Two stress strings mixing Unicode operators, uppercase names,
    // spelled-out constants, implicit multiplication and modulo.
    //
    // Note `2e ＋2e8`: the space is dropped before anything else runs,
    // leaving `2e+2e8`, and `e` between digits is an exponent — so this
    // reads as 2×10² ×10⁸, not as the constant 𝑒. Writing 𝑒 gives the
    // constant.
    let (shown, value) = paste("2e8%3 sqrt(sin−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pi*2e ＋2e8%3");
    assert_eq!(
        shown,
        "2×10^8%3√(sin-1(1)+tan-1(1))∛(8)root(16,4)3π×2×10^2×10^8%3"
    );
    assert_eq!(value, "0.001953125");

    let (shown, value) = paste("sqrt(sin−1(1)＋atan(1))cbrt(8)rOOt(16, 4)3pi*2e＋2e8 mod3");
    assert_eq!(
        shown,
        "√(sin-1(1)+tan-1(1))∛(8)root(16,4)3π×2×10^2×10^8 mod 3"
    );
    assert_eq!(value, "0.0009765625");
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
