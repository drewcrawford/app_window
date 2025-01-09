use std::cell::RefCell;
use std::fs::File;
use std::os::fd::AsFd;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use libc::{getpid, pid_t, syscall, SYS_gettid};
use memmap2::MmapMut;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_client::globals::{registry_queue_init, GlobalList, GlobalListContents};
use wayland_client::protocol::{wl_compositor, wl_registry, wl_shm};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
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
    let current_pid = unsafe{getpid()};
    let main_thread_pid = unsafe{syscall(SYS_gettid)} as pid_t;
    current_pid == main_thread_pid
}

static MAIN_THREAD_SENDER: OnceLock<Sender<Box<dyn FnOnce() + Send>>> = OnceLock::new();

struct MainThreadInfo {
    globals: GlobalList,
    queue_handle: QueueHandle<App>,
}


thread_local! {
    static MAIN_THREAD_INFO: RefCell<Option<MainThreadInfo>> = RefCell::new(None);
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    let connection = Connection::connect_to_env().expect("Failed to connect to wayland server");
    let display = connection.display();
    let (globals, mut event_queue) = registry_queue_init::<App>(&connection).expect("Can't initialize registry");
    let qh = event_queue.handle();
    let mut app = App;
    MAIN_THREAD_INFO.replace(Some(MainThreadInfo{globals, queue_handle: qh}));
    closure();
    //park
    loop {
        event_queue.blocking_dispatch(&mut app).unwrap();
    }
}

pub fn on_main_thread<F: FnOnce()>(_closure: F) {
    todo!()
}

pub struct Window {

}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

struct App;

fn create_shm_buffer(
    _shm: &wl_shm::WlShm,
    width: u32,
    height: u32,
) -> (File, MmapMut) {
    let stride = width * 4;
    let size = stride * height;
    let file = tempfile::tempfile().unwrap();
    file.set_len(size as u64).unwrap();

    let mut mmap = unsafe{MmapMut::map_mut(&file)}.unwrap();

    for pixel in mmap.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 0xFF, 0xFF]); //I guess due to endiannness we are actually BGRA?
    }

    (file, mmap)
}


impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for App {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _qh: &QueueHandle<App>,
    ) {
        println!("Got registry event {:?}",event);
    }
}
impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<App>,
    ) {
        println!("Got registry event {:?}",event);
    }
}
impl Dispatch<XdgWmBase, ()> for App {
    fn event(_state: &mut Self, proxy: &XdgWmBase, event: <XdgWmBase as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        match event {
            wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping {serial}  => {
                proxy.pong(serial)
            }
            _ => {
                println!("Unknown XdgWmBase event: {:?}", event); // Add this line

            }
        }
    }
}

impl Dispatch<WlCompositor, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlCompositor, event: <WlCompositor as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("Got compositor event {:?}",event);
    }
}

impl Dispatch<WlShm, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlShm, event: <WlShm as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("Got shm event {:?}",event);
    }
}
impl Dispatch<WlSurface, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlSurface, event: <WlSurface as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlSurface event {:?}",event);
    }
}
impl Dispatch<XdgSurface, ()> for App {
    fn event(_state: &mut Self, _proxy: &XdgSurface, event: <XdgSurface as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got XdgSurface event {:?}",event);
    }
}
impl Dispatch<XdgToplevel, ()> for App {
    fn event(_state: &mut Self, _proxy: &XdgToplevel, event: <XdgToplevel as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got XdgToplevel event {:?}",event);
        // match event {
        //     xdg_toplevel::Event::Configure {  width, height, states: _ } => {
        //         xdg_toplevel_configure_event(width, height);
        //     }
        //     _ => {
        //         println!("got XdgToplevel event {:?}",event);
        //
        //     }
        // }
    }
}
impl Dispatch<WlShmPool, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlShmPool, event: <WlShmPool as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlShmPool event {:?}",event);
    }
}
impl Dispatch<WlBuffer, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlBuffer, event: <WlBuffer as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlBuffer event {:?}",event);
    }
}


impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        crate::application::on_main_thread(|| {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            let xdg_wm_base: XdgWmBase = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let compositor: wl_compositor::WlCompositor = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let shm: WlShm = info.globals.bind(&info.queue_handle, 2..=2, ()).unwrap();
            let surface = compositor.create_surface(&info.queue_handle, ());
            // Create a toplevel surface
            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &info.queue_handle, ());
            xdg_surface.get_toplevel(&info.queue_handle, ());

            let (file, mmap) = create_shm_buffer(&shm, 200, 200);
            let pool = shm.create_pool(file.as_fd(), mmap.len() as i32, &info.queue_handle, ());
            let buffer = pool.create_buffer(
                0,
                200,
                200,
                200 * 4,
                Format::Argb8888,
                &info.queue_handle,
                (),
            );
            surface.attach(Some(&buffer), 0, 0);
            surface.commit();

            // let seat: WlSeat = globals.bind(&qh, 8..=9, ()).expect("Can't bind seat");
            // let _pointer = seat.get_pointer(&qh, surface.id());
            // let _keyboard = seat.get_keyboard(&qh, surface.id());


            MAIN_THREAD_INFO.replace(Some(info));
        }).await;



        todo!();

    }

    pub async fn default() -> Self {
        Window::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), "app_window".to_string()).await
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