use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use swift_rs::{swift, SRString};
use crate::coordinates::{Position, Size};
use crate::sys;

#[allow(non_snake_case)]
swift!(fn SwiftAppWindowIsMainThread() -> bool);

#[allow(non_snake_case)]
swift!(fn SwiftAppWindowRunMainThread());

#[allow(non_snake_case)]
swift!(fn SwiftAppWindow_WindowNew( x: f64, y: f64, width: f64, height: f64, title: SRString)  -> *mut c_void);

#[allow(non_snake_case)]
swift!(fn SwiftAppWindow_WindowFree(window: *mut c_void)  -> ());






pub fn is_main_thread() -> bool {
    unsafe{SwiftAppWindowIsMainThread()}
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    std::thread::spawn(|| {
        closure()
    });
    unsafe { SwiftAppWindowRunMainThread() }
}



pub struct Window {
    imp: *mut c_void,
}
impl Window {
    pub fn new(position: Position, size: Size, title: String) -> Self {

        let imp = unsafe{SwiftAppWindow_WindowNew(position.x(), position.y(), size.width(), size.height(), SRString::from(title.as_str()))};
        Window {
            imp,
        }
    }
    pub fn default() -> Self {
        Self::new(Position::new(640.0, 480.0), Size::new(640.0, 480.0), "app_window".to_string())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            SwiftAppWindow_WindowFree(self.imp);
        }
    }
}