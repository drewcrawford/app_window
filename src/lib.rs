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

```no_run
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

```no_run
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

```no_run
# async fn example() {
use app_window::application;

// From any thread:
let result = application::on_main_thread(|| {
    // This runs on the main thread
    42
}).await;
assert_eq!(result, 42);
# }
```

# Cargo features
* `some_executor` - Provides interop with the `some-executor` crate.
* `wgpu` - Helper functions for creating a wgpu surface.
* `app_input` - Created windows are configured to receive input via [`app_input`](https://sealedabstract.com/code/app_input) crate.

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
/// ```no_run
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
/// ```no_run
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

/// Main thread executor for async operations.
/// 
/// This module provides utilities for running futures on the main thread, which is
/// required for UI operations on many platforms. The executor integrates with the
/// native event loop to process both async tasks and platform events.
/// 
/// # Example
/// ```no_run
/// # use app_window::{application, executor};
/// # application::main(|| {
/// # async fn my_async_function() -> i32 { 42 }
/// // Run an async function on the main thread
/// let result = executor::on_main_thread_async(my_async_function());
/// # });
/// ```
pub mod executor;

/// Integration with the `some_executor` crate.
/// 
/// This module provides [`some_executor::MainThreadExecutor`], which implements
/// the `SomeExecutor` and `SomeLocalExecutor` traits from the `some_executor` crate.
/// This allows the main thread executor to be used with any library that supports
/// the `some_executor` abstraction.
#[cfg(feature = "some_executor")]
pub mod some_executor;

/// Platform-specific wgpu integration.
/// 
/// This module provides utilities for using wgpu with app_window, handling the
/// different threading requirements across platforms. Some platforms require GPU
/// access from the main thread, while others require it from a different thread.
/// 
/// # Example
/// ```no_run
/// # #[cfg(feature = "wgpu")]
/// # {
/// use app_window::wgpu::{wgpu_spawn, WGPU_STRATEGY, WGPUStrategy};
/// 
/// // Check the platform's wgpu strategy
/// match WGPU_STRATEGY {
///     WGPUStrategy::MainThread => println!("GPU access must be on main thread"),
///     WGPUStrategy::NotMainThread => println!("GPU access must NOT be on main thread"),
///     WGPUStrategy::Relaxed => println!("GPU types are Send+Sync"),
///     _ => println!("Unknown strategy"),
/// }
/// 
/// // Spawn a future on the appropriate thread for wgpu
/// wgpu_spawn(async {
///     // wgpu operations here
/// });
/// # }
/// ```
#[cfg(feature = "wgpu")]
pub mod wgpu;