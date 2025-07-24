// SPDX-License-Identifier: MPL-2.0
use crate::input::Window;
use crate::input::mouse::{MouseWindowLocation, Shared};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    XBUTTON1, XBUTTON2,
};

fn get_x_lparam(lparam: LPARAM) -> i16 {
    ((lparam.0 as usize) & 0xFFFF) as u16 as i16
}

fn get_y_lparam(lparam: LPARAM) -> i16 {
    (((lparam.0 as usize) & 0xFFFF_0000) >> 16) as u16 as i16
}

fn get_xbutton_wparam(wparam: WPARAM) -> u16 {
    ((wparam.0 & 0xFFFF_0000) >> 16) as u16
}

fn get_wheel_delta_wparam(wparam: WPARAM) -> i16 {
    ((wparam.0 & 0xFFFF_0000) >> 16) as u16 as i16
}

struct MouseState {
    shareds: Vec<Weak<Shared>>,
}
impl MouseState {
    fn new() -> Self {
        MouseState {
            shareds: Vec::new(),
        }
    }

    fn register_coalesced(&mut self, shared: &Arc<Shared>) {
        self.shareds.push(Arc::downgrade(shared));
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_all<F: Fn(&Shared)>(f: F) {
    MOUSE_STATE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .shareds
        .retain(|shared| {
            if let Some(shared) = shared.upgrade() {
                f(&shared);
                true
            } else {
                false
            }
        })
}

static MOUSE_STATE: OnceLock<Mutex<MouseState>> = OnceLock::new();
/**
Provide windows key events to raw_input.

# Returns
If we processed the message, returns LRESULT(0).  Otherwise returns non-zero.
*/
pub(crate) fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        msg if msg == WM_MOUSEMOVE => {
            let window = NonNull::new(hwnd.0).map(Window);
            let x = get_x_lparam(l_param);
            let y = get_y_lparam(l_param);
            let mut point = MaybeUninit::uninit();
            unsafe { ClientToScreen(hwnd, point.as_mut_ptr()) }
                .expect("failed to get client to screen");
            // let point = unsafe{point.assume_init()};

            let mut rect = MaybeUninit::uninit();
            unsafe { GetClientRect(hwnd, rect.as_mut_ptr()) }.expect("failed to get client rect");

            let rect = unsafe { rect.assume_init() };
            let rel_mouse = MouseWindowLocation::new(
                x as f64,
                y as f64,
                rect.right as f64,
                rect.bottom as f64,
                window,
            );

            apply_all(|shared| {
                shared.set_window_location(rel_mouse);
            });
            LRESULT(0)
        }
        msg if msg == WM_LBUTTONDOWN => {
            apply_all(|shared| {
                shared.set_key_state(0, true, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_LBUTTONUP => {
            apply_all(|shared| {
                shared.set_key_state(0, false, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_RBUTTONDOWN => {
            apply_all(|shared| {
                shared.set_key_state(1, true, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_RBUTTONUP => {
            apply_all(|shared| {
                shared.set_key_state(1, false, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_MBUTTONDOWN => {
            apply_all(|shared| {
                shared.set_key_state(2, true, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_MBUTTONUP => {
            apply_all(|shared| {
                shared.set_key_state(2, false, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_XBUTTONDOWN => {
            let xbutton = get_xbutton_wparam(w_param);
            let key = match xbutton {
                x if x == XBUTTON1 => 3,
                x if x == XBUTTON2 => 4,
                _ => {
                    unimplemented!("Unknown xbutton {:?}", xbutton)
                }
            };
            apply_all(|shared| {
                shared.set_key_state(key, true, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_XBUTTONUP => {
            let xbutton = get_xbutton_wparam(w_param);
            let key = match xbutton {
                x if x == XBUTTON1 => 3,
                x if x == XBUTTON2 => 4,
                _ => {
                    unimplemented!("Unknown xbutton {:?}", xbutton)
                }
            };
            apply_all(|shared| {
                shared.set_key_state(key, false, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_MOUSEWHEEL => {
            //todo: should this be scaled in some way?
            let delta = get_wheel_delta_wparam(w_param);
            apply_all(|shared| {
                shared.add_scroll_delta(0.0, delta as f64, hwnd.0);
            });
            LRESULT(0)
        }
        msg if msg == WM_MOUSEHWHEEL => {
            //todo: should this be scaled in some way?
            let delta = get_wheel_delta_wparam(w_param);
            apply_all(|shared| {
                shared.add_scroll_delta(delta as f64, 0.0, hwnd.0);
            });
            LRESULT(0)
        }

        _ => LRESULT(1),
    }
}

#[derive(Debug)]
pub(super) struct PlatformCoalescedMouse {}

impl PlatformCoalescedMouse {
    pub(crate) async fn new(shared: &Arc<Shared>) -> PlatformCoalescedMouse {
        MOUSE_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .register_coalesced(shared);
        PlatformCoalescedMouse {}
    }
}
