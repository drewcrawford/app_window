// SPDX-License-Identifier: MPL-2.0

use crate::input::keyboard::key::KeyboardKey;

pub fn key_to_id(key: KeyboardKey, is_numlock_enabled: bool) -> i32 {
    match key {
        // Alphabet keys
        KeyboardKey::A => 0x0061, // XK_a
        KeyboardKey::B => 0x0062, // XK_b
        KeyboardKey::C => 0x0063, // XK_c
        KeyboardKey::D => 0x0064, // XK_d
        KeyboardKey::E => 0x0065, // XK_e
        KeyboardKey::F => 0x0066, // XK_f
        KeyboardKey::G => 0x0067, // XK_g
        KeyboardKey::H => 0x0068, // XK_h
        KeyboardKey::I => 0x0069, // XK_i
        KeyboardKey::J => 0x006a, // XK_j
        KeyboardKey::K => 0x006b, // XK_k
        KeyboardKey::L => 0x006c, // XK_l
        KeyboardKey::M => 0x006d, // XK_m
        KeyboardKey::N => 0x006e, // XK_n
        KeyboardKey::O => 0x006f, // XK_o
        KeyboardKey::P => 0x0070, // XK_p
        KeyboardKey::Q => 0x0071, // XK_q
        KeyboardKey::R => 0x0072, // XK_r
        KeyboardKey::S => 0x0073, // XK_s
        KeyboardKey::T => 0x0074, // XK_t
        KeyboardKey::U => 0x0075, // XK_u
        KeyboardKey::V => 0x0076, // XK_v
        KeyboardKey::W => 0x0077, // XK_w
        KeyboardKey::X => 0x0078, // XK_x
        KeyboardKey::Y => 0x0079, // XK_y
        KeyboardKey::Z => 0x007a, // XK_z

        // Number keys
        KeyboardKey::Num0 => 0x0030, // XK_0
        KeyboardKey::Num1 => 0x0031, // XK_1
        KeyboardKey::Num2 => 0x0032, // XK_2
        KeyboardKey::Num3 => 0x0033, // XK_3
        KeyboardKey::Num4 => 0x0034, // XK_4
        KeyboardKey::Num5 => 0x0035, // XK_5
        KeyboardKey::Num6 => 0x0036, // XK_6
        KeyboardKey::Num7 => 0x0037, // XK_7
        KeyboardKey::Num8 => 0x0038, // XK_8
        KeyboardKey::Num9 => 0x0039, // XK_9

        // Keypad
        KeyboardKey::Keypad0 => {
            if is_numlock_enabled {
                0xffb0
            } else {
                0xff95
            }
        } // XK_KP_0, XK_KP_HOME
        KeyboardKey::Keypad1 => {
            if is_numlock_enabled {
                0xffb1
            } else {
                0xff9c
            }
        } // XK_KP_1, XK_KP_END
        KeyboardKey::Keypad2 => {
            if is_numlock_enabled {
                0xffb2
            } else {
                0xff99
            }
        } // XK_KP_2, XK_KP_DOWN
        KeyboardKey::Keypad3 => {
            if is_numlock_enabled {
                0xffb3
            } else {
                0xff9b
            }
        } // XK_KP_3, XK_KP_PAGE_DOWN
        KeyboardKey::Keypad4 => {
            if is_numlock_enabled {
                0xffb4
            } else {
                0xff96
            }
        } // XK_KP_4, XK_KP_LEFT
        KeyboardKey::Keypad5 => {
            if is_numlock_enabled {
                0xffb5
            } else {
                0xff9d
            }
        } // XK_KP_5, XK_KP_BEGIN
        KeyboardKey::Keypad6 => {
            if is_numlock_enabled {
                0xffb6
            } else {
                0xff98
            }
        } // XK_KP_6, XK_KP_RIGHT
        KeyboardKey::Keypad7 => {
            if is_numlock_enabled {
                0xffb7
            } else {
                0xff97
            }
        } // XK_KP_7, XK_KP_UP
        KeyboardKey::Keypad8 => {
            if is_numlock_enabled {
                0xffb8
            } else {
                0xff9a
            }
        } // XK_KP_8, XK_KP_PAGE_UP
        KeyboardKey::Keypad9 => {
            if is_numlock_enabled {
                0xffb9
            } else {
                0xff9a
            }
        } // XK_KP_9, XK_KP_PRIOR
        KeyboardKey::KeypadDecimal => {
            if is_numlock_enabled {
                0xffae
            } else {
                0xff9f
            }
        } // XK_KP_Decimal, XK_KP_Delete
        KeyboardKey::KeypadMultiply => 0xffaa, // XK_KP_Multiply
        KeyboardKey::KeypadPlus => 0xffab,     // XK_KP_Add
        KeyboardKey::KeypadClear => 0xff0b,    // XK_Clear
        KeyboardKey::KeypadDivide => 0xffaf,   // XK_KP_Divide
        KeyboardKey::KeypadEnter => 0xff8d,    // XK_KP_Enter
        KeyboardKey::KeypadMinus => 0xffad,    // XK_KP_Subtract
        KeyboardKey::KeypadEquals => 0xffbd,   // XK_KP_Equal

        // Function keys
        KeyboardKey::F1 => 0xffbe,  // XK_F1
        KeyboardKey::F2 => 0xffbf,  // XK_F2
        KeyboardKey::F3 => 0xffc0,  // XK_F3
        KeyboardKey::F4 => 0xffc1,  // XK_F4
        KeyboardKey::F5 => 0xffc2,  // XK_F5
        KeyboardKey::F6 => 0xffc3,  // XK_F6
        KeyboardKey::F7 => 0xffc4,  // XK_F7
        KeyboardKey::F8 => 0xffc5,  // XK_F8
        KeyboardKey::F9 => 0xffc6,  // XK_F9
        KeyboardKey::F10 => 0xffc7, // XK_F10
        KeyboardKey::F11 => 0xffc8, // XK_F11
        KeyboardKey::F12 => 0xffc9, // XK_F12
        KeyboardKey::F13 => 0xffca, // XK_F13
        KeyboardKey::F14 => 0xffcb, // XK_F14
        KeyboardKey::F15 => 0xffcc, // XK_F15
        KeyboardKey::F16 => 0xffcd, // XK_F16
        KeyboardKey::F17 => 0xffce, // XK_F17
        KeyboardKey::F18 => 0xffcf, // XK_F18
        KeyboardKey::F19 => 0xffd0, // XK_F19
        KeyboardKey::F20 => 0xffd1, // XK_F20
        KeyboardKey::F21 => 0xffd2, // XK_F21
        KeyboardKey::F22 => 0xffd3, // XK_F22
        KeyboardKey::F23 => 0xffd4, // XK_F23
        KeyboardKey::F24 => 0xffd5, // XK_F24

        // Special characters
        KeyboardKey::Space => 0x0020,        // XK_space
        KeyboardKey::Minus => 0x002d,        // XK_minus
        KeyboardKey::Equal => 0x003d,        // XK_equal
        KeyboardKey::LeftBracket => 0x005b,  // XK_bracketleft
        KeyboardKey::RightBracket => 0x005d, // XK_bracketright
        KeyboardKey::Backslash => 0x005c,    // XK_backslash
        KeyboardKey::Semicolon => 0x003b,    // XK_semicolon
        KeyboardKey::Quote => 0x0027,        // XK_apostrophe
        KeyboardKey::Grave => 0x0060,        // XK_grave
        KeyboardKey::Comma => 0x002c,        // XK_comma
        KeyboardKey::Period => 0x002e,       // XK_period
        KeyboardKey::Slash => 0x002f,        // XK_slash

        // Control keys
        KeyboardKey::Return => 0xff0d,        // XK_Return
        KeyboardKey::Tab => 0xff09,           // XK_Tab
        KeyboardKey::Delete => 0xff08,        // XK_backspace
        KeyboardKey::ForwardDelete => 0xffff, // XK_Delete
        KeyboardKey::Escape => 0xff1b,        // XK_Escape
        KeyboardKey::Home => 0xff50,          // XK_Home
        KeyboardKey::PageUp => 0xff55,        // XK_Page_Up
        KeyboardKey::PageDown => 0xff56,      // XK_Page_Down
        KeyboardKey::End => 0xff57,           // XK_End
        KeyboardKey::Help => 0xff6a,          // XK_Help
        KeyboardKey::LeftArrow => 0xff51,     // XK_Left
        KeyboardKey::RightArrow => 0xff53,    // XK_Right
        KeyboardKey::DownArrow => 0xff54,     // XK_Down
        KeyboardKey::UpArrow => 0xff52,       // XK_Up

        // Modifier keys
        KeyboardKey::Shift => 0xffe1,        // XK_Shift_L
        KeyboardKey::RightShift => 0xffe2,   // XK_Shift_R
        KeyboardKey::Control => 0xffe3,      // XK_Control_L
        KeyboardKey::RightControl => 0xffe4, // XK_Control_R
        KeyboardKey::Option => 0xffe9,       // XK_Alt_L
        KeyboardKey::RightOption => 0xffea,  // XK_Alt_R
        KeyboardKey::Command => 0xffeb,      // XK_Super_l
        KeyboardKey::RightCommand => 0xffec, // XK_Super_R
        KeyboardKey::Function => 0xfd1e,     // Special function key
        KeyboardKey::CapsLock => 0xffe5,     // XK_Caps_Lock

        // Media keys
        KeyboardKey::VolumeUp => 0x1008ff13, // XF86XK_AudioRaiseVolume
        KeyboardKey::VolumeDown => 0x1008ff11, // XF86XK_AudioLowerVolume
        KeyboardKey::Mute => 0x1008ff12,     // XF86XK_AudioMute
        KeyboardKey::Play => 0x1008ff14,     // XF86XK_AudioPlay
        KeyboardKey::Stop => 0x1008ff15,     // XF86XK_AudioStop
        KeyboardKey::PreviousTrack => 0x1008ff16, // XF86XK_AudioPrev
        KeyboardKey::NextTrack => 0x1008ff17, // XF86XK_AudioNext
        KeyboardKey::Eject => 0x1008ff2c,    // XF86XK_Eject

        // Additional special keys
        KeyboardKey::PrintScreen => 0xff61, // XK_Print
        KeyboardKey::ScrollLock => 0xff14,  // XK_Scroll_Lock
        KeyboardKey::Pause => 0xff13,       // XK_Pause
        KeyboardKey::Insert => 0xff63,      // XK_Insert
        KeyboardKey::NumLock => 0xff7f,     // XK_Num_Lock
        KeyboardKey::ContextMenu => 0xff67, // XK_Menu
        KeyboardKey::Power => 0x1008ff2a,   // XF86XK_PowerOff

        // Browser keys
        KeyboardKey::BrowserBack => 0x1008ff26, // XF86XK_Back
        KeyboardKey::BrowserForward => 0x1008ff27, // XF86XK_Forward
        KeyboardKey::BrowserRefresh => 0x1008ff29, // XF86XK_Refresh
        KeyboardKey::BrowserStop => 0x1008ff28, // XF86XK_Stop
        KeyboardKey::BrowserSearch => 0x1008ff1b, // XF86XK_Search
        KeyboardKey::BrowserFavorites => 0x1008ff30, // XF86XK_Favorites
        KeyboardKey::BrowserHome => 0x1008ff18, // XF86XK_HomePage

        // Application keys
        KeyboardKey::LaunchMail => 0x1008ff19,  // XF86XK_Mail
        KeyboardKey::MediaSelect => 0x1008ff32, // XF86XK_AudioMedia
        KeyboardKey::LaunchApp1 => 0x1008ff1c,  // XF86XK_Launch0
        KeyboardKey::LaunchApp2 => 0x1008ff1d,  // XF86XK_Launch1

        // International keys
        KeyboardKey::JISKana => 0xff2d,              // XK_Kana_Lock
        KeyboardKey::JISEisu => 0xff2f,              // XK_Eisu_Shift
        KeyboardKey::JISYen => 0x0a5,                // XK_yen
        KeyboardKey::JISUnderscore => 0x5f,          // XK_underscore
        KeyboardKey::JISKeypadComma => 0xffac,       // XK_KP_Separator
        KeyboardKey::ISOSection => 0xa7,             // XK_section
        KeyboardKey::InternationalBackslash => 0x5c, // XK_backslash

        // Edit keys
        KeyboardKey::Again => 0xff66,  // XK_Redo
        KeyboardKey::Undo => 0xff65,   // XK_Undo
        KeyboardKey::Cut => 0xff63,    // XK_Cut
        KeyboardKey::Copy => 0xff62,   // XK_Copy
        KeyboardKey::Paste => 0xff63,  // XK_Paste
        KeyboardKey::Find => 0xff68,   // XK_Find
        KeyboardKey::Props => 0xff62,  // XK_Execute
        KeyboardKey::Select => 0xff60, // XK_Select
        KeyboardKey::Open => 0xff62,   // XK_Execute

        // Japanese input conversion
        KeyboardKey::Convert => 0xff21,    // XK_Convert
        KeyboardKey::NonConvert => 0xff22, // XK_NonConvert

        // System
        KeyboardKey::WakeUp => 0x1008ff2b,     // XF86XK_WakeUp
        KeyboardKey::ContextualMenu => 0xff67, // XK_Menu
    }
}