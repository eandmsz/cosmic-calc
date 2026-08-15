use crate::ui::button_style::*;
use crate::ui::buttons::Button;

#[test]
fn digits_map_to_number_slot() {
    assert_eq!(category_for(Button::Num(0)), Category::Number);
    assert_eq!(category_for(Button::Num(9)), Category::Number);
}

#[test]
fn basic_ops_share_basicop_slot() {
    assert_eq!(category_for(Button::Add), Category::BasicOp);
    assert_eq!(category_for(Button::Sub), Category::BasicOp);
    assert_eq!(category_for(Button::Mul), Category::BasicOp);
    assert_eq!(category_for(Button::Div), Category::BasicOp);
}

#[test]
fn equals_has_its_own_slot() {
    assert_eq!(category_for(Button::Equals), Category::Equals);
}

#[test]
fn second_has_its_own_slot() {
    assert_eq!(category_for(Button::Second), Category::Second);
}

#[test]
fn parens_clear_and_memory_share_toprow() {
    assert_eq!(category_for(Button::LeftParen), Category::TopRow);
    assert_eq!(category_for(Button::Clear), Category::TopRow);
    assert_eq!(category_for(Button::MemRecall), Category::TopRow);
}

#[test]
fn decimal_and_negate_get_own_slots() {
    assert_eq!(category_for(Button::Decimal), Category::Decimal);
    assert_eq!(category_for(Button::Negate), Category::Negate);
}

#[test]
fn scientific_functions_share_science_slot() {
    assert_eq!(category_for(Button::Sin), Category::Science);
    assert_eq!(category_for(Button::Sqrt), Category::Science);
    assert_eq!(category_for(Button::Pi), Category::Science);
    assert_eq!(category_for(Button::Factorial), Category::Science);
    assert_eq!(category_for(Button::Pow), Category::Science);
    assert_eq!(category_for(Button::EE), Category::Science);
}

#[test]
fn category_color_reads_expected_slot() {
    use crate::theme::ThemeKind;
    let t = ThemeKind::Cosmic.get();
    assert_eq!(Category::Number.color(&t), t.number_button);
    assert_eq!(Category::BasicOp.color(&t), t.basicop_button);
    assert_eq!(Category::Equals.color(&t), t.equals_button);
}
