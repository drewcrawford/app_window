// SPDX-License-Identifier: MPL-2.0
/*!
An example that uses fullscreen APIs.
*/
use some_executor::SomeExecutor;
use some_executor::observer::Observer;
use some_executor::task::{Configuration, Task};

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    wasm_lite::set_panic_hook();
    app_window::application::main(|| {
        let task = Task::without_notifications(
            "fullscreen".to_string(),
            Configuration::default(),
            async {
                let w = app_window::window::Window::fullscreen("Hello".to_string())
                    .await
                    .expect("Can't create window");
                std::mem::forget(w);
            },
        );
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
        // The `wasm_bindgen::throw_str` "cursed hack to keep workers alive"
        // that used to be here is gone with wasm-bindgen. Worker lifetime is
        // `wasm_lite_std`'s task hooks now; throwing out of the closure would
        // just abort the module.
    });
}
