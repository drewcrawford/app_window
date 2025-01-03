pub fn main() {
    #[cfg(target_arch="wasm32")]
    console_error_panic_hook::set_once();
    app_window::application::main(|| {
        test_executors::spawn_local(async  {
            let w = app_window::window::Window::fullscreen("Hello".to_string()).await.expect("Can't create window");
            std::mem::forget(w);
        },"fullscreen etc");
        wasm_bindgen::throw_str("Cursed hack to keep workers alive. See https://github.com/rustwasm/wasm-bindgen/issues/2945");
    });
}