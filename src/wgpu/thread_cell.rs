//SPDX-License-Identifier: MPL-2.0

use send_cells::UnsafeSendCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};
use send_cells::unsafe_sync_cell::UnsafeSyncCell;
use crate::application::on_main_thread;
use crate::sys;
use crate::wgpu::{WGPU_STRATEGY, WGPUStrategy, wgpu_begin_context};

#[derive(Debug)]
struct Shared<T: 'static> {
    inner: Option<UnsafeSendCell<UnsafeSyncCell<T>>>,
    mutex: Mutex<()>,
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        //when we're dropping the last value,
        //we need to do so on the right thread
        let take = self.inner.take().unwrap();
        wgpu_begin_context(async {
            drop(take);
        })

    }
}

pub struct WgpuGuard<'a, T: 'static> {
    _guard: MutexGuard<'a, ()>,
    value: &'a mut T,
}

impl<'a, T> Deref for WgpuGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.value }
    }
}

impl<'a, T> DerefMut for WgpuGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value }
    }
}

impl<'a, T: Debug> Debug for WgpuGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuGuard")
            .field("value", unsafe { &*self.value })
            .finish()
    }
}

pub struct WgpuCell<T: 'static> {
    //option is so we can consume + custom drop
    shared: Option<Arc<Shared<T>>>,
}

impl<T> PartialEq for WgpuCell<T> {
    fn eq(&self, other: &Self) -> bool {
        let s = self.shared.as_ref().unwrap();
        let o = other.shared.as_ref().unwrap();
        Arc::ptr_eq(&s, &o)
    }
}

impl<T> Clone for WgpuCell<T> {
    fn clone(&self) -> Self {
        WgpuCell {
            shared: self.shared.clone(),
        }
    }
}

impl<T> WgpuCell<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        //I don't think we actually need to verify the thread here?
        //we promise drop is correctly handled
        let cell = unsafe { UnsafeSendCell::new_unchecked(UnsafeSyncCell::new(t))};
        WgpuCell {
            shared: Some(Arc::new(Shared {
                inner: Some(cell),
                mutex: Mutex::new(()),
            })),
        }
    }

    #[inline]
    pub fn verify_thread() {
        match WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                assert!(
                    sys::is_main_thread(),
                    "WgpuCell accessed from non-main thread when strategy is MainThread"
                );
            }
            WGPUStrategy::NotMainThread => {
                assert!(
                    !sys::is_main_thread(),
                    "WgpuCell accessed from main thread when strategy is NotMainThread"
                );
            }
            WGPUStrategy::Relaxed => {
                // No verification needed
            }
        }
    }

    pub fn lock(&self) -> WgpuGuard<'_, T> {
        Self::verify_thread();
        let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
        let value = unsafe { 
            let inner = self.shared.as_ref().unwrap().inner.as_ref().unwrap();
            inner.get().get_mut_unchecked()
        };
        WgpuGuard {
            _guard: guard,
            value,
        }
    }

    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        unsafe { self.shared.as_ref().unwrap().inner.as_ref().unwrap().get().get() }
    }



    pub fn assume<C, R>(&self, c: C) -> R
    where
        C: FnOnce(&T) -> R,
    {
        Self::verify_thread();
        let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
        let r = c(unsafe {self.shared.as_ref().unwrap().inner.as_ref().unwrap().get().get()});
        drop(guard);
        r
    }

    /**
    Be careful with this - it allows holding the lock while awaiting.
*/
    pub async fn assume_async<C, R>(&self, c: C) -> R
    where
        C: AsyncFnOnce(&T) -> R,
    {
        Self::verify_thread();
        let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
        let r = c(unsafe {self.shared.as_ref().unwrap().inner.as_ref().unwrap().get().get()}).await;
        drop(guard);
        r
    }


    /**
    Runs a closure with the inner value of the WgpuCell, ensuring that the closure is executed
    on the correct thread based on the WGPU_STRATEGY.

    # Panics
    For the duration of this function, the cell may not be otherwise used.
    */
    pub async fn with<C,R>(&self, c: C) -> R
    where
        C: FnOnce(&T) -> R + Send + 'static,
        R: Send + 'static,
        T: 'static,
    {

        match WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                if sys::is_main_thread() {
                    self.assume(c)
                } else {
                    let move_shared = self.shared.clone();
                    on_main_thread(move ||{
                        Self::verify_thread();
                        let guard = move_shared.as_ref().unwrap().mutex.lock().unwrap();
                        let r = c(unsafe {move_shared.as_ref().unwrap().inner.as_ref().unwrap().get().get()});
                        drop(guard);
                        r
                    }).await
                }
            }
            WGPUStrategy::NotMainThread => {
                if !sys::is_main_thread() {
                    // If we're not on the main thread, we can just call the closure directly
                    self.assume(c)
                } else {
                    // If we are on the main thread, we need to run it on a separate thread
                    let (s,f) = r#continue::continuation();
                    let move_shared = self.shared.clone();
                    _ = std::thread::Builder::new()
                        .name("WgpuCell thread".to_string())
                        .spawn(move || {
                            Self::verify_thread();
                            let guard = move_shared.as_ref().unwrap().mutex.lock().unwrap();
                            let r = c(unsafe {move_shared.as_ref().unwrap().inner.as_ref().unwrap().get().get()});
                            drop(guard);
                            //now we need to ship this back to the main thread
                            s.send(r);
                        }).unwrap();
                        f.await
                }
            }
            WGPUStrategy::Relaxed => {
                // Relaxed strategy allows access from any thread
                Self::verify_thread();
                let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
                let r = c(unsafe {self.shared.as_ref().unwrap().inner.as_ref().unwrap().get().get()});
                drop(guard);
                r
            }
        }
    }

    /**
    Creates a new WgpuCell by running a constructor closure on the correct thread
    based on the WGPU_STRATEGY.
    
    This function works like `with_mut` but for construction - it ensures the value
    is created on the appropriate thread for the current platform's wgpu strategy.
    */
    pub async fn new_on_thread<C,F>(c: C) -> WgpuCell<T>
    where
        C: FnOnce() -> F + Send + 'static,
        F: Future<Output = T>,
        T: 'static,
    {
        match WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                if sys::is_main_thread() {
                    WgpuCell::new(c().await)
                } else {
                    let v = Arc::new(Mutex::new(None));
                    let move_v = v.clone();
                    on_main_thread(|| {
                        let t = some_executor::task::Task::without_notifications(
                            "WgpuCell::new_on_thread".to_string(),
                            some_executor::task::Configuration::default(),
                            async move {
                                let f = c();

                                let cell = WgpuCell::new(f.await);
                                move_v.lock().unwrap().replace(cell);

                            },
                        );
                        t.spawn_thread_local();
                    }).await;
                    v.lock().unwrap()
                        .take()
                        .expect("WgpuCell value missing")
                }
            }
            WGPUStrategy::NotMainThread => {
                if !sys::is_main_thread() {
                    // If we're not on the main thread, we can just call the closure directly
                    WgpuCell::new(c().await)
                } else {
                    // If we are on the main thread, we need to run it on a separate thread
                    let (s,r) = r#continue::continuation();
                    _ = std::thread::Builder::new()
                        .name("WgpuCell new_on_thread".to_string())
                        .spawn(|| {
                            let t = some_executor::task::Task::without_notifications(
                                "WgpuCell::new_on_thread".to_string(),
                                some_executor::task::Configuration::default(),
                                async move {
                                    let r = c().await;
                                    s.send(WgpuCell::new(r));
                                },
                            );
                            t.spawn_thread_local();
                        }).unwrap();
                        r.await
                }
            }
            WGPUStrategy::Relaxed => {
                // Relaxed strategy allows access from any thread
                WgpuCell::new(c().await)
            }
        }
        
    }
    

    #[inline]
    pub fn copying(&self) -> Self
    where
        T: Copy,
    {
        unsafe {
            //fine to do directly because T: Copy
            //need to hold the lock for Sync though
            let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
            let t = *self.shared.as_ref().unwrap().inner.as_ref().unwrap().get().get();
            drop(guard);
            let inner = UnsafeSendCell::new(UnsafeSyncCell::new(t));
            WgpuCell {
                shared: Some(Arc::new(Shared {
                    inner: Some(inner),
                    mutex: Mutex::new(()),
                })),
            }
        }
    }
}

unsafe impl<T> Send for WgpuCell<T> {}



impl<T: Future> WgpuCell<T> {
    pub fn into_future(mut self) -> WgpuFuture<T> {
        let shared = self.shared.take().expect("WgpuCell value missing");
        let shared = match Arc::try_unwrap(shared){
            Ok(shared) => shared,
            Err(_) => {
                panic!("WgpuCell::into_future called on an exclusive lock");
            }
        };
        WgpuFuture {
            inner: shared,
        }
    }
}


impl<T: Debug> Debug for WgpuCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuCell")
            .finish()
    }
}




impl<T: Default> Default for WgpuCell<T> {
    fn default() -> Self {
        WgpuCell::new(Default::default())
    }
}

impl<T> From<T> for WgpuCell<T> {
    fn from(value: T) -> Self {
        WgpuCell::new(value)
    }
}

#[derive(Debug)]
pub struct WgpuFuture<T: 'static> {
    inner: Shared<T>,
}

unsafe impl<T> Send for WgpuFuture<T> {}

impl<T: Future> Future for WgpuFuture<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Verify we're polling from the correct thread
        match WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                assert!(
                    sys::is_main_thread(),
                    "WgpuFuture polled from non-main thread when strategy is MainThread"
                );
            }
            WGPUStrategy::NotMainThread => {
                assert!(
                    !sys::is_main_thread(),
                    "WgpuFuture polled from main thread when strategy is NotMainThread"
                );
            }
            WGPUStrategy::Relaxed => {
                // No verification needed
            }
        }
        let inner = unsafe {
            let self_mut = self.get_unchecked_mut();
            let _lock = self_mut.inner.mutex.lock().unwrap();
            Pin::new_unchecked(self_mut.inner.inner.as_mut().unwrap().get_mut().get_mut())
        };
        inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests that run on platforms where we can access from any thread (Relaxed strategy)
    #[cfg(target_os = "windows")]
    mod relaxed_tests {
        use super::*;
        use std::rc::Rc;

        #[test]
        fn test_wgpu_cell_basic_operations() {
            let value = 42;
            let mut cell = WgpuCell::new(value);
            assert_eq!(*cell.get(), 42);

            *cell.get_mut() = 100;
            assert_eq!(*cell.get(), 100);

            let value = cell.into_inner();
            assert_eq!(value, 100);
        }

        #[test]
        fn test_wgpu_cell_copy() {
            let cell = WgpuCell::new(42);
            let cell2 = cell.copying();
            assert_eq!(*cell.get(), *cell2.get());
        }

        #[test]
        fn test_wgpu_cell_with_non_send_type() {
            // Rc is not Send
            let rc = Rc::new(42);
            let cell = WgpuCell::new(rc);
            assert_eq!(**cell.get(), 42);
        }
    }

    // Test that verifies thread checking works correctly on macOS/wasm
    #[test]
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    fn test_main_thread_strategy() {
        // On macOS/wasm, we should be using MainThread strategy
        assert_eq!(WGPU_STRATEGY, WGPUStrategy::MainThread);

        // Since tests run on non-main threads, accessing should panic
        let result = std::panic::catch_unwind(|| {
            let cell = WgpuCell::new(42);
            let _ = cell.get(); // This should panic
        });
        assert!(
            result.is_err(),
            "Expected panic when accessing WgpuCell from non-main thread"
        );
    }

    // Test for Linux NotMainThread strategy
    #[test]
    #[cfg(target_os = "linux")]
    fn test_not_main_thread_strategy() {
        // On Linux, we should be using NotMainThread strategy
        assert_eq!(WGPU_STRATEGY, WGPUStrategy::NotMainThread);

        // Since tests run on non-main threads, accessing should work
        let cell = WgpuCell::new(42);
        assert_eq!(*cell.get(), 42); // This should work fine
    }

    struct TestFuture {
        ready: bool,
    }

    impl Future for TestFuture {
        type Output = i32;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready {
                Poll::Ready(42)
            } else {
                self.ready = true;
                Poll::Pending
            }
        }
    }

    #[test]
    fn test_wgpu_future_creation() {
        // Just test that we can create the future, not poll it
        let future = TestFuture { ready: false };
        let cell = WgpuCell::new(future);
        let _wgpu_future = cell.into_future();
    }

    // Test constructors that don't require thread access
    #[test]
    fn test_cell_construction() {
        // Just verify we can construct cells
        let _cell = WgpuCell::new(42);
        let _cell_from: WgpuCell<i32> = 42.into();
        let _cell_default: WgpuCell<i32> = Default::default();
    }

    // Test guard functionality on platforms where we can access from any thread (Relaxed strategy)
    #[cfg(target_os = "windows")]
    mod guard_tests {
        use super::*;

        #[test]
        fn test_guard_basic_operations() {
            let cell = WgpuCell::new(42);
            
            // Test that we can lock and access the value
            {
                let guard = cell.lock();
                assert_eq!(*guard, 42);
            }
            
            // Test that we can lock again after the guard is dropped
            {
                let mut guard = cell.lock();
                *guard = 100;
                assert_eq!(*guard, 100);
            }
            
            // Verify the value was actually changed
            {
                let guard = cell.lock();
                assert_eq!(*guard, 100);
            }
        }

        #[test]
        fn test_guard_debug_impl() {
            let cell = WgpuCell::new(42);
            let guard = cell.lock();
            let debug_str = format!("{:?}", guard);
            assert!(debug_str.contains("WgpuGuard"));
            assert!(debug_str.contains("42"));
        }

        #[test]
        fn test_guard_drop_behavior() {
            use std::sync::Arc;
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::thread;
            
            let cell = Arc::new(WgpuCell::new(42));
            let locked = Arc::new(AtomicBool::new(false));
            
            let cell_clone = cell.clone();
            let locked_clone = locked.clone();
            
            // Spawn a thread that tries to lock the cell
            let handle = thread::spawn(move || {
                let _guard = cell_clone.lock();
                locked_clone.store(true, Ordering::SeqCst);
                // Hold the lock for a bit
                thread::sleep(std::time::Duration::from_millis(10));
            });
            
            // Give the other thread time to acquire the lock
            thread::sleep(std::time::Duration::from_millis(5));
            
            // This should block until the other thread releases the lock
            let start = std::time::Instant::now();
            let _guard = cell.lock();
            let elapsed = start.elapsed();
            
            // We should have waited at least 5ms (remaining time from the 10ms sleep)
            assert!(elapsed.as_millis() >= 4); // Using 4ms to account for timing variations
            
            handle.join().unwrap();
        }
    }

}
