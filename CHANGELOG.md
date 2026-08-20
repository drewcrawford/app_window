# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.5] - 2026-08-20

### Added

- **`snapshot --subsystem main_thread` says whether the event loop is
  pumping**, and it is deliberately *not* behind a feature. A frozen UI is an
  operational fact about the application, and an operational fact is not
  something a consumer should have to opt into observing. The cost is three
  relaxed atomics on a path that already allocates a `String`, takes an
  `Instant` and mints a context. The probe does not itself wait on the main
  thread, so a wedged main thread is reported rather than joined.

- **`snapshot --subsystem windows` reports every window this process created**
  — what was asked for, whether a surface is attached, the geometry the
  application last observed, and whether it is still open. This one is behind
  the `exfiltrate` feature: it costs a lock and a record per window, and adds a
  field to `Window` and `Surface`.

  They are two subsystems rather than one because they fail independently. The
  window registry can be locked while the main thread is fine, and the main
  thread can be wedged while the registry answers instantly.

### Changed

- **Migrated to the logwise facade and durable context tokens.** Window and
  input instrumentation now goes through the zero-dependency facade, so an
  application that installs no runtime pays nothing for it.

## [Unreleased]

### Fixed

- **[Windows] Main-thread dispatch messages no longer carry Rust pointers.**
  Callback ownership now stays in a process-local, one-shot registry, so a
  malformed, forged, or replayed window message is ignored instead of being
  treated as an address to dereference and free.

- **[Windows] The Windows target builds again.** Its message diagnostics no
  longer ask Win32 handle wrappers to implement unsupported display formatting,
  and routine logs no longer include raw handles or pointer-sized parameters.

- **Application startup remains one-shot after shutdown.** The lifetime guard
  is now separate from the event loop's liveness flag, so a stopped application
  cannot accidentally reuse backend channels and globals that were designed to
  initialize exactly once.

- **Surfaces now keep their native windows alive.** Dropping a `Window` before
  its `Surface` can no longer leave safe `raw-window-handle` accessors handing a
  dangling native handle to wgpu or another renderer. The actual window closes
  when its last window-or-surface owner goes away.

- **[macOS] Awaiting window creation now awaits the actual AppKit window.** The
  Swift bridge completes Rust's constructor only after MainActor setup, closing
  a race where an immediate `surface()` call could force-unwrap a window that
  its detached creation task had not installed yet.

- **[WASM] The lifecycle regression test now survives panic-abort builds.** It
  verifies one-shot startup through a non-panicking internal transition, so the
  browser runner checks the rejection instead of aborting on the expected panic.

## [0.3.4] - 2026-08-17

### Added

- **[Linux] Complete external input dispatch** - Applications driving their own Wayland queue can now forward scroll events and keyboard focus loss through the public `axis_event` and `wl_keyboard_focus_leave` hooks.

### Fixed

- **Main-thread executor fairness** - Ready tasks now run in FIFO order, self-wakes wait for the current poll to finish, and late wakes from completed tasks quietly do nothing. Busy tasks can no longer cut the line forever or trip an executor panic on their way out.

- **Keyboard state across platforms** - Linux, Windows, and the browser now release held keys when focus moves away. Unknown key codes are ignored safely, several keypad/edit-key mappings are corrected, and Windows properly distinguishes left/right modifiers and Alt-modified key messages.

- **Mouse and window events** - Fixed the full `u8` mouse-button range, browser viewport coordinates, Windows wheel direction, Wayland scroll and compositor-close delivery, pointer position on enter, output tracking, and zero-sized configure handling. A surprisingly eventful cleanup for events.

- **Platform lifecycle reliability** - Wayland shared-memory pools and role objects now tear down in protocol-safe order, Windows main-thread closures survive modal loops via a message-only window, and macOS test binaries can find the Swift runtime without a fallback environment variable.

- **Main-thread ownership safety** - `MainThreadCell` construction now stays on the UI thread from start to finish, and thread-affine shared values are never destroyed on a worker after the dispatcher stops. This closes a path to undefined behavior during startup and shutdown.

- **[macOS] Swift binding correctness** - Keyboard event contexts now use their matching destructor, avoiding allocator corruption during teardown. Resize callbacks report logical dimensions, and mouse events include the real window width again.

- **[Windows] Clean shutdown and key mapping** - Stopping the application now quits the UI thread even when requested from a worker, failed callback posts release their allocation, and Delete is correctly reported as Forward Delete.

- **[Linux] No runaway idle wakeups** - Removed a debug wakeup loop that could keep posting work for nearly seventeen minutes after startup.

- **[WASM] Main-thread synchronization** - Mouse-location and resize-callback state now use wasm-aware locks, avoiding atomic waits that trap on the browser main thread.

### Changed

- **A lighter WASM stack** - The browser backend and test suite have moved from `wasm-bindgen`, `web-sys`, and `wasm_safe_thread` to `wasm_lite` 0.1.3 and its released CLI runner. Browser tests now exercise the real threaded paths instead of quietly skipping them.

- **WASM tests with receipts** - Removed `test_executors` in favor of `wasm_lite`'s own test support, brought the remaining unit and thread-affinity checks into the browser suite, and turned an executor regression target that reported zero tests into one that actually runs. The suite now also drives unmodified wgpu through wasm_lite's compatibility shim in GPU-enabled Chrome, creating an adapter and device before rendering and presenting a real surface frame. A passing test is much more persuasive when it has met the code.

- **Published dependency refresh** - Removed the sibling-checkout patches and moved to released versions of `continue`, `continue_stream`, `logwise`, `send_cells`, and `some_executor`. The minimum supported Rust version is now 1.95.

## [0.3.3] - 2026-02-15

### Fixed

- **[Linux] Dependency compatibility** - Updated Linux backend code to match the latest `accesskit` and `zune-png` APIs. This fixes CI/native Linux build breaks caused by upstream API changes (`TreeUpdate::tree_id`, `ActionRequest::target_node`, and PNG decoder input/dimensions updates).

### Changed

- **Dependency refresh** - Updated several dependencies in this release-prep cycle, including `accesskit`/`accesskit_unix` and `zune-png`, plus the WASM threading dependency update to `wasm_safe_thread`.

## [0.3.2] - 2025-12-20

### Fixed

- **[macOS] Keyboard event crash** - No more mysterious crashes when tabbing between apps! We switched to a flags-based approach for parsing key events, which fixes a fatal error that happened when the system sent us FlagsChanged events with certain key codes. Your app-switching workflow is safe again.

- **[WASM] Mouse precision** - Mouse clicks and movements on WASM now land exactly where you expect them. We switched from page-relative coordinates to canvas-relative offsets, fixing those annoying "off by a few pixels" moments.

### Changed

- **Dependency updates** - Bumped wgpu to 28.0 (hello, newer graphics goodness!), plus refreshed logwise to 0.5, thiserror to 2.0.17, wasm-bindgen to 0.2.106, test_executors to 0.4.1, and a handful of other dependencies. Everything's freshly polished.

- **Documentation** - Added comprehensive docs for keyboard input APIs, making it easier to understand how key events work across platforms.

## [0.3.1] - 2025-11-27

### Added

- **Testing infrastructure** - You can now write doctests and integration tests that actually work across all platforms! The new `test_support` module brings `doctest_main` and `integration_test_harness` to help you test your windowed apps on macOS, Windows, Linux, and even WASM. No more "works on my machine" excuses.

- **Alert dialogs** - Need to grab your user's attention? The new `alert()` function lets you show simple message dialogs without diving into platform-specific code. Perfect for those "Are you sure?" moments.

- **Developer scripts** - Added a collection of helper scripts (`scripts/check`, `scripts/tests`, `scripts/clippy`, etc.) to make development smoother. They handle both native and WASM targets so you don't have to remember all those cargo flags.

### Fixed

- **[Linux] Headless compositor support** - Weston headless (used in CI) and app_window are now best friends. We made the seat binding optional since headless mode doesn't have keyboards or mice, and expanded xdg_wm_base version support to cover both headless (v5) and desktop (v6) compositors.

- **[Linux] Window lifecycle** - Fixed a protocol error that could happen if you dropped a window before the compositor finished configuring it. We now track the configuration state properly and clean up like good citizens.

- **[Linux] Surface cleanup** - Improved Surface::drop handling to prevent resource leaks and compositor complaints.

- **[Linux] xdg-shell protocol compliance** - Fixed a protocol violation that was causing "xdg_surface has never been configured" errors. Turns out Wayland is *very* particular about the order of operations: you must `ack_configure` before committing a buffer, not after. We also stopped attaching buffers before the initial configure event (another no-no). Thanks to `WAYLAND_DEBUG=1` for helping us catch this one!

### Changed

- **Dependency updates** - Bumped wgpu to 27.0, updated Windows crates to 0.62, and refreshed logwise to 0.4. Everything's a bit shinier now.

- **Documentation** - Expanded docs and examples to make getting started easier. We even fixed some clippy warnings that were cluttering the output.

- **CI improvements** - Better logging and debugging support to catch platform-specific issues before they reach you.

## [0.3.0] - 2025-09-07

Previous release. See git history for details.

---

[Unreleased]: https://github.com/drewcrawford/app_window/compare/v0.3.4...HEAD
[0.3.4]: https://github.com/drewcrawford/app_window/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/drewcrawford/app_window/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/drewcrawford/app_window/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/drewcrawford/app_window/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/drewcrawford/app_window/releases/tag/v0.3.0
