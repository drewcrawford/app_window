use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::coordinates::Size;
use crate::sys;

pub struct Surface {
    pub(super) sys: sys::Surface,
}

impl Surface {
    pub async fn size(&self) -> Size {
        self.sys.size().await
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        self.sys.raw_window_handle()
    }
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        self.sys.raw_display_handle()
    }
    /**
    Run the attached callback when size changes.
    */
    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, update: F) {
        self.sys.size_update(update)
    }

    #[cfg(feature = "wgpu")]
    pub async fn create_wgpu_surface(&self, instance: &std::sync::Arc<wgpu::Instance>) -> Result<wgpu::Surface,wgpu::CreateSurfaceError> {
        use wgpu::SurfaceTargetUnsafe;

        //on this wasm32 we can't send instance
        //on linux can't run on main thread?
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
}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}