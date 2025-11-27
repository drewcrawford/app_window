// SPDX-License-Identifier: MPL-2.0
/**

# Doctest example

This demonstrates how to use app_window from a doctest.

*/
pub fn example1() {
    println!("Hello, world!");
}

/**
```
use app_window::test_support::doctest_main;
let (s,r) = std::sync::mpsc::channel();
doctest_main(move || {
    //verify on_main_thread works
    app_window::application::submit_to_main_thread("doctest example2".to_string(), move || {
        eprintln!("Hello from on_main_thread");
        s.send(()).unwrap();
    });
});
r.recv().unwrap();
```
*/
pub fn example2() {
    println!("Hello, world 2!");
}


