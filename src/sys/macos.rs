#![allow(non_snake_case)]

use std::error::Error;
use std::ffi::c_void;
use std::fmt::Display;
use std::ptr::NonNull;
use std::sync::{Arc, Weak};
use r#continue::Sender;
use raw_window_handle::{AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle};
use swift_rs::{swift, SRString};
use crate::coordinates::{Position, Size};

#[derive(Debug)]
pub struct FullscreenError;

impl Error for FullscreenError {}

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

swift!(fn SwiftAppWindowIsMainThread() -> bool);

swift!(fn SwiftAppWindowRunMainThread());

swift!(fn SwiftAppWindow_WindowNew( x: f64, y: f64, width: f64, height: f64, title: SRString)  -> *mut c_void);

swift!(fn SwiftAppWindow_WindowFree(window: *mut c_void)  -> ());

swift!(fn SwiftAppWindow_WindowNewFullscreen(title: SRString)  -> *mut c_void);

swift!(fn SwiftAppWindow_WindowSurface(ctx: *mut c_void, window: *mut c_void, ret: *mut c_void)  -> ());


swift!(fn SwiftAppWindow_OnMainThread(ctx: *mut c_void, c_fn: *mut c_void)  -> ());



pub fn is_main_thread() -> bool {
    unsafe{SwiftAppWindowIsMainThread()}
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    std::thread::spawn(|| {
        closure()
    });
    unsafe { SwiftAppWindowRunMainThread() }
}

extern "C" fn on_main_thread_callback<F: FnOnce()>(ctx: *mut MainThreadClosure<F>) {
    let b: MainThreadClosure<F> = *unsafe{Box::from_raw(ctx)};
    (b.closure)();
}

pub fn on_main_thread<F: FnOnce()>(closure: F) {
    let context = MainThreadClosure {
        closure,
    };
    let boxed_ptr = Box::into_raw(Box::new(context)) as *mut c_void;
    unsafe { SwiftAppWindow_OnMainThread(boxed_ptr, on_main_thread_callback::<F> as *mut c_void)}
}

struct MainThreadClosure<F> {
    closure: F,
}

extern "C" fn recv_surface(ctx: *mut Sender<Surface>, surface: *mut c_void) {
    let c: Sender<Surface> = *unsafe{Box::from_raw(ctx)};

    c.send(Surface { imp: surface, update_size: None })
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
    pub async fn new(position: Position, size: Size, title: String) -> Self {

        let imp = unsafe{SwiftAppWindow_WindowNew(position.x(), position.y(), size.width(), size.height(), SRString::from(title.as_str()))};
        Window {
            imp,
        }
    }
    pub async fn default() -> Self {
        Self::new(Position::new(0.0, 0.0), Size::new(640.0, 480.0), "app_window".to_string()).await
    }

    pub async fn fullscreen(title: String) -> Result<Self,FullscreenError> {
        let imp = unsafe { SwiftAppWindow_WindowNewFullscreen(SRString::from(title.as_str())) };
        Ok(Window {
            imp,
        })
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

swift!(fn SwiftAppWindow_SurfaceSize(ctx: *mut c_void, surface: *mut c_void, ret: *mut c_void)  -> ());

swift!(fn SwiftAppWindow_SurfaceRawHandle(surface: *mut c_void)  -> *mut c_void);

swift!(fn SwiftAppWindow_SurfaceFree(surface: *mut c_void) -> ());
swift!(fn SwiftAppWindow_SurfaceSizeUpdate(ctx: *mut c_void, surface: *mut c_void, notify: *mut c_void) -> ());

extern "C" fn notify_size<F: Fn(Size) -> ()>(ctx: *const F, width: f64, height: f64) {
    let as_weak = unsafe{Weak::from_raw(ctx)};
    if let Some(upgrade) = as_weak.upgrade() {
        (upgrade)(Size::new(width, height));
    }
    //todo: balance this somehow
    std::mem::forget(as_weak);

}
pub struct Surface {
    imp: *mut c_void,
    update_size: Option<Arc<dyn Fn(Size)>>,
}
//sendable in swift!
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { SwiftAppWindow_SurfaceFree(self.imp) }
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
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        let ptr = unsafe {
            SwiftAppWindow_SurfaceRawHandle(self.imp)
        };
        RawWindowHandle::AppKit(AppKitWindowHandle::new(NonNull::new(ptr as *mut _).unwrap()))
    }
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::AppKit(AppKitDisplayHandle::new())
    }
    /**
    Run the attached callback when size changes.
    */
    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, update: F) {
        let strong_update = Arc::new(update);
        let weak = Weak::into_raw(Arc::downgrade(&strong_update));
        self.update_size = Some(strong_update);

        unsafe{SwiftAppWindow_SurfaceSizeUpdate(weak as *mut c_void, self.imp, notify_size::<F> as *mut c_void)}
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            SwiftAppWindow_WindowFree(self.imp);
        }
    }
}