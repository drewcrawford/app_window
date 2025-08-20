// SPDX-License-Identifier: MPL-2.0

use crate::input::keyboard::key::KeyboardKey;

pub fn key_to_modifier(key: KeyboardKey) -> i32 {
    match key {
        KeyboardKey::Shift | KeyboardKey::RightShift => 1 << 0,
        KeyboardKey::CapsLock => 1 << 1,
        KeyboardKey::Control | KeyboardKey::RightControl => 1 << 2,
        KeyboardKey::Option | KeyboardKey::RightOption => 1 << 3,
        //this value discovered by snooping
        KeyboardKey::Command | KeyboardKey::RightCommand => 1 << 6,
        KeyboardKey::NumLock => 1 << 14,
        _ => 0,
    }
}

pub fn key_is_text_input(key: KeyboardKey) -> bool {
    match key {
        // Letters
        KeyboardKey::A
        | KeyboardKey::B
        | KeyboardKey::C
        | KeyboardKey::D
        | KeyboardKey::E
        | KeyboardKey::F
        | KeyboardKey::G
        | KeyboardKey::H
        | KeyboardKey::I
        | KeyboardKey::J
        | KeyboardKey::K
        | KeyboardKey::L
        | KeyboardKey::M
        | KeyboardKey::N
        | KeyboardKey::O
        | KeyboardKey::P
        | KeyboardKey::Q
        | KeyboardKey::R
        | KeyboardKey::S
        | KeyboardKey::T
        | KeyboardKey::U
        | KeyboardKey::V
        | KeyboardKey::W
        | KeyboardKey::X
        | KeyboardKey::Y
        | KeyboardKey::Z => true,

        // Numbers
        KeyboardKey::Num0
        | KeyboardKey::Num1
        | KeyboardKey::Num2
        | KeyboardKey::Num3
        | KeyboardKey::Num4
        | KeyboardKey::Num5
        | KeyboardKey::Num6
        | KeyboardKey::Num7
        | KeyboardKey::Num8
        | KeyboardKey::Num9 => true,

        // Special characters
        KeyboardKey::Space
        | KeyboardKey::Minus
        | KeyboardKey::Equal
        | KeyboardKey::LeftBracket
        | KeyboardKey::RightBracket
        | KeyboardKey::Backslash
        | KeyboardKey::Semicolon
        | KeyboardKey::Quote
        | KeyboardKey::Grave
        | KeyboardKey::Comma
        | KeyboardKey::Period
        | KeyboardKey::Slash => true,

        // Keypad numbers and symbols (when NumLock is on)
        KeyboardKey::Keypad0
        | KeyboardKey::Keypad1
        | KeyboardKey::Keypad2
        | KeyboardKey::Keypad3
        | KeyboardKey::Keypad4
        | KeyboardKey::Keypad5
        | KeyboardKey::Keypad6
        | KeyboardKey::Keypad7
        | KeyboardKey::Keypad8
        | KeyboardKey::Keypad9
        | KeyboardKey::KeypadDecimal
        | KeyboardKey::KeypadMultiply
        | KeyboardKey::KeypadPlus
        | KeyboardKey::KeypadDivide
        | KeyboardKey::KeypadMinus
        | KeyboardKey::KeypadEquals => true,

        // Japanese input characters
        KeyboardKey::JISYen | KeyboardKey::JISUnderscore | KeyboardKey::JISKeypadComma => true,

        // Everything else doesn't generate text input
        _ => false,
    }
}
