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
        todo!()
    }
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        todo!()
    }
}

#[cfg(test)] mod tests {
    use crate::surface::Surface;

    #[test] fn send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<Surface>();
    }
}