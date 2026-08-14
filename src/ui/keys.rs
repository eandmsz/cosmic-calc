//! Keyboard capture. Uses `event::listen_with` so the keypad reacts
//! to the entire window's key stream, not just the currently focused
//! widget. The mapping is deliberately permissive: NumLock-on and
//! NumLock-off produce the same `Button` messages for the digit keys,
//! and both the top-row and keypad versions of `+`, `-`, `*`, `/`
//! are routed to the same operator buttons.
//!
//! Every unhandled key – modifiers, F-keys, navigation keys that we
//! don't bind – falls through so the cosmic compositor can still use
//! them (e.g. Alt-F4 closes the window).

use cosmic::iced::event::{self, Event};
use cosmic::iced::keyboard::key::{Code, Physical};
use cosmic::iced::keyboard::{key::Named, Event as KeyEvent, Key, Modifiers};
use cosmic::iced::window::Event as WindowEvent;
use cosmic::iced::{Size, Subscription};

use crate::clipboard::ClipboardOp;
use crate::ui::app::Message;
use crate::ui::buttons::Button;

/// Subscription wired into `AppModel::subscription`. Listens for
/// every `keyboard::Event::KeyPressed` and translates the few we
/// care about into `Message::Button(_)` or `Message::Clipboard(_)`.
/// `physical_key` is inspected alongside the logical `Key` so numpad
/// digits reach us even when NumLock is off (which otherwise routes
/// them to `ArrowLeft` / `Home` / etc.).
pub fn subscription() -> Subscription<Message> {
    event::listen_with(|event, status, _window_id| match event {
        // Skip key presses that a focused widget (e.g. the rand-bound
        // text inputs in the settings panel) has already consumed —
        // otherwise digits typed into those fields would also dispatch
        // `Button::Num(_)` and end up in the expression buffer.
        Event::Keyboard(KeyEvent::KeyPressed {
            key,
            physical_key,
            modifiers,
            ..
        }) if status == event::Status::Ignored => route_key(&key, physical_key, modifiers),
        // Match the press path's logic for releases: only act on key
        // releases the focused widget didn't claim, otherwise releasing a
        // key after typing into the rand-bound inputs would clear the
        // flash for an unrelated dispatched press.
        Event::Keyboard(KeyEvent::KeyReleased {
            key,
            physical_key,
            modifiers,
            ..
        }) if status == event::Status::Ignored => route_release(&key, physical_key, modifiers),
        // Window-resize events feed the responsive font sizing on the
        // main display. `Opened` fires once at startup so the very
        // first frame already has a real width to scale against, not
        // the placeholder default.
        Event::Window(WindowEvent::Resized(Size { width, height }))
        | Event::Window(WindowEvent::Opened {
            size: Size { width, height },
            ..
        }) => Some(Message::WindowResized(width, height)),
        _ => None,
    })
}

/// Dispatch a key press to either a `Button` press or a clipboard op.
/// Ctrl+C / Ctrl+V win before the plain-character fallback, so typing
/// `c` still routes to `Button::Cos` in the normal flow. Numpad
/// physical keys are checked last, so a user with NumLock off can
/// still reach the digit keys.
pub fn route_key(key: &Key, physical: Physical, modifiers: Modifiers) -> Option<Message> {
    if modifiers.control() {
        if let Key::Character(s) = key {
            let c = s.chars().next()?;
            match c {
                'c' | 'C' => return Some(Message::Clipboard(ClipboardOp::Copy)),
                'v' | 'V' => return Some(Message::Clipboard(ClipboardOp::Paste)),
                _ => {}
            }
        }
    }
    // Physical numpad codes win over logical key mapping. With NumLock
    // off the OS delivers Numpad4/6/etc. as ArrowLeft/Right named keys,
    // so checking the logical key first would route them to cursor moves
    // instead of digits. `map_physical` returns `None` for any non-numpad
    // physical key, so non-numpad input still falls through to `map_key`.
    if let Some(b) = map_physical(physical) {
        return Some(Message::KeyboardPressed(b));
    }
    map_key(key, modifiers).map(Message::KeyboardPressed)
}

/// Same routing as `route_key` but emits the release-side message so
/// the keypad's flash on the matching button can be cleared. Clipboard
/// shortcuts are not flashed (they have no keypad cell), so Ctrl+C/V
/// don't need a release counterpart.
pub fn route_release(key: &Key, physical: Physical, modifiers: Modifiers) -> Option<Message> {
    if let Some(b) = map_physical(physical) {
        return Some(Message::KeyboardReleased(b));
    }
    map_key(key, modifiers).map(Message::KeyboardReleased)
}

/// Pure translator from iced's `Key` + `Modifiers` to a `Button`.
/// Exposed separately so unit tests can exercise it without running
/// an iced event loop.
pub fn map_key(key: &Key, modifiers: Modifiers) -> Option<Button> {
    match key {
        Key::Character(s) => {
            let c = s.chars().next()?;
            map_char(c, modifiers)
        }
        Key::Named(n) => map_named(*n, modifiers),
        Key::Unidentified => None,
    }
}

pub(crate) fn map_char(c: char, modifiers: Modifiers) -> Option<Button> {
    // Shift combinations that land on a printable char first: we let
    // them through so '+' coming from Shift+= still routes to Add.
    match c {
        '0' => Some(Button::Num(0)),
        '1' => Some(Button::Num(1)),
        '2' => Some(Button::Num(2)),
        '3' => Some(Button::Num(3)),
        '4' => Some(Button::Num(4)),
        '5' => Some(Button::Num(5)),
        '6' => Some(Button::Num(6)),
        '7' => Some(Button::Num(7)),
        '8' => Some(Button::Num(8)),
        '9' => Some(Button::Num(9)),
        '.' | ',' => Some(Button::Decimal),
        '+' => Some(Button::Add),
        '-' => Some(Button::Sub),
        '*' | '×' => Some(Button::Mul),
        '/' | '÷' => Some(Button::Div),
        '^' => Some(Button::Pow),
        '%' => Some(Button::Percent),
        '!' => Some(Button::Factorial),
        '(' => Some(Button::LeftParen),
        ')' => Some(Button::RightParen),
        '=' => Some(Button::Equals),
        'π' => Some(Button::Pi),
        '𝑒' => Some(Button::Euler),
        'p' | 'P' if !modifiers.control() => Some(Button::Pi),
        'e' | 'E' if !modifiers.control() => Some(Button::Euler),
        'r' | 'R' if !modifiers.control() => Some(Button::Rand),
        's' | 'S' if !modifiers.control() => Some(Button::Second),
        _ => None,
    }
}

pub(crate) fn map_named(named: Named, _modifiers: Modifiers) -> Option<Button> {
    match named {
        Named::Enter => Some(Button::Equals),
        Named::Backspace => Some(Button::Backspace),
        Named::Delete => Some(Button::Backspace),
        Named::Escape => Some(Button::Clear),
        Named::ArrowLeft => Some(Button::CursorLeft),
        Named::ArrowRight => Some(Button::CursorRight),
        Named::Home => Some(Button::CursorHome),
        Named::End => Some(Button::CursorEnd),
        _ => None,
    }
}

/// Translate a physical-key code into a `Button`. This is the path
/// that rescues numpad keys when NumLock is off, and also catches the
/// numpad operators that winit doesn't always emit as plain characters.
fn map_physical(physical: Physical) -> Option<Button> {
    let code = match physical {
        Physical::Code(c) => c,
        Physical::Unidentified(_) => return None,
    };
    Some(match code {
        Code::Numpad0 => Button::Num(0),
        Code::Numpad1 => Button::Num(1),
        Code::Numpad2 => Button::Num(2),
        Code::Numpad3 => Button::Num(3),
        Code::Numpad4 => Button::Num(4),
        Code::Numpad5 => Button::Num(5),
        Code::Numpad6 => Button::Num(6),
        Code::Numpad7 => Button::Num(7),
        Code::Numpad8 => Button::Num(8),
        Code::Numpad9 => Button::Num(9),
        Code::NumpadAdd => Button::Add,
        Code::NumpadSubtract => Button::Sub,
        Code::NumpadMultiply => Button::Mul,
        Code::NumpadDivide => Button::Div,
        Code::NumpadDecimal | Code::NumpadComma => Button::Decimal,
        Code::NumpadEnter | Code::NumpadEqual => Button::Equals,
        Code::NumpadBackspace => Button::Backspace,
        Code::NumpadClear | Code::NumpadClearEntry => Button::Clear,
        Code::NumpadParenLeft => Button::LeftParen,
        Code::NumpadParenRight => Button::RightParen,
        _ => return None,
    })
}
