/**
A platform-appropriate surface.
*/
pub struct Window {
    sys: crate::sys::Window,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            sys: crate::sys::Window::default(),
        }
    }
}