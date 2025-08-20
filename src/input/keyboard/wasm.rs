// SPDX-License-Identifier: MPL-2.0
use crate::input::keyboard::Shared;
use crate::input::keyboard::key::KeyboardKey;
use std::ffi::c_void;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

#[derive(Debug)]
pub(super) struct PlatformCoalescedKeyboard {}

pub(crate) const ARBITRARY_WINDOW_PTR: *mut c_void = 0x01 as *mut c_void;

impl PlatformCoalescedKeyboard {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        let shared = shared.clone();

        crate::application::on_main_thread(
            "PlatformCoalescedKeyboard::setup".to_string(),
            move || {
                let weak = Arc::downgrade(&shared);
                let weak_up = weak.clone();
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("no document on window");
                let keydown_callback = Closure::wrap(Box::new(move |event: KeyboardEvent| {
                    let key = event.key();
                    let code = event.code();

                    if let Some(shared) = weak.upgrade() {
                        let key = KeyboardKey::from_js_code(&code)
                            .expect(format!("Unknown key: {}", key).as_str());

                        shared.set_key_state(key, true, ARBITRARY_WINDOW_PTR);
                    }
                })
                    as Box<dyn FnMut(KeyboardEvent)>);
                document
                    .add_event_listener_with_callback(
                        "keydown",
                        keydown_callback.as_ref().unchecked_ref(),
                    )
                    .expect("Can't add event listener");
                keydown_callback.forget();

                let keyup_callback = Closure::wrap(Box::new(move |event: KeyboardEvent| {
                    let key = event.key();
                    let code = event.code();
                    if let Some(shared) = weak_up.upgrade() {
                        let key = KeyboardKey::from_js_code(&code)
                            .expect(format!("Unknown key: {}", key).as_str());
                        shared.set_key_state(key, false, ARBITRARY_WINDOW_PTR);
                    }
                })
                    as Box<dyn FnMut(KeyboardEvent)>);
                document
                    .add_event_listener_with_callback(
                        "keyup",
                        keyup_callback.as_ref().unchecked_ref(),
                    )
                    .expect("Can't add event listener");
                keyup_callback.forget();

                PlatformCoalescedKeyboard {}
            },
        )
        .await
    }
}

pub fn debug_window_show() {
    //nothing?
}

pub fn debug_window_hide() {
    //also nothing?
}

impl KeyboardKey {
    fn from_js_code(js: &str) -> Option<KeyboardKey> {
        //https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values
        let key = match js {
            "Unidentified" => return None,
            "Escape" => KeyboardKey::Escape,
            "Digit1" => KeyboardKey::Num1,
            "Digit2" => KeyboardKey::Num2,
            "Digit3" => KeyboardKey::Num3,
            "Digit4" => KeyboardKey::Num4,
            "Digit5" => KeyboardKey::Num5,
            "Digit6" => KeyboardKey::Num6,
            "Digit7" => KeyboardKey::Num7,
            "Digit8" => KeyboardKey::Num8,
            "Digit9" => KeyboardKey::Num9,
            "Digit0" => KeyboardKey::Num0,
            "Minus" => KeyboardKey::Minus,
            "Equal" => KeyboardKey::Equal,
            "Backspace" => KeyboardKey::Delete,
            "Tab" => KeyboardKey::Tab,
            "KeyQ" => KeyboardKey::Q,
            "KeyW" => KeyboardKey::W,
            "KeyE" => KeyboardKey::E,
            "KeyR" => KeyboardKey::R,
            "KeyT" => KeyboardKey::T,
            "KeyY" => KeyboardKey::Y,
            "KeyU" => KeyboardKey::U,
            "KeyI" => KeyboardKey::I,
            "KeyO" => KeyboardKey::O,
            "KeyP" => KeyboardKey::P,
            "BracketLeft" => KeyboardKey::LeftBracket,
            "BracketRight" => KeyboardKey::RightBracket,
            "Enter" => KeyboardKey::Return,
            "ControlLeft" => KeyboardKey::Control,
            "KeyA" => KeyboardKey::A,
            "KeyS" => KeyboardKey::S,
            "KeyD" => KeyboardKey::D,
            "KeyF" => KeyboardKey::F,
            "KeyG" => KeyboardKey::G,
            "KeyH" => KeyboardKey::H,
            "KeyJ" => KeyboardKey::J,
            "KeyK" => KeyboardKey::K,
            "KeyL" => KeyboardKey::L,
            "Semicolon" => KeyboardKey::Semicolon,
            "Quote" => KeyboardKey::Quote,
            "Backquote" => KeyboardKey::Grave,
            "ShiftLeft" => KeyboardKey::Shift,
            "Backslash" => KeyboardKey::Backslash,
            "KeyZ" => KeyboardKey::Z,
            "KeyX" => KeyboardKey::X,
            "KeyC" => KeyboardKey::C,
            "KeyV" => KeyboardKey::V,
            "KeyB" => KeyboardKey::B,
            "KeyN" => KeyboardKey::N,
            "KeyM" => KeyboardKey::M,
            "Comma" => KeyboardKey::Comma,
            "Period" => KeyboardKey::Period,
            "Slash" => KeyboardKey::Slash,
            "ShiftRight" => KeyboardKey::RightShift,
            "NumpadMultiply" => KeyboardKey::KeypadMultiply,
            "AltLeft" => KeyboardKey::Option,
            "Space" => KeyboardKey::Space,
            "CapsLock" => KeyboardKey::CapsLock,
            "F1" => KeyboardKey::F1,
            "F2" => KeyboardKey::F2,
            "F3" => KeyboardKey::F3,
            "F4" => KeyboardKey::F4,
            "F5" => KeyboardKey::F5,
            "F6" => KeyboardKey::F6,
            "F7" => KeyboardKey::F7,
            "F8" => KeyboardKey::F8,
            "F9" => KeyboardKey::F9,
            "F10" => KeyboardKey::F10,
            "Pause" => KeyboardKey::Pause,
            "ScrollLock" => KeyboardKey::ScrollLock,
            "Numpad7" => KeyboardKey::Keypad7,
            "Numpad8" => KeyboardKey::Keypad8,
            "Numpad9" => KeyboardKey::Keypad9,
            "NumpadSubtract" => KeyboardKey::KeypadMinus,
            "Numpad4" => KeyboardKey::Keypad4,
            "Numpad5" => KeyboardKey::Keypad5,
            "Numpad6" => KeyboardKey::Keypad6,
            "NumpadAdd" => KeyboardKey::KeypadPlus,
            "Numpad1" => KeyboardKey::Keypad1,
            "Numpad2" => KeyboardKey::Keypad2,
            "Numpad3" => KeyboardKey::Keypad3,
            "Numpad0" => KeyboardKey::Keypad0,
            "NumpadDecimal" => KeyboardKey::KeypadDecimal,
            "PrintScreen" => KeyboardKey::PrintScreen,
            "IntlBackslash" => KeyboardKey::InternationalBackslash,
            "F11" => KeyboardKey::F11,
            "F12" => KeyboardKey::F12,
            "NumpadEqual" => KeyboardKey::KeypadEquals,
            "F13" => KeyboardKey::F13,
            "F14" => KeyboardKey::F14,
            "F15" => KeyboardKey::F15,
            "F16" => KeyboardKey::F16,
            "F17" => KeyboardKey::F17,
            "F18" => KeyboardKey::F18,
            "F19" => KeyboardKey::F19,
            "F20" => KeyboardKey::F20,
            "F21" => KeyboardKey::F21,
            "F22" => KeyboardKey::F22,
            "F23" => KeyboardKey::F23,
            "KanaMode" => KeyboardKey::JISKana,
            "Lang2" => KeyboardKey::JISEisu,
            "Lang1" => KeyboardKey::JISKana,
            "NumpadEnter" => KeyboardKey::KeypadEnter,
            "ControlRight" => KeyboardKey::RightControl,
            "AudioVolumeMute" => KeyboardKey::Mute,
            "AudioVolumeDown" => KeyboardKey::VolumeDown,
            "AudioVolumeUp" => KeyboardKey::VolumeUp,
            "NumpadComma" => KeyboardKey::JISKeypadComma,
            "NumpadDivide" => KeyboardKey::KeypadDivide,
            "IntlRo" => KeyboardKey::JISUnderscore,
            "F24" => KeyboardKey::F24,
            "Convert" => KeyboardKey::Convert,
            "NonConvert" => KeyboardKey::NonConvert,
            "IntlYen" => KeyboardKey::JISYen,
            "MediaTrackPrevious" => KeyboardKey::PreviousTrack,
            "MediaTrackNext" => KeyboardKey::NextTrack,
            "LaunchApp2" => KeyboardKey::LaunchApp2,
            "MediaPlayPause" => KeyboardKey::Play,
            "MediaStop" => KeyboardKey::Stop,
            "VolumeDown" => KeyboardKey::VolumeDown,
            "VolumeUp" => KeyboardKey::VolumeUp,
            "BrowserHome" => KeyboardKey::BrowserHome,
            "AltRight" => KeyboardKey::RightOption,
            "NumLock" => KeyboardKey::NumLock,
            "Home" => KeyboardKey::Home,
            "ArrowUp" => KeyboardKey::UpArrow,
            "PageUp" => KeyboardKey::PageUp,
            "ArrowLeft" => KeyboardKey::LeftArrow,
            "ArrowRight" => KeyboardKey::RightArrow,
            "End" => KeyboardKey::End,
            "ArrowDown" => KeyboardKey::DownArrow,
            "PageDown" => KeyboardKey::PageDown,
            "Insert" => KeyboardKey::Insert,
            "Delete" => KeyboardKey::Delete,
            "MetaLeft" => KeyboardKey::Command,
            "MetaRight" => KeyboardKey::RightCommand,
            "ContextMenu" => KeyboardKey::ContextMenu,
            "Power" => KeyboardKey::Power,
            "Eject" => KeyboardKey::Eject,
            "BrowserSearch" => KeyboardKey::BrowserSearch,
            "BrowserFavorites" => KeyboardKey::BrowserFavorites,
            "BrowserRefresh" => KeyboardKey::BrowserRefresh,
            "BrowserStop" => KeyboardKey::BrowserStop,
            "BrowserForward" => KeyboardKey::BrowserForward,
            "BrowserBack" => KeyboardKey::BrowserBack,
            "LaunchApp1" => KeyboardKey::LaunchApp1,
            "LaunchMail" => KeyboardKey::LaunchMail,
            "MediaSelect" => KeyboardKey::MediaSelect,
            "Help" => KeyboardKey::Help,
            "Again" => KeyboardKey::Again,
            "Props" => KeyboardKey::Props,
            "Undo" => KeyboardKey::Undo,
            "Select" => KeyboardKey::Select,
            "Copy" => KeyboardKey::Copy,
            "Open" => KeyboardKey::Open,
            "Paste" => KeyboardKey::Paste,
            "Find" => KeyboardKey::Find,
            "Cut" => KeyboardKey::Cut,
            "WakeUp" => KeyboardKey::WakeUp,
            "Fn" => KeyboardKey::Function,
            _ => return None,
        };
        Some(key)
    }
}
