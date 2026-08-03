// SPDX-License-Identifier: MPL-2.0
use wasm_bindgen_test::wasm_bindgen_test_configure;

//at the moment, wasm_thread does not work in node

#[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
fn main() {
    app_window::test_support::integration_test_harness(|| {
        test_one();
        test_two();
    });
}

fn test_one() {
    let (s, r) = std::sync::mpsc::channel();
    app_window::application::submit_to_main_thread("test_one".to_string(), move || {
        s.send(()).unwrap();
    });
    r.recv().unwrap();
}

fn test_two() {
    let (s, r) = std::sync::mpsc::channel();
    app_window::application::submit_to_main_thread("test_two".to_string(), move || {
        s.send(()).unwrap();
    });
    r.recv().unwrap();
}