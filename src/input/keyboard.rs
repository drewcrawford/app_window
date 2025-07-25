// SPDX-License-Identifier: MPL-2.0

//! Cross-platform keyboard input handling.
//!
//! This module provides a high-level interface for detecting keyboard key states across
//! different platforms (Windows, macOS, Linux, and WASM). The keyboard state is tracked
//! globally, allowing you to check if any key is currently pressed.
//!
//! # Architecture
//!
//! The module uses platform-specific implementations (in submodules) to capture raw
//! keyboard events and translate them into a unified `KeyboardKey` enum. All keyboards
//! connected to the system are coalesced into a single logical keyboard.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() {
//! use app_window::input::keyboard::{Keyboard, key::KeyboardKey};
//!
//! let keyboard = Keyboard::coalesced().await;
//!
//! // Initially, keys are not pressed
//! assert_eq!(keyboard.is_pressed(KeyboardKey::A), false);
//! assert_eq!(keyboard.is_pressed(KeyboardKey::Shift), false);
//! # }
//! ```
//!
//! # Platform Requirements
//!
//! - **Windows**: Call `window_proc` from your window procedure  
//! - **Linux**: Call `wl_keyboard_event` from your Wayland dispatch queue
//! - **macOS** and **WASM**: No special integration required

use std::ffi::c_void;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr};

/// Keyboard key definitions and enumerations.
pub mod key;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm;

#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(target_os = "macos")]
pub(crate) use macos as sys;

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm as sys;

#[cfg(target_os = "windows")]
pub(crate) use windows as sys;

#[cfg(target_os = "linux")]
pub(crate) use linux as sys;

use crate::application::is_main_thread_running;
use crate::input::keyboard::key::KeyboardKey;
use crate::input::keyboard::sys::PlatformCoalescedKeyboard;

/// Internal shared state for keyboard tracking.
///
/// This struct is shared between the public `Keyboard` API and the platform-specific
/// implementations. It maintains the current state of all keyboard keys using atomic
/// operations for thread safety.
#[derive(Debug)]
struct Shared {
    /// Array of atomic booleans tracking the pressed state of each key.
    /// Indexed by the numeric value of `KeyboardKey`.
    key_states: Vec<AtomicBool>,
    /// Platform-specific window pointer that received the most recent keyboard event.
    window_ptr: AtomicPtr<c_void>,
}

impl Shared {
    fn new() -> Self {
        let mut vec = Vec::with_capacity(key::KeyboardKey::all_keys().len());
        for _ in 0..key::KeyboardKey::all_keys().len() {
            vec.push(AtomicBool::new(false));
        }
        Shared {
            key_states: vec,
            window_ptr: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn set_key_state(&self, key: KeyboardKey, state: bool, window_ptr: *mut c_void) {
        logwise::debuginternal_sync!(
            "Setting key {key} to {state}",
            key = logwise::privacy::LogIt(key),
            state = state
        );
        self.window_ptr
            .store(window_ptr, std::sync::atomic::Ordering::Relaxed);
        self.key_states[key as usize].store(state, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A cross-platform keyboard input handler.
///
/// `Keyboard` provides a unified interface for detecting keyboard key states across
/// different platforms. It represents all physical keyboards connected to the system
/// as a single logical keyboard.
///
/// # Thread Safety
///
/// `Keyboard` is `Send + Sync` and can be safely shared between threads. Key state
/// queries are lock-free and use atomic operations internally.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::keyboard::{Keyboard, key::KeyboardKey};
///
/// let keyboard = Keyboard::coalesced().await;
///
/// // Check various key states
/// let is_a_pressed = keyboard.is_pressed(KeyboardKey::A);
/// let is_shift_pressed = keyboard.is_pressed(KeyboardKey::Shift);
/// let is_escape_pressed = keyboard.is_pressed(KeyboardKey::Escape);
///
/// // Keys start in unpressed state
/// assert_eq!(is_a_pressed, false);
/// # }
/// ```
#[derive(Debug)]
pub struct Keyboard {
    shared: Arc<Shared>,
    _platform_coalesced_keyboard: PlatformCoalescedKeyboard,
}

impl Keyboard {
    /// Creates a keyboard instance representing all physical keyboards on the system.
    ///
    /// This constructor creates a single logical keyboard that coalesces input from all
    /// connected physical keyboards. This is typically what you want for most applications.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() {
    /// use app_window::input::keyboard::Keyboard;
    ///
    /// let keyboard = Keyboard::coalesced().await;
    /// // The keyboard is now ready to track key states
    /// # }
    /// ```
    pub async fn coalesced() -> Self {
        assert!(
            is_main_thread_running(),
            "Main thread must be started before creating coalesced keyboard"
        );
        let shared = Arc::new(Shared::new());
        let _platform_coalesced_keyboard = PlatformCoalescedKeyboard::new(&shared).await;
        Self {
            shared,
            _platform_coalesced_keyboard,
        }
    }

    /// Checks if the specified key is currently pressed.
    ///
    /// Returns `true` if the key is currently held down, `false` otherwise.
    /// This method uses atomic operations and is safe to call from any thread.
    ///
    /// # Arguments
    ///
    /// * `key` - The keyboard key to check
    ///
    /// # Platform specifics
    ///
    /// * **macOS** and **WASM**: No special considerations required
    /// * **Windows**: You must call `window_proc` from your window procedure
    /// * **Linux**: You must call `wl_keyboard_event` from your Wayland dispatch queue
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() {
    /// use app_window::input::keyboard::{Keyboard, key::KeyboardKey};
    ///
    /// let keyboard = Keyboard::coalesced().await;
    ///
    /// // Check if specific keys are pressed
    /// if keyboard.is_pressed(KeyboardKey::Space) {
    ///     // Handle space key
    /// }
    ///
    /// // Check multiple keys for combinations
    /// let ctrl_pressed = keyboard.is_pressed(KeyboardKey::Control);
    /// let s_pressed = keyboard.is_pressed(KeyboardKey::S);
    /// if ctrl_pressed && s_pressed {
    ///     // Handle Ctrl+S
    /// }
    /// # }
    /// ```
    pub fn is_pressed(&self, key: KeyboardKey) -> bool {
        self.shared.key_states[key as usize].load(std::sync::atomic::Ordering::Relaxed)
    }
}

//boilerplate

impl PartialEq for Keyboard {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

impl Eq for Keyboard {}

impl Hash for Keyboard {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.shared).hash(state);
    }
}

// Note: Default trait implementation removed because Keyboard::coalesced() is now async

#[cfg(test)]
mod test {
    use crate::input::keyboard::Keyboard;

    #[test]
    fn test_send_sync() {
        //I think basically the platform keyboard type operates as a kind of lifetime marker
        //(the main function is drop).  Accordingly it shouldn't be too bad to expect platforms to
        //implement send if necessary.
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        fn assert_unpin<T: Unpin>() {}

        assert_send::<Keyboard>();
        assert_sync::<Keyboard>();
        assert_unpin::<Keyboard>();
    }
}
