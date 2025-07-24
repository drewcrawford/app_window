// SPDX-License-Identifier: MPL-2.0
use app_window::input::keyboard::Keyboard;
use app_window::input::mouse::Mouse;
use app_window::input::{debug_window_hide, debug_window_show};
use some_executor::SomeExecutor;
use some_executor::observer::Observer;

async fn test_board() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let _k = Keyboard::coalesced();
    let _m = Mouse::coalesced().await;
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
    app_window::application::main(|| {
        let task = some_executor::task::Task::without_notifications(
            "input_main".into(),
            some_executor::task::Configuration::new(
                some_executor::hint::Hint::Unknown,
                some_executor::Priority::UserInteractive,
                some_executor::Instant::now(),
            ),
            test_board(),
        );
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
    });
}
