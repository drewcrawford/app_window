// SPDX-License-Identifier: MPL-2.0
use crate::input::keyboard::Shared;
use crate::input::keyboard::key::KeyboardKey;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_ADD, VK_APPS, VK_BACK, VK_BROWSER_BACK, VK_BROWSER_FAVORITES, VK_BROWSER_FORWARD,
    VK_BROWSER_HOME, VK_BROWSER_REFRESH, VK_BROWSER_SEARCH, VK_BROWSER_STOP, VK_CAPITAL, VK_CLEAR,
    VK_CONTROL, VK_CONVERT, VK_DECIMAL, VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1,
    VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14,
    VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_HELP,
    VK_HOME, VK_INSERT, VK_KANA, VK_LAUNCH_APP1, VK_LAUNCH_APP2, VK_LAUNCH_MAIL, VK_LCONTROL,
    VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
    VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_NONCONVERT, VK_NUMLOCK,
    VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7,
    VK_NUMPAD8, VK_NUMPAD9, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7,
    VK_OEM_102, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PAUSE, VK_PLAY,
    VK_PRINT, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL,
    VK_SELECT, VK_SEPARATOR, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB, VK_UP,
    VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, IDC_ARROW,
    LoadCursorW, MSG, RegisterClassExW, SW_SHOWNORMAL, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WM_KEYDOWN, WM_KEYUP, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, w};

struct KeyboardState {
    shared: Vec<Weak<Shared>>,
}
static KEYBOARD_STATE: OnceLock<Mutex<KeyboardState>> = OnceLock::new();

impl KeyboardState {
    fn new() -> Self {
        KeyboardState { shared: Vec::new() }
    }

    fn apply_all<F: Fn(&Shared)>(&mut self, f: F) {
        self.shared.retain(|shared| {
            if let Some(shared) = shared.upgrade() {
                f(&shared);
                true
            } else {
                false
            }
        });
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(super) struct PlatformCoalescedKeyboard {}
impl PlatformCoalescedKeyboard {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        KEYBOARD_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .shared
            .push(Arc::downgrade(shared));
        PlatformCoalescedKeyboard {}
    }
}

/**
Processes window key events.

Returns LResult(0) if we handled the message, or nonzero otherwise.
*/
pub fn kbd_window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, _l_param: LPARAM) -> LRESULT {
    let window_ptr = hwnd.0;
    match msg {
        m if m == WM_KEYDOWN => {
            if let Some(key) = KeyboardKey::from_vk(w_param.0) {
                KEYBOARD_STATE
                    .get_or_init(Mutex::default)
                    .lock()
                    .unwrap()
                    .apply_all(|shared| {
                        shared.set_key_state(key, true, window_ptr);
                    });
                LRESULT(0)
            } else {
                logwise::warn_sync!("Unknown key {key}", key = w_param.0);
                LRESULT(1)
            }
        }
        m if m == WM_KEYUP => {
            if let Some(key) = KeyboardKey::from_vk(w_param.0) {
                KEYBOARD_STATE
                    .get_or_init(Mutex::default)
                    .lock()
                    .unwrap()
                    .apply_all(|shared| {
                        shared.set_key_state(key, false, window_ptr);
                    });
                LRESULT(0)
            } else {
                logwise::warn_sync!("Unknown key {key}", key = w_param.0);
                LRESULT(1)
            }
        }
        _ => LRESULT(1),
    }
}

extern "system" fn debug_window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    logwise::debuginternal_sync!("got msg hwnd {hwnd} msg {msg} w_param {w_param} l_param {l_param}", hwnd = logwise::privacy::LogIt(&hwnd),
        msg = msg,
        w_param = logwise::privacy::LogIt(&w_param),
        l_param = logwise::privacy::LogIt(&l_param));
    if crate::input::window_proc(hwnd, msg, w_param, l_param) == LRESULT(0) {
        LRESULT(0)
    } else {
        unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
    }
}

pub fn debug_window_show() {
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("Can't get module");
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.expect("Can't load cursor");

    let class_name = w!("raw_input_debug_window");
    let window_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: Default::default(),
        lpfnWndProc: Some(debug_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance.into(),
        hIcon: Default::default(),
        hCursor: cursor,
        hbrBackground: HBRUSH(COLOR_WINDOW.0 as usize as *mut c_void),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: Default::default(),
    };
    let r = unsafe { RegisterClassExW(&window_class) };
    assert_ne!(r, 0, "failed to register window class: {:?}", unsafe {
        GetLastError()
    });

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0), //style
            class_name,
            w!("raw input debug window"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT, //position
            800,
            600,  //size
            None, //parent
            None, //menu
            None, //instance
            None,
        )
    }
    .expect("failed to create window");

    unsafe { _ = ShowWindow(window, SW_SHOWNORMAL) };

    // Message loop
    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, Some(window), 0, 0).into() } {
        _ = unsafe { TranslateMessage(&msg) };
        unsafe { DispatchMessageW(&msg) };
    }
}
pub fn debug_window_hide() {
    todo!()
}

impl KeyboardKey {
    fn from_vk(vk: usize) -> Option<Self> {
        match vk {
            v if v == VK_BACK.0 as usize => Some(KeyboardKey::Delete),
            v if v == VK_TAB.0 as usize => Some(KeyboardKey::Tab),
            v if v == VK_CLEAR.0 as usize => Some(KeyboardKey::Keypad5),
            v if v == VK_RETURN.0 as usize => Some(KeyboardKey::Return),
            v if v == VK_SHIFT.0 as usize => Some(KeyboardKey::Shift),
            v if v == VK_CONTROL.0 as usize => Some(KeyboardKey::Control),
            v if v == VK_MENU.0 as usize => Some(KeyboardKey::Option),
            v if v == VK_PAUSE.0 as usize => Some(KeyboardKey::Pause),
            v if v == VK_CAPITAL.0 as usize => Some(KeyboardKey::CapsLock),
            v if v == VK_KANA.0 as usize => Some(KeyboardKey::JISKana),
            //vk_hangul, vk_ime_on, vk_junja, vk_final, vk_hanja, vk_kanji
            v if v == VK_ESCAPE.0 as usize => Some(KeyboardKey::Escape),
            v if v == VK_CONVERT.0 as usize => Some(KeyboardKey::Convert),
            v if v == VK_NONCONVERT.0 as usize => Some(KeyboardKey::NonConvert),
            //vk_accept, vk_modechange
            v if v == VK_SPACE.0 as usize => Some(KeyboardKey::Space),
            v if v == VK_PRIOR.0 as usize => Some(KeyboardKey::PageUp),
            v if v == VK_NEXT.0 as usize => Some(KeyboardKey::PageDown),
            v if v == VK_END.0 as usize => Some(KeyboardKey::End),
            v if v == VK_HOME.0 as usize => Some(KeyboardKey::Home),
            v if v == VK_LEFT.0 as usize => Some(KeyboardKey::LeftArrow),
            v if v == VK_UP.0 as usize => Some(KeyboardKey::UpArrow),
            v if v == VK_RIGHT.0 as usize => Some(KeyboardKey::RightArrow),
            v if v == VK_DOWN.0 as usize => Some(KeyboardKey::DownArrow),
            v if v == VK_SELECT.0 as usize => Some(KeyboardKey::Select),
            v if v == VK_PRINT.0 as usize => Some(KeyboardKey::KeypadMultiply),
            //vk_execute?
            v if v == VK_SNAPSHOT.0 as usize => Some(KeyboardKey::PrintScreen),
            v if v == VK_INSERT.0 as usize => Some(KeyboardKey::Insert),
            v if v == VK_DELETE.0 as usize => Some(KeyboardKey::Delete),
            v if v == VK_HELP.0 as usize => Some(KeyboardKey::Help),
            0x30 => Some(KeyboardKey::Num0),
            0x31 => Some(KeyboardKey::Num1),
            0x32 => Some(KeyboardKey::Num2),
            0x33 => Some(KeyboardKey::Num3),
            0x34 => Some(KeyboardKey::Num4),
            0x35 => Some(KeyboardKey::Num5),
            0x36 => Some(KeyboardKey::Num6),
            0x37 => Some(KeyboardKey::Num7),
            0x38 => Some(KeyboardKey::Num8),
            0x39 => Some(KeyboardKey::Num9),
            0x41 => Some(KeyboardKey::A),
            0x42 => Some(KeyboardKey::B),
            0x43 => Some(KeyboardKey::C),
            0x44 => Some(KeyboardKey::D),
            0x45 => Some(KeyboardKey::E),
            0x46 => Some(KeyboardKey::F),
            0x47 => Some(KeyboardKey::G),
            0x48 => Some(KeyboardKey::H),
            0x49 => Some(KeyboardKey::I),
            0x4A => Some(KeyboardKey::J),
            0x4B => Some(KeyboardKey::K),
            0x4C => Some(KeyboardKey::L),
            0x4D => Some(KeyboardKey::M),
            0x4E => Some(KeyboardKey::N),
            0x4F => Some(KeyboardKey::O),
            0x50 => Some(KeyboardKey::P),
            0x51 => Some(KeyboardKey::Q),
            0x52 => Some(KeyboardKey::R),
            0x53 => Some(KeyboardKey::S),
            0x54 => Some(KeyboardKey::T),
            0x55 => Some(KeyboardKey::U),
            0x56 => Some(KeyboardKey::V),
            0x57 => Some(KeyboardKey::W),
            0x58 => Some(KeyboardKey::X),
            0x59 => Some(KeyboardKey::Y),
            0x5A => Some(KeyboardKey::Z),
            v if v == VK_LWIN.0 as usize => Some(KeyboardKey::Command),
            v if v == VK_RWIN.0 as usize => Some(KeyboardKey::RightCommand),
            v if v == VK_APPS.0 as usize => Some(KeyboardKey::Function),
            //vk_sleep?
            v if v == VK_NUMPAD0.0 as usize => Some(KeyboardKey::Keypad0),
            v if v == VK_NUMPAD1.0 as usize => Some(KeyboardKey::Keypad1),
            v if v == VK_NUMPAD2.0 as usize => Some(KeyboardKey::Keypad2),
            v if v == VK_NUMPAD3.0 as usize => Some(KeyboardKey::Keypad3),
            v if v == VK_NUMPAD4.0 as usize => Some(KeyboardKey::Keypad4),
            v if v == VK_NUMPAD5.0 as usize => Some(KeyboardKey::Keypad5),
            v if v == VK_NUMPAD6.0 as usize => Some(KeyboardKey::Keypad6),
            v if v == VK_NUMPAD7.0 as usize => Some(KeyboardKey::Keypad7),
            v if v == VK_NUMPAD8.0 as usize => Some(KeyboardKey::Keypad8),
            v if v == VK_NUMPAD9.0 as usize => Some(KeyboardKey::Keypad9),
            v if v == VK_MULTIPLY.0 as usize => Some(KeyboardKey::KeypadMultiply),
            v if v == VK_ADD.0 as usize => Some(KeyboardKey::KeypadPlus),
            v if v == VK_SEPARATOR.0 as usize => Some(KeyboardKey::JISKeypadComma),
            v if v == VK_SUBTRACT.0 as usize => Some(KeyboardKey::KeypadMinus),
            v if v == VK_DECIMAL.0 as usize => Some(KeyboardKey::KeypadDecimal),
            v if v == VK_DIVIDE.0 as usize => Some(KeyboardKey::KeypadDivide),
            v if v == VK_F1.0 as usize => Some(KeyboardKey::F1),
            v if v == VK_F2.0 as usize => Some(KeyboardKey::F2),
            v if v == VK_F3.0 as usize => Some(KeyboardKey::F3),
            v if v == VK_F4.0 as usize => Some(KeyboardKey::F4),
            v if v == VK_F5.0 as usize => Some(KeyboardKey::F5),
            v if v == VK_F6.0 as usize => Some(KeyboardKey::F6),
            v if v == VK_F7.0 as usize => Some(KeyboardKey::F7),
            v if v == VK_F8.0 as usize => Some(KeyboardKey::F8),
            v if v == VK_F9.0 as usize => Some(KeyboardKey::F9),
            v if v == VK_F10.0 as usize => Some(KeyboardKey::F10),
            v if v == VK_F11.0 as usize => Some(KeyboardKey::F11),
            v if v == VK_F12.0 as usize => Some(KeyboardKey::F12),
            v if v == VK_F13.0 as usize => Some(KeyboardKey::F13),
            v if v == VK_F14.0 as usize => Some(KeyboardKey::F14),
            v if v == VK_F15.0 as usize => Some(KeyboardKey::F15),
            v if v == VK_F16.0 as usize => Some(KeyboardKey::F16),
            v if v == VK_F17.0 as usize => Some(KeyboardKey::F17),
            v if v == VK_F18.0 as usize => Some(KeyboardKey::F18),
            v if v == VK_F19.0 as usize => Some(KeyboardKey::F19),
            v if v == VK_F20.0 as usize => Some(KeyboardKey::F20),
            v if v == VK_F21.0 as usize => Some(KeyboardKey::F21),
            v if v == VK_F22.0 as usize => Some(KeyboardKey::F22),
            v if v == VK_F23.0 as usize => Some(KeyboardKey::F23),
            v if v == VK_F24.0 as usize => Some(KeyboardKey::F24),
            v if v == VK_NUMLOCK.0 as usize => Some(KeyboardKey::NumLock),
            v if v == VK_SCROLL.0 as usize => Some(KeyboardKey::ScrollLock),
            v if v == VK_LSHIFT.0 as usize => Some(KeyboardKey::Shift),
            v if v == VK_RSHIFT.0 as usize => Some(KeyboardKey::RightShift),
            v if v == VK_LCONTROL.0 as usize => Some(KeyboardKey::Control),
            v if v == VK_RCONTROL.0 as usize => Some(KeyboardKey::RightControl),
            v if v == VK_LMENU.0 as usize => Some(KeyboardKey::Option),
            v if v == VK_RMENU.0 as usize => Some(KeyboardKey::RightOption),
            v if v == VK_BROWSER_BACK.0 as usize => Some(KeyboardKey::BrowserBack),
            v if v == VK_BROWSER_FORWARD.0 as usize => Some(KeyboardKey::BrowserForward),
            v if v == VK_BROWSER_REFRESH.0 as usize => Some(KeyboardKey::BrowserRefresh),
            v if v == VK_BROWSER_STOP.0 as usize => Some(KeyboardKey::BrowserStop),
            v if v == VK_BROWSER_SEARCH.0 as usize => Some(KeyboardKey::BrowserSearch),
            v if v == VK_BROWSER_FAVORITES.0 as usize => Some(KeyboardKey::BrowserFavorites),
            v if v == VK_BROWSER_HOME.0 as usize => Some(KeyboardKey::BrowserHome),
            v if v == VK_VOLUME_MUTE.0 as usize => Some(KeyboardKey::Mute),
            v if v == VK_VOLUME_DOWN.0 as usize => Some(KeyboardKey::VolumeDown),
            v if v == VK_VOLUME_UP.0 as usize => Some(KeyboardKey::VolumeUp),
            v if v == VK_MEDIA_NEXT_TRACK.0 as usize => Some(KeyboardKey::NextTrack),
            v if v == VK_MEDIA_PREV_TRACK.0 as usize => Some(KeyboardKey::PreviousTrack),
            v if v == VK_MEDIA_STOP.0 as usize => Some(KeyboardKey::Stop),
            v if v == VK_MEDIA_PLAY_PAUSE.0 as usize => Some(KeyboardKey::Play),
            v if v == VK_LAUNCH_MAIL.0 as usize => Some(KeyboardKey::LaunchMail),
            v if v == VK_LAUNCH_APP1.0 as usize => Some(KeyboardKey::LaunchApp1),
            v if v == VK_LAUNCH_APP2.0 as usize => Some(KeyboardKey::LaunchApp2),
            v if v == VK_OEM_1.0 as usize => Some(KeyboardKey::Semicolon),
            v if v == VK_OEM_PLUS.0 as usize => Some(KeyboardKey::Equal),
            v if v == VK_OEM_COMMA.0 as usize => Some(KeyboardKey::Comma),
            v if v == VK_OEM_MINUS.0 as usize => Some(KeyboardKey::Minus),
            v if v == VK_OEM_PERIOD.0 as usize => Some(KeyboardKey::Period),
            v if v == VK_OEM_2.0 as usize => Some(KeyboardKey::Slash),
            v if VK_OEM_3.0 as usize == v => Some(KeyboardKey::Grave),
            v if VK_OEM_4.0 as usize == v => Some(KeyboardKey::LeftBracket),
            v if VK_OEM_5.0 as usize == v => Some(KeyboardKey::Backslash),
            v if VK_OEM_6.0 as usize == v => Some(KeyboardKey::RightBracket),
            v if VK_OEM_7.0 as usize == v => Some(KeyboardKey::Quote),
            v if VK_OEM_102.0 as usize == v => Some(KeyboardKey::Comma),
            //vk_processkey?
            //vk_packet?
            //vk_attn, crsel, excel, erase eof,
            v if VK_PLAY.0 as usize == v => Some(KeyboardKey::Play),
            //zoom? noname, pa1, clear
            _ => None,
        }
    }
}
