<!-- The authoritative version of this document is the crate documentation in src/lib.rs. Edit there, then mirror the changes here. -->

# app_window

A cross-platform window crate with an async-first API.

![logo](art/logo.png)

`app_window` creates windows and rendering surfaces on Windows, macOS, Linux, and
WebAssembly. It is deliberately small: you get a window, a surface that plugs into
anything that consumes `raw-window-handle` (wgpu, OpenGL, Vulkan), cross-platform
keyboard and mouse input, and a main-thread executor. You bring the renderer and
the rest of your application.

The crate exists because platforms disagree about threads. macOS insists UI runs
on the main thread. Wayland compositors behave best when rendering stays *off*
the main thread. In the browser, the main thread is the event loop and blocking
it is fatal. Most windowing libraries hand this problem to you: they own an event
loop, call you back on a thread of their choosing, and your architecture bends
around theirs. `app_window` inverts that. It takes ownership of the main thread
once, at startup, and from then on every API is an async function you can call
from any thread. The crate routes each call to whatever thread the current
platform requires; you write straight-line code.

# What it is — and isn't

`app_window` provides:

- **Windows** — created from any thread; a window closes when its `Window` value drops
- **Surfaces** — implement the `raw-window-handle` traits, so wgpu, glutin, ash, and friends plug in directly
- **Input** — keyboard (physical keys, layout-independent) and mouse, unified across platforms
- **Main-thread dispatch** — `application::on_main_thread`, a main-thread executor integrated with the native event loop, and `MainThreadCell` for values pinned to that thread

It is **not** a GUI toolkit. There are no widgets, no layout engine, no text
rendering. If you want buttons out of the box, look at egui, iced, or Slint
instead. The intended pairing is `app_window` + wgpu + your own rendering code —
a game, a visualization, a custom-drawn UI.

# Where it fits in the ecosystem

```text
┌─────────────────────────────────────────────────┐
│                your application                 │
├─────────────────────────────────────────────────┤
│     wgpu / OpenGL / Vulkan / your renderer      │
│            (via raw-window-handle)              │
├─────────────────────────────────────────────────┤
│    app_window: window · surface · input ·       │
│             main-thread executor                │
├──────────┬────────────┬───────────┬─────────────┤
│  Win32   │   AppKit   │  Wayland  │   Canvas    │
│(Windows) │(macOS, via │  (Linux)  │   (Web)     │
│          │   Swift)   │           │             │
└──────────┴────────────┴───────────┴─────────────┘
```

Three integration points matter:

- **raw-window-handle** is the Rust ecosystem's standard interface between
  windowing and graphics. `Surface` implements it, so any renderer that consumes
  it works without `app_window` knowing anything about it: wgpu (recommended; see
  `examples/gpu.rs`), OpenGL via glutin, Vulkan via ash, Metal via metal-rs,
  DirectX via windows-rs.
- **Async runtimes.** The crate is executor-agnostic through
  [`some_executor`](https://sealedabstract.com/code/some_executor). At startup it
  installs its main-thread executor via those traits, and it interoperates with
  any runtime that speaks the same interface. There is no tokio dependency and no
  runtime lock-in.
- **WebAssembly.** The browser backend is built on
  [`wasm_lite`](https://github.com/drewcrawford/wasm_lite) — hand-written DOM
  bindings — rather than web-sys/wasm-bindgen, and targets shared-memory
  threading (atomics + bulk memory), so the same multithreaded architecture you
  use natively runs in the browser. wgpu still uses wasm-bindgen internally; a
  one-line patch (see the WASM + wgpu section below) lets the two coexist.

The macOS backend is written in Swift and doubles as a Swift package
(`SwiftAppWindow/`), so the same windowing layer is callable from Swift.

# Alternatives

The Rust windowing space has a clear incumbent and several specialists. What
follows is an honest comparison; `app_window` is not the right choice for every
project.

**[winit](https://crates.io/crates/winit)** is the de facto standard and the
default answer for most projects — Bevy, eframe, and iced all sit on top of it.
It supports a much larger platform matrix than `app_window`: X11 as well as
Wayland, Android, iOS. The trade is architectural: winit owns your event loop and
calls back into your `ApplicationHandler`, and each platform's thread-affinity
rules — what must happen on the main thread, what must not — are yours to know
and manage. Choose winit when you need its platform breadth or its ecosystem;
choose `app_window` when you'd rather write async code and let the library carry
the threading rules.

**[tao](https://crates.io/crates/tao)** is Tauri's fork of winit, extended with
app menus and a system tray, and GTK-backed on Linux. It's the natural choice if
you're building around a webview.

**[sdl2](https://crates.io/crates/sdl2)** / **[sdl3](https://crates.io/crates/sdl3)**
bind the C SDL library: windowing plus audio, game controllers, haptics, and
more, with decades of portability behind it. You accept a C dependency and a
polling-style API. A good fit for games that want batteries included.

**[glfw](https://crates.io/crates/glfw)** binds the C GLFW library: minimal,
OpenGL-oriented, desktop-focused.

**[miniquad](https://crates.io/crates/miniquad)** (and macroquad above it)
bundles windowing with its own graphics abstraction and produces very small wasm
builds — but you use its rendering API rather than wgpu.

**[minifb](https://crates.io/crates/minifb)** puts a CPU framebuffer in a
window. If "give me pixels" is the whole requirement, it's the simplest thing
that works.

**egui/eframe, iced, Slint, gtk4-rs, fltk-rs** are toolkits, not windowing
crates: they bundle windowing (usually winit) and give you widgets. Compare them
against `app_window` plus your renderer, not against `app_window` alone.

| Crate      | API model                    | Linux         | Mobile | Web                         | Scope                          |
|------------|------------------------------|---------------|--------|-----------------------------|--------------------------------|
| app_window | async, call from any thread  | Wayland       | —      | wasm, shared-memory threads | window + surface + input       |
| winit      | event loop, callbacks        | Wayland + X11 | yes    | wasm-bindgen                | window + surface + input       |
| tao        | event loop, callbacks        | GTK           | yes    | —                           | winit fork + menus/tray        |
| sdl2/sdl3  | C library, polling           | Wayland + X11 | yes    | emscripten                  | windowing + audio + controllers|
| glfw       | C library, polling           | Wayland + X11 | —      | —                           | OpenGL-focused windowing       |
| miniquad   | event callbacks              | X11 + Wayland | yes    | tiny wasm                   | window + built-in renderer     |

In short: reach for `app_window` for the async API, the unified threading model,
first-class shared-memory wasm, and a native Wayland backend. Pass on it if you
need X11 or mobile, if a framework you use requires winit, or if you want the
largest possible community behind your windowing layer.

# Quick Start

First, initialize the application from your main function:

```no_run
# // no_run because: application::main() must be called from the actual main thread, which is not available in doctests
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

Windows are tied to their Rust value: drop the `Window` and the window closes.
There is no separate close/destroy step to forget.

# Threading Model

Every public API is async and callable from any thread; the crate dispatches to
the right place per platform:

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

Under the hood:

- **macOS**: All UI operations dispatched to main thread via GCD
- **Windows**: UI operations can run on any thread
- **Linux (Wayland)**: Compositor-dependent, handled per-connection
- **WebAssembly**: Single-threaded, operations run directly

When you need the main thread explicitly, ask for it:

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

## wgpu threading strategies

Platforms also disagree about which thread may drive the GPU. The crate encodes
those rules in two constants so your rendering setup can branch on them instead
of hardcoding per-OS knowledge:

- `WGPU_STRATEGY` — where general wgpu work should happen
- `WGPU_SURFACE_STRATEGY` — where surfaces may be created and configured
  (notably: macOS is `Relaxed` for general wgpu use but `MainThread` for
  surface creation)

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

Keyboard input reports physical keys — the key labeled W on a QWERTY board,
regardless of active layout. That makes it a fit for game controls and
shortcuts, not for text entry. Mappings cover alphanumeric and symbol keys,
F1–F24, the numeric keypad, media and navigation keys, modifiers, and
international layouts (JIS, ISO).

```
# async fn example() {
use app_window::input::{
    keyboard::{Keyboard, key::KeyboardKey},
    mouse::{Mouse, MOUSE_BUTTON_LEFT}
};

// Create input handlers
let keyboard = Keyboard::coalesced().await;
let mut mouse = Mouse::coalesced().await;

if keyboard.is_pressed(KeyboardKey::Space) {
    println!("Space key is pressed!");
}

if keyboard.is_pressed(KeyboardKey::W) {
    println!("W key pressed - move forward!");
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
# // no_run because: full wgpu example requires graphics setup beyond scope of doctest
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

## WASM + wgpu

`wgpu` uses the wasm-bindgen API on WebAssembly, while this crate's browser
backend uses `wasm_lite`. To build an application that combines both, add the
wasm_lite compatibility patch to the application manifest:

```toml
[patch.crates-io]
wasm-bindgen = { git = "https://github.com/drewcrawford/wasm_lite", rev = "f47bf4178d666e83017abe056f07bb20d33c14cd" }
```

The patch belongs in the final application's `Cargo.toml`; Cargo does not
inherit patches from dependencies. Patch only `wasm-bindgen`—do not replace
`wasm_lite` or `wasm_lite_std` with git or path dependencies, because the
compatibility crate now resolves those released runtimes from crates.io. The
application also needs the released `wasm_lite_cli` runner and the shared-memory
WASM linker settings shown in this repository's `.cargo/config.toml`.

# Platform Support

| Platform | Backend | Status | Notes |
|----------|---------|--------|-------|
| Windows  | Win32 API | ✅ Stable | Full async support, relaxed threading |
| macOS    | AppKit via Swift | ✅ Stable | Main thread UI, Swift interop |
| Linux    | Wayland | ✅ Stable | Client-side decorations, compositor-dependent |
| Web      | Canvas API | ✅ Stable | Requires atomics & bulk memory features |

Linux support is Wayland-only; there is no X11 backend. Requires Rust 1.95+
(2024 edition).

# Cargo Features

The default feature set is empty; everything above works with no features
enabled. The optional extras are diagnostic:

- `exfiltrate` — keeps a bounded registry of the windows this process has
  created and exposes it, along with main-thread state, through the
  `exfiltrate` crate's `snapshot` command for inspecting live processes. Off by
  default because it costs a lock and a record per window.
- `logwise-diagnostic`, `logwise-forensic`, `logwise-performance` — enable
  progressively more detailed logging through the `logwise` facade. Operational
  failures — a window that won't open, a frozen main thread — are always
  compiled in and need no feature.

# Development

`scripts/check_all` runs the full gate: formatting, native and wasm checks,
clippy, tests, and docs, with warnings as errors. Per-target variants live in
`scripts/native/` and `scripts/wasm32/`; wasm tests run under the `wasm_lite`
runner on nightly.

## License

This project is licensed under the Mozilla Public License 2.0 (MPL-2.0).
