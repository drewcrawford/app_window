use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use io_uring::cqueue::Entry;
use libc::{eventfd, getpid, pid_t, syscall, SYS_gettid, EFD_SEMAPHORE};
use memmap2::MmapMut;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_client::globals::{registry_queue_init, GlobalList, GlobalListContents};
use wayland_client::protocol::{wl_compositor, wl_registry, wl_shm};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_cursor::CursorTheme;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel;
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
struct MainThreadSender {
    sender: Sender<Box<dyn FnOnce() + Send>>,
    eventfd: c_int,
}

impl MainThreadSender {
    fn send(&self, closure: Box<dyn FnOnce() + Send>)  {
        self.sender.send(closure).expect("Can't send closure");
        let val = 1 as u64;
        let w = unsafe{libc::write(self.eventfd, &val as *const _ as *const c_void, std::mem::size_of_val(&val))};
        assert_eq!(w, std::mem::size_of_val(&val) as isize, "Failed to write to eventfd: {err}",err=unsafe{*libc::__errno_location()});
    }
}

static MAIN_THREAD_SENDER: OnceLock<MainThreadSender> = OnceLock::new();

struct MainThreadInfo {
    globals: GlobalList,
    queue_handle: QueueHandle<App>,
    connection: Connection,
    app_state: Arc<AppState>,
}



thread_local! {
    static MAIN_THREAD_INFO: RefCell<Option<MainThreadInfo>> = RefCell::new(None);
}

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    let connection = Connection::connect_to_env().expect("Failed to connect to wayland server");
    let display = connection.display();
    let (globals, mut event_queue) = registry_queue_init::<App>(&connection).expect("Can't initialize registry");
    let qh = event_queue.handle();
    let mut app = App(Arc::new(AppState::new()));
    MAIN_THREAD_INFO.replace(Some(MainThreadInfo{globals, queue_handle: qh, connection, app_state: app.0.clone()}));
    let mut io_uring = io_uring::IoUring::new(2).expect("Failed to create io_uring");
    let channel_read_event = unsafe{eventfd(0, EFD_SEMAPHORE)};
    assert_ne!(channel_read_event, -1, "Failed to create eventfd");
    let (sender, receiver) = channel();

    MAIN_THREAD_SENDER.get_or_init(|| {
        MainThreadSender{sender, eventfd: channel_read_event}
    });
    closure();
    event_queue.flush().expect("Failed to flush event queue");
    let mut read_guard = event_queue.prepare_read().expect("Failed to prepare read");
    const WAYLAND_DATA_AVAILABLE: u64 = 1;
    const CHANNEL_DATA_AVAILABLE: u64 = 2;
    let fd = read_guard.connection_fd();
    let io_uring_fd = io_uring::types::Fd(fd.as_raw_fd());
    let mut wayland_entry = io_uring::opcode::PollAdd::new(io_uring_fd, libc::POLLIN as u32).build();
    wayland_entry = wayland_entry.user_data(WAYLAND_DATA_AVAILABLE);
    let mut sqs = io_uring.submission();
    unsafe{sqs.push(&wayland_entry)}.expect("Can't submit peek");
    let mut eventfd_opcode = io_uring::opcode::PollAdd::new(io_uring::types::Fd(channel_read_event), libc::POLLIN as u32).build();
    eventfd_opcode = eventfd_opcode.user_data(CHANNEL_DATA_AVAILABLE);
    unsafe{sqs.push(&eventfd_opcode)}.expect("Can't submit peek");
    drop(sqs);
    //park
    loop {
        println!("will submit_and_wait...");
        io_uring.submit_and_wait(1).expect("Can't submit and wait");
        let mut entries = Vec::new();
        for entry in io_uring.completion() {
            entries.push(entry);
        }
        for entry in entries {
            let result = entry.result();
            if result < 0 {
                panic!("Error in completion queue: {err}", err = result);
            }
            match entry.user_data() {
                WAYLAND_DATA_AVAILABLE => {
                    read_guard.read().expect("Can't read wayland socket");
                    event_queue.dispatch_pending(&mut app).expect("Can't dispatch events");
                    //prepare next read
                    //ensure writes queued during dispatch_pending go out (such as proxy replies, etc)
                    event_queue.flush().expect("Failed to flush event queue");
                    read_guard = event_queue.prepare_read().expect("Failed to prepare read");
                    let mut sqs = io_uring.submission();
                    wayland_entry = io_uring::opcode::PollAdd::new(io_uring_fd, libc::POLLIN as u32).build();
                    wayland_entry = wayland_entry.user_data(WAYLAND_DATA_AVAILABLE);
                    unsafe{sqs.push(&wayland_entry)}.expect("Can't submit peek");
                    //return to submit_and_wait
                },
                CHANNEL_DATA_AVAILABLE => {
                    let mut buf = [0u8; 8];
                    let r = unsafe{libc::read(channel_read_event, buf.as_mut_ptr() as *mut c_void, 8)};
                    assert_eq!(r, 8, "Failed to read from eventfd");
                    let closure = receiver.recv_timeout(Duration::from_secs(0)).expect("Failed to receive closure");
                    closure();
                    //let's ensure any writes went out to wayland
                    event_queue.flush().expect("Failed to flush event queue");

                    //submit new peek
                    let mut sqs = io_uring.submission();
                    unsafe{sqs.push(&eventfd_opcode)}.expect("Can't submit peek");
                    //return to submit_and_wait

                }
                other => {
                    unimplemented!("Unknown user data: {other}", other = other);
                }
            }


        }
    }
}

pub fn on_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    println!("sending on_main_thread...");
    MAIN_THREAD_SENDER.get().expect("Main thread sender not set").send(Box::new(closure));
}

pub struct Window {

}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

struct App(Arc<AppState>);

struct AppState {

}
impl AppState {
    fn new() -> Self {
        AppState{}
    }
}

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
                proxy.pong(serial);
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
impl Dispatch<WlSeat, ()> for App {
    fn event(_state: &mut Self, _proxy: &WlSeat, event: <WlSeat as Proxy>::Event, _data: &(), _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlSeat event {:?}",event);
    }
}

impl<A: AsRef<AppState>> Dispatch<WlPointer, A> for App {
    fn event(_state: &mut Self, _proxy: &WlPointer, event: <WlPointer as Proxy>::Event, _data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlPointer event {:?}",event);
    }
}


impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        crate::application::on_main_thread(move || {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            let xdg_wm_base: XdgWmBase = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let compositor: wl_compositor::WlCompositor = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let shm: WlShm = info.globals.bind(&info.queue_handle, 2..=2, ()).unwrap();
            let surface = compositor.create_surface(&info.queue_handle, ());

            let cursor_surface = compositor.create_surface(&info.queue_handle, ());
            // Create a toplevel surface
            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &info.queue_handle, ());
            let xdg_toplevel = xdg_surface.get_toplevel(&info.queue_handle, ());

            let (file, mmap) = create_shm_buffer(&shm, size.width() as u32, size.height() as u32);
            let pool = shm.create_pool(file.as_fd(), mmap.len() as i32, &info.queue_handle, ());
            let buffer = pool.create_buffer(
                0,
                size.width() as i32,
                size.height() as i32,
                200 * 4,
                Format::Argb8888,
                &info.queue_handle,
                (),
            );
            surface.attach(Some(&buffer), 0, 0);
            surface.commit();

            //cursor stuff?
            let mut cursor_theme = CursorTheme::load(&info.connection, shm, 32).expect("Can't load cursors");
            let cursor = cursor_theme.get_cursor("wait").expect("Can't get cursor");
            let frame_info = cursor.frame_and_duration(0); //todo: time
            let buffer = &cursor[frame_info.frame_index];
            cursor_surface.attach(Some(buffer), 0, 0);
            cursor_surface.commit();

            let seat: WlSeat = info.globals.bind(&info.queue_handle, 8..=9, ()).expect("Can't bind seat");
            let pointer = seat.get_pointer(&info.queue_handle, info.app_state.clone());
            pointer.set_cursor(0, Some(&cursor_surface), 0, 0);
            // let _keyboard = seat.get_keyboard(&qh, surface.id());

            MAIN_THREAD_INFO.replace(Some(info));
        }).await;

        Window {

        }

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