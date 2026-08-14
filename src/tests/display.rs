use crate::ui::display::*;
use crate::locale::DecimalSeparator;
use crate::engine::item::{BinOp, ConstKind, InputItem, UnaryFunc};

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
    render_expression_string(items, decimal, thousands)
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

// --- auto-multiplication --------------------------------------------

#[test]
fn auto_mul_inserted_between_number_and_left_paren() {
    let mut items = digits("5");
    items.push(InputItem::LeftParen);
    items.extend(digits("3"));
    items.push(InputItem::RightParen);
    let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, items.len(), DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, 2, DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, 3, DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, 0, DecimalSeparator::Dot, None, None);
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
    let segs = render_expression(&items, 2, DecimalSeparator::Dot, None, None);
    let last = segs.last().unwrap();
    assert_eq!(last.text, ")");
    assert!(!last.active);
}
