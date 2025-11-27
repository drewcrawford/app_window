// SPDX-License-Identifier: MPL-2.0

//! Cross-platform window management.
//!
//! This module provides the [`Window`](crate::window::Window) type for creating and managing platform-native windows.
//! Windows can be created in various configurations (windowed, fullscreen) and provide surfaces
//! for rendering content using graphics APIs like wgpu.
//!
//! # Platform Requirements
//!
//! Before creating windows, you must initialize the application by calling
//! [`application::main()`](crate::application::main). This sets up the platform-specific
//! event loop. Once initialized, windows can be created from any thread.
//!
//! # Example
//!
//! ```
//! #[cfg(target_arch = "wasm32")] {
//!     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
//! }
//! use app_window::test_support::doctest_main;
//! use some_executor::task::{Configuration, Task};
//!
//! doctest_main(|| {
//!     use app_window::coordinates::{Position, Size};
//!
//!     Task::without_notifications(
//!         "doctest".to_string(),
//!         Configuration::default(),
//!         async {
//!             let _window = app_window::window::Window::new(
//!                 Position::new(100.0, 100.0),
//!                 Size::new(800.0, 600.0),
//!                 "My App".to_string()
//!             ).await;
//!             // Window closes when dropped
//!         },
//!     ).spawn_static_current();
//! });
//! ```

use crate::application::CALL_MAIN;
use crate::coordinates::{Position, Size};
use crate::surface::Surface;
use crate::sys;
use std::fmt::Display;

/// A cross-platform window.
///
/// `Window` represents a native window on the current platform. It provides a uniform API
/// across Windows, macOS, Linux (Wayland), and web platforms. Windows are created asynchronously
/// and can optionally provide a [`Surface`] for rendering.
///
/// # Lifecycle
///
/// Windows remain open as long as the `Window` instance exists. Dropping a `Window` will
/// close it immediately. To keep a window open indefinitely, use [`std::mem::forget`]:
///
/// ```
/// #[cfg(target_arch = "wasm32")] {
///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// }
/// use app_window::test_support::doctest_main;
/// use some_executor::task::{Configuration, Task};
///
/// doctest_main(|| {
///     Task::without_notifications(
///         "doctest".to_string(),
///         Configuration::default(),
///         async {
///             let window = app_window::window::Window::default().await;
///             std::mem::forget(window); // Window stays open
///         },
///     ).spawn_static_current();
/// });
/// ```
///
/// # Threading
///
/// Window creation requires that [`application::main()`](crate::application::main) has been
/// called to initialize the event loop, but windows can be created from any thread after
/// that initialization. The `Window` type is `Send + Sync` and can be safely moved between
/// threads.
///
/// # Platform Behavior
///
/// - **Windows**: Uses Win32 APIs
/// - **macOS**: Uses AppKit
/// - **Linux**: Uses Wayland
/// - **Web**: Creates a canvas element
///
/// See the [crate documentation](crate) for more details about platform-specific behavior.
#[derive(Debug)]
#[must_use = "Dropping a window will close it!"]
pub struct Window {
    sys: crate::sys::Window,
    created_surface: bool,
}

/// An error that can occur when creating a fullscreen window.
///
/// This error wraps platform-specific errors that may occur when attempting
/// to create a fullscreen window. The specific reasons for failure vary by platform:
///
/// - **macOS**: May fail if fullscreen is not supported by the display
/// - **Windows**: May fail if exclusive fullscreen mode cannot be acquired
/// - **Linux**: May fail if the compositor doesn't support fullscreen
/// - **Web**: May fail if fullscreen permission is not granted
#[derive(thiserror::Error, Debug)]
pub struct FullscreenError(#[from] sys::FullscreenError);

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Window {
    /// Creates a fullscreen window.
    ///
    /// This method attempts to create a window that covers the entire screen. The exact
    /// behavior depends on the platform:
    ///
    /// - **Desktop platforms**: Creates an exclusive fullscreen window
    /// - **Web**: Requests fullscreen mode (may require user interaction)
    ///
    /// # Arguments
    ///
    /// * `title` - The window title (may not be visible in fullscreen mode)
    ///
    /// # Errors
    ///
    /// Returns [`FullscreenError`] if fullscreen mode cannot be established.
    ///
    /// # Example
    ///
    /// ```
    /// #[cfg(target_arch = "wasm32")] {
    ///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    /// }
    /// use app_window::test_support::doctest_main;
    /// use some_executor::task::{Configuration, Task};
    ///
    /// doctest_main(|| {
    ///     Task::without_notifications(
    ///         "doctest".to_string(),
    ///         Configuration::default(),
    ///         async {
    ///             match app_window::window::Window::fullscreen("My Game".to_string()).await {
    ///                 Ok(window) => {
    ///                     println!("Fullscreen window created");
    ///                     std::mem::forget(window);
    ///                 },
    ///                 Err(e) => eprintln!("Failed to create fullscreen: {}", e),
    ///             }
    ///         },
    ///     ).spawn_static_current();
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`application::main()`](crate::application::main) has not been called.
    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        assert!(
            crate::application::is_main_thread_running(),
            "{}",
            CALL_MAIN
        );
        let sys = crate::sys::Window::fullscreen(title).await?;
        Ok(Window {
            sys,
            created_surface: false,
        })
    }
    /// Creates a new window with the specified position, size, and title.
    ///
    /// The window will be created at the given position with the specified dimensions.
    /// Position and size are in logical pixels, which may differ from physical pixels
    /// on high-DPI displays.
    ///
    /// # Arguments
    ///
    /// * `position` - The initial position of the window in screen coordinates
    /// * `size` - The initial size of the window in logical pixels
    /// * `title` - The window title displayed in the title bar
    ///
    /// # Example
    ///
    /// ```
    /// #[cfg(target_arch = "wasm32")] {
    ///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    /// }
    /// use app_window::test_support::doctest_main;
    /// use some_executor::task::{Configuration, Task};
    ///
    /// doctest_main(|| {
    ///     Task::without_notifications(
    ///         "doctest".to_string(),
    ///         Configuration::default(),
    ///         async {
    ///             use app_window::coordinates::{Position, Size};
    ///
    ///             let _window = app_window::window::Window::new(
    ///                 Position::new(100.0, 100.0),
    ///                 Size::new(800.0, 600.0),
    ///                 "My Application".to_string()
    ///             ).await;
    ///             // Window closes when dropped
    ///         },
    ///     ).spawn_static_current();
    /// });
    /// ```
    ///
    /// # Platform Notes
    ///
    /// - **macOS**: Position is from the bottom-left of the screen
    /// - **Other platforms**: Position is from the top-left of the screen
    /// - **Web**: Position may be ignored by the browser
    ///
    /// # Panics
    ///
    /// Panics if [`application::main()`](crate::application::main) has not been called.
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        assert!(
            crate::application::is_main_thread_running(),
            "Call app_window::application::main"
        );
        Window {
            sys: crate::sys::Window::new(position, size, title).await,
            created_surface: false,
        }
    }

    /// Creates a [`Surface`] for this window.
    ///
    /// A surface is required for rendering content to the window using graphics APIs
    /// like wgpu. Only one surface can be created per window.
    ///
    /// # Example
    ///
    /// ```
    /// #[cfg(target_arch = "wasm32")] {
    ///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    /// }
    /// use app_window::test_support::doctest_main;
    /// use some_executor::task::{Configuration, Task};
    ///
    /// doctest_main(|| {
    ///     Task::without_notifications(
    ///         "doctest".to_string(),
    ///         Configuration::default(),
    ///         async {
    ///             let mut window = app_window::window::Window::default().await;
    ///             let surface = window.surface().await;
    ///
    ///             // Now you can use the surface with a graphics API
    ///             let (size, scale) = surface.size_scale().await;
    ///             println!("Surface size: {}x{} at {} scale",
    ///                      size.width(), size.height(), scale);
    ///         },
    ///     ).spawn_static_current();
    /// });
    /// ```
    ///
    /// # Platform Performance
    ///
    /// Creating a surface may be expensive on some platforms. Applications that
    /// don't need to render content can skip creating a surface to save resources.
    ///
    /// # Panics
    ///
    /// Panics if a surface has already been created for this window.
    pub async fn surface(&mut self) -> Surface {
        assert!(!self.created_surface, "Surface already created");
        self.created_surface = true;
        self.sys.surface().await
    }

    /// Creates a new window with platform-appropriate default settings.
    ///
    /// This is the simplest way to create a window. The platform will choose
    /// reasonable defaults for position, size, and other properties:
    ///
    /// - **Position**: Typically centered or cascaded
    /// - **Size**: A reasonable default (often 800x600 or similar)
    /// - **Title**: Platform-specific default or empty
    ///
    /// # Example
    ///
    /// ```
    /// #[cfg(target_arch = "wasm32")] {
    ///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    /// }
    /// use app_window::test_support::doctest_main;
    /// use some_executor::task::{Configuration, Task};
    ///
    /// doctest_main(|| {
    ///     Task::without_notifications(
    ///         "doctest".to_string(),
    ///         Configuration::default(),
    ///         async {
    ///             let _window = app_window::window::Window::default().await;
    ///             println!("Window created with default settings");
    ///             // Window closes when dropped
    ///         },
    ///     ).spawn_static_current();
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`application::main()`](crate::application::main) has not been called.
    pub async fn default() -> Self {
        assert!(
            crate::application::is_main_thread_running(),
            "Call app_window::application::run_main_thread"
        );
        Window {
            sys: crate::sys::Window::default().await,
            created_surface: false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::window::Window;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Window>();
        fn assert_sync<T: Sync>() {}
        assert_sync::<Window>();
    }
}
