use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};
use r#continue::Sender;
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

#[allow(non_snake_case)]
swift!(fn SwiftAppWindow_WindowNewFullscreen(title: SRString)  -> *mut c_void);

#[allow(non_snake_case)]
swift!(fn SwiftAppWindow_WindowSurface(ctx: *mut c_void, window: *mut c_void, ret: *mut c_void)  -> ());




pub fn is_main_thread() -> bool {
    unsafe{SwiftAppWindowIsMainThread()}
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    std::thread::spawn(|| {
        closure()
    });
    unsafe { SwiftAppWindowRunMainThread() }
}

extern "C" fn recv_surface(ctx: *mut Sender<Surface>, surface: *mut c_void) {
    let c: Sender<Surface> = *unsafe{Box::from_raw(ctx)};

    c.send(Surface { imp: surface })
}

extern "C" fn recv_size(ctx: *mut Sender<Size>, size_w: f64, size_h: f64) {
    let s = Size::new(size_w, size_h);
    let c: Sender<Size> = *unsafe{Box::from_raw(ctx)};
    c.send(s);
}



pub struct Window {
    imp: *mut c_void,
}
//marked as Sendable in swift
unsafe impl Send for Window {}
unsafe impl Sync for Window {}
impl Window {
    pub fn new(position: Position, size: Size, title: String) -> Self {

        let imp = unsafe{SwiftAppWindow_WindowNew(position.x(), position.y(), size.width(), size.height(), SRString::from(title.as_str()))};
        Window {
            imp,
        }
    }
    pub fn default() -> Self {
        Self::new(Position::new(0.0, 0.0), Size::new(640.0, 480.0), "app_window".to_string())
    }

    pub fn fullscreen(title: String) -> Self {
        let imp = unsafe { SwiftAppWindow_WindowNewFullscreen(SRString::from(title.as_str())) };
        Window {
            imp,
        }
    }
    pub async fn surface(&self) -> crate::surface::Surface {
        let (sender, fut) = r#continue::continuation();

        let sender_box = Box::into_raw(Box::new(sender));
        unsafe{SwiftAppWindow_WindowSurface(sender_box as *mut c_void, self.imp,  recv_surface as *mut c_void)};

        let sys_surface = fut.await;
        let crate_surface = crate::surface::Surface {
            sys: sys_surface
        };
        crate_surface
    }
}

#[allow(non_snake_case)]
swift!(fn SwiftAppWindow_SurfaceSize(ctx: *mut c_void, surface: *mut c_void, ret: *mut c_void)  -> ());
pub struct Surface {
    imp: *mut c_void,
}
//sendable in swift!
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}
impl Drop for Surface {
    fn drop(&mut self) {
        todo!()
    }
}

impl Surface {
    pub async fn size(&self) -> Size {
        let (sender,fut) = r#continue::continuation();
        let boxed_sender = Box::into_raw(Box::new(sender));
        unsafe{
            SwiftAppWindow_SurfaceSize(boxed_sender as *mut c_void, self.imp, recv_size as *mut c_void)
        }
        fut.await

    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            SwiftAppWindow_WindowFree(self.imp);
        }
    }
}