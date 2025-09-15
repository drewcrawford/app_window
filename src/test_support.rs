/*!
This module provides test support utilities working with the main thread.

Your main options are:

* doctests using [`doctest_main`]
* integration tests

Examples using this module are kept in the `examples/test_example` crate.
*/

/**
A custom test harness for integration tests.

The test harness will:

1. Bring up the main thread environment
2. Run the provided closure
3. Tear down the main thread environment

# Example

To use this, add the following to your `Cargo.toml`:
```toml
[[test]]
name = "your_custom_test_name"
path = "tests/your_custom_test_name.rs"
harness = false
```

Then in `tests/your_custom_test_name.rs`:
```rust,no_run
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn main() {
    app_window::test_support::integration_test_harness(|| {
        test_fn_1();
        test_fn_2();
    });

    fn test_fn_1() { }
    fn test_fn_2() { }
}
```

# Caveats

* This must be called from the main thread.
* On wasm32, this should be marked #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
*/
pub fn integration_test_harness<C>(c: C)
 where C: FnOnce() + 'static + Send {
    crate::application::main(|| {
        c();
        crate::sys::stop_main_thread();
    })
}

/**
A version of [`application::main`] for doctests.

doctests generally pick one of two execution models:

* On most platforms, they run in separate processes, on the main thread
* On wasm32, they run in one process, on the main thread sequentially

This function implements a version of `main` that works in both cases.
It brings up a temporary main thread environment for the duration of the test.

```
use app_window::test_support::doctest_main;
eprintln!("Will call doctest_main");
doctest_main(|| {
    eprintln!("Hello world");
});
```
*/
pub fn doctest_main<C>(c: C)
where C: FnOnce() + 'static + Send {
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