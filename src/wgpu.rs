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

pub mod thread_cell;

pub use thread_cell::{WgpuCell, WgpuFuture};

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
#[cfg(target_os = "windows")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[cfg(any(target_arch = "wasm32", target_os = "macos"))]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;

/**
Begins a context for wgpu operations.

# Context

This function begins a wgpu execution context, allowing you to run futures that interact with wgpu.

The type of context depends on the platform's wgpu strategy, which is defined by the `WGPU_STRATEGY` constant.

* `WGPUStrategy::MainThread`: Executes the future on the main thread via app_window's main thread executor.
* `WGPUStrategy::NotMainThread`: If we're not on the main thread, use [some_executor::thread_executor].  If we're on the main thread,
   spin up a new thread with a local executor.
* `WGPUStrategy::Relaxed`: If we're on the main thread, use the main thread executor.
   If we're not on the main thread, use the thread executor.

# A brief digression on Sendability

In Rust the `Send` trait indicates that a type can be transferred between threads. For a Future,
this means the future can arbitrarily be sent between polls (so you can wake up on a different
thread every time).

Meanwhile, GPU backends often require you to call their APIs "in context".  This is typically,
though not always, from a certain thread.  If so, GPU types tend to be modeled as !Send, complicating
their use in async code.  At the same time, you need Send to get into the "right context" if that
context is another thread.

Usually what we want to model is "you can Send until the future starts running, and not after that",
which is a bit complex to express in Rust.  How we do it is:

* [`wgpu_begin_context`]: Sets up the context (possibly a thread) and runs a Send future in it.
* [`wgpu_in_context`]`: Uses a previously established context to run a future that is not Send.
*/
pub fn wgpu_begin_context<F>(f: F)
where
    F: Future<Output = ()> + Send + 'static,
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
        }
        WGPUStrategy::NotMainThread => {
            if sys::is_main_thread() {
                std::thread::Builder::new()
                    .name("wgpu_begin_context".to_string())
                    .spawn(|| {
                        some_executor::thread_executor::thread_local_executor(|e| {
                            let t = Task::without_notifications(
                                "wgpu_begin_context".to_string(),
                                Configuration::default(),
                                f,
                            );
                            let t_objsafe = t.into_objsafe_local();
                            let observer = e.spawn_local_objsafe(t_objsafe);
                            observer.detach();
                        })
                    })
                    .expect("Failed to spawn wgpu_begin_context thread");
            } else {
                //dispatch onto current thread executor
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications(
                        "wgpu_begin_context".to_string(),
                        Configuration::default(),
                        f,
                    );
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                })
            }
        }
        WGPUStrategy::Relaxed => {
            if sys::is_main_thread() {
                // If we're on the main thread, we can just call the future directly.
                already_on_main_thread_submit(f);
            } else {
                // If we're not on the main thread, we need to run it on the thread executor.
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications(
                        "wgpu_begin_context".to_string(),
                        Configuration::default(),
                        f,
                    );
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                });
            }
        }
    }
}

/**
Executes a non-Send future in the current wgpu context.

# A brief digression on Sendability

In Rust the `Send` trait indicates that a type can be transferred between threads. For a Future,
this means the future can arbitrarily be sent between polls (so you can wake up on a different
thread every time).

Meanwhile, GPU backends often require you to call their APIs "in context".  This is typically,
though not always, from a certain thread.  If so, GPU types tend to be modeled as !Send, complicating
their use in async code.  At the same time, you need Send to get into the "right context" if that
context is another thread.

Usually what we want to model is "you can Send until the future starts running, and not after that",
which is a bit complex to express in Rust.  How we do it is:

* [`wgpu_begin_context`]: Sets up the context (possibly a thread) and runs a Send future in it.
* [`wgpu_in_context`]`: Uses a previously established context to run a future that is not Send.
*/
pub fn wgpu_in_context<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    match WGPU_STRATEGY {
        WGPUStrategy::MainThread => {
            //we need to intermix with the main thread executor.
            if sys::is_main_thread() {
                already_on_main_thread_submit(f);
            } else {
                panic!("wgpu_in_context called outside the context");
            }
        }
        WGPUStrategy::NotMainThread => {
            if sys::is_main_thread() {
                panic!("wgpu_in_context called outside the context");
            } else {
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications(
                        "wgpu_in_context".to_string(),
                        Configuration::default(),
                        f,
                    );
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                })
            }
        }
        WGPUStrategy::Relaxed => {
            if sys::is_main_thread() {
                //prefer our executor
                already_on_main_thread_submit(f);
            } else {
                //fall back to the thread executor
                some_executor::thread_executor::thread_local_executor(|e| {
                    let t = Task::without_notifications(
                        "wgpu_in_context".to_string(),
                        Configuration::default(),
                        f,
                    );
                    let t_objsafe = t.into_objsafe_local();
                    let observer = e.spawn_local_objsafe(t_objsafe);
                    observer.detach();
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {}
