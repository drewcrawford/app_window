/*!
This module provides test support utilities for accessing the main thread.
*/

/**
A function that runs on the main thread to support doctests.
*/
pub fn doctest_main<C>(c: C)
where C: FnOnce() + 'static + Send {
    eprintln!("doctest_main called");
    if crate::application::IS_MAIN_THREAD_RUNNING.swap(true, std::sync::atomic::Ordering::Relaxed) {
       //main already running
        c()
    }
    else {
        //main not already running
        assert!(crate::sys::is_main_thread(), "doctest_main must be called from the main thread");
        crate::application::main_postlude(|| {
            c();
            crate::sys::stop_main_thread();
        })

    }
}