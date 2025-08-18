/*!
An example that opens a window.
*/
use some_executor::SomeExecutor;
use some_executor::observer::Observer;

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;

    thread::spawn(move || {
        let task = some_executor::task::Task::without_notifications("input poll".into(),
        some_executor::task::Configuration::new(
                some_executor::hint::Hint::Unknown,
                some_executor::Priority::UserInteractive,
                some_executor::Instant::now(),
            ),
            async {
                let k = app_window::input::keyboard::Keyboard::coalesced().await;
                let m = app_window::input::mouse::Mouse::coalesced().await;
                loop {
                    for key in app_window::input::keyboard::key::KeyboardKey::all_keys() {
                        if k.is_pressed(key) {
                            println!("key {:?} is pressed", key);
                        }
                    }
                    println!("Mouse pos {:?}", m.window_pos());
                    if m.button_state(app_window::input::mouse::MOUSE_BUTTON_LEFT) {
                        println!("Mouse down");
                    }
                    thread::sleep(std::time::Duration::from_millis(1000));
                }
            },
        );
        task.spawn_current();

    });
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
                std::mem::forget(w);
            },
        );
        some_executor::current_executor::current_executor()
            .spawn_objsafe(task.into_objsafe())
            .detach();
    });
}
