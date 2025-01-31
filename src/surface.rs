use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::coordinates::Size;
use crate::sys;

/**
A type that can be drawn on, e.g. by wgpu.

Generally, one Surface can be created from an underlying window.
*/
pub struct Surface {
    pub(super) sys: sys::Surface,
}

impl Surface {
    /**
    Returns the size of the surface.
    */
    pub async fn size(&self) -> Size {
        self.sys.size().await
    }

    /**
    Returns the raw window handle.
    */
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        self.sys.raw_window_handle()
    }
    /**
    Returns the raw display handle.
*/
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        self.sys.raw_display_handle()
    }
    /**
    Cause the provided callback to run on resize.
    */
    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, update: F) {
        self.sys.size_update(update)
    }

    /**
    Create a wgpu surface in a platform-appropriate way.
*/
    #[cfg(feature = "wgpu")]
    pub async fn create_wgpu_surface(&self, instance: &std::sync::Arc<wgpu::Instance>) -> Result<wgpu::Surface,wgpu::CreateSurfaceError> {
        use wgpu::SurfaceTargetUnsafe;

        //on this wasm32 we can't send instance
        //on linux can't run on main thread?
        #[cfg(target_os = "linux")] { //and similar NotMainThread platform
            let (sender,fut) = r#continue::continuation();

            let display_handle = send_cells::unsafe_send_cell::UnsafeSendCell::new(self.raw_display_handle());
            let window_handle = send_cells::unsafe_send_cell::UnsafeSendCell::new(self.raw_window_handle());
            let move_instance = instance.clone();
            std::thread::spawn(move ||{
                let surface_target = SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: unsafe{*display_handle.get()},
                    raw_window_handle: unsafe{*window_handle.get()},
                };
                let r = unsafe{move_instance.create_surface_unsafe(surface_target)};
                sender.send(r);
            });
            let r = fut.await;
            Ok(r?)
        }
        #[cfg(not(target_os = "linux"))] //NOT NotMainThread platform (so MT or Relaxed) platform
        {
            let surface_target = SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: self.raw_display_handle(),
                raw_window_handle: self.raw_window_handle(),
            };
            let surface = unsafe { instance.create_surface_unsafe(surface_target) };
            Ok(surface?)
        }

    }
}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}