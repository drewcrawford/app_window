pub fn main() {
    #[cfg(target_arch="wasm32")]
    console_error_panic_hook::set_once();
    app_window::application::main(|| {
        let w = app_window::window::Window::fullscreen("Hello".to_string());
        std::mem::forget(w);
    });
}