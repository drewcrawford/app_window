//SPDX-License-Identifier: MPL-2.0

/*!
This module provides additional features for use with wgpu.

# Design notes
On wgpu, there is a dispute about whether or not gpu can be accessed from the background thread.
See https://caniuse.com/mdn-api_offscreencanvas_getcontext_webgpu_context.  Currently
we take the view that it can only be accessed from the main thread for widest browser compatability,
but this may change.
*/
use crate::executor::already_on_main_thread_submit;
use crate::sys;
use some_executor::observer::Observer;
use some_executor::task::{Configuration, Task};
use std::future::Future;

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
pub fn wgpu_begin_context<F>(f: F)
where
    F: Future<Output=()> + Send + 'static,
{
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
                std::thread::Builder::new()
                    .name("wgpu_begin_context".to_string())
                    .spawn(|| {
                       some_executor::thread_executor::thread_local_executor(|e| {
                           let t = Task::without_notifications("wgpu_begin_context".to_string(), Configuration::default(), f);
                           let t_objsafe = t.into_objsafe_local();
                           let observer = e.spawn_local_objsafe(t_objsafe);
                           observer.detach();
                       })
                    }).expect("Failed to spawn wgpu_begin_context thread");
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

pub fn wgpu_in_context<F>(f: F) where F: Future<Output=()> + 'static {
    match WGPU_STRATEGY {
        WGPUStrategy::MainThread => {
            //we need to intermix with the main thread executor.
            if sys::is_main_thread() {
                already_on_main_thread_submit(f);
            } else {
                panic!("wgpu_in_context called outside the context");
            }
        },
        WGPUStrategy::NotMainThread => {
            if sys::is_main_thread() {
                panic!("wgpu_in_context called outside the context");
            } else {
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications("wgpu_in_context".to_string(), Configuration::default(), f);
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                })
            }
        },
        WGPUStrategy::Relaxed => {
            if sys::is_main_thread() {
                //prefer our executor
                already_on_main_thread_submit(f);
            } else {
                //fall back to the thread executor
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications("wgpu_in_context".to_string(), Configuration::default(), f);
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
