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

}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}