//SPDX-License-Identifier: MPL-2.0

//! Application lifecycle and main thread management.
//!
//! This module is the foundation of `app_window`, providing essential initialization and
//! platform-specific event loop management. It handles the complex threading requirements
//! across different platforms, ensuring UI operations execute correctly while maintaining
//! a consistent async-first API.
//!
//! # Overview
//!
//! The module provides three key capabilities:
//! 
//! 1. **Application initialization** via [`main()`] - Sets up the platform event loop
//! 2. **Main thread execution** via [`on_main_thread()`] - Runs async operations on the UI thread
//! 3. **Direct submission** via [`submit_to_main_thread()`] - Fire-and-forget main thread tasks
//!
//! # Platform Threading Models
//!
//! Different platforms have vastly different requirements for UI operations:
//!
//! | Platform | Main Thread Requirement | Event Loop Model |
//! |----------|------------------------|------------------|
//! | macOS    | **Strict** - All UI operations must be on main thread | AppKit runs on main thread |
//! | Windows  | **Flexible** - UI can be on any thread with proper setup | Message pump per thread |
//! | Linux    | **Flexible** - Wayland allows multi-threaded operations | Event queue per connection |
//! | WASM     | **Single** - Browser is single-threaded | Event callbacks only |
//!
//! This module abstracts these differences, providing a uniform API that works correctly
//! on all platforms.
//!
//! # Getting Started
//!
//! Every `app_window` application follows this pattern:
//!
//! ```no_run
//! // ALLOW_NORUN_DOCTEST: Requires full application initialization which blocks the thread
//! use app_window::application;
//!
//! fn main() {
//!     // Must be called from the program's first thread
//!     application::main(|| {
//!         // This closure runs once the event loop is ready
//!         // On platforms with strict main thread requirements,
//!         // this runs on a worker thread
//!         
//!         println!("Application initialized!");
//!         
//!         // You can spawn async tasks here using your preferred executor
//!         // For example, with tokio:
//!         // tokio::spawn(async { /* ... */ });
//!     });
//! }
//! ```
//!
//! # Async Design
//!
//! This crate is async-first to handle platform threading differences uniformly.
//! All potentially blocking operations are async, allowing them to coordinate
//! with the main thread when necessary:
//!
//! ```
//! # async fn example() {
//! use app_window::application;
//!
//! // Can be called from any thread after initialization
//! let result = application::on_main_thread("calculation".to_owned(), || {
//!     // This runs on the main thread
//!     expensive_ui_calculation()
//! }).await;
//! # }
//! # fn expensive_ui_calculation() -> i32 { 42 }
//! ```
//!
//! # Integration with Executors
//!
//! When `some_executor` support is enabled, this module automatically installs
//! a [`MainThreadExecutor`](crate::some_executor::MainThreadExecutor) that can
//! spawn futures on the main thread. This integrates seamlessly with the
//! `some_executor` crate's executor-agnostic task spawning.
//!
//! # Performance Monitoring
//!
//! The module includes built-in performance monitoring for main thread operations.
//! Operations taking longer than 10ms will generate warnings via `logwise`,
//! helping identify UI responsiveness issues:
//!
//! ```text
//! WARN: Main thread operation took too long: 15.2ms
//! ```
//!
//! Keep main thread operations brief to maintain smooth UI performance.
//!
//! # Error Handling
//!
//! Most functions in this module will panic if [`main()`] hasn't been called yet.
//! This is intentional as it represents a programming error. Always ensure
//! [`main()`] is called at the start of your program.

use std::sync::atomic::AtomicBool;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use std::time;
#[cfg(target_arch = "wasm32")]
pub(crate) use web_time as time;

use crate::sys;

static IS_MAIN_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

/// Error message constant used when operations require initialization.
///
/// This message is used throughout the crate when operations fail because
/// [`main()`] hasn't been called yet. Provides a consistent error message
/// for users.
///
/// # Internal Use
///
/// This constant is `pub(crate)` and not part of the public API. It's used
/// internally by various modules to provide consistent error messages when
/// the application hasn't been properly initialized.
pub(crate) const CALL_MAIN: &str = "Call app_window::application::main";

/// Initializes and runs the application event loop.
///
/// This is the entry point for all `app_window` applications. It must be called
/// exactly once from the program's first thread (the thread that `main()` runs on).
/// This function transforms the calling thread into the platform's UI event loop.
///
/// # Arguments
///
/// * `closure` - A function or closure that executes once the event loop is initialized.
///   This closure is where you set up your application, spawn tasks, and create windows.
///   
///   **Important**: On platforms with strict main thread requirements (macOS),
///   this closure runs on a secondary thread, allowing the main thread to process events.
///
/// # Platform Behavior
///
/// The function behaves differently based on the platform's threading model:
///
/// | Platform | Main Thread Behavior | Closure Execution | Function Returns |
/// |----------|---------------------|-------------------|------------------|
/// | macOS    | Blocks processing AppKit events | Secondary thread | Never (unless app quits) |
/// | Windows  | Blocks processing Win32 messages | Secondary thread | Never (unless app quits) |
/// | Linux    | Blocks processing Wayland events | Secondary thread | Never (unless app quits) |
/// | WASM     | Sets up event callbacks | Same thread | Immediately |
///
/// # Executor Integration
///
/// When the `some_executor` feature is enabled, this function automatically:
/// 1. Installs a [`MainThreadExecutor`](crate::some_executor::MainThreadExecutor)
/// 2. Sets it as both the thread-local and thread-static executor
/// 3. Enables spawning futures that run on the main thread
///
/// # Panics
///
/// This function will panic if:
/// - Called from any thread other than the first thread (checked via [`is_main_thread()`])
/// - Called more than once in the program's lifetime
///
/// # Examples
///
/// ## Basic Setup
///
/// ```no_run
/// // ALLOW_NORUN_DOCTEST: Function blocks indefinitely running the event loop
/// app_window::application::main(|| {
///     println!("Application ready!");
///     // Your app initialization here
/// });
/// ```
///
/// ## With Async Task Spawning
///
/// ```no_run
/// // ALLOW_NORUN_DOCTEST: Function blocks indefinitely running the event loop
/// use app_window::application;
///
/// application::main(|| {
///     println!("Event loop started");
///     
///     // At this point you can:
///     // - Create windows using Window::new()
///     // - Spawn async tasks with your executor
///     // - Set up event handlers
///     // - Initialize your application state
/// });
/// ```
pub fn main<F: FnOnce() + Send + 'static>(closure: F) {
    assert!(sys::is_main_thread(), "Call main from the first thread");
    let old = IS_MAIN_THREAD_RUNNING.swap(true, std::sync::atomic::Ordering::Release);

    assert!(!old, "Do not call main more than once.");

    use crate::some_executor::MainThreadExecutor;
    some_executor::thread_executor::set_thread_local_executor_adapting_notifier(
        MainThreadExecutor {},
    );
    some_executor::thread_executor::set_thread_static_executor_adapting_notifier(
        MainThreadExecutor {},
    );

    sys::run_main_thread(closure);
}

/// Checks if the main thread event loop has been started.
///
/// This internal function verifies that [`main()`] has been called and the
/// event loop is running. Used by other modules to ensure proper initialization
/// before attempting window creation or other operations.
///
/// # Returns
///
/// - `true` if [`main()`] has been called and the event loop is running
/// - `false` if the application hasn't been initialized yet
///
/// # Thread Safety
///
/// Uses `Acquire` ordering to ensure all threads see the initialization
/// state correctly after it's set by [`main()`].
///
/// # Internal Use
///
/// This function is `pub(crate)` and not part of the public API.
/// It's used internally by:
/// - Window creation to ensure the event loop exists
/// - Executor initialization checks
/// - Platform-specific initialization verification
pub(crate) fn is_main_thread_running() -> bool {
    IS_MAIN_THREAD_RUNNING.load(std::sync::atomic::Ordering::Acquire)
}

/// Executes a closure on the main thread and returns its result.
///
/// This async function provides safe, cross-platform access to the main thread from
/// any thread in your application. It's essential for operations that must occur
/// on the UI thread, such as window creation, UI updates, or platform API calls.
///
/// # Type Parameters
///
/// * `R` - The return type of the closure. Must implement `Send` since the result
///   crosses thread boundaries.
/// * `F` - The closure type. Must be `FnOnce() -> R + Send + 'static`.
///
/// # Arguments
///
/// * `debug_label` - A descriptive label for this operation, used for debugging and
///   performance monitoring. This appears in log messages if the operation takes too long.
/// * `closure` - The function or closure to execute on the main thread.
///
/// # Returns
///
/// The value returned by the closure, delivered asynchronously.
///
/// # How It Works
///
/// 1. Creates a continuation channel using `r#continue::continuation()`
/// 2. Wraps your closure to send its result through the channel
/// 3. Submits the wrapped closure to the main thread queue
/// 4. Awaits the result from the receiver
///
/// # Performance Monitoring
///
/// Operations are automatically monitored for performance:
/// - Operations taking >10ms generate a warning via `logwise`
/// - The `debug_label` helps identify slow operations
/// - Each operation runs in a new `logwise` task context for tracing
///
/// # Thread Safety
///
/// This function is safe to call from any thread after [`main()`] has been called.
/// Multiple threads can call this concurrently; operations are queued and executed
/// in order on the main thread.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # async fn example() {
/// use app_window::application;
///
/// let result = application::on_main_thread(
///     "calculate_ui_size".to_owned(),
///     || {
///         // This runs on the main thread
///         let width = 800;
///         let height = 600;
///         width * height
///     }
/// ).await;
///
/// assert_eq!(result, 480000);
/// # }
/// ```
///
/// ## Platform-Specific Operations
///
/// ```
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use app_window::application;
///
/// // Access platform APIs that require main thread
/// let display_count = application::on_main_thread(
///     "get_display_count".to_owned(),
///     || {
///         // Platform-specific code to count displays
///         # 1
///     }
/// ).await;
///
/// println!("Found {} displays", display_count);
/// # Ok(())
/// # }
/// ```
///
/// ## Coordinating with UI Updates
///
/// ```
/// # async fn example() {
/// # struct UIState { title: String }
/// # impl UIState {
/// #     fn update_title(&mut self, title: String) {}
/// #     fn needs_redraw(&self) -> bool { false }
/// #     fn redraw(&mut self) {}
/// # }
/// use app_window::application;
/// use std::sync::{Arc, Mutex};
///
/// let ui_state = Arc::new(Mutex::new(UIState { title: String::new() }));
/// let ui_state_clone = ui_state.clone();
///
/// // Update UI state from a background thread
/// application::on_main_thread(
///     "update_ui_title".to_owned(),
///     move || {
///         let mut state = ui_state_clone.lock().unwrap();
///         state.update_title("New Title".to_string());
///         
///         // Trigger UI redraw if needed
///         if state.needs_redraw() {
///             state.redraw();
///         }
///     }
/// ).await;
/// # }
/// ```
///
/// # Performance Best Practices
///
/// Keep main thread operations brief:
///
/// ```
/// # async fn example() {
/// use app_window::application;
///
/// // ❌ BAD: Long-running computation blocks UI
/// // Avoid doing this:
/// // application::on_main_thread("heavy_work".to_owned(), || {
/// //     for i in 0..1_000_000 {
/// //         // Heavy computation
/// //     }
/// // }).await;
///
/// // ✅ GOOD: Quick UI operation
/// let label = "Clicked";
/// application::on_main_thread("quick_update".to_owned(), move || {
///     // Just update UI state quickly
///     // In a real app, this would update actual UI
///     println!("Button label updated to: {}", label);
/// }).await;
/// # }
/// ```
pub async fn on_main_thread<R: Send + 'static, F: FnOnce() -> R + Send + 'static>(debug_label: String, closure: F) -> R {
    let (sender, receiver) = r#continue::continuation();
    let block = move || {
        let r = closure();
        sender.send(r);
    };

    submit_to_main_thread(debug_label, block);
    receiver.await
}

/// Submits a closure to be executed on the main thread without waiting.
///
/// This is the fire-and-forget variant of [`on_main_thread()`]. Use this when you
/// need to perform main thread operations but don't need the result or completion
/// notification.
///
/// # Arguments
///
/// * `debug_label` - A descriptive label for debugging and performance monitoring.
///   Shows up in logs if the operation takes >10ms.
/// * `closure` - A function or closure to execute on the main thread.
///   Must be `FnOnce() + Send + 'static`.
///
/// # Behavior
///
/// - The closure is queued for execution on the main thread
/// - This function returns immediately without waiting
/// - No way to get the result or know when execution completes
/// - Operations are executed in the order they're submitted
///
/// # Performance Monitoring
///
/// Like [`on_main_thread()`], this function includes automatic performance monitoring:
/// - Creates a new `logwise` task context for the operation
/// - Logs a warning if execution takes >10ms
/// - Preserves the calling context for tracing
///
/// # Use Cases
///
/// This function is ideal for:
/// - UI updates that don't need confirmation
/// - Cleanup operations
/// - Event notifications
/// - Any fire-and-forget main thread work
///
/// # Examples
///
/// ## Basic Fire-and-Forget
///
/// ```no_run
//  //main thread is not running in doctests.
/// use app_window::application;
///
/// // Update UI without waiting
/// application::submit_to_main_thread(
///     "update_status".to_owned(),
///     || {
///         update_status_bar("Operation complete");
///     }
/// );
///
/// // Continue immediately without waiting
/// println!("Submitted UI update");
/// # fn update_status_bar(_: &str) {}
/// ```
///
/// ## Periodic UI Updates
///
/// ```
/// # fn example() {
/// use app_window::application;
///
/// // Submit periodic UI updates from a background task
/// for progress in [0.25, 0.5, 0.75, 1.0] {
///     application::submit_to_main_thread(
///         format!("update_progress_{}", progress),
///         move || {
///             // In a real app, this would update a progress bar UI
///             println!("Progress: {}%", (progress * 100.0) as u32);
///         }
///     );
/// }
/// # }
/// ```
///
/// ## Event Handling
///
/// ```no_run
/// # // main thread is not running in doctests.
/// use app_window::application;
///
/// fn handle_user_input(input: String) {
///     // Process input on current thread
///     let processed = input.to_uppercase();
///     
///     // Update UI on main thread
///     application::submit_to_main_thread(
///         format!("handle_input_{}", processed.clone()),
///         move || {
///             // In a real app, this would update UI elements
///             println!("Displaying result: {}", processed);
///         }
///     );
/// }
///
/// // Example usage
/// handle_user_input("hello".to_string());
/// ```
///
/// # Implementation Details
///
/// This function wraps the closure with performance monitoring before calling
/// the platform-specific `sys::on_main_thread()`. The wrapper:
/// 1. Records the start time
/// 2. Creates a new logwise task context
/// 3. Executes the closure
/// 4. Restores the previous context
/// 5. Logs if execution was slow (>10ms)
pub fn submit_to_main_thread<F: FnOnce() + Send + 'static>(debug_label: String, closure: F) {
    assert!(is_main_thread_running(), "{}",CALL_MAIN);
    let perf = move || {
        let start = time::Instant::now();
        let prior = logwise::context::Context::current();
        let c = logwise::context::Context::new_task(Some(prior.clone()), debug_label.clone());
        c.set_current();
        closure();
        prior.set_current();

        let duration = start.elapsed();
        if duration > time::Duration::from_millis(10) {
            logwise::warn_sync!(
                "Main thread operation took too long: {duration}\n",
                duration = logwise::privacy::LogIt(duration),
                debug_label = logwise::privacy::IPromiseItsNotPrivate(debug_label)
            );
        }
    };
    sys::on_main_thread(perf);
    // sys::on_main_thread(closure);
}

/// Checks if the current thread is the main thread.
///
/// Returns `true` if called from the main thread (the thread that called
/// [`main()`]), `false` otherwise.
///
/// # Platform Implementation
///
/// This delegates to the platform-specific `sys::is_main_thread()`,
/// which uses:
/// - **macOS**: `pthread_main_np()` or Swift runtime checks
/// - **Windows**: Thread ID comparison with the initial thread
/// - **Linux**: Thread ID comparison
/// - **WASM**: Always returns `true` (single-threaded)
///
/// # Examples
///
/// ```
/// use app_window::application;
///
/// if application::is_main_thread() {
///     println!("Running on the main thread");
/// } else {
///     println!("Running on a background thread");
/// }
/// ```
///
/// ## Conditional Execution
///
/// ```
/// # async fn example() {
/// use app_window::application;
///
/// async fn do_ui_operation() {
///     if application::is_main_thread() {
///         // Already on main thread, execute directly
///         perform_ui_update();
///     } else {
///         // Need to switch to main thread
///         application::on_main_thread(
///             "ui_update".to_owned(),
///             || perform_ui_update()
///         ).await;
///     }
/// }
///
/// fn perform_ui_update() {
///     // In a real app, this would update UI elements
///     println!("UI updated");
/// }
/// # }
/// ```
pub fn is_main_thread() -> bool {
    sys::is_main_thread()
}
