# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[Unreleased]: https://github.com/drewcrawford/app_window/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/drewcrawford/app_window/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/drewcrawford/app_window/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/drewcrawford/app_window/releases/tag/v0.3.0
