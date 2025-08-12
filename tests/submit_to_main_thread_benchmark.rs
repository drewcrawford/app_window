//SPDX-License-Identifier: MPL-2.0

//! Benchmark test for measuring the time between calling submit_to_main_thread
//! and when the closure starts executing.
//!
//! This test measures the latency of the main thread submission mechanism
//! across different platforms, running multiple iterations and reporting
//! statistics about the timing.
//!
//! Run with: `cargo test --test submit_to_main_thread_benchmark`
//! Run on WASM with: CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner" RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' cargo +nightly test --target wasm32-unknown-unknown -Z build-std=std,panic_abort

use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use some_executor::task::{Configuration, Task};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const NUM_ITERATIONS: usize = 50;

struct TimingStats {
    samples: Vec<Duration>,
}

impl TimingStats {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    fn add_sample(&mut self, duration: Duration) {
        self.samples.push(duration);
    }

    fn report(&self) {
        if self.samples.is_empty() {
            logwise::error_sync!("No samples collected!");
            return;
        }

        let total: Duration = self.samples.iter().sum();
        let avg = total / self.samples.len() as u32;

        let min = self.samples.iter().min().unwrap();
        let max = self.samples.iter().max().unwrap();

        let avg_micros = avg.as_micros() as f64;
        let variance = self
            .samples
            .iter()
            .map(|d| {
                let diff = d.as_micros() as f64 - avg_micros;
                diff * diff
            })
            .sum::<f64>()
            / self.samples.len() as f64;
        let std_dev = variance.sqrt();

        logwise::warn_sync!("=== Timing Statistics ===");
        logwise::warn_sync!("Samples: {samples}", samples = self.samples.len());
        logwise::warn_sync!("Average: {avg}µs", avg = format!("{:.3}", avg_micros));
        logwise::warn_sync!(
            "Min: {min}µs",
            min = format!("{:.3}", min.as_micros() as f64)
        );
        logwise::warn_sync!(
            "Max: {max}µs",
            max = format!("{:.3}", max.as_micros() as f64)
        );
        logwise::warn_sync!("Std Dev: {std_dev}µs", std_dev = format!("{:.3}", std_dev));

        // Show distribution
        logwise::warn_sync!("Distribution:");
        let buckets = [
            (0.0, 10.0, "  <10µs"),
            (10.0, 50.0, " 10-50µs"),
            (50.0, 100.0, "50-100µs"),
            (100.0, 500.0, "100-500µs"),
            (500.0, 1000.0, "500µs-1ms"),
            (1000.0, f64::INFINITY, "  >1ms"),
        ];

        for (min_us, max_us, label) in &buckets {
            let count = self
                .samples
                .iter()
                .filter(|d| {
                    let us = d.as_micros() as f64;
                    us >= *min_us && us < *max_us
                })
                .count();
            if count > 0 {
                let percentage = (count as f64 / self.samples.len() as f64) * 100.0;
                logwise::warn_sync!(
                    "{label}: {count} ({percentage}%)",
                    label = *label,
                    count = format!("{:3}", count),
                    percentage = format!("{:5.1}", percentage)
                );
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {

    logwise::warn_sync!("=== submit_to_main_thread Latency Benchmark ===");

    app_window::application::main(|| {
        thread::spawn(|| {
            let t = Task::without_notifications(
                "submit_to_main_thread_benchmark".to_string(),
                Configuration::default(),
                async {
                    run_benchmark().await;
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
    let (c, r) = r#continue::continuation();

    app_window::application::main(move || {
        logwise::warn_sync!("=== submit_to_main_thread_benchmark ===");

        let t = Task::without_notifications(
            "submit_to_main_thread_benchmark".to_string(),
            Configuration::default(),
            async move {
                logwise::info_sync!("WASM main thread started");
                run_benchmark().await;
                c.send(());
            },
        );
        t.spawn_static_current();
    });

    r.await;
}

async fn run_benchmark() {
    logwise::warn_sync!(
        "\nRunning {iterations} iterations...",
        iterations = NUM_ITERATIONS
    );

    let mut stats = TimingStats::new();

    let mut senders = Vec::new();
    let mut futures = Vec::new();
    for i in 0..NUM_ITERATIONS {
        let (tx, rx) = r#continue::continuation();
        senders.push(tx);
        futures.push(rx);
    }

    thread::spawn(move || {
        logwise::info_sync!("Thread up");
        for (s, sender) in senders.drain(..).enumerate() {
            // Record time just before submission
            let start_time = Instant::now();

            app_window::application::submit_to_main_thread("t1", move || {
                // Record time when closure starts executing
                let elapsed = start_time.elapsed();

                // Send the result back
                sender.send((s, elapsed));
            });

            // Wait a bit between submissions to get clean measurements
            thread::sleep(Duration::from_millis(60));
        }
    });

    // Collect results
    for recv in futures {
        logwise::warn_sync!("Will await next result...");
        let r = recv.await;
        stats.add_sample(r.1);
    }

    // Report results
    stats.report();
}
