/*!
An example that uses fullscreen APIs.
*/
use some_executor::SomeExecutor;
use some_executor::observer::Observer;
use some_executor::task::{Configuration, Task};

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    app_window::application::main(|| {
        let task = Task::without_notifications(
            "fullscreen".to_string(),
            async {
                let w = app_window::window::Window::fullscreen("Hello".to_string())
                    .await
                    .expect("Can't create window");
                std::mem::forget(w);
            },
            Configuration::default(),
        );
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen::throw_str(
            "Cursed hack to keep workers alive. See https://github.com/rustwasm/wasm-bindgen/issues/2945",
        );
    });
}
