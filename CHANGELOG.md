# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-11-27

### Added

- **Testing infrastructure** - You can now write doctests and integration tests that actually work across all platforms! The new `test_support` module brings `doctest_main` and `integration_test_harness` to help you test your windowed apps on macOS, Windows, Linux, and even WASM. No more "works on my machine" excuses.

- **Alert dialogs** - Need to grab your user's attention? The new `alert()` function lets you show simple message dialogs without diving into platform-specific code. Perfect for those "Are you sure?" moments.

- **Developer scripts** - Added a collection of helper scripts (`scripts/check`, `scripts/tests`, `scripts/clippy`, etc.) to make development smoother. They handle both native and WASM targets so you don't have to remember all those cargo flags.

### Fixed

- **[Linux] Headless compositor support** - Weston headless (used in CI) and app_window are now best friends. We made the seat binding optional since headless mode doesn't have keyboards or mice, and expanded xdg_wm_base version support to cover both headless (v5) and desktop (v6) compositors.

- **[Linux] Window lifecycle** - Fixed a protocol error that could happen if you dropped a window before the compositor finished configuring it. We now track the configuration state properly and clean up like good citizens.

- **[Linux] Surface cleanup** - Improved Surface::drop handling to prevent resource leaks and compositor complaints.

### Changed

- **Dependency updates** - Bumped wgpu to 27.0, updated Windows crates to 0.62, and refreshed logwise to 0.4. Everything's a bit shinier now.

- **Documentation** - Expanded docs and examples to make getting started easier. We even fixed some clippy warnings that were cluttering the output.

- **CI improvements** - Better logging and debugging support to catch platform-specific issues before they reach you.

## [0.3.0] - 2025-09-07

Previous release. See git history for details.

---

[Unreleased]: https://github.com/drewcrawford/app_window/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/drewcrawford/app_window/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/drewcrawford/app_window/releases/tag/v0.3.0
