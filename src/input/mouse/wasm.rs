// SPDX-License-Identifier: MPL-2.0
//! Connects browser pointer and wheel events to the Rust mouse state.
use crate::input::Window;
use crate::input::keyboard::wasm::ARBITRARY_WINDOW_PTR;
use crate::input::mouse::MouseWindowLocation;
use std::ptr::NonNull;
use std::sync::Arc;
use wasm_lite::Closure;
use wasm_lite::dom::{DeltaMode, MouseEvent, WheelEvent, window};

fn js_button_to_rust(button: i16) -> u8 {
    match button {
        0 => 0,
        1 => 2,
        2 => 1,
        _ => button as u8,
    }
}

#[derive(Debug)]
pub(super) struct PlatformCoalescedMouse {}

impl PlatformCoalescedMouse {
    pub async fn new(shared: &Arc<crate::input::mouse::Shared>) -> Self {
        let shared = shared.clone();

        crate::application::on_main_thread("PlatformCoalescedMouse setup".to_string(), move || {
            let window = window().expect("no global window exists");
            let document = window.document().expect("no document on window");

            let weak = Arc::downgrade(&shared);
            let weak_down = weak.clone();
            let weak_up = weak.clone();
            let weak_wheel = weak.clone();

            // Mouse move callback
            let mousemove_callback = Closure::new_with_arg(move |event| {
                let event = MouseEvent::from_js(event);
                if let Some(shared) = weak.upgrade() {
                    // Fully qualified: the enclosing scope binds `window` to a
                    // `Window` value, which would shadow the function.
                    let w = wasm_lite::dom::window().expect("no global window exists");
                    let width = w.inner_width();
                    let height = w.inner_height();
                    let window = Some(Window(NonNull::new(ARBITRARY_WINDOW_PTR).unwrap()));

                    //clientX/Y are viewport-relative, matching inner_width/inner_height;
                    //offsetX/Y would be relative to whatever element the pointer is over.
                    shared.set_window_location(MouseWindowLocation::new(
                        event.client_x(),
                        event.client_y(),
                        width,
                        height,
                        window,
                    ));
                }
            });

            document
                .add_event_listener("mousemove", mousemove_callback.as_js_value())
                .expect("Can't add event listener");
            mousemove_callback.forget();

            let mousedown_callback = Closure::new_with_arg(move |event| {
                let event = MouseEvent::from_js(event);
                if let Some(shared) = weak_down.upgrade() {
                    shared.set_key_state(
                        js_button_to_rust(event.button()),
                        true,
                        ARBITRARY_WINDOW_PTR,
                    );
                }
            });
            document
                .add_event_listener("mousedown", mousedown_callback.as_js_value())
                .expect("Can't add event listener");
            mousedown_callback.forget();

            let mouseup_callback = Closure::new_with_arg(move |event| {
                let event = MouseEvent::from_js(event);
                if let Some(shared) = weak_up.upgrade() {
                    shared.set_key_state(
                        js_button_to_rust(event.button()),
                        false,
                        ARBITRARY_WINDOW_PTR,
                    );
                }
            });
            document
                .add_event_listener("mouseup", mouseup_callback.as_js_value())
                .expect("Can't add event listener");
            mouseup_callback.forget();

            let wheel_callback = Closure::new_with_arg(move |event| {
                let event = WheelEvent::from_js(event);
                let raw_x = event.delta_x();
                let raw_y = event.delta_y();
                //deltas are in whatever unit the event says; treating lines or
                //pages as pixels scrolls ~10x or ~100x too slowly.
                let (x, y) = match event.delta_mode() {
                    DeltaMode::Pixel => (raw_x, raw_y),
                    DeltaMode::Line => (raw_x * 10.0, raw_y * 10.0),
                    DeltaMode::Page => (raw_x * 100.0, raw_y * 100.0),
                    DeltaMode::Other(_) => (raw_x, raw_y),
                };

                if let Some(shared) = weak_wheel.upgrade() {
                    shared.add_scroll_delta(x, y, ARBITRARY_WINDOW_PTR);
                }
            });
            document
                .add_event_listener("wheel", wheel_callback.as_js_value())
                .expect("Can't add event listener");
            wheel_callback.forget();

            PlatformCoalescedMouse {}
        })
        .await
    }
}
