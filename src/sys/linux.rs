use std::any::Any;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::future::Future;
use std::ops::Sub;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;
use atspi::proxy::device_event_controller::KeySynthType;
use io_uring::cqueue::Entry;
use libc::{eventfd, getpid, memfd_create, pid_t, syscall, SYS_gettid, EFD_SEMAPHORE, MFD_ALLOW_SEALING, MFD_CLOEXEC};
use memmap2::MmapMut;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_client::backend::WaylandError;
use wayland_client::globals::{registry_queue_init, GlobalList, GlobalListContents};
use wayland_client::protocol::{wl_compositor, wl_registry, wl_shm};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_keyboard::{KeyState, WlKeyboard};
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
use zune_png::zune_core::result::DecodingResult;
use crate::coordinates::{Position, Size};
use crate::executor::already_on_main_thread_submit;

mod ax {
    use std::sync::{Arc, Mutex};
    use accesskit::{Action, ActionRequest, CustomAction, NodeId, Rect, Role, TreeUpdate};
    use libc::{access, close};
    use crate::coordinates::Size;
    use crate::sys::linux::{BUTTON_WIDTH, TITLEBAR_HEIGHT};

    pub struct Inner {
        window_size: Size,
        title: String,
    }
    #[derive(Clone)]
    pub struct AX {
        inner: Arc<Inner>,
    }
    impl AX {
        pub fn new(window_size: Size, title: String) -> Self {
            AX {
                inner: Arc::new(Inner{window_size, title})
            }
        }
    }
    impl accesskit::ActivationHandler for AX {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            let mut window = accesskit::Node::new(Role::Window);
            window.set_label(self.inner.title.clone());
            let window_size = self.inner.window_size;
            //accesskit rect is min and max, not origin and height!
            window.set_bounds(Rect::new(0.0, 0.0, window_size.width(), window_size.height()));
            let mut title_bar = accesskit::Node::new(Role::TitleBar);
            title_bar.set_label("app_window");
            title_bar.set_bounds(Rect::new(0.0, 0.0, window_size.width(), TITLEBAR_HEIGHT as f64));
            let mut close_button = accesskit::Node::new(Role::Button);
            close_button.add_action(Action::Click);
            close_button.add_action(Action::Focus);

            close_button.set_bounds(Rect::new(window_size.width() - BUTTON_WIDTH as f64, 0.0, window_size.width() as f64, TITLEBAR_HEIGHT as f64));
            close_button.set_label("Close");

            let mut maximize_button = accesskit::Node::new(Role::Button);
            maximize_button.add_action(Action::Click);
            maximize_button.add_action(Action::Focus);
            maximize_button.set_bounds(Rect::new(window_size.width() - BUTTON_WIDTH as f64 * 2.0, 0.0, window_size.width() as f64 - BUTTON_WIDTH as f64 * 1.0, TITLEBAR_HEIGHT as f64));
            maximize_button.set_label("Maximize");

            let mut minimize_button = accesskit::Node::new(Role::Button);
            minimize_button.add_action(Action::Click);
            minimize_button.add_action(Action::Focus);
            minimize_button.set_bounds(Rect::new(window_size.width() - BUTTON_WIDTH as f64 * 3.0, 0.0, window_size.width() as f64 - BUTTON_WIDTH as f64 * 2.0, TITLEBAR_HEIGHT as f64));
            minimize_button.set_label("Minimize");

            //window.set_children(vec![NodeId(2)]);
            //title_bar.set_children(vec![NodeId(3),NodeId(4), NodeId(5)]);
            window.set_children(vec![NodeId(3), NodeId(4), NodeId(5)]);

            let tree = accesskit::Tree {
                root: NodeId(1),
                app_name: Some("app_window".to_string()),
                toolkit_name: Some("app_window".to_string()),
                toolkit_version: Some("0.1.0".to_string()),
            };
            let mut update = accesskit::TreeUpdate {
                nodes: vec![(NodeId(1),window), /*(NodeId(2), title_bar),*/ (NodeId(3), close_button), (NodeId(4), maximize_button), (NodeId(5), minimize_button)],
                tree: Some(tree),
                focus: NodeId(3),
            };
            Some(update)
        }
    }
    impl accesskit::ActionHandler for AX {
        fn do_action(&mut self, request: ActionRequest) {
            todo!()
        }
    }
    impl accesskit::DeactivationHandler for AX {
        fn deactivate_accessibility(&mut self) {
            todo!()
        }
    }
    
}

const TITLEBAR_HEIGHT: u64 = 25;
const BUTTON_WIDTH: u64 = 25;

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

const ROOT_PATH: &str = "/org/a11y/atspi/accessible/root";

pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    let (sender, receiver) = channel();
    let channel_read_event = unsafe{eventfd(0, EFD_SEMAPHORE)};
    assert_ne!(channel_read_event, -1, "Failed to create eventfd");
    MAIN_THREAD_SENDER.get_or_init(|| {
        MainThreadSender{sender, eventfd: channel_read_event}
    });

    let connection = Connection::connect_to_env().expect("Failed to connect to wayland server");
    let display = connection.display();
    let (globals, mut event_queue) = registry_queue_init::<App>(&connection).expect("Can't initialize registry");
    let qh = event_queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 6..=6, ()).unwrap();
    //fedora 41 KDE uses version 1?
    let shm: WlShm = globals.bind(&qh, 1..=2, ()).unwrap();

    let mut app = App(AppState::new(&qh, compositor, &connection, shm));
    let main_thread_info = MainThreadInfo{globals, queue_handle: qh, connection, app_state: app.0.clone()};

    MAIN_THREAD_INFO.replace(Some(main_thread_info));
    let mut io_uring = io_uring::IoUring::new(2).expect("Failed to create io_uring");

    //ax example
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
        // println!("will submit_and_wait...");
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
                    match read_guard.read() {
                        Ok(_) => {

                        }
                        Err(e) => {
                            match e {
                                WaylandError::Io(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::WouldBlock => {
                                            //continue
                                        }
                                        _ => {
                                            panic!("Error reading from wayland: {err}", err = e);
                                        }
                                    }
                                }
                                WaylandError::Protocol(_) => {
                                    panic!("Protocol error reading from wayland");}
                                }
                            }
                        }
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
    // println!("sending on_main_thread...");
    MAIN_THREAD_SENDER.get().expect("Main thread sender not set").send(Box::new(closure));
}

pub struct Window {
}

struct WindowInternal {
    app_state: Weak<AppState>,
    proposed_configure: Option<Configure>,
    applied_configure: Option<Configure>,
    wl_pointer_enter_serial: Option<u32>,
    wl_pointer_pos: Option<Position>,
    xdg_toplevel: Option<XdgToplevel>,
    wl_surface: Option<WlSurface>,
    buffer: WlBuffer,
    file: File,
    mmap: MmapMut,
    requested_maximize: bool,
    adapter: Option<accesskit_unix::Adapter>,
    ax: Option<ax::AX>,
    atspi_connection: Option<Arc<atspi::AccessibilityConnection>>,
}
impl WindowInternal {

    fn new(app_state: &Arc<AppState>, size: Size, title: String, queue_handle: &QueueHandle<App>, ax: bool) -> Self {

        let (file, mmap, buffer) = create_shm_buffer(size.width() as i32, size.height() as i32, &app_state.shm, queue_handle, &app_state.decor, app_state.decor_dimensions);
        let ax_impl;
        let adapter;
        if ax {
            let _aximpl = ax::AX::new(size, title.clone());
            adapter = Some(accesskit_unix::Adapter::new(_aximpl.clone(), _aximpl.clone(), _aximpl.clone()));
            ax_impl = Some(_aximpl);
        }
        else {
            adapter = None;
            ax_impl = None;
        }
        let ax = ax::AX::new(size, title);

        WindowInternal{
            app_state: Arc::downgrade(app_state),
            proposed_configure: None,
            applied_configure: None,
            wl_pointer_enter_serial: None,
            wl_pointer_pos: None,
            xdg_toplevel: None,
            wl_surface: None,
            requested_maximize: false,
            file,
            mmap,
            buffer,
            adapter,
            ax: ax_impl,
            atspi_connection: None,
        }
    }
    fn applied_size(&self) -> Size {
        let applied = self.applied_configure.clone().expect("No configure event");
        Size::new(applied.width as f64, applied.height as f64)
    }
    fn resize_buffer(&mut self, queue_handle: &QueueHandle<App>) {
        let size = self.applied_size();
        let app_state = self.app_state.upgrade().unwrap();
        let (file, mmap, buffer) = create_shm_buffer(size.width() as i32, size.height() as i32, &app_state.shm, &queue_handle, &app_state.decor, app_state.decor_dimensions);
        self.file = file;
        self.mmap = mmap;
        self.buffer = buffer;
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
    cursor_sender: Sender<CursorRequest>,
    active_request: Arc<Mutex<CursorRequest>>,
}

const CURSOR_SIZE: i32 = 16;
#[derive(Clone,PartialEq)]
struct CursorRequest {
    name: &'static str,
    hot_x: i32,
    hot_y: i32,
}
impl CursorRequest {
    fn wait() -> Self {
        CursorRequest { name: "wait", hot_x: 0, hot_y: 0}
    }
    fn right_side() -> Self {
        CursorRequest { name: "right_side", hot_x: CURSOR_SIZE/2, hot_y: 0}
    }
    fn bottom_side() -> Self {
        CursorRequest { name: "bottom_side", hot_x: 0, hot_y: CURSOR_SIZE/2}
    }
    fn left_ptr() -> Self {
        CursorRequest { name: "left_ptr", hot_x: CURSOR_SIZE/8, hot_y: CURSOR_SIZE/8}
    }
    fn bottom_right_corner() -> Self {
        CursorRequest { name: "bottom_right_corner", hot_x: CURSOR_SIZE/2, hot_y: CURSOR_SIZE/2}
    }
}
impl ActiveCursor {
    fn new(connection: &Connection, shm: WlShm, a: &Arc<AppState>, compositor: &WlCompositor, queue_handle: &QueueHandle<App>) -> Self {
        let mut cursor_theme = CursorTheme::load(&connection, shm, CURSOR_SIZE as u32).expect("Can't load cursors");
        cursor_theme.set_fallback(|name, size| {
            Some(include_bytes!("../../linux_assets/left_ptr").into())
        });
        let cursor = cursor_theme.get_cursor("wait").expect("Can't get cursor");
        let start_time = std::time::Instant::now();
        //I guess we fake an internal window here?
        let window_internal = WindowInternal::new(a, Size::new(CURSOR_SIZE as f64, CURSOR_SIZE as f64), "cursor".to_string(), queue_handle, false);
        let cursor_surface = compositor.create_surface(queue_handle, Box::new(Mutex::new(window_internal)));
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

        std::thread::spawn(move || {
            loop {
                let move_cursor_theme = move_cursor_theme.clone();
                let move_cursor_surface = move_cursor_surface.clone();
                let mt_active_request = move_active_request.clone();
                let (sender,receiver) = std::sync::mpsc::channel();

                on_main_thread(move || {
                    let mut binding = move_cursor_theme.lock().unwrap();
                    let cursor = binding.get_cursor(&mt_active_request.lock().unwrap().name).expect("Can't get cursor");
                    let present_time = start_time.elapsed();
                    let frame_info = cursor.frame_and_duration(present_time.as_millis() as u32);
                    // println!("frame_info: {:?}", frame_info);
                    let buffer = &cursor[frame_info.frame_index];
                    move_cursor_surface.attach(Some(buffer), 0, 0);
                    move_cursor_surface.damage_buffer(0, 0, buffer.dimensions().0 as i32, buffer.dimensions().1 as i32);
                    move_cursor_surface.commit();
                    let next_present_time = present_time + Duration::from_millis(frame_info.frame_duration as u64);
                    sender.send(next_present_time).expect("Can't send next present time");
                });
                let next_present_time = receiver.recv().expect("Can't receive next present time");
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
        });
        ActiveCursor {
            cursor_surface,
            cursor_sender: cursor_request_sender,
            active_request,
        }
    }
    fn cursor_request(&self, request: CursorRequest) {
        self.cursor_sender.send(request).expect("Can't send cursor request");
    }
}

struct AppState {
    compositor: WlCompositor,
    shm: WlShm,
    //option for lazy-init purposes
    active_cursor: Mutex<Option<ActiveCursor>>,
    seat: Mutex<Option<WlSeat>>,
    decor: Vec<u8>,
    decor_dimensions: (usize, usize),


}
impl AppState {
    fn new(queue_handle: &QueueHandle<App>, compositor: WlCompositor, connection: &Connection, shm: WlShm) -> Arc<Self> {
        let decor = include_bytes!("../../linux_assets/decor.png");
        let mut decode_decor = zune_png::PngDecoder::new(decor);
        let decode = decode_decor.decode().expect("Can't decode decor");
        let dimensions = decode_decor.get_dimensions().unwrap();
        let decor;
        match decode {
            DecodingResult::U8(d) => {
                decor = d;
            }
            _ => todo!()
        }

        let mut a = Arc::new(AppState{
            compositor: compositor.clone(),
            shm: shm.clone(),
            active_cursor: Mutex::new(None),
            seat: Mutex::new(None),
            decor,
            decor_dimensions: dimensions
        });
        let active_cursor = ActiveCursor::new(connection, shm, &a, &compositor, queue_handle);
        a.active_cursor.lock().unwrap().replace(active_cursor);
        a
    }

}



fn create_shm_buffer(
    width: i32,
    height: i32,
    shm: &WlShm,
    queue_handle: &QueueHandle<App>,
    decor: &[u8],
    decor_dimensions: (usize, usize),

) -> (File, MmapMut, WlBuffer) {
    let file = unsafe{memfd_create(b"mem_fd\0" as *const c_char, MFD_ALLOW_SEALING | MFD_CLOEXEC)};
    if file < 0 {
        panic!("Failed to create memfd: {err}", err = unsafe{*libc::__errno_location()});
    }
    let file = unsafe{File::from_raw_fd(file)};

    let r = unsafe{libc::ftruncate(file.as_raw_fd(), (width * height * 4) as i64)};
    if r < 0 {
        panic!("Failed to truncate memfd: {err}", err = unsafe{*libc::__errno_location()});
    }

    let mut mmap = unsafe{MmapMut::map_mut(&file)}.unwrap();

    let mut x = 0;
    let mut y = 0;
    for pixel in mmap.chunks_exact_mut(4) {
        if y < decor_dimensions.1 && x > width - decor_dimensions.0 as i32 {
            let decor_x = x - (width - decor_dimensions.0 as i32);
            let decor_y = y;

            let decor_pixel = decor[(decor_y * decor_dimensions.0 + decor_x as usize) * 4..(decor_y * decor_dimensions.0 + decor_x as usize + 1) * 4].to_vec();
            pixel.copy_from_slice(&decor_pixel);
        }
        else {
            pixel.copy_from_slice(&[0, 0, 0xFF, 0xFF]); //I guess due to endiannness we are actually BGRA?
        }
        x+= 1;
        if x == width {
            x = 0;
            y += 1;
        }
    }
    let pool = shm.create_pool(file.as_fd(), width * height * 4, queue_handle, ());
    let buffer = pool.create_buffer(
        0,
        width,
        height,
        width * 4,
        Format::Argb8888,
        queue_handle,
        (),
    );


    (file, mmap,buffer)
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
impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<WlSurface, A> for App {
    fn event(_state: &mut Self, _proxy: &WlSurface, event: <WlSurface as Proxy>::Event, _data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlSurface event {:?}",event);
    }
}

impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<XdgSurface, A> for App {
    fn event(_state: &mut Self, proxy: &XdgSurface, event: <XdgSurface as Proxy>::Event, data: &A, _conn: &Connection, qh: &QueueHandle<Self>) {
        let mut data = data.as_ref().lock().unwrap();
        match event {
            xdg_surface::Event::Configure { serial } => {
                let proposed = data.proposed_configure.take();
                if let Some(mut configure) = proposed {
                    proxy.ack_configure(serial);
                    let app_state = data.app_state.upgrade().unwrap();
                    if configure.width == 0 && configure.height == 0 {
                        //pick our own size
                        configure.width = 800;
                        configure.height = 600;
                    }
                    //check size
                    if data.applied_configure.as_ref().map(|c| c.width != configure.width || c.height != configure.height).unwrap_or(true) {
                        data.applied_configure = Some(configure);

                        data.resize_buffer(qh);
                        let surface = data.wl_surface.as_ref().unwrap();
                        surface.attach(Some(&data.buffer), 0, 0);
                        surface.commit();
                    }



                    //todo: adjust buffer size?
                }
            }
            _ => {
                println!("got XdgSurface event {:?}",event);
            }
        }
    }
}
impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<XdgToplevel, A> for App {
    fn event(_state: &mut Self, _proxy: &XdgToplevel, event: <XdgToplevel as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got XdgToplevel event {:?}",event);
        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                data.as_ref().lock().unwrap().proposed_configure = Some(Configure{width, height, states});
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

enum MouseRegion {
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
    fn from_position(size: Size, position: Position) -> Self {
        const EDGE_REGION: f64 = 10.0;
        if position.y() < TITLEBAR_HEIGHT as f64 && position.x() > size.width() - BUTTON_WIDTH as f64 {
            MouseRegion::CloseButton
        }
        else if position.y() < TITLEBAR_HEIGHT as f64 && position.x() > size.width() - BUTTON_WIDTH as f64 * 2.0 {
            MouseRegion::MaximizeButton
        }
        else if position.y() < TITLEBAR_HEIGHT as f64 && position.x() > size.width() - BUTTON_WIDTH as f64 * 3.0 {
            MouseRegion::MinimizeButton
        }
        else if position.y() < TITLEBAR_HEIGHT as f64 {
            MouseRegion::Titlebar
        }
        else if size.width() - position.x() < EDGE_REGION {
            if size.height() - position.y() < EDGE_REGION {
                MouseRegion::BottomRight
            }
            else {
                MouseRegion::Right
            }
        }
        else if size.height() - position.y() < EDGE_REGION {
            MouseRegion::Bottom
        }
        else {
            MouseRegion::Client
        }
    }
}

impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<WlPointer, A> for App {
    fn event(_state: &mut Self, proxy: &WlPointer, event: <WlPointer as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlPointer event {:?}",event);
        let mut data = data.as_ref().lock().unwrap();
        match event {
            wayland_client::protocol::wl_pointer::Event::Enter {serial, surface, surface_x, surface_y} => {
                data.wl_pointer_enter_serial = Some(serial);
                //set cursor?
                let app = data.app_state.upgrade().expect("App state gone");
                let cursor_request = app.active_cursor.lock().unwrap().as_ref().unwrap().active_request.lock().unwrap().clone();

                proxy.set_cursor(serial, Some(&app.active_cursor.lock().unwrap().as_ref().unwrap().cursor_surface), cursor_request.hot_x, cursor_request.hot_y);
            }
            wayland_client::protocol::wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                time: _,
            } => {
                //get current size
                let size = data.applied_size();
                let position = Position::new(surface_x as f64, surface_y as f64);
                data.wl_pointer_pos.replace(position);
                let cursor_request;
                match MouseRegion::from_position(size, position) {
                    MouseRegion::BottomRight => {
                        let app = data.app_state.upgrade().expect("App state gone");
                        cursor_request = CursorRequest::bottom_right_corner();
                    }
                    MouseRegion::Bottom => {
                        let app = data.app_state.upgrade().expect("App state gone");
                        cursor_request = CursorRequest::bottom_side();
                    }
                    MouseRegion::Right => {
                        let app = data.app_state.upgrade().expect("App state gone");
                        cursor_request = CursorRequest::right_side();
                    }
                    MouseRegion::Client | MouseRegion::MaximizeButton | MouseRegion::CloseButton | MouseRegion::MinimizeButton => {
                        let app = data.app_state.upgrade().expect("App state gone");
                        cursor_request = CursorRequest::left_ptr();
                    }
                    MouseRegion::Titlebar => {
                        let app = data.app_state.upgrade().expect("App state gone");
                        cursor_request = CursorRequest::left_ptr();
                    }

                }
                let app_state = data.app_state.upgrade().unwrap();
                let lock_a = app_state.active_cursor.lock().unwrap();
                let active_cursor = lock_a.as_ref().expect("No active cursor");
                let active_request = active_cursor.active_request.lock().unwrap();
                let changed = *active_request != cursor_request;
                if changed {
                    proxy.set_cursor(data.wl_pointer_enter_serial.expect("No serial"), Some(&active_cursor.cursor_surface), cursor_request.hot_x, cursor_request.hot_y);
                    active_cursor.cursor_request(cursor_request);
                }
            },
            wayland_client::protocol::wl_pointer::Event::Button {
                serial, time, button, state
            } => {
                //get current size
                let size = data.applied_size();
                let mouse_pos = data.wl_pointer_pos.clone().expect("No pointer position");
                let mouse_region = MouseRegion::from_position(size, mouse_pos);
                let pressed: u32 = state.into();
                if button == 0x110  {//BUTTON_LEFT
                   if pressed == 1 {
                       match mouse_region {
                           MouseRegion::BottomRight => {
                               let toplevel = data.xdg_toplevel.as_ref().unwrap();
                               let app_state = data.app_state.upgrade().unwrap();
                               let seat = app_state.seat.lock().unwrap();
                               toplevel.resize(seat.as_ref().unwrap(), serial, xdg_toplevel::ResizeEdge::BottomRight);
                           }
                           MouseRegion::Bottom => {
                               let toplevel = data.xdg_toplevel.as_ref().unwrap();
                               let app_state = data.app_state.upgrade().unwrap();
                               let seat = app_state.seat.lock().unwrap();
                               toplevel.resize(seat.as_ref().unwrap(), serial, xdg_toplevel::ResizeEdge::Bottom);
                           }
                            MouseRegion::Right => {
                                let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                let app_state = data.app_state.upgrade().unwrap();
                                let seat = app_state.seat.lock().unwrap();
                                toplevel.resize(seat.as_ref().unwrap(), serial, xdg_toplevel::ResizeEdge::Right);
                            }
                           MouseRegion::Client => {

                           }
                           MouseRegion::Titlebar => {
                               let toplevel = data.xdg_toplevel.as_ref().unwrap();
                               let app_state = data.app_state.upgrade().unwrap();
                               let seat = app_state.seat.lock().unwrap();
                               toplevel._move(seat.as_ref().unwrap(), serial);
                           }
                           MouseRegion::CloseButton => {
                               let toplevel = data.xdg_toplevel.as_ref().unwrap();
                               let app_state = data.app_state.upgrade().unwrap();
                               toplevel.destroy();
                           }
                           MouseRegion::MaximizeButton => {
                               if data.requested_maximize {
                                   data.requested_maximize = false;
                                   let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                   toplevel.unset_maximized();
                                 }
                                 else {
                                      data.requested_maximize = true;
                                     let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                     toplevel.set_maximized();
                               }
                           }
                            MouseRegion::MinimizeButton => {
                                 let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                 toplevel.set_minimized();
                            }

                       }
                   }
                }



            }
            _ => {
                //?
            }
        }
    }
}

impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<WlKeyboard, A> for App {
    fn event(_state: &mut Self, _proxy: &WlKeyboard, event: <WlKeyboard as Proxy>::Event, data: &A, _conn: &Connection, _qhandle: &QueueHandle<Self>) {
        println!("got WlKeyboard event {:?}",event);

        match event {
            wayland_client::protocol::wl_keyboard::Event::Enter {
                serial, surface, keys
            } => {
                data.as_ref().lock().unwrap().adapter.as_mut().map(|e| e.update_window_focus_state(true));
            }
            wayland_client::protocol::wl_keyboard::Event::Leave {
                serial, surface
            } => {
                data.as_ref().lock().unwrap().adapter.as_mut().map(|e| e.update_window_focus_state(false));
            }
            wayland_client::protocol::wl_keyboard::Event::Key {
                serial, time, key,state
            } => {
                let window_internal = data.as_ref().lock().unwrap();
                if let Some(connection) = window_internal.atspi_connection.clone() {
                    let mt_executor = crate::executor::already_on_main_thread_submit(async move {
                        let proxy = atspi::proxy::device_event_controller::DeviceEventControllerProxy::new(&connection.connection()).await.expect("Can't create proxy");

                        let synth_type = match state {
                            WEnum::Value(e) => {
                                match e {
                                    KeyState::Pressed => KeySynthType::Press,
                                    KeyState::Released => KeySynthType::Release,
                                    _ => KeySynthType::Press
                                }
                            }
                            WEnum::Unknown(_) => {KeySynthType::Press}
                        };
                        proxy.generate_keyboard_event(key as i32, "H", synth_type).await.expect("Can't generate keyboard event");
                        connection.connection().executor().tick().await;
                        println!("generated keyboard event {:?}", key);
                        std::mem::forget(connection);
                    });
                }

            }
            _ => {
            }
        }
    }
}



impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        crate::application::on_main_thread(move || {
            let info = MAIN_THREAD_INFO.take().expect("Main thread info not set");
            let xdg_wm_base: XdgWmBase = info.globals.bind(&info.queue_handle, 6..=6, ()).unwrap();
            let window_internal = Arc::new(Mutex::new(WindowInternal::new(&info.app_state, size,title, &info.queue_handle, true)));

            let surface = info.app_state.compositor.create_surface(&info.queue_handle, window_internal.clone());
            window_internal.lock().unwrap().wl_surface.replace(surface.clone());

            //initialize atspi
            let move_window_internal = window_internal.clone();
            already_on_main_thread_submit(async move {
               let atspi_connection = atspi::connection::AccessibilityConnection::new().await.expect("Can't connect");
                atspi_connection.connection().executor().tick().await;
                move_window_internal.lock().unwrap().atspi_connection = Some(Arc::new(atspi_connection));
                println!("atspi initialized!");
            });
            // Create a toplevel surface
            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &info.queue_handle, window_internal.clone());
            let xdg_toplevel = xdg_surface.get_toplevel(&info.queue_handle, window_internal.clone());
            window_internal.lock().unwrap().xdg_toplevel.replace(xdg_toplevel);


            surface.attach(Some(&window_internal.lock().unwrap().buffer), 0, 0);
            surface.commit();



            let seat: WlSeat = info.globals.bind(&info.queue_handle, 8..=9, ()).expect("Can't bind seat");
            window_internal.lock().unwrap().app_state.upgrade().unwrap().seat.lock().unwrap().replace(seat.clone());
            let pointer = seat.get_pointer(&info.queue_handle, window_internal.clone());
            let _keyboard = seat.get_keyboard(&info.queue_handle, window_internal);

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