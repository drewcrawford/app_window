//SPDX-License-Identifier: MPL-2.0
use libc::{EFD_SEMAPHORE, SYS_gettid, c_int, c_void, eventfd, getpid, pid_t, syscall};
use std::cell::RefCell;
use std::os::fd::AsRawFd;
use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;
use wayland_client::backend::WaylandError;
use wayland_client::globals::{GlobalList, registry_queue_init};
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::{wl_compositor, wl_output::WlOutput, wl_shm::WlShm};
use wayland_client::{Connection, QueueHandle};

use super::{App, AppState};

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

pub(super) struct MainThreadInfo {
    pub globals: GlobalList,
    pub queue_handle: QueueHandle<App>,
    pub connection: Connection,
    pub app_state: std::sync::Arc<AppState>,
    pub subcompositor: WlSubcompositor,
}

thread_local! {
    pub static MAIN_THREAD_INFO: RefCell<Option<MainThreadInfo>> = const { RefCell::new(None) };
}

pub fn on_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    MAIN_THREAD_SENDER
        .get()
        .expect("Main thread sender not set")
        .send(Box::new(closure));
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

    _ = std::thread::Builder::new()
        .name("app_window closure".to_string())
        .spawn(closure);

    event_queue.flush().expect("Failed to flush event queue");

    let mut read_guard = Some(event_queue.prepare_read().expect("Failed to prepare read"));
    const WAYLAND_DATA_AVAILABLE: u64 = 1;
    const CHANNEL_DATA_AVAILABLE: u64 = 2;
    let fd = read_guard.as_ref().unwrap().connection_fd();
    let io_uring_fd = io_uring::types::Fd(fd.as_raw_fd());
    let io_uring_fd_raw = io_uring_fd.0.as_raw_fd();
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
    //flush_queue_debug
    std::thread::Builder::new()
        .name("flush_queue_debug".to_string())
        .spawn(move || {
            for _ in 0..1_000_000 {
                std::thread::sleep(std::time::Duration::from_millis(1));
                on_main_thread(|| {}) //wake
            }
        })
        .unwrap();

    fn next_read_guard(
        event_queue: &mut wayland_client::EventQueue<App>,
        app: &mut App,
        read_guard: &mut Option<wayland_client::backend::ReadEventsGuard>,
    ) {
        loop {
            let _read_guard = event_queue.prepare_read();

            match _read_guard {
                Some(guard) => {
                    *read_guard = Some(guard);
                    break; //out of loop
                }
                None => {
                    event_queue
                        .dispatch_pending(app)
                        .expect("Can't dispatch events");
                    event_queue.flush().expect("Failed to flush event queue");
                    //try again
                    logwise::debuginternal_sync!("Retrying");
                }
            }
        }
    }

    //park
    loop {
        next_read_guard(&mut event_queue, &mut app, &mut read_guard);
        assert!(read_guard.as_ref().unwrap().connection_fd().as_raw_fd() == io_uring_fd_raw);
        let r = io_uring.submit_and_wait(1);
        //we also want to take once regardless of entry
        let mut take_read_guard = read_guard.take();
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
        let mut wayland_data_available = false;
        let mut channel_data_available = false;
        for entry in io_uring.completion() {
            let result = entry.result();
            if result < 0 {
                panic!("Error in completion queue: {err}", err = result);
            }
            match entry.user_data() {
                WAYLAND_DATA_AVAILABLE => {
                    wayland_data_available = true;
                }
                CHANNEL_DATA_AVAILABLE => {
                    channel_data_available = true;
                }
                other => {
                    unimplemented!("Unknown user data: {other}", other = other);
                }
            }
        }
        if wayland_data_available {
            match take_read_guard
                .take()
                .expect("Read guard not available")
                .read()
            {
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

            let mut sqs = io_uring.submission();
            wayland_entry =
                io_uring::opcode::PollAdd::new(io_uring_fd, libc::POLLIN as u32).build();
            wayland_entry = wayland_entry.user_data(WAYLAND_DATA_AVAILABLE);
            unsafe { sqs.push(&wayland_entry) }.expect("Can't submit peek");
            //return to submit_and_wait
        }
        if channel_data_available {
            drop(take_read_guard); //we don't need it anymore
            let mut buf = [0u8; 8];
            let r = unsafe { libc::read(channel_read_event, buf.as_mut_ptr() as *mut c_void, 8) };
            assert_eq!(r, 8, "Failed to read from eventfd");
            let closure = receiver
                .recv_timeout(Duration::from_secs(0))
                .expect("Failed to receive closure");
            closure();
            //let's ensure any writes went out to wayland
            event_queue
                .dispatch_pending(&mut app)
                .expect("can't dispatch events");
            event_queue.flush().expect("Failed to flush event queue");
            //submit new peek
            let mut sqs = io_uring.submission();
            unsafe { sqs.push(&eventfd_opcode) }.expect("Can't submit peek");
            //return to submit_and_wait
        }
    }
}
