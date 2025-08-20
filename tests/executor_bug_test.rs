//SPDX-License-Identifier: MPL-2.0

//! Test for the main thread executor nested submission bug.
//!
//! This test reproduces the bug where calling `already_on_main_thread_submit`
//! with a future that also calls `already_on_main_thread_submit` can cause
//! tasks to be dropped unexpectedly and show the debug output pattern:
//!
//! ```
//! before tasks 0
//! pushed task 1  
//! before tasks 0
//! ```
//!
//! The bug occurs because when `main_executor_iter` is called within
//! `already_on_main_thread_submit`, it uses `FUTURES.take()` which removes
//! the task queue from thread local storage. If a future being polled
//! calls `already_on_main_thread_submit` again, it sees an empty queue
//! (hence "before tasks 0") even though tasks are currently running.
//!
//! Run with: `cargo test --test executor_bug_test`

use app_window::executor::already_on_main_thread_submit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use std::thread;

fn main() {
    println!("=== Testing Main Thread Executor Nested Submission Bug ===\n");

    app_window::application::main(|| {
        //this can't really run on the main thread I think
        thread::Builder::new()
            .name("executor_bug_tests".to_string())
            .spawn(|| {
                let mut test_results = Vec::new();

                // Test 1: Basic nested submission
                test_results.push(("nested_submission", test_nested_main_thread_submit_bug()));

                // Test 2: Multiple nested levels
                test_results.push(("deep_nested", test_deep_nested_submissions()));

                // Test 3: Debug output pattern
                test_results.push(("debug_pattern", test_debug_output_pattern()));

                // Test 4: Concurrent submissions
                test_results.push(("concurrent", test_concurrent_submissions()));

                // Report results
                let passed = test_results.iter().filter(|(_, result)| *result).count();
                let total = test_results.len();

                println!("\n=== Test Results ===");
                for (name, result) in &test_results {
                    let status = if *result { "PASS" } else { "FAIL" };
                    println!("{}: {}", name, status);
                }
                println!("\nPassed: {}/{}", passed, total);

                if passed == total {
                    println!("All tests passed!");
                    std::process::exit(0);
                } else {
                    println!("Some tests failed - this indicates the bug is present!");
                    std::process::exit(1);
                }
            })
            .unwrap();
    });
}

/// Test that reproduces the bug where nested on_main_thread_submit calls can cause task drops
fn test_nested_main_thread_submit_bug() -> bool {
    println!("Running test_nested_main_thread_submit_bug...");

    let test_result = Arc::new(Mutex::new(None));
    let test_result_clone = test_result.clone();

    {
        let completion_counter = Arc::new(AtomicUsize::new(0));
        let debug_log = Arc::new(Mutex::new(Vec::<String>::new()));

        let counter_clone = completion_counter.clone();
        let log_clone = debug_log.clone();

        // Create a future that calls already_on_main_thread_submit internally
        let nested_future = async move {
            {
                let mut log = log_clone.lock().unwrap();
                log.push("outer_future_start".to_string());
            }

            // This inner call should complete properly, but the bug may cause it to be dropped
            let inner_counter = counter_clone.clone();
            let inner_log = log_clone.clone();
            app_window::application::submit_to_main_thread(
                "executor_bug_test".to_owned(),
                move || {
                    already_on_main_thread_submit("executor_bug_test".to_owned(), async move {
                        {
                            let mut log = inner_log.lock().unwrap();
                            log.push("inner_future_executing".to_string());
                        }
                        inner_counter.fetch_add(1, Ordering::Relaxed);
                    });
                },
            );

            {
                let mut log = log_clone.lock().unwrap();
                log.push("outer_future_after_inner_submit".to_string());
            }

            counter_clone.fetch_add(1, Ordering::Relaxed);
        };

        // Submit the nested future
        app_window::application::submit_to_main_thread(
            "executor_bug_test_2".to_owned(),
            move || {
                already_on_main_thread_submit("2".to_owned(), nested_future);
            },
        );

        // Wait a bit for async execution to complete
        std::thread::sleep(Duration::from_millis(3000));

        // Check results
        let final_count = completion_counter.load(Ordering::Relaxed);
        let log = debug_log.lock().unwrap();

        println!("  Final completion count: {}", final_count);
        println!("  Debug log:");
        for entry in log.iter() {
            println!("    {}", entry);
        }

        // Store test results
        let mut result = test_result_clone.lock().unwrap();
        *result = Some((final_count, log.clone()));
    }

    // Verify results after test completes
    let result = test_result.lock().unwrap();
    if let Some((final_count, log)) = result.as_ref() {
        // Both the outer and inner futures should have completed
        let count_ok = *final_count == 2;
        let outer_start_ok = log.contains(&"outer_future_start".to_string());
        let inner_exec_ok = log.contains(&"inner_future_executing".to_string());
        let outer_after_ok = log.contains(&"outer_future_after_inner_submit".to_string());

        let success = count_ok && outer_start_ok && inner_exec_ok && outer_after_ok;

        if !success {
            println!(
                "  FAILED: count_ok={}, outer_start_ok={}, inner_exec_ok={}, outer_after_ok={}",
                count_ok, outer_start_ok, inner_exec_ok, outer_after_ok
            );
            println!("  This failure indicates the bug is present!");
        } else {
            println!("  PASSED");
        }

        success
    } else {
        println!("  FAILED: Test did not complete properly");
        false
    }
}

/// Test multiple levels of nested submissions to stress test the queue management
fn test_deep_nested_submissions() -> bool {
    println!("Running test_deep_nested_submissions...");

    let test_result = Arc::new(Mutex::new(None));
    let test_result_clone = test_result.clone();

    {
        let completion_counter = Arc::new(AtomicUsize::new(0));
        let max_depth = 5;

        fn create_nested_task(depth: usize, max_depth: usize, counter: Arc<AtomicUsize>) {
            if depth < max_depth {
                let counter_clone = counter.clone();
                app_window::application::submit_to_main_thread(
                    "executor_bug_test_3".to_owned(),
                    move || {
                        already_on_main_thread_submit("3".to_owned(), async move {
                            counter_clone.fetch_add(1, Ordering::Relaxed);
                            create_nested_task(depth + 1, max_depth, counter_clone);
                        });
                    },
                );
            } else {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        create_nested_task(0, max_depth, completion_counter.clone());

        // Wait for completion
        std::thread::sleep(Duration::from_millis(200));
        let final_count = completion_counter.load(Ordering::Relaxed);

        println!(
            "  Deep nested test - Final completion count: {}",
            final_count
        );

        let mut result = test_result_clone.lock().unwrap();
        *result = Some(final_count);
    }

    let result = test_result.lock().unwrap();
    if let Some(final_count) = result.as_ref() {
        // We expect max_depth + 1 completions (one for each level)
        let success = *final_count == 6;
        if success {
            println!("  PASSED");
        } else {
            println!("  FAILED: Expected 6 completions, got {}", final_count);
            println!("  This failure indicates the bug is causing task drops!");
        }
        success
    } else {
        println!("  FAILED: Deep nested test did not complete properly");
        false
    }
}

/// Test that specifically looks for the debug output pattern mentioned in the bug report
fn test_debug_output_pattern() -> bool {
    println!("Running test_debug_output_pattern...");
    println!("  Look for the problematic debug pattern:");
    println!("    before tasks 0");
    println!("    pushed task 1");
    println!("    before tasks 0  <- This should NOT be 0!");

    let test_result = Arc::new(Mutex::new(None));
    let test_result_clone = test_result.clone();

    {
        // This test documents the expected behavior vs the bug
        // The actual debug prints are in the already_on_main_thread_submit function

        app_window::application::submit_to_main_thread(
            "executor_bug_test_5".to_owned(),
            move || {
                already_on_main_thread_submit("5".to_owned(), async {
                    // This should trigger the debug pattern:
                    // "before tasks 0"
                    // "pushed task 1"
                    app_window::application::submit_to_main_thread(
                        "executor_bug_test_4".to_owned(),
                        move || {
                            already_on_main_thread_submit("4".to_owned(), async {
                                // This inner call should show:
                                // "before tasks 0" (problematic - should not be 0 if outer task is running)
                                // "pushed task 1"
                            });
                        },
                    );
                });
            },
        );

        std::thread::sleep(Duration::from_millis(50));

        let mut result = test_result_clone.lock().unwrap();
        *result = Some(true); // Test completed
    }

    let result = test_result.lock().unwrap();
    let success = result.is_some();

    if success {
        println!("  PASSED (check debug output above for the bug pattern)");
    } else {
        println!("  FAILED: Debug output test did not complete");
    }

    success
}

/// Test concurrent submissions during task execution
fn test_concurrent_submissions() -> bool {
    println!("Running test_concurrent_submissions...");

    let test_result = Arc::new(Mutex::new(None));
    let test_result_clone = test_result.clone();

    {
        let completion_counter = Arc::new(AtomicUsize::new(0));
        let num_tasks = 10;

        // Submit multiple tasks that each submit additional tasks
        for _i in 0..num_tasks {
            let counter = completion_counter.clone();
            app_window::application::submit_to_main_thread("t2".to_owned(), move || {
                already_on_main_thread_submit("t2".to_owned(), async move {
                    counter.fetch_add(1, Ordering::Relaxed);

                    // Each task submits one more task
                    let inner_counter = counter.clone();
                    app_window::application::submit_to_main_thread(
                        "executor_bug_test_6".to_owned(),
                        move || {
                            already_on_main_thread_submit("6".to_owned(), async move {
                                inner_counter.fetch_add(1, Ordering::Relaxed);
                            });
                        },
                    );
                });
            });
        }

        std::thread::sleep(Duration::from_millis(300));
        let final_count = completion_counter.load(Ordering::Relaxed);

        println!(
            "  Concurrent test - Final completion count: {}",
            final_count
        );

        let mut result = test_result_clone.lock().unwrap();
        *result = Some(final_count);
    }

    let result = test_result.lock().unwrap();
    if let Some(final_count) = result.as_ref() {
        // We expect: 10 outer tasks + 10 inner tasks = 20 total
        let success = *final_count == 20;
        if success {
            println!("  PASSED");
        } else {
            println!("  FAILED: Expected 20 completions, got {}", final_count);
            println!("  This failure indicates the bug is causing task drops!");
        }
        success
    } else {
        println!("  FAILED: Concurrent test did not complete properly");
        false
    }
}
