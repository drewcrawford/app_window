use std::ffi::c_void;
use std::fmt::Display;
use std::sync::Arc;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use send_cells::send_cell::SendCell;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HINSTANCE, HSTR, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics, LoadCursorW, PeekMessageW, PostThreadMessageW, RegisterClassExW, ShowWindow, TranslateMessage, HMENU, IDC_ARROW, MSG, PM_NOREMOVE, SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_USER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW, WS_POPUP};
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

struct WinClosure(Box<dyn FnOnce() + Send + 'static>);



pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    //need to create a message queue first
    let mut message = MSG::default();
    _ = unsafe { PeekMessageW(&mut message, HWND::default(), WM_USER, WM_USER, PM_NOREMOVE) }; //create a message queue
    //we don't care about the return value of PeekMessageW, it simply tells us if messages are available or not

    //now the queue is available so subsequent calls to PostMessageW will work
    closure(); //I think it's ok to run inline on windows?
    let all_hwnd = HWND::default();
    loop {
        logwise::warn_sync!("will getMessageW");
        let message_ret = unsafe { GetMessageW(&mut message, all_hwnd, 0, 0) };
        if message_ret.0 == 0 {
            break;
        } else if message_ret.0 == -1 {
            panic!("GetMessageW failed");
        }
        match message.message {
            WM_RUN_FUNCTION => {
                let as_usize = message.wParam.0;
                let winclosure = unsafe { Box::from_raw(as_usize as *mut WinClosure) };
                winclosure.0();
            }
            _ => {
                unsafe {
                    //ms code seems to ignore this return value in practice
                    //see https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessage
                    _ = TranslateMessage(&mut message);
                    DispatchMessageW(&mut message);
                }
            }
        }
    }
}

pub fn on_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    let boxed_closure = Box::new(WinClosure(Box::new(closure)));
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
        _ => {
            unsafe{DefWindowProcW(hwnd,msg,w_param, l_param)}
        }
    }
}
fn create_window_impl(position: Position, size: Size, title: String, style: WINDOW_STYLE) -> HWND {
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
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: Default::default(),
    };
    let r = unsafe{RegisterClassExW(&window_class)};
    assert_ne!(r, 0, "failed to register window class: {:?}",unsafe{GetLastError()});

    let window = unsafe{CreateWindowExW(WINDOW_EX_STYLE(0), //style
                                        class_name,
                                        &winstr,
                                        style,
                                        position.x() as i32, position.y() as i32, //position
                                        size.width() as i32, size.height() as i32, //size
                                        HWND(std::ptr::null_mut()), //parent
                                        HMENU(std::ptr::null_mut()), //menu
                                        instance, //instance
                                        None,

    )}.expect("failed to create window");
    unsafe{_ = ShowWindow(window, SW_SHOWNORMAL)};
    window
}

impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        let window = crate::application::on_main_thread(move || {
            let window = create_window_impl(position, size, title, WS_OVERLAPPEDWINDOW);
            SendCell::new(window)
        }).await;

        Window {
            hwnd: window,
        }

    }

    pub async fn default() -> Self {
        Self::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), "app_window".to_string()).await
    }

    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        let size = Size::new(unsafe { GetSystemMetrics(SM_CXSCREEN) as f64}, unsafe { GetSystemMetrics(SM_CYSCREEN) as f64});
        let window = crate::application::on_main_thread(move || {
            let window = create_window_impl(Position::new(0.0, 0.0), size, title, WS_POPUP);
            SendCell::new(window)
        }).await;

        Ok(Window {
            hwnd: window,
        })
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