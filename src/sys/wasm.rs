// SPDX-License-Identifier: MPL-2.0
//! Implements browser window, canvas, and resize operations through wasm_lite.

use crate::coordinates::{Position, Size};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebDisplayHandle, WebWindowHandle};
use send_cells::send_cell::SendCell;
use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

// Not `std::sync::Mutex` for `SharedSizeCallback`: it is read inside the
// `onresize` handler, which is the **browser main thread**, and written by
// `Surface::size_update` from whatever thread drives rendering (a worker, in
// practice). A contended `std::sync::Mutex` on the main thread is
// `memory.atomic.wait32`, which traps there.
//
// Like the mouse-location lock, no headless test can reach this: nothing
// resizes the window, so the handler never runs.
//
// The import is crate-wide in this module, so the fullscreen one-shot senders
// below use it too. Those are main-thread-only and effectively uncontended (one
// of the two promise callbacks fires), so it changes nothing for them beyond
// consistency.
use wasm_lite::Closure;
use wasm_lite::dom::{Element, window};
use wasm_lite_std::Mutex;

#[derive(Debug)]
pub struct Window {}

thread_local! {
    static CANVAS_HOLDER: RefCell<Option<CanvasHolder>> = const { RefCell::new(None) };
}

type SizeCallback = dyn Fn(Size) + Send + 'static;
type SharedSizeCallback = Arc<Mutex<Option<Box<SizeCallback>>>>;

enum MainThreadEvent {
    Execute(Box<dyn FnOnce() + Send + 'static>),
}

static MAIN_THREAD_SENDER: OnceLock<continue_stream::Sender<MainThreadEvent>> = OnceLock::new();

struct CanvasHolder {
    handle: WebWindowHandle,
    canvas: Rc<Element>,
    closure_box: SharedSizeCallback,
}
impl CanvasHolder {
    fn new_main() -> CanvasHolder {
        let closure_box: SharedSizeCallback = Arc::new(Mutex::new(None));
        let move_closure_box = closure_box.clone();

        let window = window().expect("Can't get window");

        let document = window.document().expect("Can't get document");

        // One `Element`, so no casts: web-sys needed `Element` -> `HtmlElement`
        // -> `HtmlCanvasElement` to reach `style()` and `set_attribute`, and
        // none of those steps did anything at runtime.
        let canvas = document
            .create_element("canvas")
            .expect("Can't create canvas");

        let style = canvas.style();
        style
            .set_property("width", "100vw")
            .expect("Can't set width");
        style
            .set_property("height", "100vh")
            .expect("Can't set height");

        canvas
            .set_attribute("data-raw-handle", "1")
            .expect("Can't set data-raw-handle");
        let canvas_rc = Rc::new(canvas);
        let canvas_weak = Rc::downgrade(&canvas_rc);
        let closure = Closure::new(move || {
            match canvas_weak.upgrade() {
                None => { /* deallocated? */ }
                Some(_canvas) => {
                    //report the window's logical size, matching size_scale/size_main.
                    //The canvas width/height attributes are the buffer size, which this
                    //crate never sets, so reading them would report a stale/default size.
                    // Fully qualified: the enclosing scope binds `window` to a
                    // `Window` value, which would shadow the function.
                    let w = wasm_lite::dom::window().expect("No window?");
                    // `lock_sync`: spins on the main thread, blocks on a worker.
                    if let Some(closure) = move_closure_box.lock_sync().as_ref() {
                        closure(Size::new(w.inner_width(), w.inner_height()));
                    }
                }
            }
        });

        window.set_onresize(Some(closure.as_js_value()));
        closure.forget();

        document
            .body()
            .expect("Can't get body")
            .append_child(canvas_rc.as_ref())
            .expect("Can't append canvas to body");
        CanvasHolder {
            handle: WebWindowHandle::new(1),
            canvas: canvas_rc,
            closure_box,
        }
    }
}

#[derive(Debug)]
pub struct FullscreenError(String);

impl Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for FullscreenError {}

impl Window {
    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        let (sender, fut) = r#continue::continuation();
        let sender_mutex = Arc::new(Mutex::new(Some(sender)));
        let sender_mutex_error = sender_mutex.clone();
        let main_thread_job =
            crate::application::on_main_thread("Window::fullscreen".to_string(), move || {
                let strong_closure = Closure::new_with_arg(move |_| {
                    if let Some(lock) = sender_mutex.lock_sync().take() {
                        lock.send(Ok(()));
                    }
                });
                let error_closure = Closure::new_with_arg(move |a: wasm_lite::JsValue| {
                    if let Some(lock) = sender_mutex_error.lock_sync().take() {
                        // `Display` renders the rejection the way JS would; the
                        // old code cast it to a `TypeError` first, which was an
                        // unchecked cast to a type it might not have been.
                        lock.send(Err(a.to_string()));
                    }
                });
                let window = window().expect("Can't get window");
                let doc = window.document().expect("Can't get document");
                let canvas = CanvasHolder::new_main();
                doc.set_title(&title);
                match canvas.canvas.request_fullscreen() {
                    Ok(promise) => {
                        drop(promise_then2(
                            &promise,
                            strong_closure.as_js_value(),
                            error_closure.as_js_value(),
                        ));
                    }
                    // Fullscreen refused before it even returned a promise.
                    // Report through the same channel the rejection would use.
                    Err(e) => {
                        error_closure_call(error_closure.as_js_value(), &e);
                    }
                }
                CANVAS_HOLDER.replace(Some(canvas));
                SendCell::new((strong_closure, error_closure))
            });
        let closures = main_thread_job.await;
        logwise::log!("app_window: waiting for the fullscreen future");
        let fullscreen_result = fut.await;
        //drop our closures
        crate::application::on_main_thread("Drop fs".to_string(), move || {
            drop(closures);
        })
        .await;
        match fullscreen_result {
            Ok(..) => Ok(Window {}),
            Err(err) => Err(FullscreenError(err)),
        }
    }
    pub async fn new(_position: Position, _size: Size, title: String) -> Self {
        let f = crate::application::on_main_thread("Window::new".to_string(), move || {
            let window = window().expect("Can't get window");
            let doc = window.document().expect("Can't get document");
            doc.set_title(&title);
            CANVAS_HOLDER.replace(Some(CanvasHolder::new_main()));
        });
        f.await;
        Window {}
    }

    pub async fn surface(&self) -> Surface {
        crate::application::on_main_thread("surface".to_string(), || {
            CANVAS_HOLDER.with_borrow_mut(|canvas| {
                let canvas = canvas.as_ref().expect("no canvas");
                Surface {
                    display_handle: canvas.handle,
                    closure_box: DebugWrapper(canvas.closure_box.clone()),
                }
            })
        })
        .await
    }
    pub async fn default() -> Self {
        Window::new(
            Position::new(0.0, 0.0),
            Size::new(800.0, 600.0),
            String::from("app_window"),
        )
        .await
    }
}

/// Whether this is the browser main thread.
///
/// One `instanceof Window`, where web-sys needed two downcasts plus a
/// `Reflect`-based Node probe. Node was never supported on this path — the
/// probe existed only to produce a better panic — and wasm_lite does not target
/// Node at all, so the whole branch is gone: a worker answers `false`, and
/// anything else has no `Window`, which is the same answer.
pub fn is_main_thread() -> bool {
    wasm_lite::dom::is_main_thread()
}

pub fn run_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    let (sender, receiver) = continue_stream::continuation();

    let mut sent = false;
    MAIN_THREAD_SENDER.get_or_init(|| {
        sent = true;
        sender
    });
    assert!(sent, "Don't call run_main_thread more than once");

    // Captured here, on the calling thread, and carried into the worker. This
    // is what a durable token is for: the worker is a different realm, so a
    // thread-local "current context" could not have crossed.
    let spawn_context = logwise::context::capture();

    wasm_lite_std::spawn(move || {
        let worker_context = logwise::context::child(spawn_context, "app_window.after_main_thread");
        // Entered on the worker, restored when the closure returns.
        let _entered = logwise::context::enter(worker_context);
        closure();
    });

    let event_loop_context = logwise::context::child(
        logwise::context::capture(),
        "app_window.main_thread_eventloop",
    );
    wasm_lite_std::spawn_local(async move {
        loop {
            let event = receiver.receive().await.expect("Can't receive event");
            // Entered around the turn and never across the await above: the
            // guard is `!Send` and restores thread-local state on drop, so it
            // must not be held across a yield point.
            let _entered = logwise::context::enter(event_loop_context);
            match event {
                MainThreadEvent::Execute(f) => f(),
            }
        }
    });
}

pub fn on_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    if is_main_thread() {
        closure()
    } else {
        let mt_sender = MAIN_THREAD_SENDER
            .get()
            .expect(crate::application::CALL_MAIN);
        let boxed_closure = Box::new(closure) as Box<dyn FnOnce() + Send + 'static>;
        // let perf = logwise::perfwarn_begin!("starting SEND task");

        mt_sender.send(MainThreadEvent::Execute(boxed_closure));
    }
}

pub fn stop_main_thread() {
    //nothing to do - handled by browsers
}

pub async fn alert(message: String) {
    crate::application::on_main_thread("alert".to_string(), move || {
        let window = window().expect("Can't get window");
        window.alert(&message).expect("Alert failed");
    })
    .await
}

mod promise {
    use wasm_lite::JsValue;
    wasm_lite::import! {
        "Promise" {
            /// `promise.then(onFulfilled, onRejected)`.
            fn then2(this: &JsValue, on_ok: &JsValue, on_err: &JsValue) -> JsValue as "then";
        }
        "Function" {
            /// `f.call(undefined, arg)` — used to report a synchronous
            /// fullscreen refusal through the same closure the rejection would.
            fn call1(this: &JsValue, this_arg: &JsValue, arg: &JsValue) -> JsValue as "call";
        }
    }
}

use promise::then2 as promise_then2;

fn error_closure_call(closure: &wasm_lite::JsValue, error: &wasm_lite::JsValue) {
    promise::call1(closure, &wasm_lite::JsValue::UNDEFINED, error);
}

#[derive(Clone)]
struct DebugWrapper<T>(T);

impl<T> Debug for DebugWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DebugWrapper")
    }
}
#[derive(Debug)]
pub struct Surface {
    display_handle: WebWindowHandle,
    closure_box: DebugWrapper<SharedSizeCallback>,
}
impl Surface {
    pub async fn size_scale(&self) -> (Size, f64) {
        crate::application::on_main_thread("size_scale".to_string(), || {
            let w = window().expect("No window?");
            (
                Size::new(w.inner_width(), w.inner_height()),
                w.device_pixel_ratio(),
            )
        })
        .await
    }

    pub fn size_main(&self) -> (Size, f64) {
        let w = window().expect("No window?");
        (
            Size::new(w.inner_width(), w.inner_height()),
            w.device_pixel_ratio(),
        )
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Web(self.display_handle)
    }
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Web(WebDisplayHandle::new())
    }
    /**
    Run the attached callback when size changes.
    */
    pub fn size_update<F: Fn(Size) + Send + 'static>(&mut self, update: F) {
        self.closure_box.0.lock_sync().replace(Box::new(update));
    }
}
