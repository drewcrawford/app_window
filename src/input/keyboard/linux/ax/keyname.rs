// SPDX-License-Identifier: MPL-2.0

use crate::input::keyboard::key::KeyboardKey;

pub fn key_to_name(key: KeyboardKey, is_numlock_enabled: bool) -> &'static str {
    match key {
        // Letters - map to lowercase versions
        KeyboardKey::A => "a",
        KeyboardKey::B => "b",
        KeyboardKey::C => "c",
        KeyboardKey::D => "d",
        KeyboardKey::E => "e",
        KeyboardKey::F => "f",
        KeyboardKey::G => "g",
        KeyboardKey::H => "h",
        KeyboardKey::I => "i",
        KeyboardKey::J => "j",
        KeyboardKey::K => "k",
        KeyboardKey::L => "l",
        KeyboardKey::M => "m",
        KeyboardKey::N => "n",
        KeyboardKey::O => "o",
        KeyboardKey::P => "p",
        KeyboardKey::Q => "q",
        KeyboardKey::R => "r",
        KeyboardKey::S => "s",
        KeyboardKey::T => "t",
        KeyboardKey::U => "u",
        KeyboardKey::V => "v",
        KeyboardKey::W => "w",
        KeyboardKey::X => "x",
        KeyboardKey::Y => "y",
        KeyboardKey::Z => "z",

        // Numbers
        KeyboardKey::Num0 => "0",
        KeyboardKey::Num1 => "1",
        KeyboardKey::Num2 => "2",
        KeyboardKey::Num3 => "3",
        KeyboardKey::Num4 => "4",
        KeyboardKey::Num5 => "5",
        KeyboardKey::Num6 => "6",
        KeyboardKey::Num7 => "7",
        KeyboardKey::Num8 => "8",
        KeyboardKey::Num9 => "9",

        // Special characters
        KeyboardKey::Space => "space",
        KeyboardKey::Minus => "-",
        KeyboardKey::Equal => "=",
        KeyboardKey::LeftBracket => "[",
        KeyboardKey::RightBracket => "]",
        KeyboardKey::Backslash => "\\",
        KeyboardKey::Semicolon => ";",
        KeyboardKey::Quote => "'",
        KeyboardKey::Grave => "`",
        KeyboardKey::Comma => ",",
        KeyboardKey::Period => ".",
        KeyboardKey::Slash => "/",

        // Modifiers
        KeyboardKey::Shift => "Shift_L",
        KeyboardKey::RightShift => "Shift_R",
        KeyboardKey::Control => "Control_L",
        KeyboardKey::RightControl => "Control_R",
        KeyboardKey::Option => "Alt_L",
        KeyboardKey::RightOption => "Alt_R",
        KeyboardKey::Command => "Meta_L",
        KeyboardKey::RightCommand => "Meta_R",
        KeyboardKey::Function => "VoidSymbol", // No direct mapping
        KeyboardKey::CapsLock => "Caps_Lock",

        // Function keys
        KeyboardKey::F1 => "F1",
        KeyboardKey::F2 => "F2",
        KeyboardKey::F3 => "F3",
        KeyboardKey::F4 => "F4",
        KeyboardKey::F5 => "F5",
        KeyboardKey::F6 => "F6",
        KeyboardKey::F7 => "F7",
        KeyboardKey::F8 => "F8",
        KeyboardKey::F9 => "F9",
        KeyboardKey::F10 => "F10",
        KeyboardKey::F11 => "F11",
        KeyboardKey::F12 => "F12",
        KeyboardKey::F13 => "F13",
        KeyboardKey::F14 => "F14",
        KeyboardKey::F15 => "F15",
        KeyboardKey::F16 => "F16",
        KeyboardKey::F17 => "F17",
        KeyboardKey::F18 => "F18",
        KeyboardKey::F19 => "F19",
        KeyboardKey::F20 => "F20",
        KeyboardKey::F21 => "F21",
        KeyboardKey::F22 => "F22",
        KeyboardKey::F23 => "F23",
        KeyboardKey::F24 => "F24",

        // Navigation keys
        KeyboardKey::Return => "Return",
        KeyboardKey::Tab => "Tab",
        KeyboardKey::Delete => "BackSpace",
        KeyboardKey::ForwardDelete => "Delete",
        KeyboardKey::Escape => "Escape",
        KeyboardKey::Home => "Home",
        KeyboardKey::PageUp => "Page_Up",
        KeyboardKey::PageDown => "Page_Down",
        KeyboardKey::End => "End",
        KeyboardKey::LeftArrow => "Left",
        KeyboardKey::RightArrow => "Right",
        KeyboardKey::DownArrow => "Down",
        KeyboardKey::UpArrow => "Up",
        KeyboardKey::Help => "Help",

        // Keypad
        KeyboardKey::KeypadDecimal => {
            if is_numlock_enabled {
                "."
            } else {
                "KP_Delete"
            }
        }
        KeyboardKey::KeypadMultiply => "*",
        KeyboardKey::KeypadPlus => "+",
        KeyboardKey::KeypadClear => "Clear",
        KeyboardKey::KeypadDivide => "/",
        KeyboardKey::KeypadEnter => "KP_Enter",
        KeyboardKey::KeypadMinus => "-",
        KeyboardKey::KeypadEquals => "=",
        KeyboardKey::Keypad0 => {
            if is_numlock_enabled {
                "0"
            } else {
                "KP_Insert"
            }
        }
        KeyboardKey::Keypad1 => {
            if is_numlock_enabled {
                "1"
            } else {
                "KP_End"
            }
        }
        KeyboardKey::Keypad2 => {
            if is_numlock_enabled {
                "2"
            } else {
                "KP_Down"
            }
        }
        KeyboardKey::Keypad3 => {
            if is_numlock_enabled {
                "3"
            } else {
                "KP_Next"
            }
        } //Next is somewhat suspicious to me but matches Gnome Help
        KeyboardKey::Keypad4 => {
            if is_numlock_enabled {
                "4"
            } else {
                "KP_Left"
            }
        }
        KeyboardKey::Keypad5 => {
            if is_numlock_enabled {
                "5"
            } else {
                "KP_Begin"
            }
        }
        KeyboardKey::Keypad6 => {
            if is_numlock_enabled {
                "6"
            } else {
                "KP_Right"
            }
        }
        KeyboardKey::Keypad7 => {
            if is_numlock_enabled {
                "7"
            } else {
                "KP_Home"
            }
        }
        KeyboardKey::Keypad8 => {
            if is_numlock_enabled {
                "8"
            } else {
                "KP_Up"
            }
        }
        KeyboardKey::Keypad9 => {
            if is_numlock_enabled {
                "9"
            } else {
                "KP_Page_Up"
            }
        }

        // Lock keys
        KeyboardKey::NumLock => "Num_Lock",
        KeyboardKey::ScrollLock => "Scroll_Lock",

        // Misc keys
        KeyboardKey::PrintScreen => "Print",
        KeyboardKey::Pause => "Pause",
        KeyboardKey::Insert => "Insert",
        KeyboardKey::ContextMenu => "Menu",
        KeyboardKey::Power => "PowerOff",

        // Media keys
        KeyboardKey::VolumeUp => "AudioRaiseVolume",
        KeyboardKey::VolumeDown => "AudioLowerVolume",
        KeyboardKey::Mute => "AudioMute",
        KeyboardKey::Play => "AudioPlay",
        KeyboardKey::Stop => "AudioStop",
        KeyboardKey::PreviousTrack => "AudioPrev",
        KeyboardKey::NextTrack => "AudioNext",

        // Browser keys
        KeyboardKey::BrowserBack => "Back",
        KeyboardKey::BrowserForward => "Forward",
        KeyboardKey::BrowserRefresh => "Refresh",
        KeyboardKey::BrowserStop => "Stop",
        KeyboardKey::BrowserSearch => "Search",
        KeyboardKey::BrowserFavorites => "Favorites",
        KeyboardKey::BrowserHome => "HomePage",

        // App keys
        KeyboardKey::LaunchMail => "Mail",
        KeyboardKey::MediaSelect => "AudioMedia",
        KeyboardKey::LaunchApp1 => "Launch0",
        KeyboardKey::LaunchApp2 => "Launch1",

        // Japanese input keys
        KeyboardKey::Convert => "Henkan",
        KeyboardKey::NonConvert => "Muhenkan",
        KeyboardKey::JISKana => "Kana_Lock",
        KeyboardKey::JISEisu => "Eisu_toggle",
        KeyboardKey::JISYen => "yen",
        KeyboardKey::JISUnderscore => "underscore",
        KeyboardKey::JISKeypadComma => "KP_Separator",

        // The ISO section maps to section
        KeyboardKey::ISOSection => "section",

        // International
        KeyboardKey::InternationalBackslash => "backslash",

        // Standard editing keys
        KeyboardKey::Again => "Redo",
        KeyboardKey::Undo => "Undo",
        KeyboardKey::Cut => "Cut",
        KeyboardKey::Copy => "Copy",
        KeyboardKey::Paste => "Paste",
        KeyboardKey::Find => "Find",
        KeyboardKey::Props => "Execute",
        KeyboardKey::Select => "Select",
        KeyboardKey::Open => "Open",

        // Power/system
        KeyboardKey::WakeUp => "WakeUp",
        KeyboardKey::Eject => "Eject",
        KeyboardKey::ContextualMenu => "Menu",
    }
}
