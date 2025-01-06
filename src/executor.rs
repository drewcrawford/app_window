/*!a mini-executor for the main thread
*/
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable};
use crate::application::on_main_thread;
use crate::sys;

struct Inner {
    needs_poll: AtomicBool,
}

impl Inner {
    fn new() -> Self {
        Inner {
            needs_poll: AtomicBool::new(true),
        }
    }
}

struct Waker {
    inner: Arc<Inner>,
}



const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| {
        let w = unsafe{Arc::from_raw(data as *const Waker)};
        let w2 = w.clone();
        Arc::into_raw(w); //leave original arc unchanged
        RawWaker::new(Arc::into_raw(w2) as *const (), &WAKER_VTABLE)

    },
    |data| {
        let w = unsafe{Arc::from_raw(data as *const Waker)};
        w.inner.needs_poll.store(true, Ordering::Relaxed);
        pump_tasks();
    },
    |data| {
        let w = unsafe{Arc::from_raw(data as *const Waker)};
        w.inner.needs_poll.store(true, Ordering::Relaxed);
        pump_tasks();
        std::mem::forget(w);

    },
    |data| {
        let w = unsafe{Arc::from_raw(data as *const Waker)};
        drop(w);
    }
);
impl Waker {
    fn into_waker(self) -> std::task::Waker {
        let arc_waker = Arc::into_raw(Arc::new(self));
        unsafe {
            std::task::Waker::from_raw(RawWaker::new(arc_waker as *const (), &WAKER_VTABLE))
        }
    }
}
struct Task {
    future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    wake_inner: Arc<Inner>,
}
thread_local! {
    static FUTURES: Cell<Vec<Task> >= Cell::new(Vec::new());
}

/**
Runs the specified future on the main thread.

While the future is yielding, other events can be processed.
*/
pub async fn on_main_thread_async<R: Send + 'static, F: Future<Output=R> + Send + 'static>(future: F) -> R {
    let (sender, fut) = r#continue::continuation();
    on_main_thread_async_submit(async move {
        let r = future.await;
        sender.send(r);
    });
    fut.await
}

pub(crate) fn on_main_thread_async_submit<F: Future<Output=()> + 'static>(future: F) {
    crate::sys::on_main_thread(|| {
        on_main_thread_submit(future)
    });
}
pub(crate) fn on_main_thread_submit<F: Future<Output=()> + 'static>(future: F) {
    assert!(sys::is_main_thread());
    let mut tasks = FUTURES.take();
    let wake_inner = Arc::new(Inner::new());
    let task = Task {
        future: Box::pin(future),
        wake_inner,
    };
    tasks.push(task);
    main_executor_iter(&mut tasks);
    FUTURES.replace(tasks);
}

fn main_executor_iter(tasks: &mut Vec<Task>) {
    tasks.retain_mut(|task| {
        let old = task.wake_inner.needs_poll.swap(false, Ordering::Relaxed);
        if old {
            let waker = Waker{inner: task.wake_inner.clone()};
            let into_waker = waker.into_waker();
            let mut context = Context::from_waker(&into_waker);
            let poll = task.future.as_mut().poll(&mut context);
            if let std::task::Poll::Ready(()) = poll {
                return false;
            }
        }
        true
    });
}

fn pump_tasks() {
    crate::sys::on_main_thread(|| {
        let mut tasks = FUTURES.take();
        main_executor_iter(&mut tasks);
        FUTURES.replace(tasks);
    });
}