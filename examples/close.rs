/*!
An example that closes a window.
*/
use some_executor::SomeExecutor;
use some_executor::observer::Observer;

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    app_window::application::main(|| {
        let task = some_executor::task::Task::without_notifications(
            "main".into(),
            some_executor::task::Configuration::new(
                some_executor::hint::Hint::Unknown,
                some_executor::Priority::UserInteractive,
                some_executor::Instant::now(),
            ),
            async {
                let w = app_window::window::Window::default().await;
                std::mem::drop(w);
            },
        );
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
    });
}
