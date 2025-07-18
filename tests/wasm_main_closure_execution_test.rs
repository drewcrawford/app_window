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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn main() {
    println!("=== Testing app_window::application::main() Closure Execution ===\n");

    // Test that verifies the closure actually executes
    let success = test_main_closure_execution();
    
    if success {
        println!("\n✓ Test PASSED: Closure executed successfully");
        #[cfg(not(target_arch = "wasm32"))]
        std::process::exit(0);
    } else {
        println!("\n✗ Test FAILED: Closure did not execute within timeout");
        println!("This indicates the WASM bug is present!");
        #[cfg(not(target_arch = "wasm32"))]
        std::process::exit(1);
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // On WASM, let the test framework handle the success/failure
        if !success {
            panic!("Test failed: Closure did not execute within timeout");
        }
    }
}

/// Test that verifies the closure passed to application::main() actually executes
fn test_main_closure_execution() -> bool {
    println!("Running test_main_closure_execution...");
    
    // Shared state to track closure execution
    let closure_executed = Arc::new(AtomicBool::new(false));
    let execution_count = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();
    
    let closure_executed_clone = closure_executed.clone();
    let execution_count_clone = execution_count.clone();
    
    println!("About to call app_window::application::main()");
    
    // This is where the bug occurs - the closure may never execute on WASM
    app_window::application::main(move || {
        println!("🎯 Inside app_window::application::main() closure - EXECUTING!");
        
        // Mark that we successfully entered the closure
        closure_executed_clone.store(true, Ordering::Relaxed);
        execution_count_clone.fetch_add(1, Ordering::Relaxed);
        
        let elapsed = start_time.elapsed();
        println!("Closure executed after {:?}", elapsed);
        
        // Test completed successfully
        println!("Closure execution test completed successfully");
        
        // On native platforms, main() blocks forever, so we need to exit here
        // to avoid hanging the test
        #[cfg(not(target_arch = "wasm32"))]
        {
            println!("✓ Native platform: Closure executed successfully!");
            std::process::exit(0);
        }
    });
    
    println!("Returned from app_window::application::main()");
    
    // Wait for closure to execute with timeout
    let timeout = Duration::from_millis(500);
    let poll_interval = Duration::from_millis(10);
    let start_wait = Instant::now();

    println!("Waiting up to {:?} for closure to execute...", timeout);

    while start_wait.elapsed() < timeout {
        if closure_executed.load(Ordering::Relaxed) {
            let total_time = start_time.elapsed();
            let wait_time = start_wait.elapsed();
            let count = execution_count.load(Ordering::Relaxed);

            println!("✓ SUCCESS: Closure executed!");
            println!("  Total time from start: {:?}", total_time);
            println!("  Wait time: {:?}", wait_time);
            println!("  Execution count: {}", count);
            return true;
        }

        // Small delay to avoid busy waiting
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(poll_interval);

        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, we can't sleep, so we just continue polling
            // This might make the test less reliable on WASM, but it's better than hanging
        }
    }

    // Timeout reached without closure execution
    let wait_time = start_wait.elapsed();
    let count = execution_count.load(Ordering::Relaxed);

    println!("✗ TIMEOUT: Closure did not execute within {:?}", timeout);
    println!("  Total wait time: {:?}", wait_time);
    println!("  Execution count: {}", count);
    println!("  This indicates the bug is present!");
    
    false
}

/// Additional test with more detailed timing information
#[allow(dead_code)]
fn test_main_closure_timing_detailed() -> bool {
    println!("Running detailed timing test...");
    
    let start_time = Instant::now();
    let closure_start_time = Arc::new(std::sync::Mutex::new(None));
    let closure_end_time = Arc::new(std::sync::Mutex::new(None));
    
    let closure_start_clone = closure_start_time.clone();
    let closure_end_clone = closure_end_time.clone();
    
    println!("T+{:?}: About to call application::main()", start_time.elapsed());
    
    app_window::application::main(move || {
        let now = Instant::now();
        *closure_start_clone.lock().unwrap() = Some(now);
        
        println!("T+{:?}: 🎯 CLOSURE START", start_time.elapsed());
        
        // Simulate some work
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(Duration::from_millis(1));
        
        println!("T+{:?}: 🎯 CLOSURE END", start_time.elapsed());
        
        *closure_end_clone.lock().unwrap() = Some(Instant::now());
    });
    
    println!("T+{:?}: Returned from application::main()", start_time.elapsed());
    
    // Wait and check results
    let timeout = Duration::from_millis(1000);
    let start_wait = Instant::now();
    
    while start_wait.elapsed() < timeout {
        let start_opt = closure_start_time.lock().unwrap().clone();
        let end_opt = closure_end_time.lock().unwrap().clone();
        
        if let (Some(start), Some(end)) = (start_opt, end_opt) {
            let closure_duration = end.duration_since(start);
            let total_time = end.duration_since(start_time);
            
            println!("✓ Detailed timing SUCCESS:");
            println!("  Closure duration: {:?}", closure_duration);
            println!("  Total time: {:?}", total_time);
            return true;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(Duration::from_millis(10));
    }
    
    println!("✗ Detailed timing FAILED: Timeout waiting for closure");
    false
}