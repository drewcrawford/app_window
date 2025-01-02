use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::OnceLock;
use logwise::context::Context;
use logwise::debuginternal_sync;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebCanvasWindowHandle, WebDisplayHandle, WebWindowHandle};
use web_sys::{window, HtmlCanvasElement};
use web_sys::wasm_bindgen::JsValue;
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
        wasm_bindgen_futures::spawn_local(f);
        Window {

        }
    }
    pub fn new(_position: Position, _size: Size, title: String) -> Self {
        let f = on_main_thread(move || {
            let window = window().expect("Can't get window");
            let doc = window.document().expect("Can't get document");
            doc.set_title(&title);
        });
        wasm_bindgen_futures::spawn_local(f);
        Window {

        }
    }

    pub async fn surface(&self) -> crate::surface::Surface {
        use web_sys::wasm_bindgen::__rt::IntoJsResult;
        let a = on_main_thread(move || {
            logwise::warn_sync!("in thread 1");
            CANVAS_ELEMENT.with_borrow_mut(|thread_canvas| {

                match thread_canvas {
                    None => {
                        let window = window().expect("Can't get window");
                        let document = window.document().expect("Can't get document");

                        let element = document.create_element("canvas").expect("Can't create canvas");

                        let canvas = web_sys::HtmlCanvasElement::from(element.into_js_result().expect("Can't get canvas"));
                        document.body().unwrap().append_child(canvas.as_ref()).expect("Can't append canvas to body");
                        canvas.set_attribute("data-raw-handle", "1").expect("Can't set data-raw-handle");
                        *thread_canvas = Some(canvas);
                    }
                    Some(canvas) => ()
                }
            });
            let display_handle = WebWindowHandle::new(1);
            logwise::warn_sync!("in thread");
            crate::surface::Surface {
                sys: Surface {
                    display_handle,
                }
            }
        }).await;
        logwise::warn_sync!("back to calling thread");
        a
    }
    pub fn default() -> Self {
        Window::new(Position::new(0.0, 0.0), Size::new(800.0, 600.0), String::from("app_window"))
    }
}
pub fn is_main_thread() -> bool {
    web_sys::window().is_some()
}
pub fn run_main_thread<F: FnOnce() -> () + Send + 'static>(closure: F) {
    let (sender, mut receiver) = ampsc::channel();

    let mut sent = false;
    let main_thread_sender = MAIN_THREAD_SENDER.get_or_init(|| {
        sent = true;
        sender
    });
    assert!(sent, "Don't call run_main_thread more than once");

    let mut event_id = 0;
    wasm_bindgen_futures::spawn_local(async move {
        Context::new_task(None, "main thread eventloop").set_current();
        loop {
            logwise::warn_sync!("Waiting for main event loop...");
            let event = receiver.receive().await.expect("Can't receive event");
            logwise::warn_sync!("Got event {event_id}",event_id=event_id);
            event_id += 1;
            match event {
                MainThreadEvent::Execute(f) => f(),
            }
            logwise::warn_sync!("main loop finished execution");
        }
    });
    let push_context = Context::current();
    wasm_thread::spawn(|| {
        let new_context = Context::new_task(Some(push_context), "app_window after MT context");
        logwise::warn_sync!("app_window after MT context");
        let new_id = new_context.context_id();
        new_context.set_current();
        closure();
        Context::pop(new_id);
    });
}

pub async fn on_main_thread<R: Send + 'static,F: FnOnce() -> R + Send + 'static>(closure: F) -> R {
    if is_main_thread() {
        logwise::warn_sync!("on_main_thread: inline");
        closure()
    }
    else {
        logwise::warn_sync!("on_main_thread: outline");
        let (c_sender, c_receiver) = r#continue::continuation();
        let mut mt_sender = MAIN_THREAD_SENDER.get().expect(crate::application::CALL_MAIN).clone();
        let boxed_closure = Box::new(||{
            logwise::warn_sync!("Will run closure");
            let r = closure();
            logwise::warn_sync!("Will send c_sender");
            c_sender.send(r);
            logwise::warn_sync!("Did send c_sender");
        }) as Box<dyn FnOnce() -> () + Send + 'static>;
        logwise::warn_sync!("Will send mte");
        mt_sender.send(MainThreadEvent::Execute(boxed_closure)).await.expect("Can't schedule on main thread");
        logwise::warn_sync!("will run async_drop");
        mt_sender.async_drop().await;
        logwise::warn_sync!("did run async_drop");

        logwise::warn_sync!("Did send mte");

        let r = c_receiver.await;

        logwise::warn_sync!("Did receive mte");
        r
    }
}

pub struct Surface {
    display_handle: WebWindowHandle,
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
        todo!()
    }
}