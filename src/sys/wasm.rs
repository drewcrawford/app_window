use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use logwise::context::Context;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebDisplayHandle, WebWindowHandle};
use send_cell::send_cell::SendCell;
use wasm_bindgen::closure::{Closure};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::js_sys::{Function, Promise};
use web_sys::{window, Element, HtmlCanvasElement};
use web_sys::console::error;
use web_sys::js_sys::TypeError;
use crate::coordinates::{Position, Size};

pub struct Window {

}

thread_local! {
    static CANVAS_HOLDER: RefCell<Option<CanvasHolder>> = RefCell::new(None);
}

enum MainThreadEvent {
    Execute(Box<dyn FnOnce() + Send + 'static>)
}

static MAIN_THREAD_SENDER: OnceLock<ampsc::ChannelProducer<MainThreadEvent>> = OnceLock::new();

struct CanvasHolder {
    handle: WebWindowHandle,
    closure: Closure<dyn FnMut()>,
    canvas: Rc<HtmlCanvasElement>,
    closure_box: Arc<Mutex<Option<Box<dyn Fn(Size) + Send>>>>,
}
impl CanvasHolder {
    fn new_main() -> CanvasHolder {
        use web_sys::wasm_bindgen::__rt::IntoJsResult;
        let closure_box = Arc::new(Mutex::new(None));
        let move_closure_box = closure_box.clone();

        struct SendMe(*const Function);
        unsafe impl Send for SendMe {}
        let window = window().expect("Can't get window");

        let document = window.document().expect("Can't get document");

        let element = document.create_element("canvas").expect("Can't create canvas");
        let html_element = web_sys::HtmlElement::from(element.into_js_result().expect("Can't create html element"));

        let style = html_element.style();
        style.set_property("width","100vw").expect("Can't set width");
        style.set_property("height","100vh").expect("Can't set height");


        let canvas = web_sys::HtmlCanvasElement::from(html_element.into_js_result().expect("Can't get canvas"));
        canvas.set_attribute("data-raw-handle", "1").expect("Can't set data-raw-handle");
        let canvas_rc = Rc::new(canvas);
        let canvas_weak = Rc::downgrade(&canvas_rc);
        let closure = Closure::new(move || {
            match canvas_weak.upgrade() {
                None => { /* deallocated? */ }
                Some(canvas) => {
                    let width = canvas.width();
                    let height = canvas.height();
                    move_closure_box.lock().unwrap().as_ref().map(|closure: &Box<dyn Fn(Size) -> () + Send + 'static>| closure(Size::new(width as f64, height as f64)));
                }
            }
        });
        //I think this is safe??
        window.set_onresize(Some(closure.as_ref().unchecked_ref()));

        document.body().unwrap().append_child(canvas_rc.as_ref()).expect("Can't append canvas to body");
        CanvasHolder { handle: WebWindowHandle::new(1), closure, canvas: canvas_rc, closure_box }
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

fn build_canvas_if_needed() {

}

impl Window {
    pub async fn fullscreen(title: String) -> Result<Self,FullscreenError> {
        let (sender, fut) = r#continue::continuation();
        let sender_mutex = Arc::new(Mutex::new(Some(sender)));
        let sender_mutex_error = sender_mutex.clone();
        let main_thread_job = on_main_thread(move || {
            let strong_closure = Closure::once(move |a| {
                let mut lock = sender_mutex.lock().unwrap().take().expect("already sent?");
                lock.send(Ok(()));
            });
            let error_closure = Closure::once(move |a: JsValue| {
                let mut lock = sender_mutex_error.lock().unwrap().take().expect("already sent?");
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
            drop(promise.then2(&strong_closure,&error_closure));
            CANVAS_HOLDER.replace(Some(canvas));
            SendCell::new((strong_closure, error_closure))
        });
        logwise::warn_sync!("Waiting for main thread...");
        let closures = main_thread_job.await;
        logwise::warn_sync!("Waiting for fut...");
        let fullscreen_result = fut.await;
        //drop our closures
        let main_thread_drop = on_main_thread(move || {
            drop(closures);
        }).await;
        match fullscreen_result {
            Ok(..) => {
                Ok(Window {

                })
            }
            Err(err) => {
                Err(FullscreenError(err))
            }
        }

    }
    pub async fn new(_position: Position, _size: Size, title: String) -> Self {
        let f = on_main_thread(move || {
            let window = window().expect("Can't get window");
            let doc = window.document().expect("Can't get document");
            doc.set_title(&title);
            CANVAS_HOLDER.replace(Some(CanvasHolder::new_main()));
        });
        f.await;
        Window {

        }
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        let sys_surface = on_main_thread(|| {
           let surface = CANVAS_HOLDER.with_borrow_mut(|canvas| {
               let canvas = canvas.as_ref().expect("no canvas");
               Surface {
                   display_handle: canvas.handle,
                   closure_box: canvas.closure_box.clone(),

               }
           });
            surface
        }).await;
        crate::surface::Surface{sys: sys_surface}
    }
    pub async fn default() -> Self {
        Window::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), String::from("app_window")).await
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        todo!("don't drop for now")
    }
}
pub fn is_main_thread() -> bool {
    web_sys::window().is_some()
}
pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    let (sender, mut receiver) = ampsc::channel();

    let mut sent = false;
    MAIN_THREAD_SENDER.get_or_init(|| {
        sent = true;
        sender
    });
    assert!(sent, "Don't call run_main_thread more than once");

    let mut event_id = 0;
    let push_context = Context::current();
    let push_context_2 = push_context.clone();

    let event_loop_context = Context::new_task(Some(Context::current()), "main thread eventloop");
    wasm_bindgen_futures::spawn_local(logwise::context::ApplyContext::new(event_loop_context, async move {
        loop {
            let event = receiver.receive().await.expect("Can't receive event");
            event_id += 1;
            match event {
                MainThreadEvent::Execute(f) => f(),
            }
        }
    }));
    wasm_thread::spawn(|| {
        let new_context = Context::new_task(Some(push_context_2), "app_window after MT context");
        let new_id = new_context.context_id();
        new_context.set_current();
        closure();
        Context::pop(new_id);
    });
}

pub async fn on_main_thread<R: Send + 'static,F: FnOnce() -> R + Send + 'static>(closure: F) -> R {
    if is_main_thread() {
        closure()
    }
    else {
        let (c_sender, c_receiver) = r#continue::continuation();
        let mut mt_sender = MAIN_THREAD_SENDER.get().expect(crate::application::CALL_MAIN).clone();
        let boxed_closure = Box::new(||{
            let r = closure();
            c_sender.send(r);
        }) as Box<dyn FnOnce() -> () + Send + 'static>;
        mt_sender.send(MainThreadEvent::Execute(boxed_closure)).await.expect("Can't schedule on main thread");
        mt_sender.async_drop().await;

        let r = c_receiver.await;

        r
    }
}

pub struct Surface {
    display_handle: WebWindowHandle,
    closure_box: Arc<Mutex<Option<Box<dyn Fn(Size) -> () + Send + 'static>>>>
}
impl Surface {
    pub async fn size(&self) -> Size {
        on_main_thread(|| {
            let w = window().expect("No window?");
            let width = w.inner_width().expect("No width?").as_f64().expect("No width?");
            let height = w.inner_height().expect("No height?").as_f64().expect("No height?");
            Size::new(width, height)
        }).await
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Web(self.display_handle.clone())
    }
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Web(WebDisplayHandle::new())
    }
    /**
    Run the attached callback when size changes.
    */
    pub fn size_update<F: Fn(Size) -> () + Send + 'static>(&mut self, update: F) {
        self.closure_box.lock().unwrap().replace(Box::new(update));
    }
}