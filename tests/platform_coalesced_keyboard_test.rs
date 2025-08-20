//SPDX-License-Identifier: MPL-2.0

//! Test to verify that PlatformCoalescedKeyboard can be created from a non-main thread.
//!
//! This test ensures that the keyboard input system can be safely initialized
//! from background threads, which is important for the overall threading model
//! of the app_window crate.
//!
//! Run with: `cargo test --test platform_coalesced_keyboard_test`
//! Run on WASM with: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner" RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' cargo +nightly test --target wasm32-unknown-unknown -Z build-std=std,panic_abort

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use some_executor::task::{Configuration, Task};

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    logwise::warn_sync!("=== PlatformCoalescedKeyboard Non-Main Thread Test ===");

    app_window::application::main(|| {
        thread::spawn(|| {
            let t = Task::without_notifications(
                "platform_coalesced_keyboard_test".to_string(),
                Configuration::default(),
                async {
                    test_platform_coalesced_keyboard_creation().await;
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
        logwise::warn_sync!("=== PlatformCoalescedKeyboard Non-Main Thread Test ===");

        let t = Task::without_notifications(
            "platform_coalesced_keyboard_test".to_string(),
            Configuration::default(),
            async move {
                logwise::info_sync!("WASM main thread started");
                test_platform_coalesced_keyboard_creation().await;
                c.send(());
            },
        );
        t.spawn_static_current();
    });

    r.await;
}

async fn test_platform_coalesced_keyboard_creation() {
    logwise::info_sync!("Starting PlatformCoalescedKeyboard creation test from non-main thread");

    let (tx, rx) = r#continue::continuation();

    // Spawn a non-main thread to create PlatformCoalescedKeyboard
    thread::spawn(move || {
        logwise::info_sync!("Non-main thread started, creating PlatformCoalescedKeyboard");
        // This is the main test: creating a PlatformCoalescedKeyboard from a non-main thread
        // Note: Unlike Mouse::coalesced(), Keyboard::coalesced() is synchronous
        let spawn_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let task = Task::without_notifications(
                "keyboard_creation_test".to_string(),
                Configuration::default(),
                async move {
                    // Try to create the keyboard - this should work since it's async
                    let _keyboard = app_window::input::keyboard::Keyboard::coalesced().await;
                    logwise::info_sync!("Successfully created PlatformCoalescedKeyboard");
                    logwise::warn_sync!(
                        "✅ SUCCESS: PlatformCoalescedKeyboard created successfully"
                    );
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
        panic!("PlatformCoalescedKeyboard creation from non-main thread failed");
    }
}
