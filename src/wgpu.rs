use std::future::Future;


#[non_exhaustive] pub enum WGPUStrategy {
    MainThread,
    NotMainThread,
    Relaxed,
}

#[cfg(target_os = "linux")]
pub const WGPU_STRATEGY: WGPUStrategy = WGPUStrategy::NotMainThread;

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