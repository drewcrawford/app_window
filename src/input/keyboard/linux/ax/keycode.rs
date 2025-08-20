// SPDX-License-Identifier: MPL-2.0

use crate::input::keyboard::key::KeyboardKey;

pub fn key_to_x11(key: KeyboardKey) -> i32 {
    match key {
        // Letters
        KeyboardKey::Q => 24,
        KeyboardKey::W => 25,
        KeyboardKey::E => 26,
        KeyboardKey::R => 27,
        KeyboardKey::T => 28,
        KeyboardKey::Y => 29,
        KeyboardKey::U => 30,
        KeyboardKey::I => 31,
        KeyboardKey::O => 32,
        KeyboardKey::P => 33,
        KeyboardKey::A => 38,
        KeyboardKey::S => 39,
        KeyboardKey::D => 40,
        KeyboardKey::F => 41,
        KeyboardKey::G => 42,
        KeyboardKey::H => 43,
        KeyboardKey::J => 44,
        KeyboardKey::K => 45,
        KeyboardKey::L => 46,
        KeyboardKey::Z => 52,
        KeyboardKey::X => 53,
        KeyboardKey::C => 54,
        KeyboardKey::V => 55,
        KeyboardKey::B => 56,
        KeyboardKey::N => 57,
        KeyboardKey::M => 58,

        // Numbers across the top
        KeyboardKey::Num1 => 10,
        KeyboardKey::Num2 => 11,
        KeyboardKey::Num3 => 12,
        KeyboardKey::Num4 => 13,
        KeyboardKey::Num5 => 14,
        KeyboardKey::Num6 => 15,
        KeyboardKey::Num7 => 16,
        KeyboardKey::Num8 => 17,
        KeyboardKey::Num9 => 18,
        KeyboardKey::Num0 => 19,

        // Function keys
        KeyboardKey::F1 => 67,
        KeyboardKey::F2 => 68,
        KeyboardKey::F3 => 69,
        KeyboardKey::F4 => 70,
        KeyboardKey::F5 => 71,
        KeyboardKey::F6 => 72,
        KeyboardKey::F7 => 73,
        KeyboardKey::F8 => 74,
        KeyboardKey::F9 => 75,
        KeyboardKey::F10 => 76,
        KeyboardKey::F11 => 95,
        KeyboardKey::F12 => 96,
        KeyboardKey::F13 => 191, // Not exact, using alternative mapping
        KeyboardKey::F14 => 192,
        KeyboardKey::F15 => 193,
        KeyboardKey::F16 => 194,
        KeyboardKey::F17 => 195,
        KeyboardKey::F18 => 196,
        KeyboardKey::F19 => 197,
        KeyboardKey::F20 => 198,
        KeyboardKey::F21 => 199,
        KeyboardKey::F22 => 200,
        KeyboardKey::F23 => 201,
        KeyboardKey::F24 => 202,

        // Special characters
        KeyboardKey::Minus => 20,
        KeyboardKey::Equal => 21,
        KeyboardKey::LeftBracket => 34,
        KeyboardKey::RightBracket => 35,
        KeyboardKey::Semicolon => 47,
        KeyboardKey::Quote => 48,
        KeyboardKey::Grave => 49,
        KeyboardKey::Backslash => 51,
        KeyboardKey::Comma => 59,
        KeyboardKey::Period => 60,
        KeyboardKey::Slash => 61,

        // Modifiers
        KeyboardKey::Shift => 50,
        KeyboardKey::RightShift => 62,
        KeyboardKey::Control => 37,
        KeyboardKey::RightControl => 105,
        KeyboardKey::Option => 64,        // Alt_L
        KeyboardKey::RightOption => 108,  // Alt_R
        KeyboardKey::Command => 133,      // Super_L
        KeyboardKey::RightCommand => 134, // Super_R
        KeyboardKey::Function => 135,     // Menu as fallback
        KeyboardKey::CapsLock => 66,

        // Navigation
        KeyboardKey::Return => 36,
        KeyboardKey::Tab => 23,
        KeyboardKey::Space => 65,
        KeyboardKey::Delete => 22,         // BackSpace
        KeyboardKey::ForwardDelete => 119, // Delete
        KeyboardKey::Escape => 9,
        KeyboardKey::Home => 110,
        KeyboardKey::PageUp => 112,
        KeyboardKey::PageDown => 117,
        KeyboardKey::End => 115,
        KeyboardKey::LeftArrow => 113,
        KeyboardKey::RightArrow => 114,
        KeyboardKey::DownArrow => 116,
        KeyboardKey::UpArrow => 111,

        // Keypad
        KeyboardKey::KeypadDecimal => 91,
        KeyboardKey::KeypadMultiply => 63,
        KeyboardKey::KeypadPlus => 86,
        KeyboardKey::KeypadClear => 91, // Using KP_Delete as equivalent
        KeyboardKey::KeypadDivide => 106,
        KeyboardKey::KeypadEnter => 104,
        KeyboardKey::KeypadMinus => 82,
        KeyboardKey::KeypadEquals => 125,
        KeyboardKey::Keypad0 => 90,
        KeyboardKey::Keypad1 => 87,
        KeyboardKey::Keypad2 => 88,
        KeyboardKey::Keypad3 => 89,
        KeyboardKey::Keypad4 => 83,
        KeyboardKey::Keypad5 => 84,
        KeyboardKey::Keypad6 => 85,
        KeyboardKey::Keypad7 => 79,
        KeyboardKey::Keypad8 => 80,
        KeyboardKey::Keypad9 => 81,

        // Lock keys
        KeyboardKey::NumLock => 77,
        KeyboardKey::ScrollLock => 78,

        // Media keys
        KeyboardKey::VolumeUp => 123,
        KeyboardKey::VolumeDown => 122,
        KeyboardKey::Mute => 121,
        KeyboardKey::Play => 172,
        KeyboardKey::Stop => 174,
        KeyboardKey::PreviousTrack => 173,
        KeyboardKey::NextTrack => 171,

        // System keys
        KeyboardKey::PrintScreen => 107,
        KeyboardKey::Pause => 127,
        KeyboardKey::Insert => 118,
        KeyboardKey::Power => 124,
        KeyboardKey::Eject => 169,

        // Browser/App keys
        KeyboardKey::BrowserBack => 166,
        KeyboardKey::BrowserForward => 167,
        KeyboardKey::BrowserRefresh => 181,
        KeyboardKey::BrowserStop => 174, // Using AudioStop as alternative
        KeyboardKey::BrowserSearch => 225,
        KeyboardKey::BrowserFavorites => 164,
        KeyboardKey::BrowserHome => 180,
        KeyboardKey::LaunchMail => 163,
        KeyboardKey::MediaSelect => 234,
        KeyboardKey::LaunchApp1 => 156,
        KeyboardKey::LaunchApp2 => 157,

        // Japanese input
        KeyboardKey::Convert => 100,      // Henkan_Mode
        KeyboardKey::NonConvert => 102,   // Muhenkan
        KeyboardKey::JISKana => 101,      // Using Hiragana_Katakana
        KeyboardKey::JISEisu => 98,       // Using Katakana as alternative
        KeyboardKey::JISYen => 94,        // Using less/greater as alternative
        KeyboardKey::JISUnderscore => 20, // Using minus as alternative
        KeyboardKey::JISKeypadComma => 129,

        // Extra keys
        KeyboardKey::Help => 146,
        KeyboardKey::Again => 137,
        KeyboardKey::Undo => 139,
        KeyboardKey::Cut => 145,
        KeyboardKey::Copy => 141,
        KeyboardKey::Paste => 143,
        KeyboardKey::Find => 144,
        KeyboardKey::Props => 138, // SunProps
        KeyboardKey::Select => 0,  // No direct mapping
        KeyboardKey::Open => 142,
        KeyboardKey::ContextMenu => 135,    // Menu
        KeyboardKey::ContextualMenu => 135, // Menu
        KeyboardKey::WakeUp => 151,

        // International
        KeyboardKey::ISOSection => 94, // Using less/greater as alternative
        KeyboardKey::InternationalBackslash => 94, // Using less/greater
    }
}
