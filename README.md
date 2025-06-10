# app_window

A cross-platform window crate. Alternative to winit.

![logo](art/logo.png)

The main goal of this project is to provide a cross-platform API to bring up a window (or appropriate platform surface) for rendering application or game content. (The content is out of scope for this crate, but the idea is to use wgpu or a toolkit like GTK to do the content.)

This crate distinguishes itself from winit in a few ways:
* Support platforms with wildly-different threading requirements, on the same API that works everywhere.
* Use the obvious, modern backend for each platform. For example, use Wayland on Linux, not X11. For backend details, see the support table.
* In order to achieve this thread portability, this crate is async-first. Most APIs are async functions, designed to be called from any thread and run on any executor. When we spawn tasks we use [`some_executor`](https://sealedabstract.com/code/some_executor) which is designed to be executor-agnostic.
  * That not withstanding, include a "main-thread" executor that can spawn tasks that need to run on the main thread, while *also* processing native events on that thread.
  * Optionally provides a 'wgpu executor' that can spawn tasks for using wgpu. Notably, on platforms that require wgpu to be accessed from the main thread, it does that, and on platforms that need it NOT to be on the main thread, it does that too!

## Quick Start

```rust
use app_window::{application, window::Window, coordinates::{Position, Size}};

async fn example() {
    // Create a window
    let window = Window::new(
        Position::new(100.0, 100.0),
        Size::new(800.0, 600.0),
        "My Window".to_string()
    ).await;

    // Window stays open as long as the instance exists
}
```

## Threading Model

This crate is designed to work across platforms with very different threading requirements:

- **macOS**: UI operations must happen on the main thread
- **Windows**: Most UI operations can happen on any thread
- **Linux (Wayland)**: Thread requirements vary by compositor
- **Web**: Single-threaded environment

To handle this, we provide:
1. An async-first API that works from any thread
2. A built-in main thread executor for UI operations
3. Platform-specific handling for graphics APIs like wgpu

## Examples

### Creating a fullscreen window

```rust
use app_window::{application, window::Window};

match Window::fullscreen("Fullscreen App".to_string()).await {
    Ok(window) => {
        // Window created successfully
    }
    Err(e) => eprintln!("Failed to create fullscreen window: {:?}", e),
}
```

### Running code on the main thread

```rust
use app_window::application;

// From any thread:
let result = application::on_main_thread(|| {
    // This runs on the main thread
    42
}).await;
assert_eq!(result, 42);
```


## Cargo features
* `some_executor` - Provides interop with the `some-executor` crate.
* `wgpu` - Helper functions for creating a wgpu surface.
* `app_input` - Created windows are configured to receive input via [`app_input`](https://sealedabstract.com/code/app_input) crate.

## Supported platforms
| Platform | Backend                  |
|----------|--------------------------|
| Windows  | win32                   |
| macOS    | AppKit                   |
| Linux    | Wayland                 |
| wasm32   | Canvas                  |
| Yours    | Send a PR!               |

## License

This project is licensed under the Mozilla Public License 2.0 (MPL-2.0).