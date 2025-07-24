//SPDX-License-Identifier: MPL-2.0

//! Test to verify that PlatformCoalescedMouse can be created from a non-main thread.
//!
//! This test ensures that the mouse input system can be safely initialized
//! from background threads, which is important for the overall threading model
//! of the app_window crate.
//!
//! Run with: `cargo test --test platform_coalesced_mouse_test`
//! Run on WASM with: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner" RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' cargo +nightly test --target wasm32-unknown-unknown -Z build-std=std,panic_abort

use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use some_executor::task::{Configuration, Task};

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let logger = Arc::new(logwise::InMemoryLogger::new());

    logwise::warn_sync!("=== PlatformCoalescedMouse Non-Main Thread Test ===");

    app_window::application::main(|| {
        thread::spawn(|| {
            let t = Task::without_notifications(
                "platform_coalesced_mouse_test".to_string(),
                Configuration::default(),
                async {
                    test_platform_coalesced_mouse_creation().await;
                    std::process::exit(0);
                },
            );
            t.spawn_static_current();
        });
    });
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn wasm_main() {
    assert!(app_window::application::is_main_thread());

    let logger = Arc::new(logwise::InMemoryLogger::new());
    let dump_logger = logger.clone();
    logwise::set_global_logger(logger.clone());
    let (c, r) = r#continue::continuation();

    app_window::application::main(move || {
        logwise::warn_sync!("=== PlatformCoalescedMouse Non-Main Thread Test ===");

        let t = Task::without_notifications(
            "platform_coalesced_mouse_test".to_string(),
            Configuration::default(),
            async move {
                logwise::info_sync!("WASM main thread started");
                test_platform_coalesced_mouse_creation().await;
                c.send(());
            },
        );
        t.spawn_static_current();
    });

    futures::join!(
        r,
        dump_logger.periodic_drain_to_console(Duration::from_secs(1))
    );
}

async fn test_platform_coalesced_mouse_creation() {
    logwise::info_sync!("Starting PlatformCoalescedMouse creation test from non-main thread");

    let (tx, rx) = r#continue::continuation();

    // Spawn a non-main thread to create PlatformCoalescedMouse
    thread::spawn(move || {
        logwise::info_sync!("Non-main thread started, creating PlatformCoalescedMouse");

        // This is the main test: creating a PlatformCoalescedMouse from a non-main thread
        let result = std::panic::catch_unwind(|| {
            let mouse = app_window::input::mouse::Mouse::coalesced();
            logwise::info_sync!("Successfully created PlatformCoalescedMouse from non-main thread");
            mouse
        });

        match result {
            Ok(_mouse) => {
                logwise::warn_sync!(
                    "✅ SUCCESS: PlatformCoalescedMouse created successfully from non-main thread"
                );
                tx.send(true);
            }
            Err(e) => {
                logwise::error_sync!(
                    "❌ FAILURE: Failed to create PlatformCoalescedMouse from non-main thread: {error}",
                    error = logwise::privacy::LogIt(&e)
                );
                tx.send(false);
            }
        }
    });

    // Wait for the result
    let success = rx.await;

    if success {
        logwise::warn_sync!("🎉 Test completed successfully");
    } else {
        logwise::error_sync!("💥 Test failed");
        panic!("PlatformCoalescedMouse creation from non-main thread failed");
    }
}
