use crate::ui::buttons::Button;
use crate::ui::keys::*;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::Modifiers;

#[test]
fn digits_route_to_num_buttons() {
    for d in 0..=9u8 {
        let c = (b'0' + d) as char;
        assert_eq!(map_char(c, Modifiers::default()), Some(Button::Num(d)));
    }
}

#[test]
fn operators_route_correctly() {
    let m = Modifiers::default();
    assert_eq!(map_char('+', m), Some(Button::Add));
    assert_eq!(map_char('-', m), Some(Button::Sub));
    assert_eq!(map_char('*', m), Some(Button::Mul));
    assert_eq!(map_char('×', m), Some(Button::Mul));
    assert_eq!(map_char('/', m), Some(Button::Div));
    assert_eq!(map_char('÷', m), Some(Button::Div));
}

#[test]
fn named_keys_route_correctly() {
    let m = Modifiers::default();
    assert_eq!(map_named(Named::Enter, m), Some(Button::Equals));
    assert_eq!(map_named(Named::Backspace, m), Some(Button::Backspace));
    assert_eq!(map_named(Named::Escape, m), Some(Button::Clear));
    assert_eq!(map_named(Named::ArrowLeft, m), Some(Button::CursorLeft));
}

#[test]
fn both_decimal_glyphs_route_to_decimal() {
    let m = Modifiers::default();
    assert_eq!(map_char('.', m), Some(Button::Decimal));
    assert_eq!(map_char(',', m), Some(Button::Decimal));
}
