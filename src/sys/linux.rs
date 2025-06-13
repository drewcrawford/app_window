//SPDX-License-Identifier: MPL-2.0
use crate::coordinates::{Position, Size};
use crate::executor::on_main_thread_async;
use accesskit::NodeId;
use libc::{
    EFD_SEMAPHORE, MFD_ALLOW_SEALING, MFD_CLOEXEC, SYS_gettid, c_char, eventfd, getpid,
    memfd_create, pid_t, syscall,
};
use memmap2::MmapMut;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_int, c_void};
use std::fmt::Debug;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::ptr::NonNull;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;
use wayland_client::backend::WaylandError;
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_buffer::{Event, WlBuffer};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::{wl_compositor, wl_registry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_cursor::CursorTheme;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
use zune_png::zune_core::result::DecodingResult;

const CLOSE_ID: NodeId = NodeId(3);
const MAXIMIZE_ID: NodeId = NodeId(4);
const MINIMIZE_ID: NodeId = NodeId(5);

#[derive(Debug, Clone)]
struct OutputInfo {
    scale_factor: f64,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

mod ax {
    use crate::coordinates::Size;
    use crate::sys::linux::{BUTTON_WIDTH, CLOSE_ID, MAXIMIZE_ID, MINIMIZE_ID, TITLEBAR_HEIGHT};
    use accesskit::{Action, ActionRequest, NodeId, Rect, Role, TreeUpdate};
    use std::sync::{Arc, Mutex};
    pub fn build_tree_update(title: String, window_size: Size) -> TreeUpdate {
        let mut window = accesskit::Node::new(Role::Window);
        window.set_label(title);
        //accesskit rect is min and max, not origin and height!
        window.set_bounds(Rect::new(
            0.0,
            0.0,
            window_size.width(),
            window_size.height(),
        ));
        let mut title_bar = accesskit::Node::new(Role::TitleBar);
        title_bar.set_label("app_window");
        title_bar.set_bounds(Rect::new(
            0.0,
            0.0,
            window_size.width(),
            TITLEBAR_HEIGHT as f64,
        ));
        let mut close_button = accesskit::Node::new(Role::Button);
        close_button.add_action(Action::Click);
        close_button.add_action(Action::Focus);

        close_button.set_bounds(Rect::new(
            window_size.width() - BUTTON_WIDTH as f64,
            0.0,
            window_size.width(),
            TITLEBAR_HEIGHT as f64,
        ));
        close_button.set_label("Close");

        let mut maximize_button = accesskit::Node::new(Role::Button);
        maximize_button.add_action(Action::Click);
        maximize_button.add_action(Action::Focus);
        maximize_button.set_bounds(Rect::new(
            window_size.width() - BUTTON_WIDTH as f64 * 2.0,
            0.0,
            window_size.width() - BUTTON_WIDTH as f64 * 1.0,
            TITLEBAR_HEIGHT as f64,
        ));
        maximize_button.set_label("Maximize");

        let mut minimize_button = accesskit::Node::new(Role::Button);
        minimize_button.add_action(Action::Click);
        minimize_button.add_action(Action::Focus);
        minimize_button.set_bounds(Rect::new(
            window_size.width() - BUTTON_WIDTH as f64 * 3.0,
            0.0,
            window_size.width() - BUTTON_WIDTH as f64 * 2.0,
            TITLEBAR_HEIGHT as f64,
        ));
        minimize_button.set_label("Minimize");

        //window.set_children(vec![NodeId(2)]);
        //title_bar.set_children(vec![NodeId(3),NodeId(4), NodeId(5)]);
        window.set_children(vec![CLOSE_ID, MINIMIZE_ID, MAXIMIZE_ID]);

        let tree = accesskit::Tree {
            root: NodeId(1),
            toolkit_name: Some("app_window".to_string()),
            toolkit_version: Some("0.1.0".to_string()),
        };

        accesskit::TreeUpdate {
            nodes: vec![
                (NodeId(1), window),
                /*(NodeId(2), title_bar),*/ (CLOSE_ID, close_button),
                (MAXIMIZE_ID, maximize_button),
                (MINIMIZE_ID, minimize_button),
            ],
            tree: Some(tree),
            focus: NodeId(1),
        }
    }

    pub struct Inner {
        window_size: Size,
        title: String,
    }
    #[derive(Clone)]
    pub struct AX {
        inner: Arc<Inner>,
        window_internal: Arc<Mutex<super::WindowInternal>>,
    }
    impl AX {
        pub fn new(
            window_size: Size,
            title: String,
            window_internal: Arc<Mutex<super::WindowInternal>>,
        ) -> Self {
            AX {
                inner: Arc::new(Inner { window_size, title }),
                window_internal,
            }
        }
    }
    impl accesskit::ActivationHandler for AX {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            Some(build_tree_update(
                self.inner.title.clone(),
                self.inner.window_size,
            ))
        }
    }
    impl accesskit::ActionHandler for AX {
        fn do_action(&mut self, request: ActionRequest) {
            if request.target == CLOSE_ID {
                match request.action {
                    Action::Click => {
                        self.window_internal.lock().unwrap().close_window();
                    }
                    _ => unimplemented!(),
                }
            } else if request.target == MAXIMIZE_ID {
                match request.action {
                    Action::Click => {
                        self.window_internal.lock().unwrap().maximize();
                    }
                    _ => unimplemented!(),
                }
            } else if request.target == MINIMIZE_ID {
                match request.action {
                    Action::Click => {
                        self.window_internal.lock().unwrap().minimize();
                    }
                    _ => unimplemented!(),
                }
            } else {
                unimplemented!("Unknown action target: {target:?}", target = request.target);
            }
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
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

pub fn is_main_thread() -> bool {
    let current_pid = unsafe { getpid() };
    let main_thread_pid = unsafe { syscall(SYS_gettid) } as pid_t;
    current_pid == main_thread_pid
}
struct MainThreadSender {
    sender: Sender<Box<dyn FnOnce() + Send>>,
    eventfd: c_int,
}

impl MainThreadSender {
    fn send(&self, closure: Box<dyn FnOnce() + Send>) {
        self.sender.send(closure).expect("Can't send closure");
        let val = 1_u64;
        let w = unsafe {
            libc::write(
                self.eventfd,
                &val as *const _ as *const c_void,
                std::mem::size_of_val(&val),
            )
        };
        assert_eq!(
            w,
            std::mem::size_of_val(&val) as isize,
            "Failed to write to eventfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }
}

static MAIN_THREAD_SENDER: OnceLock<MainThreadSender> = OnceLock::new();

struct MainThreadInfo {
    globals: GlobalList,
    queue_handle: QueueHandle<App>,
    connection: Connection,
    app_state: Arc<AppState>,
    subcompositor: WlSubcompositor,
}

thread_local! {
    static MAIN_THREAD_INFO: RefCell<Option<MainThreadInfo>> = const { RefCell::new(None) };
}

pub fn run_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    let (sender, receiver) = channel();
    let channel_read_event = unsafe { eventfd(0, EFD_SEMAPHORE) };
    assert_ne!(channel_read_event, -1, "Failed to create eventfd");
    MAIN_THREAD_SENDER.get_or_init(|| MainThreadSender {
        sender,
        eventfd: channel_read_event,
    });

    let connection = Connection::connect_to_env().expect("Failed to connect to wayland server");
    let (globals, mut event_queue) =
        registry_queue_init::<App>(&connection).expect("Can't initialize registry");
    let qh = event_queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 6..=6, ()).unwrap();
    let subcompositor: WlSubcompositor = globals.bind(&qh, 1..=1, ()).unwrap();
    //fedora 41 KDE uses version 1?
    let shm: WlShm = globals.bind(&qh, 1..=2, ()).unwrap();

    // Bind all available wl_output interfaces
    for global in globals.contents().clone_list() {
        if global.interface == "wl_output" {
            let _output: WlOutput = globals
                .bind(&qh, global.version..=global.version, global.name)
                .unwrap();
        }
    }

    let mut app = App(AppState::new(&qh, compositor, &connection, shm));
    let main_thread_info = MainThreadInfo {
        globals,
        queue_handle: qh,
        connection,
        app_state: app.0.clone(),
        subcompositor,
    };

    MAIN_THREAD_INFO.replace(Some(main_thread_info));
    let mut io_uring = io_uring::IoUring::new(2).expect("Failed to create io_uring");

    _ = std::thread::Builder::new().name("app_window closure".to_string()).spawn(|| {
        closure()
    });

    event_queue.flush().expect("Failed to flush event queue");

    let mut read_guard = event_queue.prepare_read().expect("Failed to prepare read");
    const WAYLAND_DATA_AVAILABLE: u64 = 1;
    const CHANNEL_DATA_AVAILABLE: u64 = 2;
    let fd = read_guard.connection_fd();
    let io_uring_fd = io_uring::types::Fd(fd.as_raw_fd());
    let mut wayland_entry =
        io_uring::opcode::PollAdd::new(io_uring_fd, libc::POLLIN as u32).build();
    wayland_entry = wayland_entry.user_data(WAYLAND_DATA_AVAILABLE);
    let mut sqs = io_uring.submission();
    unsafe { sqs.push(&wayland_entry) }.expect("Can't submit peek");
    let mut eventfd_opcode = io_uring::opcode::PollAdd::new(
        io_uring::types::Fd(channel_read_event),
        libc::POLLIN as u32,
    )
    .build();
    eventfd_opcode = eventfd_opcode.user_data(CHANNEL_DATA_AVAILABLE);
    unsafe { sqs.push(&eventfd_opcode) }.expect("Can't submit peek");
    drop(sqs);
    //park
    loop {
        // println!("will submit_and_wait...");
        let r = io_uring.submit_and_wait(1);
        match r {
            Ok(_) => {}
            Err(e) => {
                logwise::error_sync!(
                    "Can't submit and wait: {err}",
                    err = logwise::privacy::LogIt(e)
                );
                continue;
            }
        }
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
                        Ok(_) => {}
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
                                    panic!("Protocol error reading from wayland");
                                }
                            }
                        }
                    }
                    event_queue
                        .dispatch_pending(&mut app)
                        .expect("Can't dispatch events");
                    //prepare next read
                    //ensure writes queued during dispatch_pending go out (such as proxy replies, etc)
                    event_queue.flush().expect("Failed to flush event queue");

                    loop {
                        let _read_guard = event_queue.prepare_read();

                        match _read_guard {
                            Some(guard) => {
                                read_guard = guard;
                                break; //out of loop
                            }
                            None => {
                                event_queue
                                    .dispatch_pending(&mut app)
                                    .expect("Can't dispatch events");
                                event_queue.flush().expect("Failed to flush event queue");
                                //try again
                                println!("retrying");
                            }
                        }
                    }

                    let mut sqs = io_uring.submission();
                    wayland_entry =
                        io_uring::opcode::PollAdd::new(io_uring_fd, libc::POLLIN as u32).build();
                    wayland_entry = wayland_entry.user_data(WAYLAND_DATA_AVAILABLE);
                    unsafe { sqs.push(&wayland_entry) }.expect("Can't submit peek");
                    //return to submit_and_wait
                }
                CHANNEL_DATA_AVAILABLE => {
                    let mut buf = [0u8; 8];
                    let r = unsafe {
                        libc::read(channel_read_event, buf.as_mut_ptr() as *mut c_void, 8)
                    };
                    assert_eq!(r, 8, "Failed to read from eventfd");
                    let closure = receiver
                        .recv_timeout(Duration::from_secs(0))
                        .expect("Failed to receive closure");
                    closure();
                    //let's ensure any writes went out to wayland
                    event_queue.flush().expect("Failed to flush event queue");

                    //submit new peek
                    let mut sqs = io_uring.submission();
                    unsafe { sqs.push(&eventfd_opcode) }.expect("Can't submit peek");
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
    MAIN_THREAD_SENDER
        .get()
        .expect("Main thread sender not set")
        .send(Box::new(closure));
}

#[derive(Debug)]
pub struct Window {
    internal: Arc<Mutex<WindowInternal>>,
}

struct DebugWrapper(Box<dyn Fn(Size) + Send>);
impl Debug for DebugWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DebugWrapper")
    }
}

#[derive(Debug)]
struct WindowInternal {
    app_state: Weak<AppState>,
    proposed_configure: Option<Configure>,
    applied_configure: Option<Configure>,
    wl_pointer_enter_serial: Option<u32>,
    wl_pointer_enter_surface: Option<WlSurface>,
    wl_pointer_pos: Option<Position>,
    xdg_toplevel: Option<XdgToplevel>,
    wl_surface: Option<WlSurface>,
    xdg_surface: Option<XdgSurface>,
    buffer: Option<WlBuffer>,
    requested_maximize: bool,
    adapter: Option<accesskit_unix::Adapter>,
    size_update_notify: Option<DebugWrapper>,
    decor_subsurface: Option<WlSubsurface>,
    title: String,
    current_outputs: std::collections::HashSet<u32>,
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
            buffer: None,
            adapter: None,
            size_update_notify: None,
            decor_subsurface: None,
            xdg_surface: None,
            current_outputs: HashSet::new(),
        }));
        if ax {
            let _aximpl = ax::AX::new(size, title.clone(), window_internal.clone());
            let adapter = Some(accesskit_unix::Adapter::new(
                _aximpl.clone(),
                _aximpl.clone(),
                _aximpl.clone(),
            ));
            let buffer = create_shm_buffer(
                size.width() as i32,
                size.height() as i32,
                &app_state.shm,
                queue_handle,
                window_internal.clone(),
            );
            window_internal.lock().unwrap().buffer = Some(buffer);
            window_internal.lock().unwrap().adapter = adapter;
        }
        window_internal
    }
    fn applied_size(&self) -> Size {
        let applied = self.applied_configure.clone().expect("No configure event");
        Size::new(applied.width as f64, applied.height as f64)
    }

    fn close_window(&self) {
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
    fn maximize(&mut self) {
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
    fn minimize(&self) {
        let toplevel = self.xdg_toplevel.as_ref().unwrap();
        toplevel.set_minimized();
    }
}

#[derive(Clone, Debug)]
struct Configure {
    width: i32,
    height: i32,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

struct App(Arc<AppState>);

enum SurfaceEvents {
    Standard(Arc<Mutex<WindowInternal>>),
    Cursor,
    Decor,
}

struct ActiveCursor {
    cursor_surface: Arc<WlSurface>,
    cursor_sender: Sender<CursorRequest>,
    active_request: Arc<Mutex<CursorRequest>>,
}

const CURSOR_SIZE: i32 = 16;
#[derive(Clone, PartialEq)]
struct CursorRequest {
    name: &'static str,
    hot_x: i32,
    hot_y: i32,
}
impl CursorRequest {
    fn wait() -> Self {
        CursorRequest {
            name: "wait",
            hot_x: 0,
            hot_y: 0,
        }
    }
    fn right_side() -> Self {
        CursorRequest {
            name: "right_side",
            hot_x: CURSOR_SIZE / 2,
            hot_y: 0,
        }
    }
    fn bottom_side() -> Self {
        CursorRequest {
            name: "bottom_side",
            hot_x: 0,
            hot_y: CURSOR_SIZE / 2,
        }
    }
    fn left_ptr() -> Self {
        CursorRequest {
            name: "left_ptr",
            hot_x: CURSOR_SIZE / 8,
            hot_y: CURSOR_SIZE / 8,
        }
    }
    fn bottom_right_corner() -> Self {
        CursorRequest {
            name: "bottom_right_corner",
            hot_x: CURSOR_SIZE / 2,
            hot_y: CURSOR_SIZE / 2,
        }
    }
}
impl ActiveCursor {
    fn new(
        connection: &Connection,
        shm: WlShm,
        _a: &Arc<AppState>,
        compositor: &WlCompositor,
        queue_handle: &QueueHandle<App>,
    ) -> Self {
        let mut cursor_theme =
            CursorTheme::load(connection, shm, CURSOR_SIZE as u32).expect("Can't load cursors");
        cursor_theme
            .set_fallback(|_, _| Some(include_bytes!("../../linux_assets/left_ptr").into()));
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
                        // println!("frame_info: {:?}", frame_info);
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
    fn cursor_request(&self, request: CursorRequest) {
        self.cursor_sender
            .send(request)
            .expect("Can't send cursor request");
    }
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
        let decor = include_bytes!("../../linux_assets/decor.png");
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
    opt: Mutex<Option<ReleaseOpt>>,
}
struct ReleaseOpt {
    _file: File,
    _mmap: Arc<MmapMut>,
    window_internal: Arc<Mutex<WindowInternal>>,
}

fn create_shm_buffer_decor(
    shm: &WlShm,
    queue_handle: &QueueHandle<App>,
    window_internal: Arc<Mutex<WindowInternal>>,
) -> WlBuffer {
    let decor = include_bytes!("../../linux_assets/decor.png");
    let mut decode_decor = zune_png::PngDecoder::new(decor);
    let decode = decode_decor.decode().expect("Can't decode decor");
    let dimensions = decode_decor.get_dimensions().unwrap();
    let decor = match decode {
        DecodingResult::U8(d) => d,
        _ => todo!(),
    };
    let file = unsafe {
        memfd_create(
            b"decor\0" as *const _ as *const c_char,
            MFD_ALLOW_SEALING | MFD_CLOEXEC,
        )
    };
    if file < 0 {
        panic!(
            "Failed to create memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }
    let file = unsafe { File::from_raw_fd(file) };

    let r = unsafe { libc::ftruncate(file.as_raw_fd(), (dimensions.0 * dimensions.1 * 4) as i64) };
    if r < 0 {
        panic!(
            "Failed to truncate memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }

    let mut mmap = unsafe { MmapMut::map_mut(&file) }.unwrap();
    for (pixel, decor_pixel) in mmap.chunks_exact_mut(4).zip(decor.chunks_exact(4)) {
        pixel.copy_from_slice(decor_pixel);
    }
    let pool = shm.create_pool(
        file.as_fd(),
        dimensions.0 as i32 * dimensions.1 as i32 * 4,
        queue_handle,
        (),
    );
    let release_info = BufferReleaseInfo {
        opt: Mutex::new(Some(ReleaseOpt {
            window_internal,
            _file: file,
            _mmap: Arc::new(mmap),
        })),
    };

    pool.create_buffer(
        0,
        dimensions.0 as i32,
        dimensions.1 as i32,
        dimensions.0 as i32 * 4,
        Format::Argb8888,
        queue_handle,
        release_info,
    )
}

fn create_shm_buffer(
    width: i32,
    height: i32,
    shm: &WlShm,
    queue_handle: &QueueHandle<App>,
    window_internal: Arc<Mutex<WindowInternal>>,
) -> WlBuffer {
    let file = unsafe {
        memfd_create(
            b"mem_fd\0" as *const _ as *const c_char,
            MFD_ALLOW_SEALING | MFD_CLOEXEC,
        )
    };
    if file < 0 {
        panic!(
            "Failed to create memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }
    let file = unsafe { File::from_raw_fd(file) };

    let r = unsafe { libc::ftruncate(file.as_raw_fd(), (width * height * 4) as i64) };
    if r < 0 {
        panic!(
            "Failed to truncate memfd: {err}",
            err = unsafe { *libc::__errno_location() }
        );
    }

    let mut mmap = unsafe { MmapMut::map_mut(&file) }.unwrap();
    const DEFAULT_COLOR: [u8; 4] = [0, 0, 0xFF, 0xFF];
    for pixel in mmap.chunks_exact_mut(4) {
        pixel.copy_from_slice(&DEFAULT_COLOR); //I guess due to endiannness we are actually BGRA?
    }

    let pool = shm.create_pool(file.as_fd(), width * height * 4, queue_handle, ());
    let mmap = Arc::new(mmap);
    let release_info = BufferReleaseInfo {
        opt: Mutex::new(Some(ReleaseOpt {
            _file: file,
            _mmap: mmap.clone(),
            window_internal,
        })),
    };

    pool.create_buffer(
        0,
        width,
        height,
        width * 4,
        Format::Argb8888,
        queue_handle,
        release_info,
    )
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
        println!("Got registry event {:?}", event);
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
        println!("Got registry event {:?}", event);
    }
}
impl Dispatch<XdgWmBase, ()> for App {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping { serial } => {
                proxy.pong(serial);
            }
            _ => {
                println!("Unknown XdgWmBase event: {:?}", event); // Add this line
            }
        }
    }
}

impl Dispatch<WlCompositor, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("Got compositor event {:?}", event);
    }
}

impl Dispatch<WlShm, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("Got shm event {:?}", event);
    }
}
impl Dispatch<WlSurface, SurfaceEvents> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        event: <WlSurface as Proxy>::Event,
        data: &SurfaceEvents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_surface::Event::Enter { output } => {
                if let SurfaceEvents::Standard(window_internal) = data {
                    let output_id = output.id().protocol_id();
                    window_internal
                        .lock()
                        .unwrap()
                        .current_outputs
                        .insert(output_id);
                }
            }
            wayland_client::protocol::wl_surface::Event::Leave { output } => {
                if let SurfaceEvents::Standard(window_internal) = data {
                    let output_id = output.id().protocol_id();
                    window_internal
                        .lock()
                        .unwrap()
                        .current_outputs
                        .remove(&output_id);
                }
            }
            _ => {
                println!("got WlSurface event {:?}", event);
            }
        }
    }
}

impl Dispatch<XdgSurface, Arc<Mutex<WindowInternal>>> for App {
    fn event(
        _state: &mut Self,
        proxy: &XdgSurface,
        event: <XdgSurface as Proxy>::Event,
        data: &Arc<Mutex<WindowInternal>>,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let mut locked_data = data.as_ref().lock().unwrap();
        match event {
            xdg_surface::Event::Configure { serial } => {
                let proposed = locked_data.proposed_configure.take();
                if let Some(mut configure) = proposed {
                    let app_state = locked_data.app_state.upgrade().unwrap();
                    if configure.width == 0 && configure.height == 0 {
                        //pick our own size
                        configure.width = 800;
                        configure.height = 600;
                    }
                    //check size
                    if locked_data
                        .applied_configure
                        .as_ref()
                        .map(|c| c.width != configure.width || c.height != configure.height)
                        .unwrap_or(true)
                    {
                        //apply decor position
                        locked_data
                            .decor_subsurface
                            .as_ref()
                            .unwrap()
                            .set_position(configure.width - app_state.decor_dimensions.0 as i32, 0);
                        locked_data.applied_configure = Some(configure);

                        //are we managing the buffer?
                        if locked_data.buffer.is_some() {
                            let size = locked_data.applied_size();
                            let app_state = locked_data.app_state.upgrade().unwrap();
                            let buffer = create_shm_buffer(
                                size.width() as i32,
                                size.height() as i32,
                                &app_state.shm,
                                qh,
                                data.clone(),
                            );
                            let surface = locked_data.wl_surface.as_ref().unwrap();
                            surface.attach(Some(&buffer), 0, 0);
                            surface.commit();
                            locked_data.buffer = Some(buffer);
                        }
                        let title = locked_data.title.clone();
                        let applied_size = locked_data.applied_size();
                        if let Some(a) = locked_data.adapter.as_mut() {
                            a.update_if_active(|| ax::build_tree_update(title, applied_size))
                        }
                        if let Some(f) = locked_data.size_update_notify.as_ref() {
                            f.0(locked_data.applied_size())
                        }
                    }
                }
                proxy.ack_configure(serial);
            }
            _ => {
                println!("got XdgSurface event {:?}", event);
            }
        }
    }
}
impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<XdgToplevel, A> for App {
    fn event(
        _state: &mut Self,
        _proxy: &XdgToplevel,
        event: <XdgToplevel as Proxy>::Event,
        data: &A,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got XdgToplevel event {:?}", event);
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states: _,
            } => {
                #[cfg(feature = "app_input")]
                app_input::linux::xdg_toplevel_configure_event(width, height);

                data.as_ref().lock().unwrap().proposed_configure =
                    Some(Configure { width, height });
            }
            _ => {
                //?
            }
        }
    }
}
impl Dispatch<WlShmPool, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        event: <WlShmPool as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlShmPool event {:?}", event);
    }
}
impl Dispatch<WlBuffer, BufferReleaseInfo> for App {
    fn event(
        _state: &mut Self,
        proxy: &WlBuffer,
        event: <WlBuffer as Proxy>::Event,
        data: &BufferReleaseInfo,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            Event::Release => {
                let release = data.opt.lock().unwrap().take().expect("No release info");
                //drop any existing buffer in there, we're done with it
                release.window_internal.lock().unwrap().buffer.take();
                proxy.destroy();
                drop(release);
            }
            _ => { /* not implemented yet */ }
        }
        println!("got WlBuffer event {:?}", event);
    }
}
impl Dispatch<WlSeat, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlSeat event {:?}", event);
    }
}

impl Dispatch<WlSubcompositor, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlSubcompositor,
        event: <WlSubcompositor as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlSubcompositor event {:?}", event);
    }
}
impl Dispatch<WlSubsurface, ()> for App {
    fn event(
        _state: &mut Self,
        _proxy: &WlSubsurface,
        event: <WlSubsurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlSubsurface event {:?}", event);
    }
}

impl Dispatch<WlOutput, u32> for App {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        output_id: &u32,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_output::Event::Scale { factor } => {
                let mut outputs = state.0.outputs.lock().unwrap();
                if let Some(output_info) = outputs.get_mut(output_id) {
                    output_info.scale_factor = factor as f64;
                } else {
                    outputs.insert(
                        *output_id,
                        OutputInfo {
                            scale_factor: factor as f64,
                        },
                    );
                }
            }
            wayland_client::protocol::wl_output::Event::Done => {
                // Output configuration is complete
            }
            _ => {
                // Handle other output events if needed (geometry, mode, etc.)
            }
        }
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

impl<A: AsRef<Mutex<WindowInternal>>> Dispatch<WlPointer, A> for App {
    fn event(
        _state: &mut Self,
        proxy: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        data: &A,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlPointer event {:?}", event);
        let mut data = data.as_ref().lock().unwrap();
        match event {
            wayland_client::protocol::wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x: _,
                surface_y: _,
            } => {
                data.wl_pointer_enter_serial = Some(serial);
                data.wl_pointer_enter_surface = Some(surface);
                //set cursor?
                let app = data.app_state.upgrade().expect("App state gone");
                let cursor_request = app
                    .active_cursor
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .active_request
                    .lock()
                    .unwrap()
                    .clone();

                proxy.set_cursor(
                    serial,
                    Some(
                        &app.active_cursor
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .cursor_surface,
                    ),
                    cursor_request.hot_x,
                    cursor_request.hot_y,
                );
            }
            wayland_client::protocol::wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                time: _time,
            } => {
                let parent_surface_x;
                let parent_surface_y;
                if data.wl_pointer_enter_surface != data.wl_surface {
                    //we're in the decor; slide by decor dimensions
                    let surface_dimensions = data
                        .applied_configure
                        .clone()
                        .expect("No surface dimensions");
                    parent_surface_x = surface_x + surface_dimensions.width as f64
                        - data.app_state.upgrade().unwrap().decor_dimensions.0 as f64;
                    parent_surface_y = surface_y;
                } else {
                    parent_surface_x = surface_x;
                    parent_surface_y = surface_y;
                }
                #[cfg(feature = "app_input")]
                app_input::linux::motion_event(_time, parent_surface_x, parent_surface_y);

                //get current size
                let size = data.applied_size();
                let position = Position::new(parent_surface_x, parent_surface_y);
                data.wl_pointer_pos.replace(position);
                let cursor_request = match MouseRegion::from_position(size, position) {
                    MouseRegion::BottomRight => CursorRequest::bottom_right_corner(),
                    MouseRegion::Bottom => CursorRequest::bottom_side(),
                    MouseRegion::Right => CursorRequest::right_side(),
                    MouseRegion::Client
                    | MouseRegion::MaximizeButton
                    | MouseRegion::CloseButton
                    | MouseRegion::MinimizeButton => CursorRequest::left_ptr(),
                    MouseRegion::Titlebar => CursorRequest::left_ptr(),
                };
                let app_state = data.app_state.upgrade().unwrap();
                let lock_a = app_state.active_cursor.lock().unwrap();
                let active_cursor = lock_a.as_ref().expect("No active cursor");
                let active_request = active_cursor.active_request.lock().unwrap();
                let changed = *active_request != cursor_request;
                if changed {
                    proxy.set_cursor(
                        data.wl_pointer_enter_serial.expect("No serial"),
                        Some(&active_cursor.cursor_surface),
                        cursor_request.hot_x,
                        cursor_request.hot_y,
                    );
                    active_cursor.cursor_request(cursor_request);
                }
            }
            wayland_client::protocol::wl_pointer::Event::Button {
                serial,
                time: _time,
                button,
                state,
            } => {
                #[cfg(feature = "app_input")]
                app_input::linux::button_event(
                    _time,
                    button,
                    state.into(),
                    data.wl_surface.as_ref().unwrap().id(),
                );

                //get current size
                let size = data.applied_size();
                let mouse_pos = data.wl_pointer_pos.expect("No pointer position");
                let mouse_region = MouseRegion::from_position(size, mouse_pos);
                let pressed: u32 = state.into();
                if button == 0x110 {
                    //BUTTON_LEFT
                    if pressed == 1 {
                        match mouse_region {
                            MouseRegion::BottomRight => {
                                let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                let app_state = data.app_state.upgrade().unwrap();
                                let seat = app_state.seat.lock().unwrap();
                                toplevel.resize(
                                    seat.as_ref().unwrap(),
                                    serial,
                                    xdg_toplevel::ResizeEdge::BottomRight,
                                );
                            }
                            MouseRegion::Bottom => {
                                let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                let app_state = data.app_state.upgrade().unwrap();
                                let seat = app_state.seat.lock().unwrap();
                                toplevel.resize(
                                    seat.as_ref().unwrap(),
                                    serial,
                                    xdg_toplevel::ResizeEdge::Bottom,
                                );
                            }
                            MouseRegion::Right => {
                                let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                let app_state = data.app_state.upgrade().unwrap();
                                let seat = app_state.seat.lock().unwrap();
                                toplevel.resize(
                                    seat.as_ref().unwrap(),
                                    serial,
                                    xdg_toplevel::ResizeEdge::Right,
                                );
                            }
                            MouseRegion::Client => {}
                            MouseRegion::Titlebar => {
                                let toplevel = data.xdg_toplevel.as_ref().unwrap();
                                let app_state = data.app_state.upgrade().unwrap();
                                let seat = app_state.seat.lock().unwrap();
                                toplevel._move(seat.as_ref().unwrap(), serial);
                            }
                            MouseRegion::CloseButton => {
                                data.close_window();
                            }
                            MouseRegion::MaximizeButton => data.maximize(),
                            MouseRegion::MinimizeButton => {
                                data.minimize();
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
    fn event(
        _state: &mut Self,
        _proxy: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        data: &A,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlKeyboard event {:?}", event);
        match event {
            wayland_client::protocol::wl_keyboard::Event::Enter {
                serial: _,
                surface: _,
                keys: _,
            } => {
                if let Some(e) = data.as_ref().lock().unwrap().adapter.as_mut() {
                    e.update_window_focus_state(true)
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Leave {
                serial: _,
                surface: _,
            } => {
                if let Some(e) = data.as_ref().lock().unwrap().adapter.as_mut() {
                    e.update_window_focus_state(false)
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Key {
                serial: _serial,
                time: _time,
                key: _key,
                state: _state,
            } => {
                #[cfg(feature = "app_input")]
                app_input::linux::wl_keyboard_event(
                    _serial,
                    _time,
                    _key,
                    _state.into(),
                    data.as_ref()
                        .lock()
                        .unwrap()
                        .wl_surface
                        .as_ref()
                        .unwrap()
                        .id(),
                );
            }
            _ => {}
        }
    }
}

impl Window {
    pub async fn new(_position: Position, size: Size, title: String) -> Self {
        let window_internal = crate::application::on_main_thread(move || {
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
            decor_surface.attach(Some(&decor_buffer), 0, 0);
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

            surface.attach(
                Some(
                    window_internal
                        .lock()
                        .unwrap()
                        .buffer
                        .as_ref()
                        .expect("No buffer"),
                ),
                0,
                0,
            );
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
        let display = on_main_thread_async(async {
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

#[derive(Debug)]
pub struct Surface {
    wl_display: WlDisplay,
    wl_surface: WlSurface,
    window_internal: Arc<Mutex<WindowInternal>>,
}

unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl Surface {
    pub async fn size_scale(&self) -> (Size, f64) {
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
            1.07
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
            Some(DebugWrapper(Box::new(update)));
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        todo!()
    }
}
