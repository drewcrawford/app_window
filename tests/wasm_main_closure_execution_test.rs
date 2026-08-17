// SPDX-License-Identifier: MPL-2.0

//! Test to verify that the closure passed to app_window::application::main() actually executes.
//!
//! This test reproduces the bug where the closure passed to `application::main()`
//! is never executed on WASM, causing the program to return immediately without
//! running the user's initialization code.
//!
//! Expected behavior:
//! - The closure should execute within a reasonable timeframe (500ms)
//! - Debug output should show "Inside app_window::application::main() closure"
//!
//! Bug behavior on WASM:
//! - The function returns immediately
//! - The closure is never executed
//! - We never see "Inside app_window::application::main() closure"
//!
//! Run with: `cargo test --test wasm_main_closure_execution_test`

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

//wasm_lite's runner always drives a real browser
//see https://github.com/rustwasm/wasm-bindgen/issues/4534,
//and threading comes from wasm_lite_std.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    wasm_lite_std::block_on(test())
}

#[cfg(target_arch = "wasm32")]
wasm_lite::test_main!();

async fn test() {
    #[cfg(not(target_arch = "wasm32"))]
    let (s, r) = std::sync::mpsc::channel();
    let (s2, r2) = r#continue::continuation();

    #[cfg(not(target_arch = "wasm32"))]
    thread::spawn(move || {
        //one message received here
        r.recv_timeout(Duration::from_millis(500)).unwrap();
        std::process::exit(0);
    });
    app_window::application::main(move || {
        #[cfg(not(target_arch = "wasm32"))]
        s.send(()).unwrap();
        s2.send(());
    });
    r2.await;
}

// The native `main` above drives this with `block_on`. On wasm the body
// cannot block, so a `#[wasm_lite_test]` entry point hands it to
// `async_doctest!`, which defers the verdict until the future settles.
#[cfg(target_arch = "wasm32")]
#[wasm_lite::wasm_lite_test]
fn wasm_main_closure_executes() {
    wasm_lite_std::async_doctest!(test());
}
