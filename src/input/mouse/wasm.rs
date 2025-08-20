// SPDX-License-Identifier: MPL-2.0
use crate::input::Window;
use crate::input::keyboard::wasm::ARBITRARY_WINDOW_PTR;
use crate::input::mouse::MouseWindowLocation;
use crate::main_thread_cell::MainThreadCell;
use std::ptr::NonNull;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::{MouseEvent, WheelEvent};

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
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("no document on window");

            let weak = Arc::downgrade(&shared);
            let weak_down = weak.clone();
            let weak_up = weak.clone();
            let weak_wheel = weak.clone();

            // Mouse move callback
            let mousemove_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                if let Some(shared) = weak.upgrade() {
                    let window = web_sys::window().expect("no global window exists");
                    let width = window
                        .inner_width()
                        .expect("failed to get width")
                        .as_f64()
                        .unwrap_or(0.0);

                    let height = window
                        .inner_height()
                        .expect("failed to get height")
                        .as_f64()
                        .unwrap_or(0.0);
                    let window = Some(Window(NonNull::new(ARBITRARY_WINDOW_PTR).unwrap()));

                    shared.set_window_location(MouseWindowLocation::new(
                        event.page_x() as f64,
                        event.page_y() as f64,
                        width,
                        height,
                        window,
                    ));
                }
            }) as Box<dyn FnMut(MouseEvent)>);

            document
                .add_event_listener_with_callback(
                    "mousemove",
                    mousemove_callback.as_ref().unchecked_ref(),
                )
                .expect("Can't add event listener");
            mousemove_callback.forget();

            let mousedown_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                if let Some(shared) = weak_down.upgrade() {
                    shared.set_key_state(
                        js_button_to_rust(event.button()),
                        true,
                        ARBITRARY_WINDOW_PTR,
                    );
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            document
                .add_event_listener_with_callback(
                    "mousedown",
                    mousedown_callback.as_ref().unchecked_ref(),
                )
                .expect("Can't add event listener");
            mousedown_callback.forget();

            let mouseup_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
                if let Some(shared) = weak_up.upgrade() {
                    shared.set_key_state(
                        js_button_to_rust(event.button()),
                        false,
                        ARBITRARY_WINDOW_PTR,
                    );
                }
            }) as Box<dyn FnMut(MouseEvent)>);
            document
                .add_event_listener_with_callback(
                    "mouseup",
                    mouseup_callback.as_ref().unchecked_ref(),
                )
                .expect("Can't add event listener");
            mouseup_callback.forget();

            let wheel_callback = Closure::wrap(Box::new(move |event: WheelEvent| {
                let raw_x = event.delta_x();
                let raw_y = event.delta_y();
                let mode = event.delta_mode();
                let (x, y) = match mode {
                    1 => (raw_x * 10.0, raw_y * 10.0),
                    2 => (raw_x * 100.0, raw_y * 100.0),
                    _ => (raw_x, raw_y),
                };

                if let Some(shared) = weak_wheel.upgrade() {
                    shared.add_scroll_delta(x as f64, y as f64, ARBITRARY_WINDOW_PTR);
                }
            }) as Box<dyn FnMut(WheelEvent)>);
            document
                .add_event_listener_with_callback("wheel", wheel_callback.as_ref().unchecked_ref())
                .expect("Can't add event listener");
            wheel_callback.forget();

            PlatformCoalescedMouse {}
        })
        .await
    }
}
