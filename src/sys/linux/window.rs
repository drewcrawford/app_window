//SPDX-License-Identifier: MPL-2.0
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::QueueHandle;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

use crate::coordinates::{Position, Size};
use super::{App, AppState, Configure, FullscreenError, SurfaceEvents, Surface};
use super::ax::AX;
use super::buffer::{AllocatedBuffer, create_shm_buffer_decor};
use super::main_thread::MAIN_THREAD_INFO;

pub struct DebugWrapper(pub Box<dyn Fn(Size) + Send>);
impl Debug for DebugWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DebugWrapper")
    }
}

#[derive(Debug)]
pub(crate) struct Window {
    pub(super) internal: Arc<Mutex<WindowInternal>>,
}

#[derive(Debug)]
pub(super) struct WindowInternal {
    pub app_state: Weak<AppState>,
    pub proposed_configure: Option<Configure>,
    pub applied_configure: Option<Configure>,
    pub wl_pointer_enter_serial: Option<u32>,
    pub wl_pointer_enter_surface: Option<WlSurface>,
    pub wl_pointer_pos: Option<Position>,
    pub xdg_toplevel: Option<XdgToplevel>,
    pub wl_surface: Option<WlSurface>,
    pub xdg_surface: Option<XdgSurface>,
    pub drawable_buffer: Option<AllocatedBuffer>,
    pub requested_maximize: bool,
    pub adapter: Option<accesskit_unix::Adapter>,
    pub size_update_notify: Option<DebugWrapper>,
    pub decor_subsurface: Option<WlSubsurface>,
    pub title: String,
    pub current_outputs: HashSet<u32>,
}

impl WindowInternal {
    fn new(
        app_state: &Arc<AppState>,
        size: Size,
        title: String,
        queue_handle: &QueueHandle<App>,
        ax: bool,
    ) -> Arc<Mutex<Self>> {
        let window_internal = Arc::new(Mutex::new(WindowInternal {
            title: title.clone(),
            app_state: Arc::downgrade(app_state),
            proposed_configure: None,
            //in case we are asked for size prior to configure?
            applied_configure: Some(Configure {
                width: size.width() as i32,
                height: size.height() as i32,
            }),
            wl_pointer_enter_serial: None,
            wl_pointer_enter_surface: None,
            wl_pointer_pos: None,
            xdg_toplevel: None,
            wl_surface: None,
            requested_maximize: false,
            drawable_buffer: None,
            adapter: None,
            size_update_notify: None,
            decor_subsurface: None,
            xdg_surface: None,
            current_outputs: HashSet::new(),
        }));
        if ax {
            let _aximpl = AX::new(size, title.clone(), window_internal.clone());
            let adapter = Some(accesskit_unix::Adapter::new(
                _aximpl.clone(),
                _aximpl.clone(),
                _aximpl.clone(),
            ));
            let buffer = AllocatedBuffer::new(size.width() as i32, size.height() as i32, &app_state.shm, queue_handle, window_internal.clone());
            window_internal.lock().unwrap().drawable_buffer = Some(buffer);
            window_internal.lock().unwrap().adapter = adapter;
        }
        window_internal
    }
    
    pub fn applied_size(&self) -> Size {
        let applied = self.applied_configure.clone().expect("No configure event");
        Size::new(applied.width as f64, applied.height as f64)
    }

    pub fn close_window(&self) {
        if let Some(e) = self.xdg_toplevel.as_ref() {
            e.destroy()
        }
        if let Some(s) = self.xdg_surface.as_ref() {
            s.destroy()
        }
        if let Some(s) = self.wl_surface.as_ref() {
            s.destroy()
        }
    }
    
    pub fn maximize(&mut self) {
        if self.requested_maximize {
            self.requested_maximize = false;
            let toplevel = self.xdg_toplevel.as_ref().unwrap();
            toplevel.unset_maximized();
        } else {
            self.requested_maximize = true;
            let toplevel = self.xdg_toplevel.as_ref().unwrap();
            toplevel.set_maximized();
        }
    }
    
    pub fn minimize(&self) {
        let toplevel = self.xdg_toplevel.as_ref().unwrap();
        toplevel.set_minimized();
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub async fn new(_position: Position, size: Size, title: String) -> Self {
        let window_internal = crate::application::on_main_thread("Window::new".to_string(),move || {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            let xdg_wm_base: XdgWmBase = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let window_internal =
                WindowInternal::new(&info.app_state, size, title, &info.queue_handle, true);

            let surface = info.app_state.compositor.create_surface(
                &info.queue_handle,
                SurfaceEvents::Standard(window_internal.clone()),
            );

            let decor_surface = info
                .app_state
                .compositor
                .create_surface(&info.queue_handle, SurfaceEvents::Decor);
            let decor_subsurface =
                info.subcompositor
                    .get_subsurface(&decor_surface, &surface, &info.queue_handle, ());
            let decor_buffer = create_shm_buffer_decor(
                &info.app_state.shm,
                &info.queue_handle,
                window_internal.clone(),
            );
            decor_surface.attach(Some(&decor_buffer.buffer), 0, 0);
            decor_surface.commit();
            decor_subsurface.set_position(
                size.width() as i32 - info.app_state.decor_dimensions.0 as i32,
                0,
            );
            window_internal
                .lock()
                .unwrap()
                .decor_subsurface
                .replace(decor_subsurface);
            window_internal
                .lock()
                .unwrap()
                .wl_surface
                .replace(surface.clone());

            // Create a toplevel surface
            let xdg_surface =
                xdg_wm_base.get_xdg_surface(&surface, &info.queue_handle, window_internal.clone());
            let xdg_toplevel =
                xdg_surface.get_toplevel(&info.queue_handle, window_internal.clone());
            window_internal
                .lock()
                .unwrap()
                .xdg_surface
                .replace(xdg_surface);

            window_internal
                .lock()
                .unwrap()
                .xdg_toplevel
                .replace(xdg_toplevel);

            //convert to compositor-owned buffer
            let mut lock = window_internal.lock().unwrap();
            let drawable_buffer = lock.drawable_buffer.take().expect("No drawable buffer available");
            surface.attach(
                Some(
                    &drawable_buffer.buffer
                ),
                0,
                0,
            );
            drop(lock);
            surface.commit();

            let seat: WlSeat = info
                .globals
                .bind(&info.queue_handle, 8..=9, ())
                .expect("Can't bind seat");
            window_internal
                .lock()
                .unwrap()
                .app_state
                .upgrade()
                .unwrap()
                .seat
                .lock()
                .unwrap()
                .replace(seat.clone());
            let _pointer = seat.get_pointer(&info.queue_handle, window_internal.clone());
            let _keyboard = seat.get_keyboard(&info.queue_handle, window_internal.clone());

            MAIN_THREAD_INFO.replace(Some(info));
            window_internal
        })
        .await;

        Window {
            internal: window_internal,
        }
    }

    pub async fn default() -> Self {
        Window::new(
            Position::new(0.0, 0.0),
            Size::new(800.0, 600.0),
            "app_window".to_string(),
        )
        .await
    }

    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        let w = Self::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), title).await;
        w.internal
            .lock()
            .unwrap()
            .xdg_toplevel
            .as_ref()
            .expect("No xdg_toplevel")
            .set_fullscreen(None);
        Ok(w)
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        let display = crate::application::on_main_thread("surface".to_string(), || {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            info.connection.display()
        })
        .await;
        let surface = self
            .internal
            .lock()
            .unwrap()
            .wl_surface
            .as_ref()
            .expect("No surface")
            .clone();
        crate::surface::Surface {
            sys: Surface {
                wl_display: display,
                wl_surface: surface,
                window_internal: self.internal.clone(),
            },
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.internal.lock().unwrap().close_window();
    }
}