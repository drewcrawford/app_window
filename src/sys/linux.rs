use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::ops::Sub;
use std::os::fd::{AsFd, AsRawFd};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock, Weak};
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
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_cursor::CursorTheme;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
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
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 6..=6, ()).unwrap();
    let shm: WlShm = globals.bind(&qh, 2..=2, ()).unwrap();
    let mut app = App(AppState::new(&qh, compositor, &connection, shm));
    let main_thread_info = MainThreadInfo{globals, queue_handle: qh, connection, app_state: app.0.clone()};

    MAIN_THREAD_INFO.replace(Some(main_thread_info));
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

struct WindowInternal {
    app_state: Weak<AppState>,
    proposed_configure: Mutex<Option<Configure>>,
    applied_configure: Mutex<Option<Configure>>,
    wl_pointer_enter_serial: Mutex<Option<u32>>,
}
impl WindowInternal {
    fn new(app_state: Weak<AppState>) -> Self {
        WindowInternal{
            app_state,
            proposed_configure: Mutex::new(None),
            applied_configure: Mutex::new(None),
            wl_pointer_enter_serial: Mutex::new(None),
        }
    }
}

#[derive(Clone)]
struct Configure {
    width: i32,
    height: i32,
    states: Vec<u8>,
}


unsafe impl Send for Window {}
unsafe impl Sync for Window {}

struct App(Arc<AppState>);

struct ActiveCursor {
    cursor_surface: Arc<WlSurface>,
}
impl ActiveCursor {
    fn new(connection: &Connection, shm: WlShm, a: Weak<AppState>, compositor: &WlCompositor, queue_handle: &QueueHandle<App>) -> Self {
        let mut cursor_theme = CursorTheme::load(&connection, shm, 32).expect("Can't load cursors");
        cursor_theme.set_fallback(|name, size| {
            Some(include_bytes!("../../linux_assets/left_ptr").into())
        });
        let cursor = cursor_theme.get_cursor("wait").expect("Can't get cursor");
        let start_time = std::time::Instant::now();
        //I guess we fake an internal window here?
        let window_internal = WindowInternal::new(a);
        let cursor_surface = compositor.create_surface(queue_handle, Box::new(window_internal));
        let start_time = std::time::Instant::now();
        let frame_info = cursor.frame_and_duration(start_time.elapsed().as_millis() as u32);
        let buffer = &cursor[frame_info.frame_index];
        cursor_surface.attach(Some(buffer), 0, 0);
        cursor_surface.commit();
        let cursor_surface = Arc::new(cursor_surface);
        let move_cursor_surface = cursor_surface.clone();
        let move_cursor_theme = Arc::new(Mutex::new(cursor_theme));
        std::thread::spawn(move || {
            loop {
                let move_cursor_theme = move_cursor_theme.clone();
                let move_cursor_surface = move_cursor_surface.clone();
                let (sender,receiver) = std::sync::mpsc::channel();
                on_main_thread(move || {
                    let mut binding = move_cursor_theme.lock().unwrap();
                    let cursor = binding.get_cursor("wait").expect("Can't get cursor");
                    let present_time = start_time.elapsed();

                    let frame_info = cursor.frame_and_duration(present_time.as_millis() as u32);
                    println!("drawing frame {}", frame_info.frame_index);
                    let buffer = &cursor[frame_info.frame_index];
                    move_cursor_surface.attach(Some(buffer), 0, 0);
                    move_cursor_surface.damage_buffer(0, 0, buffer.dimensions().0 as i32, buffer.dimensions().1 as i32);
                    move_cursor_surface.commit();
                    let next_present_time = present_time + Duration::from_millis(frame_info.frame_duration as u64);
                    sender.send(next_present_time).expect("Can't send next present time");
                });
                let next_present_time = receiver.recv().expect("Can't receive next present time");
                println!("next_present_time: {:?} start_time {:?} start_time_elased {:?}", next_present_time,start_time, start_time.elapsed());
                let sleep_time = next_present_time.saturating_sub(start_time.elapsed());

                std::thread::sleep(sleep_time);


            }
        });
        ActiveCursor {
            cursor_surface,
        }
    }
}

struct AppState {
    compositor: WlCompositor,
    shm: WlShm,
    //option for lazy-init purposes
    active_cursor: Mutex<Option<ActiveCursor>>,

}
impl AppState {
    fn new(queue_handle: &QueueHandle<App>, compositor: WlCompositor, connection: &Connection, shm: WlShm) -> Arc<Self> {
        //cursor stuff?
        let mut a = Arc::new(AppState{
            compositor: compositor.clone(),
            shm: shm.clone(),
            active_cursor: Mutex::new(None),
        });
        let active_cursor = ActiveCursor::new(connection, shm, Arc::downgrade(&a), &compositor, queue_handle);
        a.active_cursor.lock().unwrap().replace(active_cursor);
        a
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
impl<A: AsRef<WindowInternal>> Dispatch<WlSurface, A> for App {
    fn event(_state: &mut Self, _proxy: &WlSurface, event: <WlSurface as Proxy>::Event, _data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlSurface event {:?}",event);
    }
}

impl<A: AsRef<WindowInternal>> Dispatch<XdgSurface, A> for App {
    fn event(_state: &mut Self, proxy: &XdgSurface, event: <XdgSurface as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                let proposed = data.as_ref().proposed_configure.lock().unwrap().take();
                if let Some(configure) = proposed {
                    *data.as_ref().applied_configure.lock().unwrap() = Some(configure);
                    proxy.ack_configure(serial);
                    //todo: adjust buffer size?
                }
            }
            _ => {
                println!("got XdgSurface event {:?}",event);
            }
        }
    }
}
impl<A: AsRef<WindowInternal>> Dispatch<XdgToplevel, A> for App {
    fn event(_state: &mut Self, _proxy: &XdgToplevel, event: <XdgToplevel as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got XdgToplevel event {:?}",event);
        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                *data.as_ref().proposed_configure.lock().unwrap() = Some(Configure{width, height, states});
            }
            _ => {
                //?
            }
        }

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

impl<A: AsRef<WindowInternal>> Dispatch<WlPointer, A> for App {
    fn event(_state: &mut Self, proxy: &WlPointer, event: <WlPointer as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlPointer event {:?}",event);
        match event {
            wayland_client::protocol::wl_pointer::Event::Enter {serial, surface, surface_x, surface_y} => {
                *data.as_ref().wl_pointer_enter_serial.lock().expect("Can't lock serial") = Some(serial);
                //set cursor?
                let app = data.as_ref().app_state.upgrade().expect("App state gone");
                proxy.set_cursor(serial, Some(&app.active_cursor.lock().unwrap().as_ref().unwrap().cursor_surface), 0, 0);
            }
            wayland_client::protocol::wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                time: _,


            } => {
                //get current size
                let size = data.as_ref().applied_configure.lock().unwrap().clone().expect("No configure event");
                const EDGE_REGION: i32 = 10;
                if size.width - (surface_x as i32) < EDGE_REGION {
                    todo!();
                }

            }
            _ => {
                //?
            }
        }
    }
}


impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        crate::application::on_main_thread(move || {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            let xdg_wm_base: XdgWmBase = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let window_internal = Arc::new(WindowInternal::new(Arc::downgrade(&info.app_state)));

            let surface = info.app_state.compositor.create_surface(&info.queue_handle, window_internal.clone());

            // Create a toplevel surface
            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &info.queue_handle, window_internal.clone());
            let xdg_toplevel = xdg_surface.get_toplevel(&info.queue_handle, window_internal.clone());

            let (file, mmap) = create_shm_buffer(&info.app_state.shm, size.width() as u32, size.height() as u32);
            let pool = info.app_state.shm.create_pool(file.as_fd(), mmap.len() as i32, &info.queue_handle, ());
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



            let seat: WlSeat = info.globals.bind(&info.queue_handle, 8..=9, ()).expect("Can't bind seat");
            let pointer = seat.get_pointer(&info.queue_handle, window_internal);
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