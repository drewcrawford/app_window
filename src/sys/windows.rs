use std::ffi::c_void;
use std::fmt::Display;
use std::sync::Arc;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use send_cells::send_cell::SendCell;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HINSTANCE, HSTR, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW, PostThreadMessageW, RegisterClassExW, ShowWindow, TranslateMessage, HMENU, IDC_ARROW, MSG, SW_SHOWNORMAL, WINDOW_EX_STYLE, WM_USER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW};
use crate::coordinates::{Position, Size};
const WM_RUN_FUNCTION: u32 = WM_USER;

#[derive(Debug)]
pub struct FullscreenError;

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "FullscreenError")
    }
}
impl std::error::Error for FullscreenError {}

fn main_thread_id() -> u32 {
    static mut MAIN_THREAD_ID: u32 = 0;
    #[used]
    #[allow(non_upper_case_globals)]
    #[link_section = ".CRT$XCU"]
    static INIT_MAIN_THREAD_ID: unsafe fn() = {
        unsafe fn initer() {
            MAIN_THREAD_ID = windows::Win32::System::Threading::GetCurrentThreadId();
        }
        initer
    };

    unsafe { MAIN_THREAD_ID }
}


pub fn is_main_thread() -> bool {
    //windows does not have a clear concept of a main thread but allows any thread to be in charge
    //of a window.  However for compatibility we project a 'main thread-like' concept onto windows
    let current_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    current_id == main_thread_id()
}


pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    closure(); //I think it's ok to run inline on windows?
    let mut message = MSG::default();
    let all_hwnd = HWND::default();
    loop {
        logwise::warn_sync!("will getMessageW");
        let message_ret = unsafe{GetMessageW(&mut message, all_hwnd, 0, 0)};
        logwise::warn_sync!("got messageW");

        if message_ret.0 == 0 {
            break;
        }
        else if message_ret.0 == -1 {
            panic!("GetMessageW failed");
        }
        unsafe {
            //ms code seems to ignore this return value in practice
            //see https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessage
            _ = TranslateMessage(&mut message);
            DispatchMessageW(&mut message);
        }
    }
}

pub fn on_main_thread<F: FnOnce() + Send>(closure: F) {
    let boxed_closure = Box::new(closure) as Box<dyn FnOnce() + Send>;
    let closure_ptr = Box::into_raw(boxed_closure) as *mut ();
    let as_usize = closure_ptr as usize;
    unsafe { PostThreadMessageW(main_thread_id(), WM_RUN_FUNCTION, WPARAM(as_usize), LPARAM(0)) }.expect("PostThreadMessageW failed");
}

pub struct Window {
    hwnd: SendCell<HWND>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

extern "system" fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    eprintln!("got msg hwnd {hwnd:?} msg {msg} w_param {w_param:?} l_param {l_param:?}");
    match msg {
        msg if msg == WM_RUN_FUNCTION => {
            let closure = l_param.0 as *mut Box<dyn FnOnce() + Send>;
            let closure = unsafe{Box::from_raw(closure)};
            closure();
            LRESULT(0)
        }
        _ => {
            unsafe{DefWindowProcW(hwnd,msg,w_param, l_param)}
        }
    }
}

impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        let window = crate::application::on_main_thread(move || {

            let instance = unsafe{GetModuleHandleW(PCWSTR::null())}.expect("Can't get module");
            let cursor = unsafe{LoadCursorW(HINSTANCE::default(), IDC_ARROW)}.expect("Can't load cursor");
            let winstr: HSTRING = title.into();
            let class_name = w!("raw_input_debug_window");
            let window_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: Default::default(),
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance.into(),
                hIcon: Default::default(),
                hCursor: cursor,
                hbrBackground: HBRUSH(COLOR_WINDOW.0 as usize as *mut c_void),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: Default::default(),
            };
            let r = unsafe{RegisterClassExW(&window_class)};
            assert_ne!(r, 0, "failed to register window class: {:?}",unsafe{GetLastError()});

            let window = unsafe{CreateWindowExW(WINDOW_EX_STYLE(0), //style
                                   class_name,
                                   &winstr,
                                   WS_OVERLAPPEDWINDOW,
                                   position.x() as i32, position.y() as i32, //position
                                   size.width() as i32, size.height() as i32, //size
                                   HWND(std::ptr::null_mut()), //parent
                                   HMENU(std::ptr::null_mut()), //menu
                                   instance, //instance
                                   None,

            )}.expect("failed to create window");
            unsafe{_ = ShowWindow(window, SW_SHOWNORMAL)};
            todo!()
            //SendCell::new(window)
        }).await;

        Window {
            hwnd: window,
        }

    }

    pub async fn default() -> Self {
        Self::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), "app_window".to_string()).await
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
    imp: *mut c_void,
    update_size: Option<Arc<dyn Fn(Size)>>,
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