// SPDX-License-Identifier: MPL-2.0
//! Test to verify that PlatformCoalescedMouse can be created from a non-main thread.
//!
//! This test ensures that the mouse input system can be safely initialized
//! from background threads, which is important for the overall threading model
//! of the app_window crate.
//!
//! Run with: `cargo test --test platform_coalesced_mouse_test`
//! Run on WASM with: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner" RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' cargo +nightly test --target wasm32-unknown-unknown -Z build-std=std,panic_abort
logwise::declare_logging_domain!();

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_safe_thread as thread;

use some_executor::task::{Configuration, Task};

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn wasm_main() {
    assert!(app_window::application::is_main_thread());

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

    r.await;
}

async fn test_platform_coalesced_mouse_creation() {
    logwise::info_sync!("Starting PlatformCoalescedMouse creation test from non-main thread");

    let (tx, rx) = r#continue::continuation();

    // Spawn a non-main thread to create PlatformCoalescedMouse
    thread::spawn(move || {
        logwise::info_sync!("Non-main thread started, creating PlatformCoalescedMouse");

        // This is the main test: creating a PlatformCoalescedMouse from a non-main thread
        // Note: Since Mouse::coalesced() is now async, we spawn an async task
        let spawn_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let task = Task::without_notifications(
                "mouse_creation_test".to_string(),
                Configuration::default(),
                async move {
                    // Try to create the mouse - this will now happen on the main thread via MainThreadCell
                    app_window::input::mouse::Mouse::coalesced().await;
                    logwise::info_sync!("Successfully created PlatformCoalescedMouse");
                    logwise::warn_sync!("✅ SUCCESS: PlatformCoalescedMouse created successfully");
                    tx.send(true);
                },
            );
            task.spawn_static_current();
        }));

        spawn_result.unwrap();
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
