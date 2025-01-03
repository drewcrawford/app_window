use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};
use logwise::context::Context;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebDisplayHandle, WebWindowHandle};
use wasm_bindgen::closure::{Closure};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::js_sys::Function;
use web_sys::{window, HtmlCanvasElement};
use crate::coordinates::{Position, Size};

pub struct Window {

}

thread_local! {
    static CANVAS_ELEMENT: RefCell<Option<HtmlCanvasElement>> = RefCell::new(None);
}

enum MainThreadEvent {
    Execute(Box<dyn FnOnce() + Send + 'static>)
}

static MAIN_THREAD_SENDER: OnceLock<ampsc::ChannelProducer<MainThreadEvent>> = OnceLock::new();


impl Window {
    pub fn fullscreen(title: String) -> Self {
         let f = on_main_thread(move || {
            let window = window().expect("Can't get window");
            let doc = window.document().expect("Can't get document");
            doc.set_title(&title);
        });
        wasm_bindgen_futures::spawn_local(logwise::context::ApplyContext::new(Context::current(), f));
        Window {

        }
    }
    pub fn new(_position: Position, _size: Size, title: String) -> Self {
        let f = on_main_thread(move || {
            let window = window().expect("Can't get window");
            let doc = window.document().expect("Can't get document");
            doc.set_title(&title);
        });
        wasm_bindgen_futures::spawn_local(logwise::context::ApplyContext::new(Context::current(), f));
        Window {

        }
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        use web_sys::wasm_bindgen::__rt::IntoJsResult;
        let closure_box = Arc::new(Mutex::new(None));
        let move_closure_box = closure_box.clone();
        let closure = Closure::new(move || {
            CANVAS_ELEMENT.with_borrow(|c| {

                match c {
                    Some(canvas) => {
                        let width = canvas.width();
                        let height = canvas.height();
                        move_closure_box.lock().unwrap().as_ref().map(|closure: &Box<dyn Fn(Size) -> () + Send + 'static>| closure(Size::new(width as f64, height as f64)));
                    }
                    None => {
                        //no canvas element?
                    }
                }
            });
        });
        struct SendMe(*const Function);
        unsafe impl Send for SendMe {}
        let closure_ref = SendMe(closure.as_ref().unchecked_ref());
        let display_handle = on_main_thread(move || {
            let closure_ref = closure_ref;
            CANVAS_ELEMENT.with_borrow_mut(|thread_canvas| {
                match thread_canvas {
                    None => {
                        let window = window().expect("Can't get window");
                        //I think this is safe??
                        window.set_onresize(Some(unsafe{&*closure_ref.0}));
                        let document = window.document().expect("Can't get document");

                        let element = document.create_element("canvas").expect("Can't create canvas");
                        let html_element = web_sys::HtmlElement::from(element.into_js_result().expect("Can't create html element"));

                        let style = html_element.style();
                        style.set_property("width","100vw").expect("Can't set width");
                        style.set_property("height","100vh").expect("Can't set height");


                        let canvas = web_sys::HtmlCanvasElement::from(html_element.into_js_result().expect("Can't get canvas"));
                        document.body().unwrap().append_child(canvas.as_ref()).expect("Can't append canvas to body");
                        canvas.set_attribute("data-raw-handle", "1").expect("Can't set data-raw-handle");
                        *thread_canvas = Some(canvas);
                    }
                    Some(..) => ()
                }
            });
            WebWindowHandle::new(1)


        }).await;
        crate::surface::Surface {
            sys: Surface {
                display_handle,
                _resize_closure: closure,
                closure_box,
            }
        }

    }
    pub fn default() -> Self {
        Window::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), String::from("app_window"))
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
    _resize_closure: Closure<dyn FnMut()>,
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