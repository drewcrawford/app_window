use std::sync::Arc;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wgpu::SurfaceTargetUnsafe;
use crate::application::on_main_thread;
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
    pub async fn create_wgpu_surface(self: Arc<Self>, instance: &Arc<wgpu::Instance>) -> wgpu::Surface {
        let clone_instance = instance.clone();
        on_main_thread(move || unsafe {

            clone_instance.create_surface_unsafe(
                SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: self.raw_display_handle(),
                    raw_window_handle: self.raw_window_handle(),
                }
            )
        }).await.expect("Failed to create surface")
    }
}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}