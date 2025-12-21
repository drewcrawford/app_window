// SPDX-License-Identifier: MPL-2.0
use crate::input::keyboard::Shared;
use crate::input::keyboard::key::KeyboardKey;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::Weak;

#[derive(Debug)]
pub(super) struct PlatformCoalescedKeyboard {
    imp: *mut c_void,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn raw_input_key_notify_func(
    ctx: *mut c_void,
    window: *mut c_void,
    key_code: u16,
    down: bool,
) {
    let shared = unsafe { Weak::from_raw(ctx as *const Shared) };
    if let Some(shared) = shared.upgrade() {
        let key_code = KeyboardKey::from_code(key_code).expect("Unknown key code {key_code}");
        shared.set_key_state(key_code, down, window);
    }
    std::mem::forget(shared); //keep weak reference alive as it is still owned by the target function
}

#[unsafe(no_mangle)]
unsafe extern "C" fn raw_input_finish_event_context(ctx: *mut c_void) {
    unsafe { Weak::from_raw(ctx as *const Shared) };
}

unsafe extern "C" {
    fn PlatformCoalescedKeyboardNew(context: *const c_void) -> *mut c_void;
    fn PlatformCoalescedKeyboardFree(imp: *mut c_void);

    fn SwiftRawInputDebugWindowShow();
    fn SwiftRawInputDebugWindowHide();
}

/// Shows the debug window for raw keyboard input on macOS.
///
/// This function displays a debug window that shows real-time information about
/// keyboard input events being received by the application. This is useful for
/// debugging keyboard handling, understanding key codes, and troubleshooting
/// input-related issues.
///
/// # Platform Support
///
/// This function is only available on macOS. The debug window provides macOS-specific
/// information about keyboard events.
///
/// # Examples
///
/// ```no_run
/// # // no_run because: requires macOS-specific UI interactions that cannot be tested in doctests
/// use app_window::input::keyboard::macos::debug_window_show;
///
/// // Show the debug window to inspect keyboard events
/// debug_window_show();
/// ```
pub fn debug_window_show() {
    unsafe { SwiftRawInputDebugWindowShow() }
}

/// Hides the debug window for raw keyboard input on macOS.
///
/// This function hides the debug window that was previously shown by
/// [`debug_window_show`]. Call this when you're done debugging keyboard input
/// to remove the debug window from the screen.
///
/// # Platform Support
///
/// This function is only available on macOS.
///
/// # Examples
///
/// ```no_run
/// # // no_run because: requires macOS-specific UI interactions that cannot be tested in doctests
/// use app_window::input::keyboard::macos::{debug_window_show, debug_window_hide};
///
/// // Show the debug window
/// debug_window_show();
///
/// // ... do some debugging ...
///
/// // Hide the debug window when done
/// debug_window_hide();
/// ```
pub fn debug_window_hide() {
    unsafe { SwiftRawInputDebugWindowHide() }
}

//Swift type implements Sendable
unsafe impl Send for PlatformCoalescedKeyboard {}
unsafe impl Sync for PlatformCoalescedKeyboard {}

impl PlatformCoalescedKeyboard {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        let weak = Arc::downgrade(shared);
        let weak_raw = Weak::into_raw(weak) as *const c_void;
        PlatformCoalescedKeyboard {
            imp: unsafe { PlatformCoalescedKeyboardNew(weak_raw) },
        }
    }
}

impl Drop for PlatformCoalescedKeyboard {
    fn drop(&mut self) {
        unsafe { PlatformCoalescedKeyboardFree(self.imp) }
    }
}

//keyboard codes, HIToolbox/Events.h
impl KeyboardKey {
    /// Converts a macOS hardware key code to a `KeyboardKey`.
    ///
    /// This function maps macOS-specific hardware key codes (as defined in
    /// HIToolbox/Events.h) to the platform-independent `KeyboardKey` enum.
    /// These are the raw key codes received from NSEvent on macOS.
    ///
    /// # Arguments
    ///
    /// * `code` - A 16-bit hardware key code from macOS NSEvent
    ///
    /// # Returns
    ///
    /// Returns `Some(KeyboardKey)` if the code maps to a known key, or `None`
    /// if the code is unrecognized or unmapped.
    ///
    /// # Platform Support
    ///
    /// This function is specific to macOS and uses macOS hardware key codes.
    ///
    /// # Examples
    ///
    /// ```
    /// use app_window::input::keyboard::key::KeyboardKey;
    ///
    /// // macOS hardware code 0x00 is the 'A' key
    /// # #[cfg(target_os = "macos")]
    /// let key = KeyboardKey::from_code(0x00);
    /// # #[cfg(target_os = "macos")]
    /// assert_eq!(key, Some(KeyboardKey::A));
    ///
    /// // macOS hardware code 0x31 is the Space key
    /// # #[cfg(target_os = "macos")]
    /// let key = KeyboardKey::from_code(0x31);
    /// # #[cfg(target_os = "macos")]
    /// assert_eq!(key, Some(KeyboardKey::Space));
    ///
    /// // Unknown codes return None
    /// # #[cfg(target_os = "macos")]
    /// let key = KeyboardKey::from_code(0xFFFF);
    /// # #[cfg(target_os = "macos")]
    /// assert_eq!(key, None);
    /// ```
    pub fn from_code(code: u16) -> Option<KeyboardKey> {
        match code {
            0x00 => Some(KeyboardKey::A),
            0x01 => Some(KeyboardKey::S),
            0x02 => Some(KeyboardKey::D),
            0x03 => Some(KeyboardKey::F),
            0x04 => Some(KeyboardKey::H),
            0x05 => Some(KeyboardKey::G),
            0x06 => Some(KeyboardKey::Z),
            0x07 => Some(KeyboardKey::X),
            0x08 => Some(KeyboardKey::C),
            0x09 => Some(KeyboardKey::V),
            0x0A => Some(KeyboardKey::InternationalBackslash),
            0x0B => Some(KeyboardKey::B),
            0x0C => Some(KeyboardKey::Q),
            0x0D => Some(KeyboardKey::W),
            0x0E => Some(KeyboardKey::E),
            0x0F => Some(KeyboardKey::R),
            0x10 => Some(KeyboardKey::Y),
            0x11 => Some(KeyboardKey::T),
            0x12 => Some(KeyboardKey::Num1),
            0x13 => Some(KeyboardKey::Num2),
            0x14 => Some(KeyboardKey::Num3),
            0x15 => Some(KeyboardKey::Num4),
            0x16 => Some(KeyboardKey::Num6),
            0x17 => Some(KeyboardKey::Num5),
            0x18 => Some(KeyboardKey::Equal),
            0x19 => Some(KeyboardKey::Num9),
            0x1A => Some(KeyboardKey::Num7),
            0x1B => Some(KeyboardKey::Minus),
            0x1C => Some(KeyboardKey::Num8),
            0x1D => Some(KeyboardKey::Num0),
            0x1E => Some(KeyboardKey::RightBracket),
            0x1F => Some(KeyboardKey::O),
            0x20 => Some(KeyboardKey::U),
            0x21 => Some(KeyboardKey::LeftBracket),
            0x22 => Some(KeyboardKey::I),
            0x23 => Some(KeyboardKey::P),
            0x24 => Some(KeyboardKey::Return),
            0x25 => Some(KeyboardKey::L),
            0x26 => Some(KeyboardKey::J),
            0x27 => Some(KeyboardKey::Quote),
            0x28 => Some(KeyboardKey::K),
            0x29 => Some(KeyboardKey::Semicolon),
            0x2A => Some(KeyboardKey::Backslash),
            0x2B => Some(KeyboardKey::Comma),
            0x2C => Some(KeyboardKey::Slash),
            0x2D => Some(KeyboardKey::N),
            0x2E => Some(KeyboardKey::M),
            0x2F => Some(KeyboardKey::Period),
            0x30 => Some(KeyboardKey::Tab),
            0x31 => Some(KeyboardKey::Space),
            0x32 => Some(KeyboardKey::Grave),
            0x33 => Some(KeyboardKey::Delete),
            0x34 => Some(KeyboardKey::KeypadEnter),
            0x35 => Some(KeyboardKey::Escape),
            0x36 => Some(KeyboardKey::RightCommand),
            0x37 => Some(KeyboardKey::Command),
            0x38 => Some(KeyboardKey::Shift),
            0x39 => Some(KeyboardKey::CapsLock),
            0x3A => Some(KeyboardKey::Option),
            0x3B => Some(KeyboardKey::Control),
            0x3C => Some(KeyboardKey::RightShift),
            0x3D => Some(KeyboardKey::RightOption),
            0x3E => Some(KeyboardKey::RightControl),
            0x3F => Some(KeyboardKey::Function),
            0x40 => Some(KeyboardKey::F17),
            0x41 => Some(KeyboardKey::KeypadDecimal),
            //nobody seems to know what 42 is!
            0x43 => Some(KeyboardKey::KeypadMultiply),
            //0x44  ??
            0x45 => Some(KeyboardKey::KeypadPlus),
            //0x46 ??
            0x47 => Some(KeyboardKey::NumLock),

            0x48 => Some(KeyboardKey::VolumeUp),
            0x49 => Some(KeyboardKey::VolumeDown),
            0x4A => Some(KeyboardKey::Mute),
            0x4B => Some(KeyboardKey::KeypadDivide),
            0x4C => Some(KeyboardKey::KeypadEnter),
            //0x4d ??
            0x4E => Some(KeyboardKey::KeypadMinus),
            0x4F => Some(KeyboardKey::F18),
            0x50 => Some(KeyboardKey::F19),
            0x51 => Some(KeyboardKey::KeypadEquals),
            0x52 => Some(KeyboardKey::Keypad0),
            0x53 => Some(KeyboardKey::Keypad1),
            0x54 => Some(KeyboardKey::Keypad2),
            0x55 => Some(KeyboardKey::Keypad3),
            0x56 => Some(KeyboardKey::Keypad4),
            0x57 => Some(KeyboardKey::Keypad5),
            0x58 => Some(KeyboardKey::Keypad6),
            0x59 => Some(KeyboardKey::Keypad7),
            0x5A => Some(KeyboardKey::F20),
            0x5B => Some(KeyboardKey::Keypad8),
            0x5C => Some(KeyboardKey::Keypad9),
            0x5D => Some(KeyboardKey::JISYen),
            0x5E => Some(KeyboardKey::JISUnderscore),
            0x5F => Some(KeyboardKey::JISKeypadComma),
            0x60 => Some(KeyboardKey::F5),
            0x61 => Some(KeyboardKey::F6),
            0x62 => Some(KeyboardKey::F7),
            0x63 => Some(KeyboardKey::F3),
            0x64 => Some(KeyboardKey::F8),
            0x65 => Some(KeyboardKey::F9),
            0x66 => Some(KeyboardKey::JISEisu),
            0x67 => Some(KeyboardKey::F11),
            0x68 => Some(KeyboardKey::JISKana),
            0x69 => Some(KeyboardKey::F13),
            0x6A => Some(KeyboardKey::F16),
            0x6B => Some(KeyboardKey::F14),
            //0x6c ??
            0x6D => Some(KeyboardKey::F10),
            0x6E => Some(KeyboardKey::ContextualMenu),
            0x6F => Some(KeyboardKey::F12),
            //0x70 ??
            0x71 => Some(KeyboardKey::F15),
            0x72 => Some(KeyboardKey::Help),
            0x73 => Some(KeyboardKey::Home),
            0x74 => Some(KeyboardKey::PageUp),
            0x75 => Some(KeyboardKey::ForwardDelete),
            0x76 => Some(KeyboardKey::F4),
            0x77 => Some(KeyboardKey::End),
            0x78 => Some(KeyboardKey::F2),
            0x79 => Some(KeyboardKey::PageDown),
            0x7A => Some(KeyboardKey::F1),
            0x7B => Some(KeyboardKey::LeftArrow),
            0x7C => Some(KeyboardKey::RightArrow),
            0x7D => Some(KeyboardKey::DownArrow),
            0x7E => Some(KeyboardKey::UpArrow),
            _ => None, // Return None if the code doesn't match any key
        }
    }
}
