use crate::engine::item::{BinOp, BinaryFunc, ConstKind, InputItem, UnaryFunc};
use crate::engine::script::Notation;
use crate::locale::DecimalSeparator;
use crate::ui::display::*;

fn digits(s: &str) -> Vec<InputItem> {
    s.chars()
        .map(|c| match c {
            '.' => InputItem::DecimalPoint,
            d if d.is_ascii_digit() => InputItem::Digit(d),
            _ => unreachable!("test helper only handles digit/decimal"),
        })
        .collect()
}

fn render_str(items: &[InputItem], decimal: DecimalSeparator, thousands: Option<char>) -> String {
    render_expression_string(items, decimal, thousands, Notation::Pretty)
}

#[test]
fn small_integer_unchanged() {
    let s = render_str(&digits("7"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "7");
}

#[test]
fn thousands_separator_dot_locale() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "1,234,567");
}

#[test]
fn thousands_separator_comma_locale() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, "1.234.567");
}

#[test]
fn thousands_disabled_renders_no_grouping() {
    let s = render_str(&digits("1234567"), DecimalSeparator::Dot, None);
    assert_eq!(s, "1234567");
}

#[test]
fn fractional_part_uses_configured_decimal() {
    let s = render_str(&digits("1234.5678"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, "1.234,5678");
}

#[test]
fn leading_dot_run_emits_only_fraction() {
    let s = render_str(&digits(".5"), DecimalSeparator::Comma, Some('.'));
    assert_eq!(s, ",5");
}

#[test]
fn mixed_sequence_groups_each_number_independently() {
    let mut items = digits("12345");
    items.push(InputItem::BinOp(BinOp::Add));
    items.extend(digits("6789"));
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "12,345+6,789");
}

#[test]
fn auto_mul_after_constant_before_number() {
    // Per spec: a constant on the LEFT side of a numeric run shows
    // an auto-multiplication glyph, since the user is starting a
    // new operand. (Compare with the digit-then-constant case below
    // where the constant attaches without a glyph.)
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Digit('1'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "π×1,000");
}

#[test]
fn explicit_mul_between_constant_and_number_unchanged() {
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::BinOp(BinOp::Mul),
        InputItem::Digit('1'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
        InputItem::Digit('0'),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "π×1,000");
}

#[test]
fn no_auto_mul_after_percent() {
    // `5%` followed by a left paren must NOT show an auto-mul –
    // percent is treated as a non-value-ender for display purposes.
    let mut items = digits("5");
    items.push(InputItem::Percent);
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "5%(3)");
}

#[test]
fn auto_mul_between_two_constants() {
    // π·π should display the glyph because the right-hand item is
    // a Constant and the item it abuts is itself a Constant — not
    // a digit run, so the "5π" suppression rule doesn't apply.
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Constant(ConstKind::Pi),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "π×π");
}

#[test]
fn auto_mul_between_constant_and_euler() {
    let items = vec![
        InputItem::Constant(ConstKind::Pi),
        InputItem::Constant(ConstKind::E),
    ];
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "π×𝑒");
}

#[test]
fn no_auto_mul_after_digits_before_pi() {
    let mut items = digits("5");
    items.push(InputItem::Constant(ConstKind::Pi));
    let s = render_str(&items, DecimalSeparator::Dot, None);
    assert_eq!(s, "5π");
}

#[test]
fn unary_funcs_render_paren_normally() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Sqrt),
        InputItem::Digit('9'),
        InputItem::RightParen,
    ];
    let s = render_str(&items, DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "√(9)");
}

#[test]
fn three_digit_integers_are_not_grouped() {
    let s = render_str(&digits("999"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "999");
}

#[test]
fn exactly_four_digits_grouped() {
    let s = render_str(&digits("1234"), DecimalSeparator::Dot, Some(','));
    assert_eq!(s, "1,234");
}

// --- raised exponents, lowered log bases ----------------------------

/// `base` raised to `exponent`, as the buffer stores it.
fn power(base: &str, exponent: &[InputItem]) -> Vec<InputItem> {
    let mut items = digits(base);
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend_from_slice(exponent);
    items
}

#[test]
fn an_exponent_is_raised() {
    let items = power("2", &digits("2"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2²");
    // The debug toggle asks for exactly what the buffer holds.
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "2^2"
    );
}

#[test]
fn a_negative_exponent_raises_its_sign_too() {
    let mut exponent = vec![InputItem::BinOp(BinOp::Sub)];
    exponent.extend(digits("12"));
    let items = power("2", &exponent);
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁻¹²");
}

#[test]
fn a_bracketed_exponent_is_raised_whole() {
    let mut exponent = vec![InputItem::LeftParen];
    exponent.extend(digits("3"));
    exponent.push(InputItem::BinOp(BinOp::Add));
    exponent.extend(digits("4"));
    exponent.push(InputItem::RightParen);
    let items = power("2", &exponent);
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2⁽³⁺⁴⁾");
}

#[test]
fn an_exponent_without_a_superscript_form_stays_flat() {
    // `2^2!` is 2^(2!) to the engine, and `!` has no superscript — so
    // raising only the `2` would read as (2²)!.
    let mut exponent = digits("2");
    exponent.push(InputItem::Factorial);
    assert_eq!(
        render_str(&power("2", &exponent), DecimalSeparator::Dot, None),
        "2^2!"
    );

    // A chained power is right-associative: `2^2^2` is 2^(2^2), so
    // the outer exponent is the whole `2^2` and raising only its first
    // half would read as (2²)². The innermost power has a plain
    // operand and still raises, which leaves `2^2²` — a faithful
    // reading of 2^(2²), and the same 16 the engine returns.
    let mut chained = digits("2");
    chained.push(InputItem::BinOp(BinOp::Pow));
    chained.extend(digits("2"));
    assert_eq!(
        render_str(&power("2", &chained), DecimalSeparator::Dot, None),
        "2^2²"
    );

    // And for a bracketed exponent holding a glyph with no raised
    // form: there is no superscript `×`.
    let mut product = vec![InputItem::LeftParen];
    product.extend(digits("3"));
    product.push(InputItem::BinOp(BinOp::Mul));
    product.extend(digits("4"));
    product.push(InputItem::RightParen);
    assert_eq!(
        render_str(&power("2", &product), DecimalSeparator::Dot, None),
        "2^(3×4)"
    );
}

#[test]
fn a_raised_exponent_still_ends_a_value() {
    // `2^3(4)`: the exponent binds tighter than the implicit
    // multiplication, so the auto-mul glyph lands after the power.
    let mut items = power("2", &digits("3"));
    items.push(InputItem::LeftParen);
    items.extend(digits("4"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // The base keeps its own segment; the `^` and the items it raises
    // share the one after it.
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["2", "³", "×", "(", "4", ")"]);
    assert!(!segs[2].active);
}

#[test]
fn only_the_operand_the_exponent_covers_is_raised() {
    // `2^3π` is (2³)×π — the constant is a separate factor, and it
    // attaches without an auto-mul glyph exactly as `3π` would.
    let mut items = power("2", &digits("3"));
    items.push(InputItem::Constant(ConstKind::Pi));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "2³π");
}

#[test]
fn a_pasted_power_of_ten_reads_as_one() {
    // What `3e4` becomes on the way through the paste sanitiser.
    let mut items = digits("3");
    items.push(InputItem::BinOp(BinOp::Mul));
    items.extend(digits("10"));
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("4"));
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "3×10⁴");
}

#[test]
fn exponents_are_never_digit_grouped() {
    // The base is grouped as usual; the exponent is not — a raised
    // thousands separator does not exist, and no calculator groups an
    // exponent anyway.
    let mut items = digits("1234");
    items.push(InputItem::BinOp(BinOp::Pow));
    items.extend(digits("1234"));
    assert_eq!(
        render_str(&items, DecimalSeparator::Dot, Some(',')),
        "1,234¹²³⁴"
    );
}

#[test]
fn log_bases_are_lowered() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Log2),
        InputItem::Digit('8'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₂(8)");
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "log2(8)"
    );

    let items = vec![
        InputItem::LogN(7),
        InputItem::Digit('2'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "log₇(2)");
}

#[test]
fn the_debug_toggle_swaps_the_two_spellings_of_one_expression() {
    // `root(2^2,6)` against `root(2²,6)`: two-argument functions keep
    // their written form in both notations — there is no radical sign
    // that carries a degree and a comma on one line — while the power
    // inside the first argument raises as it does anywhere else.
    let items = vec![
        InputItem::BinaryFunc(BinaryFunc::Root),
        InputItem::Digit('2'),
        InputItem::BinOp(BinOp::Pow),
        InputItem::Digit('2'),
        InputItem::Comma,
        InputItem::Digit('6'),
        InputItem::RightParen,
    ];
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Pretty),
        "root(2²,6)"
    );
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "root(2^2,6)"
    );
}

#[test]
fn inverse_functions_use_a_raised_minus_one() {
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Asin),
        InputItem::Digit('1'),
        InputItem::RightParen,
    ];
    assert_eq!(render_str(&items, DecimalSeparator::Dot, None), "sin⁻¹(1)");
    assert_eq!(
        render_expression_string(&items, DecimalSeparator::Dot, None, Notation::Raw),
        "sin-1(1)"
    );
}

// --- auto-multiplication --------------------------------------------

#[test]
fn auto_mul_inserted_between_number_and_left_paren() {
    let mut items = digits("5");
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // Segments: "5", inactive "×", "(", "3", ")"
    assert_eq!(segs[1], DisplaySegment::inactive("×"));
    assert_eq!(
        segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        vec!["5", "×", "(", "3", ")"]
    );
}

#[test]
fn auto_mul_inserted_between_number_and_unary_func() {
    let mut items = digits("3");
    items.push(InputItem::UnaryFunc(UnaryFunc::Sin));
    items.extend(digits("0"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    assert_eq!(segs[1], DisplaySegment::inactive("×"));
}

#[test]
fn auto_mul_inserted_between_close_paren_and_left_paren() {
    let mut items = vec![InputItem::LeftParen];
    items.extend(digits("2"));
    items.push(InputItem::RightParen);
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["(", "2", ")", "×", "(", "3", ")"]);
    assert!(!segs[3].active);
}

#[test]
fn no_auto_mul_after_binary_operator() {
    let mut items = digits("5");
    items.push(InputItem::BinOp(BinOp::Add));
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(
        &items,
        items.len(),
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let texts: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["5", "+", "(", "3", ")"]);
}

// --- inactive closing paren when cursor inside ---------------------

#[test]
fn closing_paren_inactive_when_cursor_inside_pair() {
    // Items: ( 3 )    Cursor at index 2 (between 3 and ')')
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        2,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    // Last segment is the ')' – should be inactive.
    let last = segs.last().unwrap();
    assert_eq!(last.text, ")");
    assert!(!last.active);
}

#[test]
fn closing_paren_active_when_cursor_past_it() {
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        3,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert!(last.active);
}

#[test]
fn closing_paren_active_when_cursor_before_opener() {
    // Cursor at 0 – before the whole group.
    let items = vec![
        InputItem::LeftParen,
        InputItem::Digit('3'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        0,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert!(last.active);
}

#[test]
fn unary_func_paren_inactive_with_cursor_inside() {
    // sin( . )    where the function-with-paren itself counts as the
    // opener at index 0, so cursor between digits and `)` flags the
    // closer inactive.
    let items = vec![
        InputItem::UnaryFunc(UnaryFunc::Sin),
        InputItem::Digit('0'),
        InputItem::RightParen,
    ];
    let segs = render_expression(
        &items,
        2,
        DecimalSeparator::Dot,
        None,
        None,
        Notation::Pretty,
    );
    let last = segs.last().unwrap();
    assert_eq!(last.text, ")");
    assert!(!last.active);
}
