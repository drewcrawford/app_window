//SPDX-License-Identifier: MPL-2.0

use send_cells::UnsafeSendCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use crate::application::on_main_thread;
use crate::executor::on_main_thread_async;
use crate::sys;
use crate::wgpu::{WGPU_STRATEGY, WGPUStrategy};

pub struct WgpuCell<T> {
    inner: Option<UnsafeSendCell<T>>,
}

impl<T> WgpuCell<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        //I don't think we actually need to verify the thread here?
        WgpuCell {
            inner: unsafe { Some(UnsafeSendCell::new_unchecked(t) )},
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

    #[inline]
    pub unsafe fn get_unchecked(&self) -> &T {
        unsafe { &*self.inner.as_ref().unwrap().get() }
    }

    #[inline]
    pub fn get(&self) -> &T {
        Self::verify_thread();
        unsafe { self.get_unchecked() }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        unsafe {
            &mut *self
                .inner
                .as_mut()
                .unwrap()
                .get_mut()
        }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        Self::verify_thread();
        unsafe { self.get_unchecked_mut() }
    }

    #[inline]
    pub unsafe fn into_unchecked_inner(mut self) -> T {
        unsafe {
            self.inner.take().unwrap().into_inner()
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        Self::verify_thread();
        unsafe { self.into_unchecked_inner() }
    }

    pub async fn assume<C, R>(&self, c: C) -> R
    where
        C: AsyncFnOnce(&T) -> R,
    {
        Self::verify_thread();
        c(unsafe { self.get_unchecked() }).await
    }

    pub async fn assume_mut<C, R>(&mut self, c: C) -> R
    where
        C: AsyncFnOnce(&mut T) -> R,
    {
        Self::verify_thread();
        c(unsafe { self.get_unchecked_mut() }).await
    }

    /**
    Runs a closure with the inner value of the WgpuCell, ensuring that the closure is executed
    on the correct thread based on the WGPU_STRATEGY.

    # Panics
    For the duration of this function, the cell may not be otherwise used.
    */
    pub async fn with_mut<C,F,R>(&mut self, c: C) -> R
    where
        C: FnOnce(&mut T) -> F + Send + 'static,
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
        T: 'static,
    {
        //for the duration of this function, we take the inner value out of the cell
        let mut take = self.inner.take().expect("WgpuCell value missing");
        match WGPU_STRATEGY {
            WGPUStrategy::MainThread => {
                if sys::is_main_thread() {
                    c(unsafe { take.get_mut() }).await
                } else {
                    on_main_thread_async(async move {
                        c(unsafe { take.get_mut() }).await
                    }).await
                }
            }
            WGPUStrategy::NotMainThread => {
                if !sys::is_main_thread() {
                    // If we're not on the main thread, we can just call the closure directly
                    c(unsafe { take.get_mut() }).await
                } else {
                    // If we are on the main thread, we need to run it on a separate thread
                    let (s,f) = r#continue::continuation();
                    _ = std::thread::Builder::new()
                        .name("WgpuCell thread".to_string())
                        .spawn(|| {
                            let t = some_executor::task::Task::without_notifications(
                                "WgpuCell::with_mut".to_string(),
                                some_executor::task::Configuration::default(),
                                async move {
                                    let r = c(unsafe { take.get_mut() }).await;
                                    s.send(r);
                                },
                            );
                            t.spawn_thread_local();
                        }).unwrap();
                        f.await
                }
            }
            WGPUStrategy::Relaxed => {
                // Relaxed strategy allows access from any thread
                c(unsafe { take.get_mut() }).await
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
            WgpuCell {
                inner: Some(UnsafeSendCell::new_unchecked(*self.get())),
            }
        }
    }
}

unsafe impl<T> Send for WgpuCell<T> {}


impl<T: Future> WgpuCell<T> {
    pub fn into_future(mut self) -> WgpuFuture<T> {
        WgpuFuture {
            inner: self.inner.take().expect("WgpuCell value missing"),
        }
    }
}

impl<T> Drop for WgpuCell<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            Self::verify_thread();
        }
    }
}

impl<T: Debug> Debug for WgpuCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl<T> AsRef<T> for WgpuCell<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T> AsMut<T> for WgpuCell<T> {
    fn as_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<T> Deref for WgpuCell<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for WgpuCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
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
pub struct WgpuFuture<T> {
    inner: UnsafeSendCell<T>,
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

        // SAFETY: After thread verification, we can safely poll the inner future
        let inner = unsafe {
            let self_mut = self.get_unchecked_mut();
            Pin::new_unchecked(self_mut.inner.get_mut())
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

}
