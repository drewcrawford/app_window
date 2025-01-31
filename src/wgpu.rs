/*!
This module provides additional features for use with wgpu.
*/
use std::future::Future;

/**
Describes the preferred strategy for interacting with wgpu on this platform.
*/
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
#[cfg(target_os = "windows")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::Relaxed;


#[cfg(target_os = "windows")]
pub fn wgpu_spawn<F: Future<Output=()> + Send + 'static>(f: F) {
    use some_executor::task::Task;
    use some_executor::SomeExecutor;
    //'relaxed' version.  Let's submit to current executor?
    let mut ex = some_executor::current_executor::current_executor();
    let task = Task::without_notifications("wgpu_spawn".into(), f,some_executor::task::Configuration::default());
    use some_executor::observer::Observer;
    ex.spawn_objsafe(task.into_objsafe()).detach();

}

#[cfg(target_os = "linux")]
pub fn wgpu_spawn<F: Future<Output=()> + Send + 'static>(f: F) {
    if crate::sys::is_main_thread() {
        std::thread::Builder::new().name("wgpu_spawn".into()).spawn(|| {
            let mut exec = some_local_executor::Executor::new();
            let task = some_local_executor::Task::without_notifications("wgpu_spawn".into(), f,some_local_executor::Configuration::default());
            use some_local_executor::some_executor::SomeLocalExecutor;
            let o = exec.spawn_local(task);
            std::mem::forget(o);
            exec.drain();
        }).expect("Can't spawn thread");

    }
    else {
        let mut exec = some_local_executor::Executor::new();
        let task = some_local_executor::Task::without_notifications("wgpu_spawn".into(), f,some_local_executor::Configuration::default());
        use some_local_executor::some_executor::SomeLocalExecutor;
        let o = exec.spawn_local(task);
        std::mem::forget(o);
        exec.drain();
    }
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