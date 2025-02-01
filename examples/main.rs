/*!
An example that opens a window.
*/
use some_executor::observer::Observer;
use some_executor::SomeExecutor;

pub fn main() {
    #[cfg(target_arch="wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(target_arch="wasm32")]
    use wasm_thread as thread;
    #[cfg(feature = "app_input")]
    #[cfg(not(target_arch="wasm32"))]
    use std::thread as thread;


    #[cfg(feature = "app_input")]
    thread::spawn(move || {
        let k = app_input::keyboard::Keyboard::coalesced();
        let m = app_input::mouse::Mouse::coalesced();
        loop {
            for key in app_input::keyboard::key::KeyboardKey::all_keys() {
                if k.is_pressed(key) {
                    println!("key {:?} is pressed", key);
                }
            }
            println!("Mouse pos {:?}", m.window_pos());
            if m.button_state(app_input::mouse::MOUSE_BUTTON_LEFT) {
                println!("Mouse down");
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
    app_window::application::main(|| {
        let task = some_executor::task::Task::without_notifications("main".into(), async {
            let w = app_window::window::Window::default().await;
            std::mem::forget(w);
        }, some_executor::task::Configuration::new(some_executor::hint::Hint::Unknown, some_executor::Priority::UserInteractive, some_executor::Instant::now()));
        some_executor::current_executor::current_executor().spawn_objsafe(task.into_objsafe()).detach();
    });


}