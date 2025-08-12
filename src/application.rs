//SPDX-License-Identifier: MPL-2.0

//! Application lifecycle and main thread management.
//!
//! This module provides the core functionality for initializing and running the application's
//! event loop. It handles platform-specific requirements for UI event processing and provides
//! utilities for executing code on the main thread.
//!
//! # Main Thread Requirements
//!
//! Many platforms (notably macOS) require that UI operations occur on the main thread. This module
//! provides abstractions to handle these requirements portably across all platforms.
//!
//! # Getting Started
//!
//! Every application using `app_window` must call [`main`](crate::application::main) exactly once from the first thread
//! of the program. This initializes the platform event loop and allows windows to be created.
//!
//! ```no_run
//! app_window::application::main(|| {
//!     println!("Application initialized!");
//!     // Your application setup code here
//! });
//! ```

use std::sync::atomic::AtomicBool;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use std::time;
#[cfg(target_arch = "wasm32")]
pub(crate) use web_time as time;

use crate::sys;

static IS_MAIN_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) const CALL_MAIN: &str = "Call app_window::application::main";

/// Initializes and runs the application event loop.
///
/// This function sets up the platform-specific event loop and must be called exactly once
/// from the first thread in your program. It turns the calling thread into the main event
/// loop thread.
///
/// # Arguments
///
/// * `closure` - A function or closure that will be executed once the event loop is ready.
///   On platforms where the main thread must handle events, this closure runs on a
///   secondary thread.
///
/// # Platform Behavior
///
/// - **macOS/Windows/Linux**: Parks the calling thread to process UI events
/// - **WebAssembly**: May return immediately after setting up event handlers
///
/// # Panics
///
/// - If called from any thread other than the first thread
/// - If called more than once
///
/// # Examples
///
/// Basic application setup:
///
/// ```no_run
/// app_window::application::main(|| {
///     println!("Event loop is ready!");
/// });
/// ```
///
/// Creating a window after initialization:
///
/// ```no_run
/// use app_window::coordinates::{Position, Size};
///
/// app_window::application::main(|| {
///     # #[cfg(feature = "some_executor")]
///     # {
///     // Spawn an async task to create a window
///     let task = async {
///         let window = app_window::window::Window::new(
///             Position::new(100.0, 100.0),
///             Size::new(800.0, 600.0),
///             "My Window".to_string()
///         ).await;
///         
///         // Keep window alive
///         std::mem::forget(window);
///     };
///     
///     // In a real app, spawn this with your executor
///     # }
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
/// This internal function is used to verify that [`main()`] has been called before
/// attempting operations that require the event loop.
pub(crate) fn is_main_thread_running() -> bool {
    IS_MAIN_THREAD_RUNNING.load(std::sync::atomic::Ordering::Acquire)
}

/// Executes a closure on the main thread and returns its result.
///
/// This async function allows code running on any thread to execute operations that must
/// occur on the main thread. The closure is scheduled to run on the main thread, and this
/// function waits for its completion.
///
/// # Type Parameters
///
/// * `R` - The return type of the closure (must be `Send`)
/// * `F` - The closure type
///
/// # Arguments
///
/// * `closure` - A function or closure to execute on the main thread
///
/// # Returns
///
/// The value returned by the closure
///
/// # Performance Considerations
///
/// The main thread may be blocked or unable to process other events while the closure
/// is executing. Keep main thread operations brief to maintain UI responsiveness.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use app_window::application;
///
/// // Execute a UI operation on the main thread
/// let result = application::on_main_thread("ex".to_owned(),|| {
///     println!("Running on main thread!");
///     42
/// }).await;
///
/// assert_eq!(result, 42);
/// # }
/// ```
///
/// Platform-specific operations:
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use app_window::application;
///
/// // Many UI operations must happen on the main thread
/// let window_title = application::on_main_thread("ex".to_owned(),|| {
///     // Platform-specific window operations would go here
///     "Main Window".to_string()
/// }).await;
/// # Ok(())
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

/// Submits a closure to be executed on the main thread.
///
/// This function provides the low-level mechanism for scheduling work on the
/// main thread. Unlike [`on_main_thread()`], this function does not wait for the closure
/// to complete or return a result.
///
/// # Arguments
///
/// * `closure` - A function or closure to execute on the main thread
///
/// # Implementation Notes
///
/// This function handles platform-specific details of main thread execution and may
/// install a main thread executor if necessary for the current platform.
pub fn submit_to_main_thread<F: FnOnce() + Send + 'static>(debug_label: String, closure: F) {
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

/**
Checks if the current thread is the main thread.
*/
pub fn is_main_thread() -> bool {
    sys::is_main_thread()
}
