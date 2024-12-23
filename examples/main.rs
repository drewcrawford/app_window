pub fn main() {
    app_window::application::main(|| {
        //let w = app_window::window::Window::fullscreen("Hello".to_string());
        let w = app_window::window::Window::default();
        std::mem::forget(w);

    });
}