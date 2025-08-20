//SPDX-License-Identifier: MPL-2.0
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};
use wayland_cursor::CursorTheme;

use super::main_thread::on_main_thread;
use super::{App, AppState, BUTTON_WIDTH, SurfaceEvents, TITLEBAR_HEIGHT};
use crate::coordinates::{Position, Size};

const CURSOR_SIZE: i32 = 16;

#[derive(Clone, PartialEq)]
pub struct CursorRequest {
    pub name: &'static str,
    pub hot_x: i32,
    pub hot_y: i32,
}

impl CursorRequest {
    pub fn wait() -> Self {
        CursorRequest {
            name: "wait",
            hot_x: 0,
            hot_y: 0,
        }
    }
    pub fn right_side() -> Self {
        CursorRequest {
            name: "right_side",
            hot_x: CURSOR_SIZE / 2,
            hot_y: 0,
        }
    }
    pub fn bottom_side() -> Self {
        CursorRequest {
            name: "bottom_side",
            hot_x: 0,
            hot_y: CURSOR_SIZE / 2,
        }
    }
    pub fn left_ptr() -> Self {
        CursorRequest {
            name: "left_ptr",
            hot_x: CURSOR_SIZE / 8,
            hot_y: CURSOR_SIZE / 8,
        }
    }
    pub fn bottom_right_corner() -> Self {
        CursorRequest {
            name: "bottom_right_corner",
            hot_x: CURSOR_SIZE / 2,
            hot_y: CURSOR_SIZE / 2,
        }
    }
}

pub struct ActiveCursor {
    pub cursor_surface: Arc<WlSurface>,
    pub cursor_sender: Sender<CursorRequest>,
    pub active_request: Arc<Mutex<CursorRequest>>,
}

impl ActiveCursor {
    pub(super) fn new(
        connection: &Connection,
        shm: WlShm,
        _a: &Arc<AppState>,
        compositor: &WlCompositor,
        queue_handle: &QueueHandle<App>,
    ) -> Self {
        let mut cursor_theme =
            CursorTheme::load(connection, shm, CURSOR_SIZE as u32).expect("Can't load cursors");
        cursor_theme
            .set_fallback(|_, _| Some(include_bytes!("../../../linux_assets/left_ptr").into()));
        let cursor = cursor_theme.get_cursor("wait").expect("Can't get cursor");
        //I guess we fake an internal window here?
        let cursor_surface = compositor.create_surface(queue_handle, SurfaceEvents::Cursor);
        let start_time = std::time::Instant::now();
        let frame_info = cursor.frame_and_duration(start_time.elapsed().as_millis() as u32);
        let buffer = &cursor[frame_info.frame_index];
        cursor_surface.attach(Some(buffer), 0, 0);
        cursor_surface.commit();
        let cursor_surface = Arc::new(cursor_surface);
        let move_cursor_surface = cursor_surface.clone();
        let move_cursor_theme = Arc::new(Mutex::new(cursor_theme));
        let (cursor_request_sender, cursor_request_receiver) = std::sync::mpsc::channel();
        let active_request = Arc::new(Mutex::new(CursorRequest::wait()));
        let move_active_request = active_request.clone();
        std::thread::Builder::new()
            .name("Cursor thread".to_string())
            .spawn(move || {
                loop {
                    let move_cursor_theme = move_cursor_theme.clone();
                    let move_cursor_surface = move_cursor_surface.clone();
                    let mt_active_request = move_active_request.clone();
                    let (sender, receiver) = std::sync::mpsc::channel();

                    on_main_thread(move || {
                        let mut binding = move_cursor_theme.lock().unwrap();
                        let cursor = binding
                            .get_cursor(mt_active_request.lock().unwrap().name)
                            .expect("Can't get cursor");
                        let present_time = start_time.elapsed();
                        let frame_info = cursor.frame_and_duration(present_time.as_millis() as u32);
                        let buffer = &cursor[frame_info.frame_index];
                        move_cursor_surface.attach(Some(buffer), 0, 0);
                        move_cursor_surface.damage_buffer(
                            0,
                            0,
                            buffer.dimensions().0 as i32,
                            buffer.dimensions().1 as i32,
                        );
                        move_cursor_surface.commit();
                        let next_present_time =
                            present_time + Duration::from_millis(frame_info.frame_duration as u64);
                        sender
                            .send(next_present_time)
                            .expect("Can't send next present time");
                    });
                    let next_present_time =
                        receiver.recv().expect("Can't receive next present time");
                    let sleep_time = next_present_time.saturating_sub(start_time.elapsed());
                    // println!("sleep_time {:?}", sleep_time);
                    match cursor_request_receiver.recv_timeout(sleep_time) {
                        Ok(request) => {
                            *move_active_request.lock().unwrap() = request;
                        }
                        Err(e) => {
                            match e {
                                std::sync::mpsc::RecvTimeoutError::Timeout => {
                                    //continue
                                }
                                std::sync::mpsc::RecvTimeoutError::Disconnected => {
                                    panic!("Cursor request channel disconnected");
                                }
                            }
                        }
                    }
                }
            })
            .expect("Can't launch cursor thread");

        ActiveCursor {
            cursor_surface,
            cursor_sender: cursor_request_sender,
            active_request,
        }
    }
    pub fn cursor_request(&self, request: CursorRequest) {
        self.cursor_sender
            .send(request)
            .expect("Can't send cursor request");
    }
}

pub enum MouseRegion {
    BottomRight,
    Bottom,
    Right,
    Titlebar,
    CloseButton,
    MaximizeButton,
    MinimizeButton,
    Client,
}

impl MouseRegion {
    pub fn from_position(size: Size, position: Position) -> Self {
        const EDGE_REGION: f64 = 10.0;
        if position.y() < TITLEBAR_HEIGHT as f64
            && position.x() > size.width() - BUTTON_WIDTH as f64
        {
            MouseRegion::CloseButton
        } else if position.y() < TITLEBAR_HEIGHT as f64
            && position.x() > size.width() - BUTTON_WIDTH as f64 * 2.0
        {
            MouseRegion::MaximizeButton
        } else if position.y() < TITLEBAR_HEIGHT as f64
            && position.x() > size.width() - BUTTON_WIDTH as f64 * 3.0
        {
            MouseRegion::MinimizeButton
        } else if position.y() < TITLEBAR_HEIGHT as f64 {
            MouseRegion::Titlebar
        } else if size.width() - position.x() < EDGE_REGION {
            if size.height() - position.y() < EDGE_REGION {
                MouseRegion::BottomRight
            } else {
                MouseRegion::Right
            }
        } else if size.height() - position.y() < EDGE_REGION {
            MouseRegion::Bottom
        } else {
            MouseRegion::Client
        }
    }
}
