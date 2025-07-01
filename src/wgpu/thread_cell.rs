//SPDX-License-Identifier: MPL-2.0

use send_cells::UnsafeSendCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};

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
            inner: Some(unsafe { UnsafeSendCell::new_unchecked(t) }),
        }
    }

    #[inline]
    fn verify_thread() {
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
        unsafe { &*self.inner.as_ref().expect("WgpuCell value missing").get() }
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
                .expect("WgpuCell value missing")
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
            self.inner
                .take()
                .expect("WgpuCell value missing")
                .into_inner()
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        Self::verify_thread();
        unsafe { self.into_unchecked_inner() }
    }

    pub async fn assume<F, Fut, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = R>,
    {
        Self::verify_thread();
        f(unsafe { self.get_unchecked() }).await
    }

    pub async fn assume_mut<F, Fut, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> Fut,
        Fut: Future<Output = R>,
    {
        Self::verify_thread();
        f(unsafe { self.get_unchecked_mut() }).await
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
