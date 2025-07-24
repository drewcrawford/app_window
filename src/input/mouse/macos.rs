// SPDX-License-Identifier: MPL-2.0
use crate::input::Window;
use crate::input::mouse::{MouseWindowLocation, Shared};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, Weak};

#[derive(Debug)]
pub(super) struct PlatformCoalescedMouse {
    imp: *mut c_void,
}

//swift side is Sendable
unsafe impl Send for PlatformCoalescedMouse {}
unsafe impl Sync for PlatformCoalescedMouse {}

#[unsafe(no_mangle)]
extern "C" fn raw_input_finish_mouse_event_context(ctx: *mut c_void) {
    let _weak = unsafe { Weak::from_raw(ctx as *const Shared) };
}

#[unsafe(no_mangle)]
extern "C" fn raw_input_mouse_move(
    ctx: *const c_void,
    window: *mut c_void,
    window_pos_x: f64,
    window_pos_y: f64,
    window_width: f64,
    window_height: f64,
) {
    let weak = unsafe { Weak::from_raw(ctx as *const Shared) };
    if let Some(shared) = weak.upgrade() {
        if !window.is_null() {
            let window = Some(Window(NonNull::new(window).unwrap()));
            let loc = MouseWindowLocation::new(
                window_pos_x,
                window_pos_y,
                window_width,
                window_height,
                window,
            );
            shared.set_window_location(loc);
        }
    }
    std::mem::forget(weak);
}

#[unsafe(no_mangle)]
extern "C" fn raw_input_mouse_button(
    ctx: *const c_void,
    window: *mut c_void,
    button: u8,
    down: bool,
) {
    let weak = unsafe { Weak::from_raw(ctx as *const Shared) };
    if let Some(shared) = weak.upgrade() {
        shared.set_key_state(button, down, window);
    }
    std::mem::forget(weak);
}

#[unsafe(no_mangle)]
extern "C" fn raw_input_mouse_scroll(
    ctx: *const c_void,
    window: *mut c_void,
    delta_x: f64,
    delta_y: f64,
) {
    let weak = unsafe { Weak::from_raw(ctx as *const Shared) };
    if let Some(shared) = weak.upgrade() {
        shared.add_scroll_delta(delta_x, delta_y, window);
    }
    std::mem::forget(weak);
}

unsafe extern "C" {
    fn PlatformCoalescedMouseNew(ctx: *const c_void) -> *mut c_void;
    fn PlatformCoalescedMouseFree(imp: *mut c_void);
}

impl PlatformCoalescedMouse {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        let weak = Arc::downgrade(shared);
        let weak_raw = Weak::into_raw(weak) as *const c_void;
        PlatformCoalescedMouse {
            imp: unsafe { PlatformCoalescedMouseNew(weak_raw) },
        }
    }
}

impl Drop for PlatformCoalescedMouse {
    fn drop(&mut self) {
        unsafe { PlatformCoalescedMouseFree(self.imp) }
    }
}
