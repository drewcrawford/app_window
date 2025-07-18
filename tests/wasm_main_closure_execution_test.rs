//SPDX-License-Identifier: MPL-2.0

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

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn main() {
    let (s,r) = std::sync::mpsc::channel();
    let (s2, r2) = std::sync::mpsc::channel();

    thread::spawn(move || {
        //one message received here
        r.recv_timeout(Duration::from_millis(500)).unwrap();
        std::process::exit(0);
    });
    app_window::application::main(move || {
        //send two messages to the channel
        s.send(()).unwrap();
        s2.send(()).unwrap();
    });

    r2.recv_timeout(Duration::from_millis(500)).unwrap();


}

