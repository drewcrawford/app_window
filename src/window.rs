use crate::coordinates::{Position, Size};
use crate::surface::Surface;
/**
A platform-appropriate surface.
*/
pub struct Window {
    sys: crate::sys::Window,
}

impl Window {
    pub fn fullscreen(title: String) -> Self {
        Window {
            sys: crate::sys::Window::fullscreen(title)
        }
    }
    pub fn new(position: Position, size: Size, title: String) -> Self {
        Window {
            sys: crate::sys::Window::new(position, size, title)
        }
    }

    pub async fn surface(&self) -> Surface {
        self.sys.surface().await
    }

}

impl Default for Window {
    fn default() -> Self {
        Window {
            sys: crate::sys::Window::default(),
        }
    }
}

#[cfg(test)] mod test {
    use crate::window::Window;

    #[test] fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Window>();
        fn assert_sync<T: Sync>() {}
        assert_sync::<Window>();
    }
}