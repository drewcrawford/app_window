// SPDX-License-Identifier: MPL-2.0

//! A bounded record of the windows and surfaces this process has created.
//!
//! Window state is exactly what differs between "works on my machine" and a bug
//! report, and none of it is visible from outside the process. A [`Window`](crate::window::Window) has
//! no accessors -- it is created, it is dropped, and everything in between is
//! the platform's business -- so this records what the crate itself knows at
//! the moments it knows it.
//!
//! # What is recorded, and what is not
//!
//! **Recorded:** what the caller asked for (position, size, title, fullscreen),
//! when, whether a surface was ever attached, and whether the window is still
//! alive. Plus the most recent
//! size and scale the *application* observed, because a renderer calls
//! [`Surface::size_scale`](crate::surface::Surface::size_scale) every time it
//! resizes and that is the number it is acting on.
//!
//! **Not recorded:** the platform's current answer for focus, visibility, or
//! position. Reading those means a main-thread round trip into four backends
//! that do not expose them today; see the issue for why that is a separate
//! piece of work. The size and scale here are therefore *last observed*, and
//! their age is reported alongside so a stale value is visibly stale rather
//! than quietly wrong.
//!
//! # Bounded, and honest about it
//!
//! Keeps the most recent `N`; `N` comes from `APP_WINDOW_REGISTRY_CAPACITY` and
//! defaults to [`DEFAULT_CAPACITY`](crate::registry::DEFAULT_CAPACITY). Overflow forgets *closed* windows before
//! live ones -- a live one is the whole reason to look -- and every drop is
//! counted and reported, so a caller cannot mistake "that is all of them" for
//! "I lost some".
//!
//! Behind the `exfiltrate` feature. With it off none of this is compiled and
//! creating a window does not touch a lock.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::application::time;
use crate::coordinates::{Position, Size};

/// How many window records to keep when `APP_WINDOW_REGISTRY_CAPACITY` is
/// unset.
pub const DEFAULT_CAPACITY: usize = 256;

/// Which constructor made the window, since they differ in what the caller
/// controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    /// [`Window::new`](crate::window::Window::new) -- caller chose position and
    /// size.
    Requested,
    /// [`Window::default`](crate::window::Window::default) -- the platform
    /// chose, so the recorded position and size are unknown rather than zero.
    PlatformDefault,
    /// [`Window::fullscreen`](crate::window::Window::fullscreen).
    Fullscreen,
}

impl Origin {
    /// The stable wire name reported by `snapshot`, matched by external
    /// tooling rather than rendered from `Debug`.
    pub const fn name(self) -> &'static str {
        match self {
            Origin::Requested => "requested",
            Origin::PlatformDefault => "platform_default",
            Origin::Fullscreen => "fullscreen",
        }
    }
}

/// One window's record.
#[derive(Clone, Debug)]
pub struct Entry {
    /// A locally minted id, always distinct. What `--id` selects.
    pub id: u64,
    /// How the window came to exist, which is what says whether `requested`
    /// means anything.
    pub origin: Origin,
    /// The window's title. Caller-derived, so it is local-only.
    pub title: String,
    /// What was asked for. `None` for [`Origin::PlatformDefault`], where the
    /// caller asked for nothing.
    pub requested: Option<(Position, Size)>,
    /// When the window was created.
    pub created_at: time::Instant,
    /// `None` while the window is open.
    pub closed_at: Option<time::Instant>,
    /// Whether a rendering surface has been attached. An open window with no
    /// surface draws nothing, which looks like a hang from outside.
    pub surface_attached: bool,
    /// The last size and scale the application itself observed, and when.
    pub last_observed: Option<(Size, f64, time::Instant)>,
}

impl Entry {
    /// Whether the window is still open.
    pub fn is_open(&self) -> bool {
        self.closed_at.is_none()
    }
}

struct Registry {
    entries: VecDeque<Entry>,
    capacity: usize,
    dropped: u64,
}

impl Registry {
    fn new(capacity: usize) -> Registry {
        Registry {
            entries: VecDeque::new(),
            capacity,
            dropped: 0,
        }
    }

    /// Inserts, evicting a closed window first and only then the oldest live
    /// one.
    ///
    /// A method rather than a free function so the policy can be tested against
    /// a local instance. The global registry is shared with every other test in
    /// the binary, and asserting eviction against it is a race.
    fn push(&mut self, entry: Entry) {
        // A ceiling of zero turns the registry off. Handled before the loop
        // below, which would otherwise spin forever: with `capacity == 0` its
        // condition is true on an empty deque, and removing from an empty deque
        // is a no-op rather than an error.
        if self.capacity == 0 {
            self.dropped += 1;
            return;
        }
        while self.entries.len() >= self.capacity {
            let victim = self
                .entries
                .iter()
                .position(|existing| !existing.is_open())
                .unwrap_or(0);
            self.entries.remove(victim);
            self.dropped += 1;
        }
        self.entries.push_back(entry);
    }

    fn find(&mut self, id: u64) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }
}

fn capacity() -> usize {
    std::env::var("APP_WINDOW_REGISTRY_CAPACITY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_CAPACITY)
}

fn registry() -> &'static Mutex<Registry> {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::new(capacity())))
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Records a new window and returns its id.
///
/// `try_lock`, like every other access here: a registry that blocked window
/// creation would be worse than one that occasionally misses a window. A miss
/// costs the id, which is why this returns `None` and the caller records
/// nothing further.
///
/// No creation site is recorded. The obvious `#[track_caller]` is a documented
/// no-op on `async fn`, and every constructor here is one -- it would report
/// the constructor's own line for every window in the process, which is worse
/// than reporting nothing.
pub(crate) fn opened(
    origin: Origin,
    title: String,
    requested: Option<(Position, Size)>,
) -> Option<u64> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut registry = registry().try_lock().ok()?;
    registry.push(Entry {
        id,
        origin,
        title,
        requested,
        created_at: time::Instant::now(),
        closed_at: None,
        surface_attached: false,
        last_observed: None,
    });
    Some(id)
}

pub(crate) fn closed(id: u64) {
    if let Ok(mut registry) = registry().try_lock()
        && let Some(entry) = registry.find(id)
    {
        entry.closed_at = Some(time::Instant::now());
    }
}

pub(crate) fn surface_attached(id: u64) {
    if let Ok(mut registry) = registry().try_lock()
        && let Some(entry) = registry.find(id)
    {
        entry.surface_attached = true;
    }
}

/// Records what the application just learned about a surface's geometry.
pub(crate) fn observed(id: u64, size: Size, scale: f64) {
    if let Ok(mut registry) = registry().try_lock()
        && let Some(entry) = registry.find(id)
    {
        entry.last_observed = Some((size, scale, time::Instant::now()));
    }
}

/// A snapshot of every recorded window, or `None` if the registry is locked.
pub fn entries() -> Option<Vec<Entry>> {
    let registry = registry().try_lock().ok()?;
    Some(registry.entries.iter().cloned().collect())
}

/// How many records were evicted, and the ceiling they were evicted against.
pub fn stats() -> Option<(u64, usize)> {
    let registry = registry().try_lock().ok()?;
    Some((registry.dropped, registry.capacity))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, open: bool) -> Entry {
        Entry {
            id,
            origin: Origin::Requested,
            title: "t".to_string(),
            requested: None,
            created_at: time::Instant::now(),
            closed_at: if open {
                None
            } else {
                Some(time::Instant::now())
            },
            surface_attached: false,
            last_observed: None,
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn overflow_forgets_closed_windows_before_live_ones() {
        let mut registry = Registry::new(2);
        registry.push(entry(1, true));
        registry.push(entry(2, false));
        registry.push(entry(3, true));

        let ids: Vec<u64> = registry.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(ids, vec![1, 3], "the closed window is the one to lose");
        assert_eq!(registry.dropped, 1);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn overflow_of_all_live_windows_drops_the_oldest_and_counts_it() {
        let mut registry = Registry::new(2);
        registry.push(entry(1, true));
        registry.push(entry(2, true));
        registry.push(entry(3, true));

        let ids: Vec<u64> = registry.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(ids, vec![2, 3]);
        assert_eq!(
            registry.dropped, 1,
            "a drop is counted so the report can say so"
        );
    }

    /// A capacity of zero is a legitimate way to turn the registry off at
    /// runtime, and it must not panic on the window-creation path.
    #[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn a_capacity_of_zero_retains_nothing_and_counts_everything() {
        let mut registry = Registry::new(0);
        registry.push(entry(1, true));
        assert!(registry.entries.is_empty());
        assert_eq!(registry.dropped, 1);
    }
}
