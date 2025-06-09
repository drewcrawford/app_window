//SPDX-License-Identifier: MPL-2.0

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::coordinates::Size;
use crate::sys;

/// A type that can be drawn on, e.g. by wgpu.
/// 
/// A `Surface` represents a drawable area within a window. It provides the necessary
/// handles for graphics APIs like wgpu to render content, and notifies about size changes.
///
/// # Creating a Surface
/// 
/// A Surface is created from a [`Window`](crate::window::Window) using the 
/// [`Window::surface()`](crate::window::Window::surface) method. Only one Surface can 
/// be created per window.
///
/// ```no_run
/// # async fn example() {
/// use app_window::window::Window;
/// use app_window::coordinates::{Position, Size};
/// 
/// let mut window = Window::new(
///     Position::new(100.0, 100.0),
///     Size::new(800.0, 600.0),
///     "My Window".to_string()
/// ).await;
/// 
/// let surface = window.surface().await;
/// # }
/// ```
///
/// # Platform Implementation Details
///
/// The Surface abstraction exists for several reasons:
///
/// 1. **Future flexibility**: While currently limited to one surface per window, this
///    design allows for potential multi-surface support in the future.
/// 2. **Performance**: On some platforms, creating a drawable surface is more expensive
///    than creating a blank window. Applications that don't need to draw can skip this cost.
/// 3. **Compositing**: Platform window decorations (title bars, borders) often require
///    special handling when composited with the application's rendered content.
#[derive(Debug)]
#[must_use = "Dropping a surface may release resources"]
pub struct Surface {
    pub(super) sys: sys::Surface,
}

impl Surface {
    /// Returns the size and scale factor of the surface.
    /// 
    /// The size is returned in logical pixels, which may differ from physical pixels
    /// on high-DPI displays. The scale factor indicates the ratio between logical
    /// and physical pixels.
    /// 
    /// # Returns
    /// 
    /// A tuple containing:
    /// - The [`Size`] of the surface in logical pixels
    /// - The scale factor as a `f64` (e.g., 2.0 for a Retina display)
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::window::Window;
    /// # let mut window: Window = todo!();
    /// let mut surface = window.surface().await;
    /// let (size, scale) = surface.size_scale().await;
    /// 
    /// println!("Surface size: {}x{}", size.width(), size.height());
    /// println!("Scale factor: {}", scale);
    /// 
    /// // Calculate physical pixel dimensions
    /// let physical_width = (size.width() * scale) as u32;
    /// let physical_height = (size.height() * scale) as u32;
    /// # }
    /// ```
    pub async fn size_scale(&self) -> (Size,f64) {
        self.sys.size_scale().await
    }

    /// Returns the raw window handle for this surface.
    /// 
    /// This handle can be used with graphics APIs like wgpu to create a rendering surface.
    /// The handle is platform-specific and follows the [`raw-window-handle`](https://docs.rs/raw-window-handle)
    /// standard.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::window::Window;
    /// # let mut window: Window = todo!();
    /// let surface = window.surface().await;
    /// let raw_handle = surface.raw_window_handle();
    /// 
    /// // Use with wgpu (requires 'wgpu' feature)
    /// # #[cfg(feature = "wgpu")]
    /// # {
    /// # use wgpu::{Instance, SurfaceTargetUnsafe};
    /// # let instance: Instance = todo!();
    /// unsafe {
    ///     let wgpu_surface = instance.create_surface_unsafe(
    ///         SurfaceTargetUnsafe::RawHandle {
    ///             raw_display_handle: surface.raw_display_handle(),
    ///             raw_window_handle: raw_handle,
    ///         }
    ///     );
    /// }
    /// # }
    /// # }
    /// ```
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        self.sys.raw_window_handle()
    }
    
    /// Returns the raw display handle for this surface.
    /// 
    /// This handle represents the display or connection that owns the window. It's required
    /// alongside the window handle when creating graphics surfaces. The handle is
    /// platform-specific and follows the [`raw-window-handle`](https://docs.rs/raw-window-handle)
    /// standard.
    /// 
    /// # Platform Details
    /// 
    /// - **Windows**: Returns a Windows display handle
    /// - **macOS**: Returns an AppKit display handle  
    /// - **Linux (Wayland)**: Returns a Wayland display handle
    /// - **Web**: Returns a web display handle
    /// 
    /// # Example
    /// 
    /// See [`raw_window_handle()`](Self::raw_window_handle) for usage with graphics APIs.
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        self.sys.raw_display_handle()
    }
    /// Registers a callback to be invoked when the surface is resized.
    /// 
    /// The callback will be called whenever the surface size changes, such as when the user
    /// resizes the window or the window is moved between displays with different DPI settings.
    /// The callback receives the new [`Size`] in logical pixels.
    /// 
    /// # Thread Safety
    /// 
    /// The callback must be `Send` and `'static` as it may be called from different threads
    /// depending on the platform. The callback should be efficient as it may be called
    /// frequently during resize operations.
    /// 
    /// # Example
    /// 
    /// ```no_run
    /// # async fn example() {
    /// # use app_window::window::Window;
    /// # use app_window::coordinates::Size;
    /// # let mut window: Window = todo!();
    /// let mut surface = window.surface().await;
    /// 
    /// surface.size_update(|new_size: Size| {
    ///     println!("Surface resized to: {}x{}", 
    ///              new_size.width(), 
    ///              new_size.height());
    ///     
    ///     // Trigger a re-render or update your graphics pipeline
    ///     // with the new dimensions
    /// });
    /// # }
    /// ```
    /// 
    /// # Platform Behavior
    /// 
    /// - **All platforms**: The callback is invoked after the resize has occurred
    /// - **macOS**: May be called multiple times during a resize drag operation
    /// - **Windows/Linux**: Typically called at the end of a resize operation
    pub fn size_update<F: Fn(Size) + Send + 'static>(&mut self, update: F) {
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