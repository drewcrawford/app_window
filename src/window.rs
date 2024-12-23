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
}

impl Default for Window {
    fn default() -> Self {
        Window {
            sys: crate::sys::Window::default(),
        }
    }
}