# Repository Guidelines

## Project Structure & Module Organization
- Rust crate lives in `src/`, with platform backends under `src/sys/{macos,windows,linux,wasm}` and input plumbing in `src/input`.
- Public examples are in `examples/` (e.g., `examples/fullscreen.rs`, `examples/gpu.rs`); Swift bindings sit in `SwiftAppWindow/` as a Swift Package.
- Integration tests and custom harnesses live in `tests/` plus `examples/test_example/tests`; assets for Linux pointers live in `linux_assets/`; build helpers and CI-local tooling are in `scripts/`.

## Build, Test, and Development Commands
- `scripts/check_all` – runs fmt, native + wasm checks, clippy, tests, and docs with warnings-as-errors.
- `scripts/check` – `cargo check` for both native and `wasm32-unknown-unknown` (nightly), respecting `RUSTFLAGS="-D warnings"`.
- `scripts/tests` – runs native tests then wasm tests via `cargo +nightly test --target wasm32-unknown-unknown`.
- `scripts/clippy`, `scripts/fmt`, `scripts/docs` – lint, format check, and docs for native + wasm; use these before pushing.
- For focused work, call `scripts/native/*` or `scripts/wasm32/*` directly (e.g., `scripts/wasm32/check` when touching `src/sys/wasm.rs`).

## Coding Style & Naming Conventions
- Rust 2024 edition; keep code `rustfmt` clean. The tree enforces warnings as errors; fix all lints and clippy notes.
- Use snake_case for modules/functions, UpperCamelCase for types, and keep async-first APIs consistent with existing patterns in `application` and `window` modules.
- Prefer small, composable modules; mirror platform-specific implementations across backends when adding features.

## Testing Guidelines
- Prefer integration-style tests in `tests/` named `*_test.rs`; platform-specific behaviors belong near their backend modules.
- Wasm tests rely on `wasm-bindgen-test` via `cargo +nightly test --target wasm32-unknown-unknown`; ensure new async code works under that target.
- Examples can double as smoke tests; keep them building with `cargo test --examples`.

## Commit & Pull Request Guidelines
- Commit messages follow the current history: short, imperative summaries (`Add scripts`, `bump wgpu`). Group related changes; avoid drive-by edits.
- Before opening a PR, run `scripts/check_all` and note any platform coverage limitations (e.g., “wasm tests skipped”).
- PRs should explain the behavior change, affected platforms, and any UI/ABI surface impacts; link issues when available. Include screenshots only when changing visible behavior.***
