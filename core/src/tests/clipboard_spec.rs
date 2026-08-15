//! Spec conformance for the clipboard paste pipeline.
//!
//! These tests are written directly against the specification rather
//! than against the implementation: the character allow-list, the
//! substitution table and the function-name rewrites are each
//! enumerated in full here, so a rule silently dropped from
//! `clipboard.rs` fails a test rather than passing unnoticed.

use crate::clipboard::*;
use crate::engine::item::InputItem;
use crate::engine::{evaluate_to_string, AngleMode, InputBuffer};

/// Every character the spec permits, in the order the spec lists them.
const ALLOWED: &str = concat!(
    "0123456789",
    ",.(){}[]!",
    "hHcCtTsSoOaAlLgGnNmMdDqQrRbBpPiI",
    "π𝛑𝜋𝝅𝝿",
    "eEℯｅ𝐞𝑒𝒆𝓮𝖾𝗲𝘦𝙚𝚎",
    "/／∕➗÷",
    "＊*﹡×⋅✕✖",
    "🞡🞢🞣🞤🞥🞦🞧✚＋+﹢",
    "－−-﹣˗",
    "⁒％%﹪",
    "√∛",
    "^",
    " ",
);

/// Evaluate a pasted string end to end, the way the app does.
fn paste_and_eval(raw: &str) -> Option<String> {
    let items = paste_items(Some(raw))?;
    let mut buf = InputBuffer::new();
    buf.replace(items);
    Some(evaluate_to_string(
        &buf.ascii_expression(),
        AngleMode::Deg,
        15,
    ))
}

// =====================================================================
// Silently ignored pastes
// =====================================================================

#[test]
fn ignores_a_non_text_clipboard() {
    // The UI surfaces "the clipboard did not hold text" as a `None`
    // payload, decided before any content is read.
    assert_eq!(paste_items(None), None);
}

#[test]
fn ignores_an_empty_clipboard() {
    assert_eq!(paste_items(Some("")), None);
    // Whitespace alone reduces to nothing once spaces are dropped.
    assert_eq!(paste_items(Some("   ")), None);
}

#[test]
fn ignores_a_paste_past_the_length_limit() {
    assert_eq!(MAX_PASTE_CHARS, 255);
    let at_limit = "1".repeat(MAX_PASTE_CHARS);
    assert!(
        paste_items(Some(&at_limit)).is_some(),
        "255 chars must pass"
    );

    let past_limit = "1".repeat(MAX_PASTE_CHARS + 1);
    assert_eq!(paste_items(Some(&past_limit)), None, "256 chars must fail");

    // The cap counts characters, not bytes: 255 multi-byte characters
    // is far more than 255 bytes and must still be accepted.
    let wide = "π".repeat(MAX_PASTE_CHARS);
    assert!(sanitize_paste(&wide).is_some(), "255 wide chars must pass");
    assert_eq!(sanitize_paste(&"π".repeat(MAX_PASTE_CHARS + 1)), None);
}

#[test]
fn every_character_the_spec_allows_is_accepted() {
    for ch in ALLOWED.chars() {
        // Wrapped in digits so the surrounding expression is always
        // well formed and only the character under test can fail.
        let probe = format!("1{ch}1");
        assert!(
            sanitize_paste(&probe).is_some(),
            "spec allows {ch:?} (U+{:04X}) but the paste was rejected",
            ch as u32
        );
    }
}

#[test]
fn a_character_outside_the_list_rejects_the_whole_paste() {
    // Letters that spell no function name, plus punctuation and
    // symbols the spec does not mention.
    for ch in "xyzwvXYZWVfjkuFJKU@#$&|<>?~`'\"\\:;_=".chars() {
        let probe = format!("1{ch}1");
        assert_eq!(
            sanitize_paste(&probe),
            None,
            "{ch:?} (U+{:04X}) is not on the spec list but was accepted",
            ch as u32
        );
    }
    // Rejection is all-or-nothing: the valid prefix is not kept.
    assert_eq!(paste_items(Some("12+34x")), None);
}

// =====================================================================
// Whitespace
// =====================================================================

#[test]
fn whitespace_is_dropped_except_after_a_comma() {
    assert_eq!(sanitize_paste("1 + 2").unwrap(), "1+2");
    assert_eq!(sanitize_paste("  1  2  ").unwrap(), "12");
    // A space preceded by ',' survives.
    assert_eq!(sanitize_paste("root(9, 2)").unwrap(), "root(9, 2)");
    // Only the space immediately after the comma; later ones still go.
    assert_eq!(sanitize_paste("root(9, 2 )").unwrap(), "root(9, 2)");
}

// =====================================================================
// Character substitutions
// =====================================================================

#[test]
fn every_substitution_in_the_spec_is_applied() {
    // (inputs, canonical form) — the full table from the spec.
    const TABLE: &[(&str, &str)] = &[
        // Case folding.
        ("H", "h"),
        ("C", "c"),
        ("T", "t"),
        ("S", "s"),
        ("O", "o"),
        ("A", "a"),
        ("L", "l"),
        ("G", "g"),
        ("N", "n"),
        ("M", "m"),
        ("D", "d"),
        ("Q", "q"),
        ("R", "r"),
        ("B", "b"),
        ("P", "p"),
        ("I", "i"),
        ("E", "e"),
        // Brackets.
        ("{", "("),
        ("[", "("),
        ("}", ")"),
        ("]", ")"),
        // Division.
        ("/", "÷"),
        ("／", "÷"),
        ("∕", "÷"),
        ("➗", "÷"),
        // Multiplication.
        ("＊", "×"),
        ("*", "×"),
        ("﹡", "×"),
        ("⋅", "×"),
        ("✕", "×"),
        ("✖", "×"),
        // Addition.
        ("🞡", "+"),
        ("🞢", "+"),
        ("🞣", "+"),
        ("🞤", "+"),
        ("🞥", "+"),
        ("🞦", "+"),
        ("🞧", "+"),
        ("✚", "+"),
        ("＋", "+"),
        ("﹢", "+"),
        // Subtraction.
        ("－", "-"),
        ("−", "-"),
        ("﹣", "-"),
        ("˗", "-"),
        // Percent.
        ("％", "%"),
        ("⁒", "%"),
        ("﹪", "%"),
        // Pi.
        ("𝛑", "π"),
        ("𝜋", "π"),
        ("𝝅", "π"),
        ("𝝿", "π"),
        // Euler.
        ("ℯ", "𝑒"),
        ("ｅ", "𝑒"),
        ("𝐞", "𝑒"),
        ("𝒆", "𝑒"),
        ("𝓮", "𝑒"),
        ("𝖾", "𝑒"),
        ("𝗲", "𝑒"),
        ("𝘦", "𝑒"),
        ("𝙚", "𝑒"),
        ("𝚎", "𝑒"),
    ];
    for (from, to) in TABLE {
        let got =
            sanitize_paste(from).unwrap_or_else(|| panic!("{from:?} should be on the allow-list"));
        assert_eq!(&got, to, "{from:?} should canonicalise to {to:?}");
    }
}

#[test]
fn characters_already_canonical_are_left_alone() {
    for ch in ["÷", "×", "+", "-", "%", "π", "𝑒", "(", ")", "^", "!", "."] {
        assert_eq!(sanitize_paste(ch).unwrap(), ch);
    }
}

#[test]
fn substitutions_survive_in_a_whole_expression() {
    // Every family at once, in the shape a spreadsheet might hand over.
    let out = sanitize_paste("{1＋2}✕[3－4]∕5％").unwrap();
    assert_eq!(out, "(1+2)×(3-4)÷5%");
}

// =====================================================================
// Function-name rewrites
// =====================================================================

#[test]
fn every_function_name_rewrite_in_the_spec_is_applied() {
    const TABLE: &[(&str, &str)] = &[
        ("asin", "sin-1"),
        ("acos", "cos-1"),
        ("atan", "tan-1"),
        ("asinh", "sinh-1"),
        ("acosh", "cosh-1"),
        ("atanh", "tanh-1"),
        ("sqrt", "√"),
        ("cbrt", "∛"),
    ];
    for (from, to) in TABLE {
        assert_eq!(
            &sanitize_paste(from).unwrap(),
            to,
            "{from} should be rewritten to {to}"
        );
    }
    // The longer hyperbolic names must win over their prefixes, or
    // `asinh` would come out as `sin-1` followed by a stray `h`.
    assert_eq!(sanitize_paste("asinh(1)").unwrap(), "sinh-1(1)");
    assert_eq!(sanitize_paste("asin(1)").unwrap(), "sin-1(1)");
}

#[test]
fn rewrites_apply_after_case_folding() {
    // Uppercase input reaches the rewrite table through the case fold.
    assert_eq!(sanitize_paste("ASIN(1)").unwrap(), "sin-1(1)");
    assert_eq!(sanitize_paste("SqRt(4)").unwrap(), "√(4)");
    assert_eq!(sanitize_paste("CBRT(8)").unwrap(), "∛(8)");
}

#[test]
fn mod_reaches_the_engine_as_the_modulo_operator() {
    // The spec writes this rewrite as `mod` -> `%`. It is kept as the
    // word instead, because `%` is also the percent postfix and the
    // tokenizer then has to guess which was meant from the following
    // character — which is what made `7 mod -3` evaluate as `7% - 3`
    // = -2.93. What the rule is *for* is asserted here: the word
    // reaches the engine as a modulo operation, including with a
    // negative right operand, which the `%` spelling cannot express.
    let items = paste_items(Some("5 mod 3")).unwrap();
    assert!(items.contains(&InputItem::Modulo));
    assert_eq!(paste_and_eval("5 mod 3").unwrap(), "2");
    assert_eq!(paste_and_eval("5MOD3").unwrap(), "2");
    assert_eq!(paste_and_eval("7 mod -3").unwrap(), "1");
    // The `%` spelling keeps working for callers that use it.
    assert_eq!(paste_and_eval("5%3").unwrap(), "2");
}

// =====================================================================
// End-to-end
// =====================================================================

#[test]
fn the_readme_compatibility_examples_paste_and_evaluate() {
    // Mixed Unicode operators, uppercase names and spelled-out
    // constants, as advertised in the README.
    for (raw, expected) in [
        ("sQrt(4)", "2"),
        ("CBRT(8)", "2"),
        ("rOOt(16, 4)", "2"),
        ("asin(1)", "90"),
        ("3pI", "9.42477796076938"),
        ("2＊3", "6"),
        ("10−4", "6"),
        ("{2+3}✕2", "10"),
    ] {
        assert_eq!(
            paste_and_eval(raw).unwrap_or_else(|| panic!("{raw:?} was rejected")),
            expected,
            "pasting {raw:?}"
        );
    }
}

// =====================================================================
// `e`: exponent or Euler's number
// =====================================================================

#[test]
fn an_e_is_an_exponent_only_when_a_number_is_attached_on_both_sides() {
    // Decided before whitespace is dropped, because the space is the
    // only thing distinguishing the two readings once it is gone.
    for (raw, expect) in [
        ("2e8", "2e8"), // mantissa and exponent attached
        ("2e3", "2e3"),
        ("1e-4", "1e-4"), // a sign may sit between
        ("2e+2", "2e+2"),
        ("1,5e3", "1,5e3"), // decimal comma mantissa
        ("2e", "2𝑒"),       // nothing after: the constant
        ("2e +2", "2𝑒+2"),  // space after: the constant, then addition
        ("2e 8", "2𝑒8"),    // space after: the constant, then 8
    ] {
        assert_eq!(&sanitize_paste(raw).unwrap(), expect, "reading of {raw:?}");
    }
    // A bare `e` with no mantissa before it is the constant already, so
    // it is left alone and the spec's `E` -> `e` fold still holds.
    assert_eq!(sanitize_paste("E").unwrap(), "e");
    assert_eq!(sanitize_paste("3*e*5").unwrap(), "3×e×5");
}

#[test]
fn the_two_readings_evaluate_differently() {
    assert_eq!(paste_and_eval("2e+2").unwrap(), "200");
    assert_eq!(paste_and_eval("2e +2").unwrap(), "7.43656365691809");
    assert_eq!(paste_and_eval("2e8").unwrap(), "200000000");
    assert_eq!(paste_and_eval("2e 8").unwrap(), "43.4925092553447");
}

#[test]
fn a_stray_allowlisted_letter_is_dropped() {
    // The allow-list only admits letters that appear in function names,
    // so one that starts no keyword is a stray character. Anything off
    // the list still rejects the whole paste.
    assert_eq!(paste_and_eval("l root(5, 4)").unwrap(), "1.49534878122122");
    assert_eq!(paste_and_eval("2+3n").unwrap(), "5");
    assert_eq!(paste_items(Some("2+3z")), None);
}

#[test]
fn a_half_understood_word_rejects_the_paste() {
    // `hello` is made only of allow-listed letters, and its `e` is a
    // valid token on its own — so dropping the rest would have left
    // Euler's number and evaluated to 2.718. A run of letters has to be
    // understood completely or not at all.
    for word in ["hello", "chat", "cost", "moral", "digital", "salmon"] {
        assert_eq!(
            paste_items(Some(word)),
            None,
            "{word:?} is not an expression and must be refused"
        );
    }
    // Still true of a word embedded in a real expression.
    assert_eq!(paste_items(Some("2+hello")), None);
}

#[test]
fn a_stray_letter_is_still_dropped_next_to_a_real_one() {
    // The space is what separates `l` from `root`, so the two are
    // different runs: one wholly unread, one wholly read.
    assert_eq!(paste_and_eval("l root(5, 4)").unwrap(), "1.49534878122122");
    assert_eq!(paste_and_eval("2+3n").unwrap(), "5");
    // Glued together they are one run, half of which was understood.
    assert_eq!(paste_items(Some("lroot(5, 4)")), None);
}
