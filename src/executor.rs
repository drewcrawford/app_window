//SPDX-License-Identifier: MPL-2.0

/*!
A mini-executor for running futures on the main thread.

This module provides an executor specifically designed to run futures on the application's
main thread while cooperating with the native event loop. This is essential for platforms
that require certain operations (like UI updates) to happen on the main thread.

The executor uses a task queue that's processed whenever the main thread's event loop
allows it, ensuring that futures can yield control back to the native event loop for
smooth operation.

# Thread Safety

All futures submitted to this executor will run on the main thread. The executor
provides two main entry points:
- [`on_main_thread_async`](crate::executor::on_main_thread_async): Can be called from any thread to run a future on the main thread
- [`already_on_main_thread_submit`](crate::executor::already_on_main_thread_submit): Must be called from the main thread

# Integration with `some_executor`

When the `some_executor` feature is enabled, this executor can be wrapped with
`crate::some_executor::MainThreadExecutor` to provide a `some_executor::SomeExecutor`
implementation.
*/
use crate::sys;
use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable};
use logwise::perfwarn_begin;
use crate::application::submit_to_main_thread;

/// Static counter for generating unique task IDs.
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

/// Internal state shared between a task and its waker.
///
/// This struct tracks the task ID for waker operations.
struct Inner {
    task_id: usize,
}

impl Inner {
    fn new(task_id: usize) -> Self {
        Inner { task_id }
    }
}

/// A waker implementation that integrates with the main thread executor.
///
/// When woken, this sets a flag indicating the associated task needs polling
/// and submits a request to pump the task queue on the main thread.
struct Waker {
    inner: Arc<Inner>,
}

/// Virtual function table for our custom waker implementation.
///
/// This defines how to clone, wake, wake_by_ref, and drop our waker type.
const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| {
        let w = unsafe { Arc::from_raw(data as *const Waker) };
        let w2 = w.clone();
        _ = Arc::into_raw(w); //leave original arc unchanged
        RawWaker::new(Arc::into_raw(w2) as *const (), &WAKER_VTABLE)
    },
    |data| {
        let w = unsafe { Arc::from_raw(data as *const Waker) };
        wake_task(w.inner.task_id);
    },
    |data| {
        let w = unsafe { Arc::from_raw(data as *const Waker) };
        wake_task(w.inner.task_id);
        std::mem::forget(w);
    },
    |data| {
        let w = unsafe { Arc::from_raw(data as *const Waker) };
        drop(w);
    },
);
impl Waker {
    fn into_waker(self) -> std::task::Waker {
        let arc_waker = Arc::into_raw(Arc::new(self));
        unsafe { std::task::Waker::from_raw(RawWaker::new(arc_waker as *const (), &WAKER_VTABLE)) }
    }
}
/// A task in the executor's queue.
///
/// Each task contains a pinned future, unique ID, and shared state for wake notifications.
struct Task {
    id: usize,
    future: Pin<Box<dyn Future<Output = ()> + 'static>>,
    wake_inner: Arc<Inner>,
}

/// Wakes a task by moving it from RUNNING to POLLABLE and scheduling executor iteration.
///
/// This function handles the wake notification for a specific task ID.
fn wake_task(task_id: usize) {
    // Schedule main executor iteration on the main thread
    crate::application::submit_to_main_thread(move || {
        // Add the task to the pollable queue
        let mut pollable = POLLABLE.take();
        pollable.push(task_id);
        POLLABLE.replace(pollable);
        main_executor_iter();
    });
}

thread_local! {
    // Thread-local storage for tasks that are running but not currently pollable.
    static RUNNING: Cell<Option<HashMap<usize, Task>>> = const { Cell::new(None) };
    // Thread-local storage for task IDs that are ready to be polled.
    static POLLABLE: Cell<Vec<usize>> = const { Cell::new(Vec::new()) };
}

/// Runs the specified future on the main thread and returns its result.
///
/// This function can be called from any thread. It submits the given future to the
/// main thread executor and waits for its completion. While the future is executing,
/// the main thread can still process other events, allowing for cooperative multitasking.
///
/// # Type Parameters
///
/// * `R` - The return type of the future, must be `Send + 'static`
/// * `F` - The future type, must be `Send + 'static` and return `R`
///
/// # Examples
///
/// ```
/// # use std::future::Future;
/// # fn test() -> impl Future<Output = ()> {
/// # async {
/// // Call from any thread to compute on the main thread
/// let result = app_window::executor::on_main_thread_async(async {
///     // This code runs on the main thread
///     // Perform computation that needs main thread access
///     2 + 2
/// }).await;
///
/// assert_eq!(result, 4);
/// # }
/// # }
/// ```
///
/// # Platform Behavior
///
/// On all supported platforms, this ensures the future runs on the thread that owns
/// the native event loop, which is required for UI operations.
pub async fn on_main_thread_async<R: Send + 'static, F: Future<Output = R> + Send + 'static>(
    future: F,
) -> R {
    let (sender, fut) = r#continue::continuation();
    crate::application::submit_to_main_thread(|| {
        already_on_main_thread_submit(async move {
            let r = future.await;
            sender.send(r);
        })
    });
    fut.await
}

/// Submits a future to the main thread executor for execution.
///
/// This function must be called from the main thread. It adds the future to the
/// executor's task queue where it will be polled during the main event loop.
///
/// # Panics
///
/// This function will panic if not called from the main thread.
///
/// # Examples
///
/// ```no_run
/// # use std::future::Future;
/// # fn setup_main_thread() {
/// // This code must run on the main thread
/// app_window::executor::already_on_main_thread_submit(async {
///     println!("Running async task on main thread");
///     // Perform async operations that yield to the event loop
/// });
/// # }
/// ```
///
/// # Use Case
///
/// This is primarily used internally by the `crate::some_executor::MainThreadExecutor`
/// (when the `some_executor` feature is enabled) when spawning tasks, but can be used
/// directly when you're already on the main thread and want to submit work to be
/// executed asynchronously.
pub fn already_on_main_thread_submit<F: Future<Output = ()> + 'static>(future: F) {
    assert!(sys::is_main_thread());

    // Generate unique task ID
    let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);

    // Create task with unique ID
    let wake_inner = Arc::new(Inner::new(task_id));
    let task = Task {
        id: task_id,
        future: Box::pin(future),
        wake_inner,
    };

    // Add task to POLLABLE queue
    let mut pollable = POLLABLE.take();
    logwise::info_sync!("Submitting task {id} to main executor", id=task_id);
    pollable.push(task_id);
    POLLABLE.replace(pollable);

    // Add task to RUNNING collection
    let mut running = RUNNING.take().unwrap_or_default();
    running.insert(task_id, task);
    RUNNING.replace(Some(running));

    // Execute the tasks
    main_executor_iter();
}

/// Polls all tasks that need attention.
///
/// This function loops while there are pollable tasks, handling new tasks
/// that may be added during polling without losing them.
fn main_executor_iter() {
    // Pop off a pollable task
    let iter = perfwarn_begin!("main_executor_iter");
    let mut swap_pollable = POLLABLE.take();
    let poll = swap_pollable.pop();
    POLLABLE.replace(swap_pollable);
    match poll {
        None => {
            //No more pollable tasks, nothing to do.
        }
        Some(task) => {
            // Get the task from RUNNING
            let mut running = RUNNING.take().unwrap_or_default();
            let mut task = running.remove(&task).unwrap();
            RUNNING.replace(Some(running));

            //with that out of the way, we can poll the task
            let waker = Waker {
                inner: task.wake_inner.clone(),
            };
            let into_waker = waker.into_waker();
            let parent = logwise::context::Context::current();
            let new_context = logwise::context::Context::new_task(Some(parent), "main_executor_iter");
            let new_id = new_context.context_id();
            new_context.set_current();
            logwise::info_sync!("Polling task {id}",id=task.id);
            let mut context = Context::from_waker(&into_waker);
            let poll_result = task.future.as_mut().poll(&mut context);
            logwise::context::Context::pop(new_id);
            match poll_result {
                std::task::Poll::Ready(()) => {
                    // Task completed, don't put it back
                }
                std::task::Poll::Pending => {
                    // Task is still running, put it back in RUNNING
                    let mut running = RUNNING.take().unwrap_or_default();
                    running.insert(task.id, task);
                    RUNNING.replace(Some(running));
                }
            }
            //there MAY be more pollable tasks.  However, we want to yield here
            submit_to_main_thread(main_executor_iter);
        }
    }
    drop(iter);
}
