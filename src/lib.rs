// SPDX-License-Identifier: MPL-2.0

/*!
A cross-platform window management crate with async-first APIs.

![logo](https://github.com/drewcrawford/app_window/raw/main/art/logo.png)

`app_window` provides a modern alternative to winit for creating and managing windows across
Windows, macOS, Linux, and WebAssembly. The crate's primary goal is to provide a unified,
async-first API that works seamlessly across platforms with wildly different threading
requirements.

# Key Features

- **Async-first design**: All APIs are async functions that can be called from any thread
- **Modern platform backends**: Win32 on Windows, AppKit on macOS, Wayland on Linux, Canvas on Web
- **Unified threading model**: Works correctly whether the platform requires UI on the main thread or not
- **Graphics API integration**: Provides `raw-window-handle` for wgpu, OpenGL, Vulkan, etc.
- **Built-in input handling**: Cross-platform keyboard and mouse support
- **Executor-agnostic**: Works with any async runtime via [`some_executor`](https://sealedabstract.com/code/some_executor)

# Quick Start

First, initialize the application from your main function:

```no_run
# // ALLOW_NORUN_DOCTEST: application::main() must be called from the actual main thread, which is not available in doctests
use app_window::application;
fn main() {
    application::main(|| {
        // Your application code here
        async fn run() {
            // Create windows, handle events, etc.
        }
        futures::executor::block_on(run());
    });
}
#[allow(clippy::needless_doctest_main)]
```

Then create windows from any async context:

```
# async fn example() {
use app_window::{window::Window, coordinates::{Position, Size}};

// Create a window at a specific position
let window = Window::new(
    Position::new(100.0, 100.0),
    Size::new(800.0, 600.0),
    "My Application".to_string()
).await;

// The window stays open as long as the Window instance exists
// When dropped, the window automatically closes
# }
```

# Design Principles

## 1. Async-First API

Unlike traditional windowing libraries, `app_window` uses async functions throughout.
This design elegantly handles platform differences:

```
# async fn example() {
use app_window::window::Window;

// This works on any thread, on any platform
let window = Window::default().await;

// Platform-specific threading is handled internally:
// - On macOS: dispatched to main thread
// - On Windows/Linux: may run on current thread
// - On Web: runs on the single thread
# }
```

## 2. Window Lifetime Management

Windows are tied to their Rust object lifetime. No manual cleanup needed:

```
# async fn example() {
use app_window::window::Window;

{
    let window = Window::default().await;
    // Window is open and visible
} // Window automatically closes when dropped
# }
```

## 3. Platform-Specific Strategies

The crate provides platform-specific strategies for graphics APIs:

```
use app_window::{WGPU_STRATEGY, WGPUStrategy};

match WGPU_STRATEGY {
    WGPUStrategy::MainThread => {
        // Platform requires wgpu on main thread (Web, some macOS configs)
    }
    WGPUStrategy::NotMainThread => {
        // Platform requires wgpu NOT on main thread (Linux/Wayland)
    }
    WGPUStrategy::Relaxed => {
        // Platform allows wgpu on any thread (Windows, most macOS)
    }
    _ => {
        // Future-proof: handle any new strategies
        // Default to the safest option
    }
}
```

# Threading Model

This crate abstracts over platform threading differences:

- **macOS**: All UI operations dispatched to main thread via GCD
- **Windows**: UI operations can run on any thread
- **Linux (Wayland)**: Compositor-dependent, handled per-connection
- **WebAssembly**: Single-threaded, operations run directly

You write the same async code for all platforms:

```
# async fn example() {
use app_window::application;

// This works everywhere, regardless of platform requirements
let result = application::on_main_thread("my_task".to_string(), || {
    // Guaranteed to run on main thread
    42
}).await;
# }
```

# Examples

## Creating a fullscreen window

```
# async fn example() {
use app_window::window::Window;

match Window::fullscreen("My Game".to_string()).await {
    Ok(mut window) => {
        // Fullscreen window created
        let surface = window.surface().await;
        // Set up rendering...
    }
    Err(e) => eprintln!("Failed to create fullscreen window: {:?}", e),
}
# }
```

## Handling window resize

```
# async fn example() {
use app_window::{window::Window, coordinates::Size};

let mut window = Window::default().await;
let mut surface = window.surface().await;

// Register a callback for size changes
surface.size_update(|new_size: Size| {
    println!("Window resized to {}x{}", new_size.width(), new_size.height());
    // Update your rendering viewport...
});
# }
```

## Input handling

```
# async fn example() {
use app_window::input::{
    keyboard::{Keyboard, key::KeyboardKey},
    mouse::{Mouse, MOUSE_BUTTON_LEFT}
};

// Create input handlers
let keyboard = Keyboard::coalesced().await;
let mut mouse = Mouse::coalesced().await;

// Check keyboard state - KeyboardKey represents physical keys,
// not logical characters, making it ideal for game controls and shortcuts.
// Supports comprehensive key mappings including alphanumeric, function keys,
// numeric keypad, media controls, navigation keys, and international layouts.
if keyboard.is_pressed(KeyboardKey::Space) {
    println!("Space key is pressed!");
}

if keyboard.is_pressed(KeyboardKey::F11) {
    println!("F11 (fullscreen) pressed!");
}

if keyboard.is_pressed(KeyboardKey::W) {
    println!("W key pressed - move forward!");
}

// Media control keys
if keyboard.is_pressed(KeyboardKey::Play) {
    println!("Play/Pause media key pressed!");
}

// Numeric keypad keys
if keyboard.is_pressed(KeyboardKey::KeypadEnter) {
    println!("Numeric keypad Enter pressed!");
}

// Check mouse state
if let Some(pos) = mouse.window_pos() {
    println!("Mouse at ({}, {})", pos.pos_x(), pos.pos_y());
}

if mouse.button_state(MOUSE_BUTTON_LEFT) {
    println!("Left mouse button is pressed!");
}

// Get scroll delta (clears after reading)
let (scroll_x, scroll_y) = mouse.load_clear_scroll_delta();
if scroll_y != 0.0 {
    println!("Scrolled vertically by {}", scroll_y);
}
# }
```

## Integrating with wgpu

For wgpu integration, use the platform-specific strategy:

```no_run
# // ALLOW_NORUN_DOCTEST: Full wgpu example requires graphics setup beyond scope of doctest
# async fn example() -> Result<(), Box<dyn std::error::Error>> {
use app_window::{window::Window, application, WGPU_STRATEGY, WGPUStrategy};

let mut window = Window::default().await;
let surface = window.surface().await;

// Use the appropriate strategy for your platform
match WGPU_STRATEGY {
    WGPUStrategy::MainThread => {
        application::on_main_thread("wgpu_init".to_string(), move || {
            // Create wgpu instance and surface on main thread
        }).await;
    }
    WGPUStrategy::NotMainThread => {
        // Create wgpu instance and surface on worker thread
    }
    WGPUStrategy::Relaxed => {
        // Create wgpu instance and surface on any thread
    }
    _ => {
        // Handle future strategies
    }
}
# Ok(())
# }
```

See `examples/gpu.rs` for a complete wgpu integration example.

# Platform Support

| Platform | Backend | Status | Notes |
|----------|---------|--------|-------|
| Windows  | Win32 API | ✅ Stable | Full async support, relaxed threading |
| macOS    | AppKit via Swift | ✅ Stable | Main thread UI, Swift interop |
| Linux    | Wayland | ✅ Stable | Client-side decorations, compositor-dependent |
| Web      | Canvas API | ✅ Stable | Requires atomics & bulk memory features |

# Performance Considerations

- **Lazy surface creation**: Surfaces are only allocated when requested via `window.surface()`
- **Input coalescing**: Input events can be coalesced for better performance in high-frequency scenarios
- **Efficient executor**: The main thread executor processes both async tasks and native events
- **Platform optimizations**: Each backend uses platform-specific optimizations

# Integration with Graphics APIs

The crate implements `raw-window-handle` traits, enabling integration with:
- **wgpu** (recommended, see `examples/gpu.rs`)
- **OpenGL/WebGL** via glutin or similar
- **Vulkan** via ash or vulkano
- **Metal** (macOS) via metal-rs
- **DirectX** (Windows) via windows-rs

*/

/// Window creation and management.
///
/// This module provides the [`window::Window`] type for creating and managing windows
/// across different platforms. Windows can be created from any thread after the
/// application has been initialized.
///
/// # Example
/// ```
/// # async fn example() {
/// use app_window::{window::Window, coordinates::{Position, Size}};
///
/// // Create a window with specific position and size
/// let window = Window::new(
///     Position::new(100.0, 100.0),
///     Size::new(800.0, 600.0),
///     "My Application".to_string()
/// ).await;
/// # }
/// ```
pub mod window;

/// Application lifecycle and main thread management.
///
/// This module provides the entry point for app_window applications and utilities
/// for executing code on the main thread. The [`application::main`] function must
/// be called once from the first thread to initialize the platform event loop.
///
/// Key functions:
/// - [`application::main`] - Initialize the application and event loop
/// - [`application::on_main_thread`] - Execute async code on the main thread
/// - [`application::submit_to_main_thread`] - Fire-and-forget main thread tasks
///
/// Most functions in this module will panic if [`application::main`] hasn't been called yet.
/// [`application::main`] is called at the start of your program.
///
/// # Example
/// ```no_run
/// # // ALLOW_NORUN_DOCTEST: application::main() must be called from the actual main thread, which is not available in doctests
/// use app_window::application;
///
/// fn main() {
///     application::main(|| {
///         // Application code here
///     });
/// }
/// ```
pub mod application;

mod sys;

/// Coordinate types for window positioning and sizing.
///
/// This module provides [`coordinates::Position`] and [`coordinates::Size`] types
/// for working with window coordinates. All values are in logical pixels, which
/// may differ from physical pixels on high-DPI displays.
///
/// # Example
/// ```
/// use app_window::coordinates::{Position, Size};
///
/// let pos = Position::new(100.0, 200.0);
/// assert_eq!(pos.x(), 100.0);
/// assert_eq!(pos.y(), 200.0);
///
/// let size = Size::new(800.0, 600.0);
/// assert_eq!(size.width(), 800.0);
/// assert_eq!(size.height(), 600.0);
/// ```
pub mod coordinates;

/// Rendering surface abstraction.
///
/// This module provides the [`surface::Surface`] type, which represents a drawable
/// area within a window. Surfaces integrate with graphics APIs like wgpu through
/// the `raw-window-handle` trait implementations.
///
/// # Example
/// ```
/// # async fn example() {
/// # use app_window::{application, window::Window};
/// let mut window = Window::default().await;
/// let surface = window.surface().await;
///
/// // Get size and scale factor
/// let (size, scale) = surface.size_scale().await;
///
/// // Get handles for graphics API integration
/// let window_handle = surface.window_handle();
/// let display_handle = surface.display_handle();
/// # }
/// ```
pub mod surface;

/// Cross-platform mouse and keyboard input handling.
///
/// This module provides keyboard and mouse input functionality that integrates
/// with app_window. It handles platform-specific input events and provides
/// a unified API across Windows, macOS, Linux, and WebAssembly.
///
/// # Keyboard Input
///
/// The keyboard module uses physical key mappings rather than logical characters.
/// This means [`input::keyboard::key::KeyboardKey`] represents actual physical keys on the
/// keyboard (e.g., the key labeled 'A' on QWERTY), independent of keyboard layout.
/// This approach is ideal for game controls and shortcuts but not for text input.
///
/// Comprehensive key mappings include:
/// - Standard alphanumeric keys (A-Z, 0-9) and symbol keys (brackets, quotes, etc.)
/// - Function keys (F1-F24) with extensive coverage up to F24
/// - Numeric keypad keys with full support (0-9, operators, decimal, Enter, Clear/Num Lock)
/// - Media control keys (Play/Pause, Stop, Volume Up/Down, Mute, Previous/Next Track)
/// - Navigation keys (arrows, Home, End, Page Up/Down, Insert, Delete)
/// - Modifier keys (Shift, Control, Option/Alt, Command/Windows, Function)
/// - International keyboard layouts (Japanese JIS keys: Yen, Kana, Eisu, Convert; ISO Section key)
/// - Browser and application launcher keys
/// - Editing keys (Undo, Copy, Cut, Paste, Find, Select)
///
/// On macOS, debug windows are available via `input::keyboard::macos::debug_window_show()`
/// and `input::keyboard::macos::debug_window_hide()` to inspect real-time raw keyboard events,
/// useful for debugging keyboard handling and understanding platform-specific key codes.
///
/// # Example
/// ```
/// # async fn example() {
/// use app_window::input::{keyboard::Keyboard, mouse::Mouse};
///
/// // Create input handlers
/// let keyboard = Keyboard::coalesced().await;
/// let mouse = Mouse::coalesced().await;
/// # }
/// ```
pub mod input;

/// Main thread executor for async operations.
///
/// This module provides utilities for running futures on the main thread, which is
/// required for UI operations on many platforms. The executor integrates with the
/// native event loop to process both async tasks and platform events.
///
/// # Example
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
///             use app_window::executor;
///
///             async fn my_async_function() -> i32 { 42 }
///
///             // Run an async function on the main thread
///             let result = executor::on_main_thread_async(
///                 "ex".to_owned(),
///                 my_async_function()
///             ).await;
///             assert_eq!(result, 42);
///         },
///     ).spawn_static_current();
/// });
/// ```
pub mod executor;

/// Integration with the `some_executor` crate.
///
/// This module provides [`some_executor::MainThreadExecutor`], which implements
/// the `SomeExecutor` and `SomeLocalExecutor` traits from the `some_executor` crate.
/// This allows the main thread executor to be used with any library that supports
/// the `some_executor` abstraction.
///
/// When [`application::main`] is called, a `MainThreadExecutor` is automatically
/// installed as the thread-local and thread-static executor, making it available
/// to any code using `some_executor`'s convenience functions.
pub mod some_executor;

/// Thread-safe cell for main-thread-only values.
///
/// `MainThreadCell<T>` is a thread-safe container that allows `T` to be shared across threads
/// while ensuring all access to the inner value happens on the main thread.
/// This is useful for platform-specific resources that have main-thread requirements.
///
/// # Example
/// ```
/// # async fn example() {
/// use app_window::main_thread_cell::MainThreadCell;
///
/// // Create a cell (can be called from any thread)
/// let cell = MainThreadCell::new(42);
///
/// // Access from main thread directly
/// if app_window::application::is_main_thread() {
///     let guard = cell.lock();
///     println!("Value: {}", *guard);
/// }
///
/// // Access from any thread via async dispatch
/// let result = cell.with(|value| {
///     // This runs on the main thread
///     *value * 2
/// }).await;
/// assert_eq!(result, 84);
/// # }
/// ```
pub mod main_thread_cell;

/// Test support utilities for working with the main thread.
///
/// This module provides utilities for writing tests (both doctests and integration tests)
/// that need to interact with the main thread. Since many platforms require UI operations
/// to run on the main thread, these utilities help set up and tear down the appropriate
/// environment for testing.
///
/// # Available utilities
///
/// - `doctest_main` - For writing doctests that need main thread access
/// - `integration_test_harness` - For integration tests with custom harness
///
/// # Example
///
/// For doctests that use async window operations:
/// ```
/// #[cfg(target_arch = "wasm32")] {
///     wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// }
/// use app_window::test_support::doctest_main;
///
/// doctest_main(|| {
///     // Your test code here - has access to main thread
/// });
/// ```
///
/// See the module documentation for more details and integration test examples.
pub mod test_support;

/// Describes the preferred strategy for interacting with wgpu on different platforms.
///
/// Different platforms have different requirements for which thread can access
/// graphics APIs. This enum encodes those platform-specific requirements to help
/// applications use wgpu correctly.
///
/// # Example
///
/// ```
/// use app_window::{WGPU_STRATEGY, WGPUStrategy};
///
/// // Check the platform's wgpu threading requirements
/// match WGPU_STRATEGY {
///     WGPUStrategy::MainThread => {
///         println!("wgpu must be accessed from the main thread");
///     }
///     WGPUStrategy::NotMainThread => {
///         println!("wgpu must NOT be accessed from the main thread");
///     }
///     WGPUStrategy::Relaxed => {
///         println!("wgpu can be accessed from any thread");
///     }
///     _ => {
///         println!("Unknown strategy - using default behavior");
///     }
/// }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WGPUStrategy {
    /// The main thread should be used to access wgpu.
    ///
    /// This is required on WebAssembly and some macOS configurations where
    /// the graphics context must be created and used from the main thread.
    MainThread,

    /// The main thread should NOT be used to access wgpu.
    ///
    /// This is required on Linux with Wayland, where blocking the main thread
    /// with graphics operations can cause compositor issues.
    NotMainThread,

    /// On this platform, wgpu types are sendable and can be used from any thread.
    ///
    /// This is the case on Windows and most macOS configurations, providing
    /// maximum flexibility for application architecture.
    Relaxed,
}

/// Displays an alert dialog with the given message.
///
/// This function displays a modal alert dialog to the user. The behavior is platform-specific:
///
/// - **WebAssembly**: Uses the browser's native `window.alert()` function
/// - **macOS, Windows, Linux**: Not yet implemented (will panic with `todo!`)
///
/// # Platform-specific behavior
///
/// On WebAssembly, this function will block execution until the user dismisses the alert dialog.
/// The function automatically dispatches to the main thread as required by the platform.
///
/// # Example
///
/// ```no_run
/// # // ALLOW_NORUN_DOCTEST: alert() requires application::main() to be called first, which is not available in doctests
/// # async fn example() {
/// use app_window::alert;
///
/// alert("Hello, world!".to_string()).await;
/// # }
/// ```
///
/// # Panics
///
/// Currently panics with `todo!` on macOS, Windows, and Linux platforms.
pub async fn alert(message: String) {
    sys::alert(message).await
}

/// The preferred strategy for interacting with wgpu on the current platform.
///
/// This constant provides the platform-specific threading requirements for wgpu
/// operations. Applications should check this value to determine the correct
/// thread to use for wgpu initialization and rendering.
///
/// # Platform Values
///
/// - **Linux**: `NotMainThread` - wgpu should be accessed from a worker thread
/// - **Windows**: `Relaxed` - wgpu can be accessed from any thread
/// - **macOS**: `Relaxed` - wgpu can be accessed from any thread
/// - **WebAssembly**: `MainThread` - wgpu must be accessed from the main thread
#[cfg(target_os = "linux")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

/// The preferred strategy for interacting with wgpu on the current platform.
///
/// See [`WGPU_STRATEGY`] documentation for details.
#[cfg(target_os = "windows")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/// The preferred strategy for interacting with wgpu on the current platform.
///
/// See [`WGPU_STRATEGY`] documentation for details.
#[cfg(target_os = "macos")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/// The preferred strategy for interacting with wgpu on the current platform.
///
/// See [`WGPU_STRATEGY`] documentation for details.
#[cfg(target_arch = "wasm32")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;

/// The preferred strategy for interacting with wgpu surfaces on the current platform.
///
/// This constant provides the platform-specific threading requirements for wgpu
/// surface creation and configuration. Some platforms have different requirements
/// for surface operations compared to general wgpu operations.
///
/// # Platform Values
///
/// - **Linux**: `NotMainThread` - surfaces should be created from a worker thread
/// - **Windows**: `Relaxed` - surfaces can be created from any thread
/// - **macOS**: `MainThread` - surfaces must be created from the main thread
/// - **WebAssembly**: `MainThread` - surfaces must be created from the main thread
///
/// # Difference from `WGPU_STRATEGY`
///
/// While `WGPU_STRATEGY` applies to general wgpu operations, `WGPU_SURFACE_STRATEGY`
/// specifically applies to surface creation and configuration. On macOS, for example,
/// general wgpu operations are `Relaxed` but surface operations require `MainThread`.
#[cfg(target_os = "linux")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

/// The preferred strategy for interacting with wgpu surfaces on the current platform.
///
/// See [`WGPU_SURFACE_STRATEGY`] documentation for details.
#[cfg(target_os = "windows")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/// The preferred strategy for interacting with wgpu surfaces on the current platform.
///
/// See [`WGPU_SURFACE_STRATEGY`] documentation for details.
#[cfg(target_os = "macos")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;

/// The preferred strategy for interacting with wgpu surfaces on the current platform.
///
/// See [`WGPU_SURFACE_STRATEGY`] documentation for details.
#[cfg(target_arch = "wasm32")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;

logwise::declare_logging_domain!();
