// SPDX-License-Identifier: MPL-2.0
use app_window::input::keyboard::Keyboard;
use app_window::input::mouse::Mouse;
use app_window::input::{debug_window_hide, debug_window_show};

fn test_board() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let _k = Keyboard::coalesced();
    let _m = Mouse::coalesced();
    debug_window_show();
    debug_window_hide();

    //on wasm32 this thread completes
    #[cfg(target_arch = "wasm32")]
    {
        std::mem::forget(_k);
        std::mem::forget(_m);
    }
}

fn main() {
    test_board();
}
