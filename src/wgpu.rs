//SPDX-License-Identifier: MPL-2.0

/*!
This module provides additional features for use with wgpu.

# Design notes
On wgpu, there is a dispute about whether or not gpu can be accessed from the background thread.
See https://caniuse.com/mdn-api_offscreencanvas_getcontext_webgpu_context.  Currently
we take the view that it can only be accessed from the main thread for widest browser compatability,
but this may change.
*/
use std::future::Future;
use crate::sys;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
#[derive(Debug,Copy,Clone,PartialEq,Eq,Hash)]
#[non_exhaustive] pub enum WGPUStrategy {
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
#[cfg(any(target_arch="wasm32",target_os = "macos"))]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::MainThread;


/**
Spawns a future onto an executor suitable for wgpu.

This function will panic if not executed on the main thread.

The details of this vary per platform, as does the platform signature.  On most platforms we require
that the future be Send and 'static, but on MainThread platforms we do not.
*/
#[cfg(target_os = "windows")]
pub fn wgpu_spawn<F: Future<Output=()> + Send + 'static>(f: F) {
    assert!(sys::is_main_thread(), "call wgpu_spawn from the main thread");
    use some_executor::task::Task;
    use some_executor::SomeExecutor;
    //'relaxed' version.  Let's submit to current executor?
    let mut ex = some_executor::current_executor::current_executor();
    let task = Task::without_notifications("wgpu_spawn".into(), f,some_executor::task::Configuration::default());
    use some_executor::observer::Observer;
    ex.spawn_objsafe(task.into_objsafe()).detach();
}

/**
Spawns a future onto an executor suitable for wgpu.

This function will panic if not executed on the main thread.

The details of this vary per platform, as does the platform signature.  On most platforms we require
that the future be Send and 'static, but on MainThread platforms we do not.
*/
#[cfg(any(target_arch="wasm32",target_os = "macos"))]
pub fn wgpu_spawn<F: Future<Output=()> + 'static>(f: F) {
    //MainThread implementation
    assert!(sys::is_main_thread(), "call wgpu_spawn from the main thread");
    crate::executor::already_on_main_thread_submit(f);
}

/**
Spawns a future onto an executor suitable for wgpu.

This function will panic if not executed on the main thread.

The details of this vary per platform, as does the platform signature.  On most platforms we require
that the future be Send and 'static, but on MainThread platforms we do not.
*/
#[cfg(target_os = "linux")]
pub fn wgpu_spawn<F: Future<Output=()> + Send + 'static>(f: F) {
    assert!(sys::is_main_thread(), "call wgpu_spawn from the main thread");
    std::thread::Builder::new().name("wgpu_spawn".into()).spawn(|| {
        let mut exec = some_local_executor::Executor::new();
        let task = some_local_executor::Task::without_notifications("wgpu_spawn".into(), f, some_local_executor::Configuration::default());
        use some_local_executor::some_executor::SomeLocalExecutor;
        let o = exec.spawn_local(task);
        std::mem::forget(o);
        exec.drain();
    }).expect("Can't spawn thread");

}

#[cfg(test)] mod tests {
    #[cfg(target_os = "windows")] //list 'relaxed' platforms here
    #[test] fn test_relaxed() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<wgpu::Instance>();
        assert_sync::<wgpu::Instance>();
        assert_send::<wgpu::Surface>();
        assert_sync::<wgpu::Surface>();
        assert_send::<wgpu::Adapter>();
        assert_sync::<wgpu::Adapter>();
    }
}