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
fn parens_and_memory_share_toprow() {
    assert_eq!(category_for(Button::LeftParen), Category::TopRow);
    assert_eq!(category_for(Button::RightParen), Category::TopRow);
    assert_eq!(category_for(Button::MemRecall), Category::TopRow);
    assert_eq!(category_for(Button::ToggleAngleMode), Category::TopRow);
}

#[test]
fn the_two_keys_that_take_something_away_share_a_slot() {
    // `AC`/`C` and backspace are a group of their own so a theme can
    // mark them; the shipped ones paint the group exactly as the top
    // row, so nothing on screen has moved for the split.
    assert_eq!(category_for(Button::Clear), Category::Delete);
    assert_eq!(category_for(Button::Backspace), Category::Delete);
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
fn category_colors_read_expected_slot() {
    use crate::theme::ThemeKind;
    let t = ThemeKind::Cosmic.get();
    assert_eq!(Category::Number.colors(&t), t.number);
    assert_eq!(Category::BasicOp.colors(&t), t.basicop);
    assert_eq!(Category::Equals.colors(&t), t.equals);
    assert_eq!(Category::Delete.colors(&t), t.delete);
}
