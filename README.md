# app_window

A cross-platform window management crate with async-first APIs.

![logo](art/logo.png)

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

```rust
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
```

Then create windows from any async context:

```rust
use app_window::{window::Window, coordinates::{Position, Size}};

// Create a window at a specific position
let window = Window::new(
    Position::new(100.0, 100.0),
    Size::new(800.0, 600.0),
    "My Application".to_string()
).await;

// The window stays open as long as the Window instance exists
// When dropped, the window automatically closes
```

# Design Principles

## 1. Async-First API

Unlike traditional windowing libraries, `app_window` uses async functions throughout.
This design elegantly handles platform differences:

```rust
use app_window::window::Window;

// This works on any thread, on any platform
let window = Window::default().await;

// Platform-specific threading is handled internally:
// - On macOS: dispatched to main thread
// - On Windows/Linux: may run on current thread
// - On Web: runs on the single thread
```

## 2. Window Lifetime Management

Windows are tied to their Rust object lifetime. No manual cleanup needed:

```rust
use app_window::window::Window;

{
    let window = Window::default().await;
    // Window is open and visible
} // Window automatically closes when dropped
```

## 3. Platform-Specific Strategies

The crate provides platform-specific strategies for graphics APIs:

```rust
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

```rust
use app_window::application;

// This works everywhere, regardless of platform requirements
let result = application::on_main_thread("my_task".to_string(), || {
    // Guaranteed to run on main thread
    42
}).await;
```

# Examples

## Creating a fullscreen window

```rust
use app_window::window::Window;

match Window::fullscreen("My Game".to_string()).await {
    Ok(mut window) => {
        // Fullscreen window created
        let surface = window.surface().await;
        // Set up rendering...
    }
    Err(e) => eprintln!("Failed to create fullscreen window: {:?}", e),
}
```

## Handling window resize

```rust
use app_window::{window::Window, coordinates::Size};

let mut window = Window::default().await;
let mut surface = window.surface().await;

// Register a callback for size changes
surface.size_update(|new_size: Size| {
    println!("Window resized to {}x{}", new_size.width(), new_size.height());
    // Update your rendering viewport...
});
```

## Input handling

```rust
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
```

## Integrating with wgpu

For wgpu integration, use the platform-specific strategy:

```rust
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

## License

This project is licensed under the Mozilla Public License 2.0 (MPL-2.0).