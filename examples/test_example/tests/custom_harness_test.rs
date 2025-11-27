// SPDX-License-Identifier: MPL-2.0
use wasm_bindgen_test::wasm_bindgen_test_configure;

//at the moment, wasm_thread does not work in node
#[cfg(target_arch = "wasm32")]
wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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