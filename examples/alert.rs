// SPDX-License-Identifier: MPL-2.0
/*!
An example that displays an alert with "Hello World".
*/
use some_executor::SomeExecutor;
use some_executor::observer::Observer;
use some_executor::task::{Configuration, Task};

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    app_window::application::main(|| {
        let task =
            Task::without_notifications("alert".to_string(), Configuration::default(), async {
                app_window::alert("Hello World".to_string()).await;
            });
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen::throw_str(
            "Cursed hack to keep workers alive. See https://github.com/rustwasm/wasm-bindgen/issues/2945",
        );
    });
}
