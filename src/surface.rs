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
    pub fn create_wgpu_surface(&self, instance: &wgpu::Instance) -> Result<wgpu::Surface,wgpu::CreateSurfaceError> {
        use wgpu::SurfaceTargetUnsafe;

        //on this wasm32 we can't send instance
        Ok(unsafe{instance.create_surface_unsafe(
            SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: self.raw_display_handle(),
                raw_window_handle: self.raw_window_handle(),
            }
        )?})
    }
}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}