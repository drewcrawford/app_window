pub fn main() {
    #[cfg(target_arch="wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(target_arch="wasm32")]
    use wasm_thread as thread;
    #[cfg(not(target_arch="wasm32"))]
    use std::thread as thread;
    let k = app_input::keyboard::Keyboard::coalesced();
    let m = app_input::mouse::Mouse::coalesced();

    #[cfg(feature = "app_input")]
    thread::spawn(move || {

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
        //let w = app_window::window::Window::fullscreen("Hello".to_string());
        let w = app_window::window::Window::default();
        std::mem::forget(w);
    });


}