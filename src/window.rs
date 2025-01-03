use std::fmt::Display;
use crate::application::CALL_MAIN;
use crate::coordinates::{Position, Size};
use crate::surface::Surface;
use crate::sys;

/**
A platform-appropriate surface.
*/
pub struct Window {
    sys: crate::sys::Window,
}

#[derive(thiserror::Error,Debug)]
pub struct FullscreenError(#[from]sys::FullscreenError);

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Window {
    pub async fn fullscreen(title: String) -> Result<Self,FullscreenError> {
        assert!(crate::application::is_main_thread_running(), "{}",CALL_MAIN);
        let sys = crate::sys::Window::fullscreen(title).await?;
        Ok(Window {
            sys: sys
        })
    }
    pub fn new(position: Position, size: Size, title: String) -> Self {
        assert!(crate::application::is_main_thread_running(), "Call app_window::application::run_main_thread");
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
        assert!(crate::application::is_main_thread_running(), "Call app_window::application::run_main_thread");
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