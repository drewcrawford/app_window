use std::ffi::c_void;
use std::fmt::Display;
use std::sync::Arc;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};
use crate::coordinates::{Position, Size};

#[derive(Debug)]
pub struct FullscreenError;

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "FullscreenError")
    }
}
impl std::error::Error for FullscreenError {}

fn main_thread_id() -> u32 {
    static mut MAIN_THREAD_ID: u32 = 0;
    #[used]
    #[allow(non_upper_case_globals)]
    #[link_section = ".CRT$XCU"]
    static INIT_MAIN_THREAD_ID: unsafe fn() = {
        unsafe fn initer() {
            MAIN_THREAD_ID = windows::Win32::System::Threading::GetCurrentThreadId();
        }
        initer
    };

    unsafe { MAIN_THREAD_ID }
}


pub fn is_main_thread() -> bool {
    //windows does not have a clear concept of a main thread but allows any thread to be in charge
    //of a window.  However for compatibility we project a 'main thread-like' concept onto windows
    let current_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    current_id == main_thread_id()
}


pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    closure(); //I think it's ok to run inline on windows?
    let mut message = MSG::default();
    let all_hwnd = HWND::default();
    loop {
        let message_ret = unsafe{GetMessageW(&mut message, all_hwnd, 0, 0)};
        if message_ret.0 == 0 {
            break;
        }
        else if message_ret.0 == -1 {
            panic!("GetMessageW failed");
        }
        unsafe {
            //ms code seems to ignore this return value in practice
            //see https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessage
            _ = TranslateMessage(&mut message);
            DispatchMessageW(&mut message);
        }
    }
}

pub fn on_main_thread<F: FnOnce()>(_closure: F) {
    todo!()
}

pub struct Window {
    imp: *mut c_void,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub async fn new(_position: Position, _size: Size, _title: String) -> Self {
        todo!()
    }

    pub async fn default() -> Self {
        todo!()
    }

    pub async fn fullscreen(_title: String) -> Result<Self, FullscreenError> {
        todo!()
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        todo!()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        todo!()
    }
}

pub struct Surface {
    imp: *mut c_void,
    update_size: Option<Arc<dyn Fn(Size)>>,
}

unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl Surface {
    pub async fn size(&self) -> Size {
        todo!()
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        todo!()
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        todo!()
    }

    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, _update: F) {
        todo!()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        todo!()
    }
}