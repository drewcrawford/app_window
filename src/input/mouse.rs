// SPDX-License-Identifier: MPL-2.0
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
use std::ffi::c_void;
use std::hash::{Hash, Hasher};

#[cfg(target_arch = "wasm32")]
pub(crate) use wasm as sys;

#[cfg(target_os = "windows")]
pub(crate) use windows as sys;

#[cfg(target_os = "linux")]
pub(crate) use linux as sys;

use crate::application::is_main_thread_running;
use crate::input::Window;
use atomic_float::AtomicF64;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// Mouse button constant for the left mouse button.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::mouse::{Mouse, MOUSE_BUTTON_LEFT};
///
/// let mouse = Mouse::coalesced().await;
/// let left_pressed = mouse.button_state(MOUSE_BUTTON_LEFT);
/// # }
/// ```
pub const MOUSE_BUTTON_LEFT: u8 = 0;

/// Mouse button constant for the right mouse button.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::mouse::{Mouse, MOUSE_BUTTON_RIGHT};
///
/// let mouse = Mouse::coalesced().await;
/// let right_pressed = mouse.button_state(MOUSE_BUTTON_RIGHT);
/// # }
/// ```
pub const MOUSE_BUTTON_RIGHT: u8 = 1;

/// Mouse button constant for the middle mouse button (wheel button).
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::mouse::{Mouse, MOUSE_BUTTON_MIDDLE};
///
/// let mouse = Mouse::coalesced().await;
/// let middle_pressed = mouse.button_state(MOUSE_BUTTON_MIDDLE);
/// # }
/// ```
pub const MOUSE_BUTTON_MIDDLE: u8 = 2;

/// Mouse's location within a window, in points.
///
/// The coordinate system has its origin at the upper-left corner of the window.
/// The position is reported in logical points, not physical pixels.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::mouse::Mouse;
///
/// let mouse = Mouse::coalesced().await;
/// if let Some(location) = mouse.window_pos() {
///     println!("Mouse at ({}, {})", location.pos_x(), location.pos_y());
///     println!("Window size: {}x{}", location.window_width(), location.window_height());
/// }
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MouseWindowLocation {
    pos_x: f64,
    pos_y: f64,
    window_width: f64,
    window_height: f64,
    window: Option<Window>,
}

impl MouseWindowLocation {
    fn new(
        pos_x: f64,
        pos_y: f64,
        window_width: f64,
        window_height: f64,
        window: Option<Window>,
    ) -> Self {
        MouseWindowLocation {
            pos_x,
            pos_y,
            window_width,
            window_height,
            window,
        }
    }

    /// Returns the X coordinate of the mouse position within the window.
    ///
    /// The X coordinate is measured from the left edge of the window.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::input::mouse::Mouse;
    /// # let mouse = Mouse::coalesced().await;
    /// if let Some(location) = mouse.window_pos() {
    ///     let x = location.pos_x();
    ///     println!("Mouse X: {}", x);
    /// }
    /// # }
    /// ```
    pub fn pos_x(&self) -> f64 {
        self.pos_x
    }

    /// Returns the Y coordinate of the mouse position within the window.
    ///
    /// The Y coordinate is measured from the top edge of the window.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::input::mouse::Mouse;
    /// # let mouse = Mouse::coalesced().await;
    /// if let Some(location) = mouse.window_pos() {
    ///     let y = location.pos_y();
    ///     println!("Mouse Y: {}", y);
    /// }
    /// # }
    /// ```
    pub fn pos_y(&self) -> f64 {
        self.pos_y
    }

    /// Returns the width of the window containing the mouse.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::input::mouse::Mouse;
    /// # let mouse = Mouse::coalesced().await;
    /// if let Some(location) = mouse.window_pos() {
    ///     let width = location.window_width();
    ///     println!("Window width: {}", width);
    /// }
    /// # }
    /// ```
    pub fn window_width(&self) -> f64 {
        self.window_width
    }

    /// Returns the height of the window containing the mouse.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::input::mouse::Mouse;
    /// # let mouse = Mouse::coalesced().await;
    /// if let Some(location) = mouse.window_pos() {
    ///     let height = location.window_height();
    ///     println!("Window height: {}", height);
    /// }
    /// # }
    /// ```
    pub fn window_height(&self) -> f64 {
        self.window_height
    }
}

#[derive(Debug)]
struct Shared {
    window: std::sync::Mutex<Option<MouseWindowLocation>>,

    buttons: [AtomicBool; 255],
    scroll_delta_x: AtomicF64,
    scroll_delta_y: AtomicF64,
    last_window: AtomicPtr<c_void>,
}
impl Shared {
    fn new() -> Self {
        Shared {
            window: std::sync::Mutex::new(None),
            buttons: [const { AtomicBool::new(false) }; 255],
            scroll_delta_x: AtomicF64::new(0.0),
            scroll_delta_y: AtomicF64::new(0.0),
            last_window: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn set_window_location(&self, location: MouseWindowLocation) {
        logwise::debuginternal_sync!(
            "Set mouse window location {location}",
            location = logwise::privacy::LogIt(&location)
        );
        *self.window.lock().unwrap() = Some(location);
        self.last_window.store(
            location
                .window
                .map(|e| e.0.as_ptr())
                .unwrap_or(std::ptr::null_mut()),
            Ordering::Relaxed,
        )
    }
    fn set_key_state(&self, key: u8, down: bool, window: *mut c_void) {
        logwise::debuginternal_sync!("Set mouse key {key} state {down}", key = key, down = down);
        self.buttons[key as usize].store(down, std::sync::atomic::Ordering::Relaxed);
        self.last_window
            .store(window, std::sync::atomic::Ordering::Relaxed);
    }

    fn add_scroll_delta(&self, delta_x: f64, delta_y: f64, window: *mut c_void) {
        logwise::debuginternal_sync!(
            "Add mouse scroll delta {delta_x},{delta_y}",
            delta_x = delta_x,
            delta_y = delta_y
        );
        self.scroll_delta_x
            .fetch_add(delta_x, std::sync::atomic::Ordering::Relaxed);
        self.scroll_delta_y
            .fetch_add(delta_y, std::sync::atomic::Ordering::Relaxed);
        self.last_window
            .store(window, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Provides access to mouse input from all mice on the system.
///
/// This type coalesces input from all connected mice into a single interface.
/// It provides access to:
/// - Mouse position within windows
/// - Button states (left, right, middle, and others)
/// - Accumulated scroll deltas
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::input::mouse::{Mouse, MOUSE_BUTTON_LEFT};
///
/// let mouse = Mouse::coalesced().await;
///
/// // Check if left button is pressed
/// if mouse.button_state(MOUSE_BUTTON_LEFT) {
///     println!("Left button is pressed");
/// }
///
/// // Get mouse position
/// if let Some(pos) = mouse.window_pos() {
///     println!("Mouse at ({}, {})", pos.pos_x(), pos.pos_y());
/// }
/// # }
/// ```
///
/// # Platform-specific behavior
///
/// Different platforms require different integration:
/// - **macOS** and **wasm**: Work out of the box
/// - **Windows**: You must call `window_proc` from your window procedure
/// - **Linux**: You must call the appropriate wayland event handlers
#[derive(Debug)]
pub struct Mouse {
    shared: Arc<Shared>,
    _sys: sys::PlatformCoalescedMouse,
}

impl Mouse {
    /// Creates a new `Mouse` instance that coalesces input from all mice on the system.
    ///
    /// This is the primary way to create a `Mouse` instance. The returned object
    /// will aggregate input from all connected mice.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use app_window::input::mouse::Mouse;
    ///
    /// let mouse = Mouse::coalesced().await;
    /// // Now you can query mouse state
    /// # }
    /// ```
    pub async fn coalesced() -> Self {
        assert!(
            is_main_thread_running(),
            "Main thread must be started before creating coalesced mouse"
        );
        let shared = Arc::new(Shared::new());
        let coalesced = sys::PlatformCoalescedMouse::new(&shared).await;
        Mouse {
            shared,
            _sys: coalesced,
        }
    }

    #[allow(rustdoc::broken_intra_doc_links)] //references to the platform-specific code
    /**
        Returns the [MouseWindowLocation]

        # Platform specifics

        * macOS and wasm require no special considerations.
        * On windows, you must call [crate::window_proc] from your window.
        * * On Linux,you must call from appropriate wayland events:
            * [crate::input::mouse::linux::motion_event],
            * [crate::input::mouse::linux::button_event]
            * [crate::input::mouse::linux::xdg_toplevel_configure_event]
    */
    pub fn window_pos(&self) -> Option<MouseWindowLocation> {
        *self.shared.window.lock().unwrap()
    }

    /// Determines if the specified mouse button is currently pressed.
    ///
    /// # Arguments
    ///
    /// * `button` - The button to check. Use constants like [`MOUSE_BUTTON_LEFT`],
    ///   [`MOUSE_BUTTON_RIGHT`], or [`MOUSE_BUTTON_MIDDLE`]. Other button values
    ///   (e.g., for mice with additional buttons) may be supported on a best-effort basis.
    ///
    /// # Returns
    ///
    /// `true` if the button is currently pressed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use app_window::input::mouse::{Mouse, MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT};
    ///
    /// let mouse = Mouse::coalesced().await;
    ///
    /// if mouse.button_state(MOUSE_BUTTON_LEFT) {
    ///     println!("Left button is pressed");
    /// }
    ///
    /// if mouse.button_state(MOUSE_BUTTON_RIGHT) {
    ///     println!("Right button is pressed");
    /// }
    /// # }
    /// ```
    pub fn button_state(&self, button: u8) -> bool {
        self.shared.buttons[button as usize].load(Ordering::Relaxed)
    }

    /// Returns the accumulated scroll delta and resets it to zero.
    ///
    /// This method is useful for implementing scroll handling in your application.
    /// The scroll delta accumulates between calls, so you should call this
    /// periodically (e.g., once per frame) to process scroll events.
    ///
    /// # Returns
    ///
    /// A tuple `(delta_x, delta_y)` containing the horizontal and vertical
    /// scroll amounts since the last call to this method.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use app_window::input::mouse::Mouse;
    ///
    /// let mut mouse = Mouse::coalesced().await;
    ///
    /// // In your update loop:
    /// let (scroll_x, scroll_y) = mouse.load_clear_scroll_delta();
    /// if scroll_y != 0.0 {
    ///     println!("Scrolled vertically by {}", scroll_y);
    /// }
    /// # }
    /// ```
    pub fn load_clear_scroll_delta(&mut self) -> (f64, f64) {
        let x = self.shared.scroll_delta_x.swap(0.0, Ordering::Relaxed);
        let y = self.shared.scroll_delta_y.swap(0.0, Ordering::Relaxed);
        (x, y)
    }
}

impl PartialEq for Mouse {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

impl Eq for Mouse {}

impl Hash for Mouse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.shared).hash(state);
    }
}

#[cfg(test)]
mod test {
    use crate::input::mouse::Mouse;

    #[test]
    fn test_send_sync() {
        //I think basically the platform keyboard type operates as a kind of lifetime marker
        //(the main function is drop).  Accordingly it shouldn't be too bad to expect platforms to
        //implement send if necessary.
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        fn assert_unpin<T: Unpin>() {}

        assert_send::<Mouse>();
        assert_sync::<Mouse>();
        assert_unpin::<Mouse>();
    }
}
