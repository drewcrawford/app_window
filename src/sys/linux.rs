use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::coordinates::{Position, Size};

#[derive(Debug)]
pub struct FullscreenError;

impl std::error::Error for FullscreenError {}

impl std::fmt::Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub fn is_main_thread() -> bool {
    todo!()
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(_closure: F) {
    todo!()
}

pub fn on_main_thread<F: FnOnce()>(_closure: F) {
    todo!()
}

pub struct Window {

}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub async fn new(_position: Position, _size: Size, _title: String) -> Self {
        todo!()
    }

    pub async fn default() -> Self {
        todo!()
    }

    pub async fn fullscreen(_title: String) -> Result<Self, FullscreenError> {
        todo!()
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        todo!()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        todo!()
    }
}

pub struct Surface {
}

unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl Surface {
    pub async fn size(&self) -> Size {
        todo!()
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        todo!()
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        todo!()
    }

    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, _update: F) {
        todo!()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        todo!()
    }
}