// SPDX-License-Identifier: MPL-2.0

use crate::coordinates::{Position, Size};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
};
use send_cells::send_cell::SendCell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Display;
use std::num::NonZero;
use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetSystemMetrics, HWND_MESSAGE, IDC_ARROW, LoadCursorW, MSG, PostMessageW, PostQuitMessage,
    RegisterClassExW, SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WM_SIZE, WM_USER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
    WS_POPUP,
};
use windows::core::{HSTRING, PCWSTR, w};

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
    #[unsafe(link_section = ".CRT$XCU")]
    static INIT_MAIN_THREAD_ID: unsafe fn() = {
        unsafe fn initer() {
            unsafe { MAIN_THREAD_ID = windows::Win32::System::Threading::GetCurrentThreadId() };
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

#[derive(Default)]
struct HwndImp {
    size_notify: Option<Box<dyn Fn(Size)>>,
}
thread_local! {
    static HWND_IMPS: RefCell<HashMap<*mut c_void /* hwnd */, HwndImp>> = RefCell::new(HashMap::new());
}

//The message-only window that receives WM_RUN_FUNCTION.  We post closures to a window
//rather than via PostThreadMessageW because thread messages are silently discarded by
//modal message loops (window move/size, menus, dialogs), which would leak the closure
//and hang the awaiting future.
static DISPATCH_HWND: std::sync::atomic::AtomicPtr<c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

extern "system" fn dispatch_window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_RUN_FUNCTION => {
            let as_usize = w_param.0;
            let winclosure = unsafe { Box::from_raw(as_usize as *mut WinClosure) };
            winclosure.0();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

pub fn run_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    //create the message-only dispatch window; this also creates the message queue
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("Can't get module");
    let class_name = w!("app_window_main_dispatch");
    let window_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: Default::default(),
        lpfnWndProc: Some(dispatch_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance.into(),
        hIcon: Default::default(),
        hCursor: Default::default(),
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: Default::default(),
    };
    let r = unsafe { RegisterClassExW(&window_class) };
    assert_ne!(r, 0, "failed to register dispatch class: {:?}", unsafe {
        GetLastError()
    });
    let dispatch_window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE), //message-only window
            None,
            None,
            None,
        )
    }
    .expect("failed to create dispatch window");
    DISPATCH_HWND.store(dispatch_window.0, std::sync::atomic::Ordering::Release);

    //Run the closure on a secondary thread, like other platforms.  Running it inline
    //would deadlock any closure that blocks on a future needing the main-thread
    //message loop (e.g. the documented `block_on(Window::default())` pattern),
    //since the loop below wouldn't be running yet.
    std::thread::spawn(closure);
    let mut message = MSG::default();
    loop {
        let message_ret = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if message_ret.0 == 0 {
            break;
        } else if message_ret.0 == -1 {
            panic!("GetMessageW failed");
        }
        unsafe {
            //ms code seems to ignore this return value in practice
            //see https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmessage
            _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    crate::application::IS_MAIN_THREAD_RUNNING.store(false, std::sync::atomic::Ordering::Release);
}

pub fn on_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    let boxed_closure = Box::new(WinClosure(Box::new(closure)));
    let closure_ptr = Box::into_raw(boxed_closure) as *mut ();
    let as_usize = closure_ptr as usize;
    //The dispatch window is created slightly after IS_MAIN_THREAD_RUNNING is set;
    //briefly wait for it rather than failing.
    let hwnd = loop {
        let ptr = DISPATCH_HWND.load(std::sync::atomic::Ordering::Acquire);
        if !ptr.is_null() {
            break HWND(ptr);
        }
        std::thread::yield_now();
    };
    if let Err(error) =
        unsafe { PostMessageW(Some(hwnd), WM_RUN_FUNCTION, WPARAM(as_usize), LPARAM(0)) }
    {
        // A failed post did not transfer ownership to the window procedure.
        drop(unsafe { Box::from_raw(closure_ptr as *mut WinClosure) });
        panic!("PostMessageW failed: {error}");
    }
}

pub fn stop_main_thread() {
    // PostQuitMessage targets the calling thread. Route it through the dispatch
    // window so callers on worker threads stop the actual UI message loop.
    on_main_thread(|| unsafe { PostQuitMessage(0) });
}

pub async fn alert(message: String) {
    todo!("alert not yet implemented for Windows: {}", message)
}

#[derive(Debug)]
pub struct Window {
    hwnd: SendCell<HWND>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

extern "system" fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    logwise::debuginternal_sync!(
        "got msg hwnd {hwnd} msg {msg} w_param {w_param} l_param {l_param}",
        hwnd = logwise::privacy::LogIt(&hwnd),
        msg = msg,
        w_param = logwise::privacy::LogIt(&w_param),
        l_param = logwise::privacy::LogIt(&l_param)
    );
    if crate::input::window_proc(hwnd, msg, w_param, l_param) == LRESULT(0) {
        return LRESULT(0);
    }

    match msg {
        m if m == WM_SIZE => {
            let width = (l_param.0 as u32 & 0xFFFF) as i32; // LOWORD(lParam)
            let height = ((l_param.0 as u32 >> 16) & 0xFFFF) as i32; // HIWORD(lParam)
            let size = Size::new(width as f64, height as f64);
            //take the callback out of the map before calling it: window procs are
            //re-entrant on the same thread, and a callback that sends a message
            //would otherwise hit a nested RefCell borrow (panic → abort).
            let notify =
                HWND_IMPS.with_borrow_mut(|c| c.entry(hwnd.0).or_default().size_notify.take());
            if let Some(f) = notify {
                f(size);
                HWND_IMPS.with_borrow_mut(|c| {
                    let entry = c.entry(hwnd.0).or_default();
                    //don't clobber a callback registered during the call
                    if entry.size_notify.is_none() {
                        entry.size_notify = Some(f);
                    }
                });
            }
            LRESULT(0)
        }
        m if m == WM_DESTROY => {
            //drop any registered callback so the entry can't fire for a recycled HWND
            HWND_IMPS.with_borrow_mut(|c| {
                c.remove(&hwnd.0);
            });
            unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}
fn create_window_impl(position: Position, size: Size, title: String, style: WINDOW_STYLE) -> HWND {
    let instance = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("Can't get module");
    let cursor =
        unsafe { LoadCursorW(Some(HINSTANCE::default()), IDC_ARROW) }.expect("Can't load cursor");
    let winstr: HSTRING = title.into();
    let class_name = w!("app_window_window");
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
    let r = unsafe { RegisterClassExW(&window_class) };
    if r == 0 {
        //the class persists for the process lifetime, so a second window
        //re-registers it; that's fine
        let err = unsafe { GetLastError() };
        assert_eq!(
            err, ERROR_CLASS_ALREADY_EXISTS,
            "failed to register window class: {:?}",
            err
        );
    }

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0), //style
            class_name,
            &winstr,
            style,
            position.x() as i32,
            position.y() as i32, //position
            size.width() as i32,
            size.height() as i32, //size
            None,                 //parent
            None,                 //menu
            None,                 //instance
            None,
        )
    }
    .expect("failed to create window");
    unsafe { _ = ShowWindow(window, SW_SHOWNORMAL) };
    window
}

impl Window {
    pub async fn new(position: Position, size: Size, title: String) -> Self {
        let window = crate::application::on_main_thread("Window::new".into(), move || {
            let window = create_window_impl(position, size, title, WS_OVERLAPPEDWINDOW);
            SendCell::new(window)
        })
        .await;

        Window { hwnd: window }
    }

    pub async fn default() -> Self {
        Self::new(
            Position::new(0.0, 0.0),
            Size::new(800.0, 600.0),
            "app_window".to_string(),
        )
        .await
    }

    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        let size = Size::new(unsafe { GetSystemMetrics(SM_CXSCREEN) as f64 }, unsafe {
            GetSystemMetrics(SM_CYSCREEN) as f64
        });
        let window = crate::application::on_main_thread("Window::fullscreen".into(), move || {
            let window = create_window_impl(Position::new(0.0, 0.0), size, title, WS_POPUP);
            SendCell::new(window)
        })
        .await;

        Ok(Window { hwnd: window })
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        let copy_hwnd = self.hwnd.copying();
        crate::surface::Surface {
            sys: Surface { imp: copy_hwnd },
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let unsafe_hwnd = unsafe { *self.hwnd.get_unchecked() };
        let unsafe_port_hwnd = send_cells::unsafe_send_cell::UnsafeSendCell::new(unsafe_hwnd);
        logwise::debuginternal_sync!("Destroying window");
        on_main_thread(move || {
            unsafe { DestroyWindow(*unsafe_port_hwnd.get()) }.expect("Can't close window");
        });
    }
}

#[derive(Debug)]
pub struct Surface {
    imp: SendCell<HWND>,
}

unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}

impl Surface {
    fn size_imp(hwnd: HWND) -> (Size, f64) {
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect).expect("Can't get size") }
        let s = Size::new(rect.right as f64, rect.bottom as f64);
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let scale = dpi as f64 / 96.0;
        (s, scale)
    }
    pub async fn size_scale(&self) -> (Size, f64) {
        let send_hwnd = self.imp.copying();
        crate::application::on_main_thread("Surface::size_scale".into(), move || {
            Self::size_imp(*send_hwnd.get())
        })
        .await
    }
    pub fn size_main(&self) -> (Size, f64) {
        assert!(
            crate::application::is_main_thread(),
            "Call from main thread only"
        );
        Self::size_imp(*self.imp.get())
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        //should be fine since we're just reading the value
        let unsafe_hwnd: HWND = unsafe { *self.imp.get_unchecked() };
        RawWindowHandle::Win32(Win32WindowHandle::new(
            NonZero::new(unsafe_hwnd.0 as isize).expect("HWND is null"),
        ))
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Windows(WindowsDisplayHandle::new())
    }

    pub fn size_update<F: Fn(Size) + Send + 'static>(&mut self, _update: F) {
        let move_hwnd = self.imp.copying();
        on_main_thread(move || {
            let hwnd = move_hwnd.get();
            HWND_IMPS.with_borrow_mut(|c| {
                let entry = c.entry(hwnd.0).or_default();
                entry.size_notify = Some(Box::new(_update));
            });
        });
    }
}

impl Drop for Surface {
    fn drop(&mut self) {}
}
