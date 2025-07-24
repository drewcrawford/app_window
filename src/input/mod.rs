// SPDX-License-Identifier: MPL-2.0
/*!

Input handling functionality for app_window - a cross-platform library for receiving keyboard and mouse events.

# Design principles

* Use the best modern backend on each platform
* Zero magic, this library can be easily used without interference alongside any other native code.
    * When native code may interfere, instead this library will be a no-op by default.  You must
      call into it from your eventloop before our events are delivered.
* Mouse events:
    * Mouse position is determined by the compositor.  Platform-specific acceleration will be applied.
        * This is appropriate for GUI apps and topdown strategy games, or anytime you have a system-drawn cursor.
          It is appropriate for some fullscreen games. It is not appropriate for Counter Strike.
    * Coordinates are translated into a platform-independent upper-left coordinate system that works everywhere
    * Mouse events may require the window to be 'active' to be delivered, per platform conventions
* Keyboard events:
   * Report raw up/down events whenever possible
        * We map hardware keys rather than unicode characters
        * If you are trying to implement text input, you have much work to do, including but not limited to the shift key
   * Keycodes are translated into a platform-independent enum that works everywhere
   * On Linux, key events are broadcasted over ATSPI.  Due to some [questionable decisions in the Linux ecosystem](https://github.com/AccessKit/accesskit/discussions/503#discussioncomment-11862133)
     this is required for screenreaders to work but nobody does it.  We do!


# Supported platforms
| Platform | Backend                  |
|----------|--------------------------|
| Windows  | win32*                   |
| macOS    | AppKit                   |
| Linux    | Wayland*                 |
| wasm32   | KeyboardEvent \| MouseEvent  |
| Yours    | Send a PR!               |


* `*`: Needs platform-native event integration before events are delivered.  Consider using [app_window](https://sealedabstract.com/code/app_window)!

# WASM/JavaScript Support

This library is also available as an npm package for JavaScript/TypeScript projects targeting WebAssembly.
The package provides type definitions and can be used in web applications.

*/
///Provides information about keyboard events.
pub mod keyboard;
///Provides information about mouse events.
pub mod mouse;

/// Shows a debug window for testing keyboard input (macOS only).
///
/// This function displays a native window that can be used to test keyboard input
/// without interference from other applications. Useful for debugging keyboard event handling.
///
/// # Platform availability
/// - Currently only implemented on macOS
/// - No-op on other platforms
pub use keyboard::sys::debug_window_show;

/// Hides the debug window (macOS only).
///
/// Closes the debug window that was previously shown with `debug_window_show()`.
///
/// # Platform availability
/// - Currently only implemented on macOS
/// - No-op on other platforms
pub use keyboard::sys::debug_window_hide;

/**
Provides information about the window an event was delivered to.

# Platform specifics
* On Windows, this value contains an HWND.
* on macOS, this is the pointer of an NSWindow.  No memory management is performed, so dereferencing the window may be invalid.
* on wasm32, we attach to the global DOM window, and we choose an opaque value arbitrarily for this type.
* on Linux, we return the wayland surface ID.  No memory management is performed, so values may refer to previous surfaces, etc.
*/
#[derive(Debug, Copy, Clone)]
pub struct Window(pub std::ptr::NonNull<std::ffi::c_void>);
//we don't do anything with it so it's fine to send
unsafe impl Send for Window {}

#[cfg(target_os = "linux")]
pub mod linux {
    pub use crate::input::keyboard::linux::wl_keyboard_event;
    pub use crate::input::mouse::linux::{button_event, motion_event, xdg_toplevel_configure_event};
}

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
pub fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if mouse::windows::window_proc(hwnd, msg, w_param, l_param) == LRESULT(0)
        || keyboard::windows::kbd_window_proc(hwnd, msg, w_param, l_param) == LRESULT(0)
    {
        LRESULT(0)
    } else {
        LRESULT(1)
    }
}
