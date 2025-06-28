//SPDX-License-Identifier: MPL-2.0

/*!
This module provides additional features for use with wgpu.

# Design notes
On wgpu, there is a dispute about whether or not gpu can be accessed from the background thread.
See https://caniuse.com/mdn-api_offscreencanvas_getcontext_webgpu_context.  Currently
we take the view that it can only be accessed from the main thread for widest browser compatability,
but this may change.
*/
use crate::executor::{already_on_main_thread_submit, on_main_thread_async};
use crate::sys;
use some_executor::SomeExecutor;
use some_executor::observer::{FinishedObservation, Observer};
use some_executor::task::{Configuration, Task};
use std::future::Future;
use some_local_executor::some_executor::SomeLocalExecutor;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WGPUStrategy {
    /**
        The main thread should be used to access wgpu.
    */
    MainThread,
    /**
        The main thread should **NOT be used to access wgpu.
    */
    NotMainThread,
    /**
        On this platform, wgpu types are sendable and can be used from any thread.

        Platforms with this type should use test_relaxed to verify
    */
    Relaxed,
}
/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(target_os = "linux")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(any(target_os = "windows"))]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(any(target_arch = "wasm32", target_os = "macos"))]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub struct NotDirect<F>(F);

impl<F> std::fmt::Display for NotDirect<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "This future cannot be called directly on the current thread. \
             Use `wgpu_call_context` to ensure it is executed in a suitable context."
        )
    }
}

/**
Attempts to call the future directly, returning an error
if the current thread is not suitable for direct wgpu calls.

This is an alternative to [wgpu_call_context] that does not
require the future to be `Send` or `'static`. It is useful when you want to ensure that the future
is executed on the correct thread for wgpu operations.
*/
pub async fn wgpu_call_direct_or_err<F, R>(f: F) -> Result<R, NotDirect<F>>
where
    F: Future<Output = R>,
{
    match WGPU_STRATEGY {
        WGPUStrategy::MainThread => {
            // MainThread strategy; if we're on the main thread, we can just call the future directly.
            if sys::is_main_thread() {
                Ok(f.await)
            } else {
                Err(NotDirect(f))
            }
        }
        WGPUStrategy::NotMainThread => {
            if sys::is_main_thread() {
                Err(NotDirect(f))
            } else {
                // NotMainThread strategy; we can call the future directly.
                Ok(f.await)
            }
        }
        WGPUStrategy::Relaxed => {
            //we can call the future directly.
            Ok(f.await)
        }
    }
}

/**
Calls the future in a context suitable for wgpu operations.
This function is designed to ensure that the future is executed in a context that is compatible with wgpu operations
on the current platform. It handles the differences in threading models across platforms.

# Arguments
- `f`: The future to be executed. It must implement `Future<Output = R>`, where `R` is the expected output type.
# Returns
- `R`: The result of the future execution.

# See also
- [`wgpu_call_direct_or_err`] for a version that does not require the future to be `Send` or `'static`.
*/
pub async fn wgpu_call_context<F, R>(f: F) -> R
where
    F: Future<Output = R> + Send + 'static,
    R: Send + Unpin + 'static,
{
    match wgpu_call_direct_or_err(f).await {
        Ok(result) => result,
        Err(f) => {
            match WGPU_STRATEGY {
                WGPUStrategy::MainThread => on_main_thread_async(f.0).await,
                WGPUStrategy::NotMainThread => {
                    //run on the executor pool
                    let mut exec = some_executor::current_executor::current_executor();
                    let task = Task::without_notifications(
                        "wgpu_call_context".into(),
                        some_executor::task::Configuration::default(),
                        f.0,
                    );
                    let observer = exec.spawn_objsafe(task.into_objsafe());
                    let pinned_fut = Box::pin(observer);
                    let result = pinned_fut.await;
                    match result {
                        FinishedObservation::Cancelled => todo!(),
                        FinishedObservation::Ready(r) => {
                            let a = r
                                .downcast::<R>()
                                .expect("wgpu_call_context: expected result to be of type R");
                            *a
                        }
                    }
                }
                WGPUStrategy::Relaxed => {
                    unreachable!()
                }
            }
        }
    }
}

/**
A function that can be used to call a future in a relaxed context.

On this platform, we require that the future is `Send` and `'static`.
However on other platforms we may not require this.

On this platform, the function can be called from any thread.
However on other platforms, it can only be called on the main thread.
*/
#[cfg(not(target_arch = "wasm32"))]
pub async fn wgpu_call_context_relaxed<F, R>(f: F) -> R
where
    F: Future<Output = R> + Send + 'static,
    R: Send + Unpin + 'static,
{
    wgpu_call_context(f).await
}

/**
A function that can be used to call a future in a relaxed context.

On this platform, we do not require that the future is `Send` or `'static`.
However on other platforms we may require this.

On this platform, this function can only be called on the main thread.
However, on other platforms, it can be called from any thread.
*/
#[cfg(target_arch = "wasm32")]
pub fn wgpu_call_context_relaxed<F, R>(f: F) -> impl Future<Output=R> + Send
where
    F: Future<Output = R>,
    R: Unpin + 'static,
{
    if !sys::is_main_thread() {
        panic!("wgpu_call_context_relaxed can only be called on the main thread on wasm32");
    }
    let cell = send_cells::send_cell::SendCell::new(f);
    cell.into_future()
}

/**
Begins a context for wgpu operations.

The behavior of this function depends on the platform's wgpu strategy:
* `WGPUStrategy::MainThread`: Executes the future on the main thread via app_window's main thread executor.
* `WGPUStrategy::NotMainThread`: If we're not on the main thread, use the thread executor or panic.  If we're on the main thread, spin up a new thread with a local executor.
* `WGPUStrategy::Relaxed`: If we're on the main thread, use the main thread executor.  If we're not on the main thread, use the thread executor or panic.

*/
fn wgpu_begin_context<F>(f: F)
where
    F: Future<Output=()> + Send + 'static,
{
    if some_executor::thread_executor::thread_executor(|e| e.is_none()) {
        logwise::warn_sync!("wgpu_begin_context called without a thread executor.  On some platforms this may panic.")
    }
    match WGPU_STRATEGY {
        WGPUStrategy::MainThread => {
            if sys::is_main_thread() {
                // If we're on the main thread, we can just call the future directly.
                already_on_main_thread_submit(f);
            } else {
                // If we're not on the main thread, we need to run it on the main thread executor.
                sys::on_main_thread(|| {
                    already_on_main_thread_submit(f);
                })
            }
        },
        WGPUStrategy::NotMainThread => {
            if sys::is_main_thread() {
                let t = std::thread::Builder::new()
                    .name("wgpu_begin_context".to_string())
                    .spawn(|| {
                       some_executor::thread_executor::thread_local_executor(|e| {
                           let t = Task::without_notifications("wgpu_begin_context".to_string(), Configuration::default(), f);
                           let t_objsafe = t.into_objsafe_local();
                           let observer = e.spawn_local_objsafe(t_objsafe);
                           observer.detach();
                       })
                    });
            }
            else {
                //dispatch onto current thread executor
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications("wgpu_begin_context".to_string(), Configuration::default(), f);
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                })
            }
        },
        WGPUStrategy::Relaxed => {
            if sys::is_main_thread() {
                // If we're on the main thread, we can just call the future directly.
                already_on_main_thread_submit(f);
            } else {
                // If we're not on the main thread, we need to run it on the thread executor.
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications("wgpu_begin_context".to_string(), Configuration::default(), f);
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                });
            }
        },
    }
}


#[cfg(test)]
mod tests {
    use crate::wgpu::wgpu_call_context;

    #[cfg(target_os = "windows")] //list 'relaxed' platforms here
    #[test]
    fn test_relaxed() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<wgpu::Instance>();
        assert_sync::<wgpu::Instance>();
        assert_send::<wgpu::Surface>();
        assert_sync::<wgpu::Surface>();
        assert_send::<wgpu::Adapter>();
        assert_sync::<wgpu::Adapter>();
    }

    #[test]
    fn test_wgpu_call_context_is_send() {
        fn assert_send<T: Send>(t: T) {}
        let f = wgpu_call_context(async {});
        assert_send(f);
    }
}
