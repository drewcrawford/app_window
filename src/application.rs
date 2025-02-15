use std::sync::atomic::AtomicBool;

use crate::sys;

static IS_MAIN_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) const CALL_MAIN: &'static str = "Call app_window::application::run_main_thread";


/**
Performs the runloop or event loop.

Call this function exactly once, from the first thread in your program.

On most platforms, this function parks the thread, possibly in a platform-specific way to receive UI events.

On platforms like wasm, this function may return immediately.

# Arguments

Accepts a function/closure that will be run "once the main loop is ready".  On platforms where
the main thread must be parked, this closure will be invoked on a secondary thread.


# Discussion

On many platforms, UI needs some kind of application-wide runloop or event loop.  Calling this function
turns the current thread into that runloop (on platforms where this is necessary).

Many platforms, such as macOS, require that the first thread created by the application perform the runloop
(you can't do it on an arbitrary thread).  Accordingly on all platforms, require this function to be called
from the first thread.


*/
pub fn main<F: FnOnce() -> () + Send + 'static>(closure: F) {
    assert!(sys::is_main_thread(), "Call main from the first thread");
    let old = IS_MAIN_THREAD_RUNNING.swap(true, std::sync::atomic::Ordering::Release);

    assert!(!old, "Do not call main more than once.");
    #[cfg(feature = "some_executor")] {
        use crate::some_executor::MainThreadExecutor;
        some_executor::thread_executor::set_thread_executor(Box::new(MainThreadExecutor{}));
        some_executor::thread_executor::set_thread_local_executor_adapting_notifier(MainThreadExecutor{});
    }
    sys::run_main_thread(closure);
}

/**
Determines if the main thread was started.
*/
pub(crate) fn is_main_thread_running() -> bool {
    IS_MAIN_THREAD_RUNNING.load(std::sync::atomic::Ordering::Acquire)
}

/**
Run the specified closure on the main thread.

The main thread may be blocked or unable to receive events while this closure is running.
*/
pub async fn on_main_thread<R: Send + 'static,F: FnOnce() -> R + Send + 'static>(closure: F) -> R {
    let(sender,receiver) = r#continue::continuation();
    let block = move ||{
        let r = closure();
        sender.send(r);
    };

    submit_to_main_thread(block);
    receiver.await
}

/**
Submits the closure to the main thread, installing a main thread executor if necessary.
*/

pub(crate) fn submit_to_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    sys::on_main_thread(closure);
}

