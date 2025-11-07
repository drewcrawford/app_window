// SPDX-License-Identifier: MPL-2.0

// Re-export main types and functions
pub use buffer::AllocatedBuffer;
pub use cursor::ActiveCursor;
pub use main_thread::{alert, is_main_thread, on_main_thread, run_main_thread, stop_main_thread};
pub(crate) use window::Window;
// Module declarations
pub mod ax;
pub mod buffer;
pub mod cursor;
pub mod dispatchers;
pub mod main_thread;
pub mod window;

use crate::coordinates::Size;
use crate::sys::window::WindowInternal;
use accesskit::NodeId;
use memmap2::MmapMut;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::File;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy, QueueHandle};
use zune_png::zune_core::result::DecodingResult;

// Constants
const CLOSE_ID: NodeId = NodeId(3);
const MAXIMIZE_ID: NodeId = NodeId(4);
const MINIMIZE_ID: NodeId = NodeId(5);
const TITLEBAR_HEIGHT: u64 = 25;
const BUTTON_WIDTH: u64 = 25;

#[derive(Debug)]
pub struct FullscreenError;

impl std::error::Error for FullscreenError {}

impl std::fmt::Display for FullscreenError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
struct OutputInfo {
    scale_factor: f64,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

#[derive(Clone, Debug)]
struct Configure {
    width: i32,
    height: i32,
}

pub(super) struct App(Arc<AppState>);

enum SurfaceEvents {
    Standard(Arc<Mutex<WindowInternal>>),
    Cursor,
    Decor,
}

struct AppState {
    compositor: WlCompositor,
    shm: WlShm,
    //option for lazy-init purposes
    active_cursor: Mutex<Option<ActiveCursor>>,
    seat: Mutex<Option<WlSeat>>,
    outputs: Mutex<HashMap<u32, OutputInfo>>,
    _decor: Vec<u8>,
    decor_dimensions: (usize, usize),
}

impl AppState {
    fn new(
        queue_handle: &QueueHandle<App>,
        compositor: WlCompositor,
        connection: &Connection,
        shm: WlShm,
    ) -> Arc<Self> {
        let decor = include_bytes!("../../../linux_assets/decor.png");
        let mut decode_decor = zune_png::PngDecoder::new(decor);
        let decode = decode_decor.decode().expect("Can't decode decor");
        let dimensions = decode_decor.get_dimensions().unwrap();
        let decor = match decode {
            DecodingResult::U8(d) => d,
            _ => todo!(),
        };

        let a = Arc::new(AppState {
            compositor: compositor.clone(),
            shm: shm.clone(),
            active_cursor: Mutex::new(None),
            seat: Mutex::new(None),
            outputs: Mutex::new(HashMap::new()),
            _decor: decor,
            decor_dimensions: dimensions,
        });
        let active_cursor = ActiveCursor::new(connection, shm, &a, &compositor, queue_handle);
        a.active_cursor.lock().unwrap().replace(active_cursor);
        a
    }
}

struct BufferReleaseInfo {
    opt: Arc<Mutex<Option<ReleaseOpt>>>,
    decor: bool,
}

struct ReleaseOpt {
    _file: File,
    _mmap: Arc<MmapMut>,
    allocated_buffer: Option<AllocatedBuffer>,
    window_internal: Arc<Mutex<WindowInternal>>,
}

#[derive(Debug)]
pub struct Surface {
    wl_display: WlDisplay,
    wl_surface: WlSurface,
    window_internal: Arc<Mutex<WindowInternal>>,
}

unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl Surface {
    fn size_scale_impl(&self) -> (Size, f64) {
        let size = self.window_internal.lock().unwrap().applied_size();

        // Get the scale factor from the app state directly (accessible from any thread)
        let window_internal = self.window_internal.lock().unwrap();
        let current_outputs = window_internal.current_outputs.clone();
        let app_state = window_internal
            .app_state
            .upgrade()
            .expect("App state is gone");
        drop(window_internal);

        // Get the scale factor for the outputs this window is currently on
        let outputs = app_state.outputs.lock().unwrap();
        let scale = if current_outputs.is_empty() {
            // If no outputs are tracked yet, default to 1.0
            1.0
        } else {
            // Use the scale factor of the first output the window is on
            // In a proper implementation, you might want to use the "primary" output
            // or the one with the largest intersection area with the window
            current_outputs
                .iter()
                .filter_map(|output_id| outputs.get(output_id))
                .map(|output_info| output_info.scale_factor)
                .next()
                .unwrap_or(1.0)
        };

        (size, scale)
    }

    pub async fn size_scale(&self) -> (Size, f64) {
        self.size_scale_impl()
    }

    pub fn size_main(&self) -> (Size, f64) {
        //on this platform we can call size_scale_impl on main thread
        self.size_scale_impl()
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(self.wl_surface.id().as_ptr() as *mut c_void)
                .expect("Can't convert wayland surface to non-null"),
        ))
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        let ptr = self
            .wl_display
            .backend()
            .upgrade()
            .unwrap()
            .display_id()
            .as_ptr();

        RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(ptr as *mut c_void).expect("Can't convert wayland display to non-null"),
        ))
    }

    pub fn size_update<F: Fn(Size) + Send + 'static>(&mut self, update: F) {
        self.window_internal.lock().unwrap().size_update_notify =
            Some(window::DebugWrapper(Box::new(update)));
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        todo!()
    }
}
