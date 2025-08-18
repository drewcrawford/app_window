// SPDX-License-Identifier: MPL-2.0
use crate::input::Window;
use crate::input::mouse::{MouseWindowLocation, Shared};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use wayland_client::backend::ObjectId;

#[derive(Debug)]
pub(super) struct PlatformCoalescedMouse {}

#[derive(Default)]
struct MouseState {
    shareds: Vec<Weak<Shared>>,
    recent_x_pos: Option<f64>,
    recent_y_pos: Option<f64>,
    recent_window_width: Option<i32>,
    recent_window_height: Option<i32>,
    recent_window: Option<ObjectId>,
}

impl MouseState {
    fn apply_all<F: Fn(&Shared)>(&mut self, f: F) {
        self.shareds.retain(|shared| {
            if let Some(shared) = shared.upgrade() {
                f(&shared);
                true
            } else {
                false
            }
        })
    }
    fn send_events_if_needed(&mut self) {
        if let (
            Some(recent_window_width),
            Some(recent_window_height),
            Some(recent_x_pos),
            Some(recent_y_pos),
        ) = (
            self.recent_window_width,
            self.recent_window_height,
            self.recent_x_pos,
            self.recent_y_pos,
        ) {
            let window = match self.recent_window.as_ref() {
                None => None,
                Some(object_id) => NonNull::new(object_id.protocol_id() as *mut c_void).map(Window),
            };
            let pos = MouseWindowLocation::new(
                recent_x_pos,
                recent_y_pos,
                recent_window_width as f64,
                recent_window_height as f64,
                window,
            );
            self.apply_all(|shared| {
                shared.set_window_location(pos);
            })
        }
    }
}

/**
Call this to handle [wayland_client::protocol::wl_pointer::Event::Motion].

Call this from your wayland dispatch queue.
*/
pub fn motion_event(_time: u32, surface_x: f64, surface_y: f64) {
    let mut lock = MOUSE_STATE.get_or_init(Mutex::default).lock().unwrap();
    lock.recent_x_pos = Some(surface_x);
    lock.recent_y_pos = Some(surface_y);
    lock.send_events_if_needed();
}

/**
Call this to handle [wayland_protocols::xdg::shell::client::xdg_toplevel::Event::Configure].

Call this from your wayland dispatch queue.
*/
pub fn xdg_toplevel_configure_event(width: i32, height: i32) {
    let mut lock = MOUSE_STATE.get_or_init(Mutex::default).lock().unwrap();
    lock.recent_window_width = Some(width);
    lock.recent_window_height = Some(height);
    lock.send_events_if_needed();
}

/**
Call this to handle wayland_client::protocol::wl_pointer::Event::Button.

Call this from your wayland dispatch queue.
*/
pub fn button_event(_time: u32, button: u32, state: u32, window: ObjectId) {
    let down = state != 0;
    //see https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h
    let btn_code = match button {
        0x110 => 0, //BTN_LEFT
        0x111 => 1, //BTN_RIGHT
        0x112 => 2, //BTN_MIDDLE
        0x113 => 3, //BTN_SIDE
        0x114 => 4, //BTN_EXTRA
        0x115 => 5, //BTN_FORWARD
        0x116 => 6, //BTN_BACK
        0x117 => 7, //BTN_TASK
        0x118 => 8,
        0x119 => 9,
        _ => {
            logwise::warn_sync!("Unknown button code: {button}",button=button);
            return;
        }
    };
    MOUSE_STATE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .apply_all(|shared| {
            shared.set_key_state(btn_code, down, window.protocol_id() as *mut c_void);
        });
    crate::input::keyboard::linux::ax::ax_mouse();
}

pub fn axis_event(_time: u32, axis: u32, value: f64, window: ObjectId) {
    if axis == 0 {
        //vertical
        MOUSE_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .apply_all(|shared| {
                shared.add_scroll_delta(0.0, value, window.protocol_id() as *mut c_void);
            })
    } else {
        //horizontal
        MOUSE_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .apply_all(|shared| {
                shared.add_scroll_delta(value, 0.0, window.protocol_id() as *mut c_void);
            })
    }
}

static MOUSE_STATE: OnceLock<Mutex<MouseState>> = OnceLock::new();

impl PlatformCoalescedMouse {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        MOUSE_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .shareds
            .push(Arc::downgrade(shared));
        PlatformCoalescedMouse {}
    }
}
