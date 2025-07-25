// SPDX-License-Identifier: MPL-2.0
use crate::input::keyboard::Shared;
use crate::input::keyboard::key::KeyboardKey;
use crate::input::mouse::linux::motion_event;
use crate::input::mouse::sys::{axis_event, button_event, xdg_toplevel_configure_event};
use memmap2::MmapMut;
use std::ffi::c_void;
use std::fs::File;
use std::os::fd::AsFd;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use wayland_client::backend::ObjectId;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::{wl_compositor, wl_registry, wl_shm};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::{Event, XdgWmBase};

pub(crate) mod ax;

#[derive(Default)]
struct KeyboardState {
    shareds: Vec<Weak<Shared>>,
}
impl KeyboardState {
    fn apply_all<F: Fn(&Shared)>(&mut self, f: F) {
        self.shareds.retain(|shared| {
            if let Some(shared) = shared.upgrade() {
                f(&shared);
                true
            } else {
                false
            }
        })
    }
}
static KEYBOARD_STATE: OnceLock<Mutex<KeyboardState>> = OnceLock::new();

#[derive(Debug)]
pub(super) struct PlatformCoalescedKeyboard {}

impl PlatformCoalescedKeyboard {
    pub async fn new(shared: &Arc<Shared>) -> Self {
        KEYBOARD_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .shareds
            .push(Arc::downgrade(shared));
        PlatformCoalescedKeyboard {}
    }
}

fn create_shm_buffer(_shm: &wl_shm::WlShm, width: u32, height: u32) -> (File, MmapMut) {
    let stride = width * 4;
    let size = stride * height;
    let file = tempfile::tempfile().unwrap();
    file.set_len(size as u64).unwrap();

    let mut mmap = unsafe { MmapMut::map_mut(&file) }.unwrap();

    for pixel in mmap.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 0xFF, 0xFF]); //I guess due to endiannness we are actually BGRA?
    }

    (file, mmap)
}

struct AppData {}
impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<AppData>,
    ) {
        println!("Got registry event {:?}", event);
    }
}

impl Dispatch<WlCompositor, ()> for AppData {
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

impl Dispatch<WlShm, ()> for AppData {
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

impl Dispatch<WlSurface, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlSurface event {:?}", event);
    }
}
impl Dispatch<WlShmPool, ()> for AppData {
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

impl Dispatch<WlBuffer, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        event: <WlBuffer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got WlBuffer event {:?}", event);
    }
}

impl Dispatch<XdgWmBase, ()> for AppData {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            Event::Ping { serial } => proxy.pong(serial),
            _ => {
                println!("Unknown XdgWmBase event: {:?}", event); // Add this line
            }
        }
    }
}

impl Dispatch<XdgSurface, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &XdgSurface,
        event: <XdgSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        println!("got XdgSurface event {:?}", event);
    }
}

impl Dispatch<WlSeat, ()> for AppData {
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

// You need to provide a Dispatch<WlRegistry, GlobalListContents> impl for your app
impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppData {
    fn event(
        _state: &mut AppData,
        _proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        // This mutex contains an up-to-date list of the currently known globals
        // including the one that was just added or destroyed
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<AppData>,
    ) {
        println!("got registry event {:?}", event);
    }
}

impl Dispatch<XdgToplevel, ()> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &XdgToplevel,
        event: <XdgToplevel as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states: _,
            } => {
                xdg_toplevel_configure_event(width, height);
            }
            _ => {
                println!("got XdgToplevel event {:?}", event);
            }
        }
    }
}

impl Dispatch<WlPointer, ObjectId> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        window: &ObjectId,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                motion_event(time, surface_x, surface_y);
            }
            wayland_client::protocol::wl_pointer::Event::Button {
                serial: _,
                time,
                button,
                state,
            } => button_event(time, button, state.into(), window.clone()),
            wayland_client::protocol::wl_pointer::Event::Axis { time, axis, value } => {
                axis_event(time, axis.into(), value, window.clone());
            }
            _ => println!("got WlPointer event {:?}", event),
        }
    }
}

/**
Call this from [WlKeyboard] dispatch for [wayland_client::protocol::wl_keyboard::Event::Key] event.
*/
pub fn wl_keyboard_event(_serial: u32, _time: u32, key: u32, state: u32, surface_id: ObjectId) {
    if let Some(key) = KeyboardKey::from_vk(key) {
        let down = state == 1;
        KEYBOARD_STATE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .apply_all(|shared| {
                shared.set_key_state(key, down, surface_id.protocol_id() as *mut c_void)
            });
        ax::ax_press(key, down);
    } else {
        println!("Unknown key {key}");
    }
}

impl Dispatch<WlKeyboard, ObjectId> for AppData {
    fn event(
        _state: &mut Self,
        _proxy: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_keyboard::Event::Key {
                serial,
                time,
                key,
                state,
            } => {
                wl_keyboard_event(serial, time, key, state.into(), data.clone());
            }
            _ => println!("got WlKeyboard event {:?}", event),
        }
    }
}

pub fn debug_window_show() {
    let conn = Connection::connect_to_env().expect("Can't connect to wayland environment");
    let display = conn.display();

    let mut app_data = AppData {};
    let (globals, mut event_queue) =
        registry_queue_init::<AppData>(&conn).expect("Can't initialize registry");
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());
    let xdg_wm_base: XdgWmBase = globals.bind(&qh, 6..=6, ()).unwrap();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 6..=6, ()).unwrap();
    let shm = globals.bind(&qh, 2..=2, ()).unwrap();

    let surface = compositor.create_surface(&qh, ());

    // Create a toplevel surface
    let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
    xdg_surface.get_toplevel(&qh, ());

    let (file, mmap) = create_shm_buffer(&shm, 200, 200);
    let pool = shm.create_pool(file.as_fd(), mmap.len() as i32, &qh, ());
    let buffer = pool.create_buffer(0, 200, 200, 200 * 4, Format::Argb8888, &qh, ());
    surface.attach(Some(&buffer), 0, 0);
    surface.commit();

    let seat: WlSeat = globals.bind(&qh, 8..=9, ()).expect("Can't bind seat");
    let _pointer = seat.get_pointer(&qh, surface.id());
    let _keyboard = seat.get_keyboard(&qh, surface.id());

    println!("Window should be displayed. Running event loop...");

    loop {
        event_queue.blocking_dispatch(&mut app_data).unwrap();
    }
}

pub fn debug_window_hide() {
    todo!()
}

impl KeyboardKey {
    fn from_vk(vk: u32) -> Option<Self> {
        //taken from https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h

        match vk {
            1 => Some(KeyboardKey::Escape),
            2 => Some(KeyboardKey::Num1),
            3 => Some(KeyboardKey::Num2),
            4 => Some(KeyboardKey::Num3),
            5 => Some(KeyboardKey::Num4),
            6 => Some(KeyboardKey::Num5),
            7 => Some(KeyboardKey::Num6),
            8 => Some(KeyboardKey::Num7),
            9 => Some(KeyboardKey::Num8),
            10 => Some(KeyboardKey::Num9),
            11 => Some(KeyboardKey::Num0),
            12 => Some(KeyboardKey::Minus),
            13 => Some(KeyboardKey::Equal),
            14 => Some(KeyboardKey::Delete),
            15 => Some(KeyboardKey::Tab),
            16 => Some(KeyboardKey::Q),
            17 => Some(KeyboardKey::W),
            18 => Some(KeyboardKey::E),
            19 => Some(KeyboardKey::R),
            20 => Some(KeyboardKey::T),
            21 => Some(KeyboardKey::Y),
            22 => Some(KeyboardKey::U),
            23 => Some(KeyboardKey::I),
            24 => Some(KeyboardKey::O),
            25 => Some(KeyboardKey::P),
            26 => Some(KeyboardKey::LeftBracket),
            27 => Some(KeyboardKey::RightBracket),
            28 => Some(KeyboardKey::Return),
            29 => Some(KeyboardKey::Control),
            30 => Some(KeyboardKey::A),
            31 => Some(KeyboardKey::S),
            32 => Some(KeyboardKey::D),
            33 => Some(KeyboardKey::F),
            34 => Some(KeyboardKey::G),
            35 => Some(KeyboardKey::H),
            36 => Some(KeyboardKey::J),
            37 => Some(KeyboardKey::K),
            38 => Some(KeyboardKey::L),
            39 => Some(KeyboardKey::Semicolon),
            40 => Some(KeyboardKey::Quote),
            41 => Some(KeyboardKey::Grave),
            42 => Some(KeyboardKey::Shift),
            43 => Some(KeyboardKey::Backslash),
            44 => Some(KeyboardKey::Z),
            45 => Some(KeyboardKey::X),
            46 => Some(KeyboardKey::C),
            47 => Some(KeyboardKey::V),
            48 => Some(KeyboardKey::B),
            49 => Some(KeyboardKey::N),
            50 => Some(KeyboardKey::M),
            51 => Some(KeyboardKey::Comma),
            52 => Some(KeyboardKey::Period),
            53 => Some(KeyboardKey::Slash),
            54 => Some(KeyboardKey::RightShift),
            55 => Some(KeyboardKey::KeypadMultiply),
            56 => Some(KeyboardKey::Option),
            57 => Some(KeyboardKey::Space),
            58 => Some(KeyboardKey::CapsLock),
            59 => Some(KeyboardKey::F1),
            60 => Some(KeyboardKey::F2),
            61 => Some(KeyboardKey::F3),
            62 => Some(KeyboardKey::F4),
            63 => Some(KeyboardKey::F5),
            64 => Some(KeyboardKey::F6),
            65 => Some(KeyboardKey::F7),
            66 => Some(KeyboardKey::F8),
            67 => Some(KeyboardKey::F9),
            68 => Some(KeyboardKey::F10),
            69 => Some(KeyboardKey::NumLock),
            70 => Some(KeyboardKey::ScrollLock),
            71 => Some(KeyboardKey::Keypad7),
            72 => Some(KeyboardKey::Keypad8),
            73 => Some(KeyboardKey::Keypad9),
            74 => Some(KeyboardKey::KeypadMinus),
            75 => Some(KeyboardKey::Keypad4),
            76 => Some(KeyboardKey::Keypad5),
            77 => Some(KeyboardKey::Keypad6),
            78 => Some(KeyboardKey::KeypadPlus),
            79 => Some(KeyboardKey::Keypad1),
            80 => Some(KeyboardKey::Keypad2),
            81 => Some(KeyboardKey::Keypad3),
            82 => Some(KeyboardKey::Keypad0),
            83 => Some(KeyboardKey::KeypadDecimal),
            //84 ??
            //85 - KEY_ZENKAKUHANKAKU
            //86 - KEY_102ND
            87 => Some(KeyboardKey::F11),
            88 => Some(KeyboardKey::F12),
            89 => Some(KeyboardKey::JISUnderscore),
            //KEY_KATAKANA
            //KEY_HIRAGANA
            //KEY_KATAKANAHIRAGANA
            //KEY_MUHENKAN
            95 => Some(KeyboardKey::JISKeypadComma),
            96 => Some(KeyboardKey::KeypadEnter),
            97 => Some(KeyboardKey::RightControl),
            98 => Some(KeyboardKey::KeypadDivide),
            //sysreq
            100 => Some(KeyboardKey::RightOption),
            //linefeed
            102 => Some(KeyboardKey::Home),
            103 => Some(KeyboardKey::UpArrow),
            104 => Some(KeyboardKey::PageUp),
            105 => Some(KeyboardKey::LeftArrow),
            106 => Some(KeyboardKey::RightArrow),
            107 => Some(KeyboardKey::End),
            108 => Some(KeyboardKey::DownArrow),
            109 => Some(KeyboardKey::PageDown),
            110 => Some(KeyboardKey::Insert),
            111 => Some(KeyboardKey::ForwardDelete),
            //macro
            113 => Some(KeyboardKey::Mute),
            114 => Some(KeyboardKey::VolumeDown),
            115 => Some(KeyboardKey::VolumeUp),
            116 => Some(KeyboardKey::Power),
            117 => Some(KeyboardKey::KeypadEquals),
            119 => Some(KeyboardKey::Pause),
            //scale
            121 => Some(KeyboardKey::JISKeypadComma),
            //key_hanguel, hanja,
            124 => Some(KeyboardKey::JISYen),
            125 => Some(KeyboardKey::Command),
            126 => Some(KeyboardKey::RightCommand),
            127 => Some(KeyboardKey::ContextMenu),
            //compose
            128 => Some(KeyboardKey::Stop),
            129 => Some(KeyboardKey::Again),
            130 => Some(KeyboardKey::Props),
            131 => Some(KeyboardKey::Undo),
            //front
            133 => Some(KeyboardKey::Copy),
            134 => Some(KeyboardKey::Open),
            135 => Some(KeyboardKey::Paste),
            136 => Some(KeyboardKey::Find),
            137 => Some(KeyboardKey::Cut),
            138 => Some(KeyboardKey::Help),
            139 => Some(KeyboardKey::ContextMenu),
            //calc
            //setup
            //sleep
            //wakeup
            //file, sendfile, deletefile
            //xfer
            148 => Some(KeyboardKey::LaunchApp1),
            149 => Some(KeyboardKey::LaunchApp2),
            150 => Some(KeyboardKey::BrowserHome),
            //msdos
            //coffee (lock)
            //screenlock
            //rotate display
            //direction
            //cycle windows
            155 => Some(KeyboardKey::LaunchMail),
            //bookmarks
            //computer
            158 => Some(KeyboardKey::BrowserBack),
            159 => Some(KeyboardKey::BrowserForward),
            //close cd
            161 => Some(KeyboardKey::Eject),
            //eject+close
            163 => Some(KeyboardKey::NextTrack),
            164 => Some(KeyboardKey::Play),
            165 => Some(KeyboardKey::PreviousTrack),
            166 => Some(KeyboardKey::Stop),
            //record, rewind, phone, iso, config
            172 => Some(KeyboardKey::BrowserHome),
            173 => Some(KeyboardKey::BrowserRefresh),
            //exit, move, edit, scrollup, scrolldown, kpleftparen, kprightparen,
            //key new, redo
            183 => Some(KeyboardKey::F13),
            184 => Some(KeyboardKey::F14),
            185 => Some(KeyboardKey::F15),
            186 => Some(KeyboardKey::F16),
            187 => Some(KeyboardKey::F17),
            188 => Some(KeyboardKey::F18),
            189 => Some(KeyboardKey::F19),
            190 => Some(KeyboardKey::F20),
            191 => Some(KeyboardKey::F21),
            192 => Some(KeyboardKey::F22),
            193 => Some(KeyboardKey::F23),
            194 => Some(KeyboardKey::F24),

            200 => Some(KeyboardKey::Play),
            201 => Some(KeyboardKey::Pause),
            //prog3, prog4
            //all applications
            //dashboard, suspend, close
            207 => Some(KeyboardKey::Play),
            //fastforward, bass boost
            //print
            //hp, camera, sound, question, email, chat,
            217 => Some(KeyboardKey::BrowserSearch),
            //connect, finance, sport, shop
            //alterase
            //cancel
            //brightness up/down
            226 => Some(KeyboardKey::MediaSelect),
            _ => None,
        }
    }
}
