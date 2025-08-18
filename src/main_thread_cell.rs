//SPDX-License-Identifier: MPL-2.0

//! A cell type for main-thread-only values that can be shared across threads.
//!
//! [`MainThreadCell<T>`] is a thread-safe container that allows `T` to be shared across threads
//! while ensuring all access to the inner value happens on the main thread. This is useful for
//! wrapping platform-specific resources that must only be accessed from the main thread.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() {
//! use app_window::main_thread_cell::MainThreadCell;
//!
//! // Create a cell with a main-thread-only value
//! let cell = MainThreadCell::new(42);
//!
//! // Access from main thread
//! if app_window::application::is_main_thread() {
//!     let guard = cell.lock();
//!     println!("Value: {}", *guard);
//! }
//!
//! // Access from any thread via async
//! let result = cell.with(|value| {
//!     // This closure runs on the main thread
//!     *value * 2
//! }).await;
//! assert_eq!(result, 84);
//! # }
//! ```

use crate::application;
use send_cells::UnsafeSendCell;
use send_cells::unsafe_sync_cell::UnsafeSyncCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

/// Internal shared state for MainThreadCell
#[derive(Debug)]
struct Shared<T: 'static> {
    inner: Option<UnsafeSendCell<UnsafeSyncCell<T>>>,
    mutex: Mutex<()>,
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        // When we're dropping the last value, we need to do so on the right thread
        if let Some(take) = self.inner.take() {
            let drop_shared = format!(
                "MainThreadCell::drop({})",
                std::any::type_name::<T>()
            );
            application::submit_to_main_thread(drop_shared, || {
                drop(take);
            });
        }
    }
}

/// A guard providing mutable access to the inner value of a MainThreadCell.
///
/// This guard ensures that the value is only accessed on the main thread and
/// holds a mutex lock for the duration of access.
pub struct MainThreadGuard<'a, T: 'static> {
    _guard: MutexGuard<'a, ()>,
    value: &'a mut T,
}

impl<'a, T> Deref for MainThreadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<'a, T> DerefMut for MainThreadGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<'a, T: Debug> Debug for MainThreadGuard<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThreadGuard")
            .field("value", &*self.value)
            .finish()
    }
}

/// A thread-safe cell that ensures all access to its contents happens on the main thread.
///
/// `MainThreadCell<T>` allows you to share `T` across threads while guaranteeing that
/// all access to the inner value occurs on the main thread. This is particularly useful
/// for platform-specific resources that have main-thread-only requirements.
///
/// # Thread Safety
///
/// - The cell itself can be cloned and sent across threads
/// - All access methods verify they're called from the main thread
/// - The `with` and `with_async` methods automatically dispatch to the main thread
/// - Drop operations are automatically handled on the main thread
pub struct MainThreadCell<T: 'static> {
    shared: Option<Arc<Shared<T>>>,
}

impl<T> PartialEq for MainThreadCell<T> {
    fn eq(&self, other: &Self) -> bool {
        let s = self.shared.as_ref().unwrap();
        let o = other.shared.as_ref().unwrap();
        Arc::ptr_eq(s, o)
    }
}

impl<T> Clone for MainThreadCell<T> {
    fn clone(&self) -> Self {
        MainThreadCell {
            shared: self.shared.clone(),
        }
    }
}

impl<T> MainThreadCell<T> {
    /// Creates a new MainThreadCell containing the given value.
    ///
    /// This can be called from any thread.
    #[inline]
    pub fn new(t: T) -> Self {
        let cell = unsafe { UnsafeSendCell::new_unchecked(UnsafeSyncCell::new(t)) };
        MainThreadCell {
            shared: Some(Arc::new(Shared {
                inner: Some(cell),
                mutex: Mutex::new(()),
            })),
        }
    }

    /// Verifies that the current thread is the main thread.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-main thread.
    #[inline]
    fn verify_main_thread() {
        assert!(
            application::is_main_thread(),
            "MainThreadCell accessed from non-main thread"
        );
    }

    /// Locks the cell and returns a guard providing mutable access to the inner value.
    ///
    /// This method can only be called from the main thread.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-main thread.
    pub fn lock(&self) -> MainThreadGuard<'_, T> {
        Self::verify_main_thread();
        let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
        let value = unsafe {
            let inner = self.shared.as_ref().unwrap().inner.as_ref().unwrap();
            inner.get().get_mut_unchecked()
        };
        MainThreadGuard {
            _guard: guard,
            value,
        }
    }

    /// Runs a closure with immutable access to the inner value.
    ///
    /// This method can only be called from the main thread.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-main thread.
    pub fn assume<C, R>(&self, c: C) -> R
    where
        C: FnOnce(&T) -> R,
    {
        Self::verify_main_thread();
        let guard = self.shared.as_ref().unwrap().mutex.lock().unwrap();
        let r = c(unsafe {
            self.shared
                .as_ref()
                .unwrap()
                .inner
                .as_ref()
                .unwrap()
                .get()
                .get()
        });
        drop(guard);
        r
    }

    /// Runs a closure with the inner value, ensuring execution on the main thread.
    ///
    /// If called from the main thread, the closure executes immediately.
    /// If called from another thread, it's dispatched to the main thread.
    ///
    /// # Panics
    ///
    /// For the duration of this function, the cell may not be otherwise used.
    pub async fn with<C, R>(&self, c: C) -> R
    where
        C: FnOnce(&T) -> R + Send + 'static,
        R: Send + 'static,
        T: 'static,
    {
        let shared = self.shared.clone();
        let main_thread_cell = format!("MainThreadCell({})", std::any::type_name::<T>());
        application::on_main_thread(main_thread_cell, move || {
            Self::verify_main_thread();
            let guard = shared.as_ref().unwrap().mutex.lock().unwrap();
            let r = c(unsafe { shared.as_ref().unwrap().inner.as_ref().unwrap().get().get() });
            drop(guard);
            r
        })
        .await
    }

    /// Runs an async closure with the inner value, ensuring execution on the main thread.
    ///
    /// If called from the main thread, the closure executes immediately.
    /// If called from another thread, it's dispatched to the main thread.
    ///
    /// This method is more restrictive than `with_async` - it requires that access to the
    /// inner value doesn't cross async boundaries within the closure.
    ///
    /// # Panics
    ///
    /// For the duration of this function, the cell may not be otherwise used.
    pub async fn with_async<C, R, F>(&self, c: C) -> R
    where
        C: FnOnce(&T) -> F + Send + 'static,
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
        T: 'static,
    {
        let shared = self.shared.clone();

        let main_thread_cell = format!("MainThreadCell({})", std::any::type_name::<T>());
        // First, get the future from the closure on the main thread
        let future = application::on_main_thread(main_thread_cell, move || {
            Self::verify_main_thread();
            let guard = shared.as_ref().unwrap().mutex.lock().unwrap();
            let future = c(unsafe { shared.as_ref().unwrap().inner.as_ref().unwrap().get().get() });
            drop(guard);
            future
        })
        .await;

        // Then await the future (this can happen on any thread since F: Send)
        future.await
    }

    /// Creates a new MainThreadCell by running a constructor closure on the main thread.
    ///
    /// This function ensures the value is created on the main thread, which is useful
    /// for resources that must be constructed there.
    pub async fn new_on_main_thread<C, F>(c: C) -> MainThreadCell<T>
    where
        C: FnOnce() -> F + Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        logwise::info_sync!("MainThreadCell::new_on_main_thread() started");
        let new_on_main_thread = format!("MainThreadCell::new_on_main_thread({})", std::any::type_name::<T>());
        let value = application::on_main_thread(new_on_main_thread,  || async move {
            logwise::info_sync!("Inside main thread closure");
            let f = c();
            logwise::info_sync!("Calling provided closure f()...");
            let r = f.await;
            logwise::info_sync!("Closure completed, creating MainThreadCell...");
            MainThreadCell::new(r)
        })
        .await
        .await;
        logwise::info_sync!("Main thread execution completed, returning value");
        value
    }
}

// Safety: MainThreadCell ensures all access happens on the main thread
unsafe impl<T> Send for MainThreadCell<T> {}

impl<T: Debug> Debug for MainThreadCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThreadCell").finish()
    }
}

impl<T: Default> Default for MainThreadCell<T> {
    fn default() -> Self {
        MainThreadCell::new(Default::default())
    }
}

impl<T> From<T> for MainThreadCell<T> {
    fn from(value: T) -> Self {
        MainThreadCell::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use std::thread;
    #[cfg(target_arch = "wasm32")]
    use wasm_thread as thread;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[test]
    fn test_cell_construction() {
        // Verify we can construct cells
        let cell = MainThreadCell::new(42);
        let cell_from: MainThreadCell<i32> = 42.into();
        let cell_default: MainThreadCell<i32> = Default::default();
        //these require drop on the main thread, so let's not!
        std::mem::forget(cell);
        std::mem::forget(cell_from);
        std::mem::forget(cell_default);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[test]
    fn test_debug_impl() {
        let cell = MainThreadCell::new(42);
        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("MainThreadCell"));
        //can't drop on the main thread, so let's not
        std::mem::forget(cell);
    }

    #[test_executors::async_test]
    async fn test_send_across_threads() {
        //for the time being, wasm_thread only works in browser
        //see https://github.com/rustwasm/wasm-bindgen/issues/4534,
        //though we also need wasm_thread support.
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
        let cell = MainThreadCell::new(42);
        let (c, f) = r#continue::continuation();

        // Verify we can send the cell to another thread
        thread::spawn(move || {
            // We can hold the cell in another thread, just not access it
            let _held_cell = cell;
            c.send(());
        });

        f.await;
    }
}
