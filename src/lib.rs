//SPDX-License-Identifier: MPL-2.0

/*!
A cross-platform window crate.  Alternative to winit.

![logo](../../../art/logo.png)

The main goal of this project is to provide a cross-platform API to bring up a window (or appropriate
platform surface) for rendering application or game content.  (The content is out of scope for
this crate, but the idea is to use wgpu or a toolkit like GTK to do the content.)

This crate distinguishes itself from winit in a few ways:
* Support platforms with wildly-different threading requirements, on the same API that works everywhere.
* Use the obvious, modern backend for each platform.  For example, use Wayland on Linux, not X11.  For backend details, see the support table.
* In order to achieve this thread portability, this crate is async-first.  Most APIs are async functions, designed to be called from any thread and
  run on any executor.  When we spawn tasks we use [`some_executor`](https://sealedabstract.com/code/some_executor) which is designed to be executor-agnostic.
   * That not withstanding, include a "main-thread" executor that can spawn tasks that need to run on the main thread, while *also* processing native events on that thread.
   * Optionally provides a 'wgpu executor' that can spawn tasks for using wgpu.  Notably, on platforms that require wgpu to be accessed from the main thread,
     it does that, and on platforms that need it NOT to be on the main thread, it does that too!

# Quick Start

```
# async fn example() {
use app_window::{application, window::Window, coordinates::{Position, Size}};

// Create a window
let window = Window::new(
    Position::new(100.0, 100.0),
    Size::new(800.0, 600.0),
    "My Window".to_string()
).await;

// Window stays open as long as the instance exists
# }
```

# Threading Model

This crate is designed to work across platforms with very different threading requirements:

- **macOS**: UI operations must happen on the main thread
- **Windows**: Most UI operations can happen on any thread
- **Linux (Wayland)**: Thread requirements vary by compositor
- **Web**: Single-threaded environment

To handle this, we provide:
1. An async-first API that works from any thread
2. A built-in main thread executor for UI operations
3. Platform-specific handling for graphics APIs like wgpu

# Examples

## Creating a fullscreen window

```
# async fn example() {
use app_window::{application, window::Window};

match Window::fullscreen("Fullscreen App".to_string()).await {
    Ok(window) => {
        // Window created successfully
    }
    Err(e) => eprintln!("Failed to create fullscreen window: {:?}", e),
}
# }
```

## Running code on the main thread

```
# async fn example() {
use app_window::application;

// From any thread:
let result = application::on_main_thread("ex".to_owned(),|| {
    // This runs on the main thread
    42
}).await;
assert_eq!(result, 42);
# }
```

# Cargo features
* `wgpu` - Helper functions for creating a wgpu surface.

# Input handling
This crate includes built-in cross-platform input handling for keyboard and mouse events. See the [`input`] module for details.

# Supported platforms
| Platform | Backend                  |
|----------|--------------------------|
| Windows  | win32                   |
| macOS    | AppKit                   |
| Linux    | Wayland                 |
| wasm32   | Canvas                  |
| Yours    | Send a PR!               |

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
/// # Example
/// ```no_run
/// # // can't use main thread in doctests
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
/// # }
/// ```
pub mod surface;

/// Cross-platform mouse and keyboard input handling.
///
/// This module provides keyboard and mouse input functionality that integrates
/// with app_window. It handles platform-specific input events and provides
/// a unified API across Windows, macOS, Linux, and WebAssembly.
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
/// ```no_run
/// # // can't run on main thread in doctests
/// # use app_window::{application, executor};
/// # application::main(|| {
/// # async fn my_async_function() -> i32 { 42 }
/// // Run an async function on the main thread
/// let result = executor::on_main_thread_async("ex".to_owned(),my_async_function());
/// # });
/// ```
pub mod executor;

/// Integration with the `some_executor` crate.
///
/// This module provides [`some_executor::MainThreadExecutor`], which implements
/// the `SomeExecutor` and `SomeLocalExecutor` traits from the `some_executor` crate.
/// This allows the main thread executor to be used with any library that supports
/// the `some_executor` abstraction.
pub mod some_executor;

/// Thread-safe cell for main-thread-only values.
///
/// This module provides [`main_thread_cell::MainThreadCell`], which allows sharing
/// values across threads while ensuring all access happens on the main thread.
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

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WGPUStrategy {
    /**
    The main thread should be used to access wgpu.
    */
    MainThread,
    /**
    The main thread should **NOT be used to access wgpu.
    */
    NotMainThread,
    /**
    On this platform, wgpu types are sendable and can be used from any thread.

    Platforms with this type should use test_relaxed to verify
    */
    Relaxed,
}
/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_os = "linux")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_os = "windows")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_os = "macos")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_arch = "wasm32")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;



/**
Describes the preferred strategy for interacting with wgpu surfaces on this platform.
*/
#[cfg(target_os = "linux")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_os = "windows")]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(any(target_os="macos"))]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;
#[cfg(any(target_arch = "wasm32"))]
pub const WGPU_SURFACE_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;


logwise::declare_logging_domain!();