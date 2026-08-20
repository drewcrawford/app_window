// SPDX-License-Identifier: MPL-2.0
//! The `windows` and `main_thread` subsystems, against the real platform loop.
//!
//! Both providers are only meaningful once `application::main` is running --
//! a window cannot be created before it, and the main-thread probe declines to
//! submit anything. So this drives the real event loop rather than a stand-in,
//! which is why it is `harness = false` like every other test here.
//!
//! Run with: `cargo test --features exfiltrate --test exfiltrate_snapshot_test`
//! On WASM: `scripts/wasm32/tests --features exfiltrate --test exfiltrate_snapshot_test`

// `harness = false` means this file owns `fn main`, so the feature gate cannot
// be a file-level `#![cfg]` -- that would delete `main` along with everything
// else and the target would not link. The stub at the bottom is the other half.
#![cfg_attr(not(feature = "exfiltrate"), allow(unused))]

#[cfg(feature = "exfiltrate")]
mod enabled {
    use app_window::coordinates::{Position, Size};
    use app_window::window::Window;
    use some_executor::task::{Configuration, Task};

    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;

    /// Asks the registry the same question the provider does, and returns the row
    /// for one window as a debug string.
    ///
    /// The provider's own output is `SnapshotResponse`, which only exfiltrate can
    /// build a request for; going through the registry asserts the data the
    /// provider projects, which is where every bug this test could find lives.
    fn window_row(id: u64) -> Option<app_window::registry::Entry> {
        app_window::registry::entries()
            .expect("the registry is not locked in a single-threaded test")
            .into_iter()
            .find(|entry| entry.id == id)
    }

    fn newest_window() -> app_window::registry::Entry {
        app_window::registry::entries()
            .expect("the registry is not locked in a single-threaded test")
            .pop()
            .expect("a window was just created")
    }

    async fn check_snapshot_contract() {
        // -- the window registry --------------------------------------------------

        let before = app_window::registry::entries().unwrap().len();

        let mut window = Window::new(
            Position::new(100.0, 100.0),
            Size::new(640.0, 480.0),
            "exfiltrate_snapshot_test".to_string(),
        )
        .await;

        let entry = newest_window();
        let id = entry.id;
        assert_eq!(
            app_window::registry::entries().unwrap().len(),
            before + 1,
            "creating a window records exactly one entry"
        );
        assert!(entry.is_open(), "a window that was just created is open");
        assert_eq!(entry.origin, app_window::registry::Origin::Requested);
        assert_eq!(entry.title, "exfiltrate_snapshot_test");
        assert_eq!(
            entry
                .requested
                .map(|(_, size)| (size.width(), size.height())),
            Some((640.0, 480.0)),
            "the requested size is what the caller asked for"
        );
        assert!(
            !entry.surface_attached,
            "no surface has been created for this window yet"
        );
        assert!(
            entry.last_observed.is_none(),
            "nothing has observed this surface's geometry yet"
        );

        // -- a surface, and the geometry the application observes -----------------

        let surface = window.surface().await;
        assert!(
            window_row(id).unwrap().surface_attached,
            "creating a surface is recorded against its window"
        );

        let (size, scale) = surface.size_scale().await;
        let observed = window_row(id).unwrap().last_observed;
        let (recorded_size, recorded_scale, _) =
            observed.expect("asking the surface for its size records what was learned");
        assert_eq!(
            (recorded_size.width(), recorded_size.height()),
            (size.width(), size.height()),
            "the recorded geometry is the value the application was handed"
        );
        assert_eq!(recorded_scale, scale);

        // -- the main thread ------------------------------------------------------
        //
        // Everything above went through `on_main_thread`, so by now the counters
        // have moved and a turn has completed.

        let stats = app_window::instrument::stats();
        assert!(stats.running, "the event loop is running; we are inside it");
        assert!(
            stats.submitted > 0 && stats.completed > 0,
            "creating a window and a surface goes through the main thread: {stats:?}"
        );
        assert!(
            stats.since_last_turn.is_some(),
            "a completed turn stamps the clock: {stats:?}"
        );

        // The probe is fire-and-forget by design, so this is two steps: start one,
        // then let the main thread answer it.
        app_window::instrument::probe();
        let answered = loop {
            let stats = app_window::instrument::stats();
            if stats.probe_outstanding.is_none() {
                break stats;
            }
            assert!(
                stats.probe_outstanding.unwrap() < std::time::Duration::from_secs(5),
                "the main thread did not answer a do-nothing closure in 5s: {stats:?}"
            );
            wasm_lite_std::sleep_async(std::time::Duration::from_millis(10)).await;
        };
        assert!(
            answered.last_probe_round_trip.is_some(),
            "an answered probe records its round trip: {answered:?}"
        );

        // -- closing --------------------------------------------------------------

        drop(surface);
        drop(window);
        let closed = window_row(id).expect("the record outlives the window");
        assert!(
            !closed.is_open(),
            "dropping a window closes it, and the record says so"
        );
        assert!(closed.closed_at.is_some());

        // `harness = false`, so nothing else says this ran.
        println!("app_window exfiltrate snapshot contract: OK");
    }

    // -- entry points -------------------------------------------------------------

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run() {
        app_window::application::main(|| {
            thread::spawn(|| {
                let task = Task::without_notifications(
                    "exfiltrate_snapshot_test".to_string(),
                    Configuration::default(),
                    async {
                        check_snapshot_contract().await;
                        std::process::exit(0);
                    },
                );
                task.spawn_static_current();
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_lite::wasm_lite_test]
    pub fn wasm_main() {
        wasm_lite_std::async_doctest!(async {
            assert!(app_window::application::is_main_thread());
            let (done, wait) = r#continue::continuation();
            app_window::application::main(move || {
                let task = Task::without_notifications(
                    "exfiltrate_snapshot_test".to_string(),
                    Configuration::default(),
                    async move {
                        check_snapshot_contract().await;
                        done.send(());
                    },
                );
                task.spawn_static_current();
            });
            wait.await;
        });
    }
}

#[cfg(all(feature = "exfiltrate", target_arch = "wasm32"))]
wasm_lite::test_main!();

/// Without the feature there is nothing to test, but `harness = false` still
/// requires the target to link.
#[cfg(all(not(feature = "exfiltrate"), not(target_arch = "wasm32")))]
fn main() {}

#[cfg(all(not(feature = "exfiltrate"), target_arch = "wasm32"))]
fn main() {}

#[cfg(all(feature = "exfiltrate", not(target_arch = "wasm32")))]
fn main() {
    enabled::run();
}
