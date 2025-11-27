// SPDX-License-Identifier: MPL-2.0

use crate::coordinates::{Position, Size};
use logwise::Level;
use logwise::context::Context;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebDisplayHandle, WebWindowHandle};
use send_cells::send_cell::SendCell;
use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::js_sys::Promise;
use web_sys::js_sys::TypeError;
use web_sys::{HtmlCanvasElement, window};

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
    canvas: Rc<HtmlCanvasElement>,
    closure_box: SharedSizeCallback,
}
impl CanvasHolder {
    fn new_main() -> CanvasHolder {
        use web_sys::wasm_bindgen::__rt::IntoJsResult;
        let closure_box: SharedSizeCallback = Arc::new(Mutex::new(None));
        let move_closure_box = closure_box.clone();

        let window = window().expect("Can't get window");

        let document = window.document().expect("Can't get document");

        let element = document
            .create_element("canvas")
            .expect("Can't create canvas");
        let html_element = web_sys::HtmlElement::from(
            element.into_js_result().expect("Can't create html element"),
        );

        let style = html_element.style();
        style
            .set_property("width", "100vw")
            .expect("Can't set width");
        style
            .set_property("height", "100vh")
            .expect("Can't set height");

        let canvas = web_sys::HtmlCanvasElement::from(
            html_element.into_js_result().expect("Can't get canvas"),
        );
        canvas
            .set_attribute("data-raw-handle", "1")
            .expect("Can't set data-raw-handle");
        let canvas_rc = Rc::new(canvas);
        let canvas_weak = Rc::downgrade(&canvas_rc);
        let closure = Closure::<dyn FnMut()>::new(move || {
            match canvas_weak.upgrade() {
                None => { /* deallocated? */ }
                Some(canvas) => {
                    let width = canvas.width();
                    let height = canvas.height();
                    if let Some(closure) = move_closure_box.lock().unwrap().as_ref() {
                        closure(Size::new(width as f64, height as f64));
                    }
                }
            }
        });

        //I think this is safe??
        window.set_onresize(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        document
            .body()
            .unwrap()
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

#[wasm_bindgen]
extern "C" {
    type Element2;
    #[wasm_bindgen::prelude::wasm_bindgen(method,js_class="Element",js_name=requestFullscreen)]
    fn request_fullscreen_2(this: &Element2) -> Promise;
}

impl Window {
    pub async fn fullscreen(title: String) -> Result<Self, FullscreenError> {
        let (sender, fut) = r#continue::continuation();
        let sender_mutex = Arc::new(Mutex::new(Some(sender)));
        let sender_mutex_error = sender_mutex.clone();
        let main_thread_job =
            crate::application::on_main_thread("Window::fullscreen".to_string(), move || {
                let strong_closure = Closure::once(move |_| {
                    let lock = sender_mutex.lock().unwrap().take().expect("already sent?");
                    lock.send(Ok(()));
                });
                let error_closure = Closure::once(move |a: JsValue| {
                    let lock = sender_mutex_error
                        .lock()
                        .unwrap()
                        .take()
                        .expect("already sent?");
                    let a_typeerror: TypeError = a.unchecked_into();
                    let a_string = a_typeerror.to_string();

                    lock.send(Err(ToString::to_string(&a_string)));
                });
                let window = window().expect("Can't get window");
                let doc = window.document().expect("Can't get document");
                let canvas = CanvasHolder::new_main();
                let as_element_2: &Element2 = canvas.canvas.as_ref().unchecked_ref();
                doc.set_title(&title);
                let promise = as_element_2.request_fullscreen_2();
                drop(promise.then2(&strong_closure, &error_closure));
                CANVAS_HOLDER.replace(Some(canvas));
                SendCell::new((strong_closure, error_closure))
            });
        let closures = main_thread_job.await;
        logwise::warn_sync!("Waiting for fut...");
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

    pub async fn surface(&self) -> crate::surface::Surface {
        let sys_surface = crate::application::on_main_thread("surface".to_string(), || {
            CANVAS_HOLDER.with_borrow_mut(|canvas| {
                let canvas = canvas.as_ref().expect("no canvas");
                Surface {
                    display_handle: canvas.handle,
                    closure_box: DebugWrapper(canvas.closure_box.clone()),
                }
            })
        })
        .await;
        crate::surface::Surface { sys: sys_surface }
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

pub fn is_main_thread() -> bool {
    let g = web_sys::js_sys::global();

    // Browser: main thread vs Web Worker
    if g.dyn_ref::<web_sys::Window>().is_some() {
        return true;
    }
    if g.dyn_ref::<web_sys::WorkerGlobalScope>().is_some() {
        return false;
    }

    // Node: detect environment, then query worker_threads.isMainThread
    if is_node_env(&g) {
        return node_is_main_thread_cjs(); // sync, works when `require` is available
    }

    // Unknown host
    panic!("Unknown global object type: {:?}", g);
}

fn is_node_env(g: &wasm_bindgen::JsValue) -> bool {
    // typeof process === 'object' && !!process?.versions?.node
    if let Ok(process) = web_sys::js_sys::Reflect::get(g, &"process".into())
        && !process.is_undefined()
        && !process.is_null()
        && let Ok(versions) = web_sys::js_sys::Reflect::get(&process, &"versions".into())
        && let Ok(node) = web_sys::js_sys::Reflect::get(&versions, &"node".into())
    {
        return !node.is_undefined() && !node.is_null();
    }
    false
}

// --- Node (CommonJS): synchronous path ---
// Uses `require('node:worker_threads').isMainThread` if `require` exists.
#[wasm_bindgen(inline_js = r#"
export function nodeIsMainThreadCJS() {
  try {
    if (typeof require !== 'undefined') {
      return require('node:worker_threads').isMainThread;
    }
  } catch (_) {}
  // If require isn't available, caller can try the async ESM variant.
  return true; // sensible default on main thread
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = nodeIsMainThreadCJS)]
    fn node_is_main_thread_cjs() -> bool;
}
pub fn run_main_thread<F: FnOnce() + Send + 'static>(closure: F) {
    let (sender, receiver) = continue_stream::continuation();

    let mut sent = false;
    MAIN_THREAD_SENDER.get_or_init(|| {
        sent = true;
        sender
    });
    assert!(sent, "Don't call run_main_thread more than once");

    let push_context = Context::current();
    let push_context_2 = push_context.clone();

    // logwise::info_sync!("wasm_thread WILL spawn");

    wasm_thread::spawn(|| {
        // logwise::info_sync!("wasm_thread spawn");
        let new_context = Context::new_task(
            Some(push_context_2),
            "app_window after MT context".to_string(),
            Level::DebugInternal,
            logwise::log_enabled!(Level::DebugInternal),
        );
        let new_id = new_context.context_id();
        new_context.set_current();
        closure();
        Context::pop(new_id);
    });

    let event_loop_context = Context::new_task(
        Some(Context::current()),
        "main thread eventloop".to_string(),
        Level::DebugInternal,
        logwise::log_enabled!(Level::DebugInternal),
    );
    let apply_context = logwise::context::ApplyContext::new(event_loop_context, async move {
        loop {
            // logwise::debuginternal_sync!("Waiting for main thread event");
            let event = receiver.receive().await.expect("Can't receive event");
            // logwise::debuginternal_sync!("Received main thread event");
            match event {
                MainThreadEvent::Execute(f) => f(),
            }
        }
    });
    wasm_bindgen_futures::spawn_local(apply_context);
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
        window.alert_with_message(&message).expect("Alert failed");
    })
    .await
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
            let width = w
                .inner_width()
                .expect("No width?")
                .as_f64()
                .expect("No width?");
            let height = w
                .inner_height()
                .expect("No height?")
                .as_f64()
                .expect("No height?");
            let px = w.device_pixel_ratio();

            (Size::new(width, height), px)
        })
        .await
    }

    pub fn size_main(&self) -> (Size, f64) {
        let w = window().expect("No window?");
        let width = w
            .inner_width()
            .expect("No width?")
            .as_f64()
            .expect("No width?");
        let height = w
            .inner_height()
            .expect("No height?")
            .as_f64()
            .expect("No height?");
        let px = w.device_pixel_ratio();

        (Size::new(width, height), px)
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
        self.closure_box.0.lock().unwrap().replace(Box::new(update));
    }
}
