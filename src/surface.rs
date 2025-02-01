use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::coordinates::Size;
use crate::sys;

/**
A type that can be drawn on, e.g. by wgpu.

Generally, only one Surface can be created from an underlying [crate::window::Window].

# Discussion

You may well ask, why have a separate Surface type?  The answer is:

1.  Backwards-compatibility; maybe we want to support more than one surface per window later
2.  On some platforms setting up a surface for drawing is more complex than simply opening a blank window and so this can be skipped if you don't need it.
3.  On most platforms, drawing decorations (title bar, etc) requires some type of compositing with the surface, perhaps by the OS but perhaps by us.

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
    Causes the provided callback to run when the window is resized.
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