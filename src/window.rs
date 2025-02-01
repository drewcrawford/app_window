use std::fmt::Display;
use crate::application::CALL_MAIN;
use crate::coordinates::{Position, Size};
use crate::surface::Surface;
use crate::sys;

/**
A platform-appropriate window.

See the crate documentation for more information on what backend this will use.
*/
pub struct Window {
    sys: crate::sys::Window,
    created_surface: bool,
}

/**
An error that can occur when creating a fullscreen window.
*/
#[derive(thiserror::Error,Debug)]
pub struct FullscreenError(#[from]sys::FullscreenError);

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Window {
    /**
    Create a fullscreen window.
*/
    pub async fn fullscreen(title: String) -> Result<Self,FullscreenError> {
        assert!(crate::application::is_main_thread_running(), "{}",CALL_MAIN);
        let sys = crate::sys::Window::fullscreen(title).await?;
        Ok(Window {
            sys: sys,
            created_surface: false,
        })
    }
    /**
    Create a new window.
*/
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        assert!(crate::application::is_main_thread_running(), "Call app_window::application::run_main_thread");
        Window {
            sys: crate::sys::Window::new(position, size, title).await,
            created_surface: false,
        }
    }

    /**
    Create a new Surface.
*/
    pub async fn surface(&mut self) -> Surface {
        assert!(!self.created_surface, "Surface already created");
        self.created_surface = true;
        self.sys.surface().await
    }

    /**
    Create a new Window with default options appropriate for the platform.
*/
    pub async fn default() -> Self {
        assert!(crate::application::is_main_thread_running(), "Call app_window::application::run_main_thread");
        Window {
            sys: crate::sys::Window::default().await,
            created_surface: false,
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