// SPDX-License-Identifier: MPL-2.0

//! Main-thread liveness accounting.
//!
//! Compiled in unconditionally, unlike the window registry next door. A wedged
//! main thread is an operational fact about the application -- the UI is frozen
//! and the user can see it -- and this crate's rule is that operational facts
//! are not something a consumer has to opt into observing. The cost is three
//! relaxed atomics on a path that already allocates a `String`, takes an
//! `Instant` and mints a context.
//!
//! # Idle and wedged look identical from outside, so this measures both ends
//!
//! Counting main-thread *turns* alone cannot distinguish them: a healthy
//! application with nothing to do produces no turns either. So the record has
//! two halves.
//!
//! `submitted` and `completed` -- private, called from
//! [`submit_to_main_thread`](crate::application::submit_to_main_thread) --
//! bracket every hand-off that goes through
//! that function, which is the single cross-platform choke point for
//! main-thread work. The
//! difference between the two counts is what is queued or in flight right now,
//! and `last_turn` is when one last finished.
//!
//! [`probe()`](crate::instrument::probe) is the other half: an observer submits a closure that does nothing
//! but stamp its own arrival. If it comes back, the main thread is pumping and
//! the application was merely idle. If it does not come back, the main thread
//! is wedged, and the *age of the outstanding probe* is the measurement -- the
//! one number a caller actually wants, and the one a blocking round-trip could
//! never report, because it would be blocked too.

use std::sync::atomic::{AtomicU64, Ordering};
// `Duration` is `core::time::Duration` on both targets, so the public struct
// below names it directly rather than through the crate's `Instant` alias --
// a caller should not have to reach into a private module to spell a field's
// type. Only `Instant` differs, and it stays internal.
use std::time::Duration;

use crate::application::time;

/// Millisecond timestamps are relative to this, so they fit an atomic.
///
/// `Instant` has no representation that survives into an `AtomicU64`, and there
/// is no wall clock on wasm32 worth using here. Everything below is an offset
/// from the first time this is touched, which is early enough that the
/// arithmetic never goes negative.
fn epoch() -> time::Instant {
    use std::sync::OnceLock;
    static EPOCH: OnceLock<time::Instant> = OnceLock::new();
    *EPOCH.get_or_init(time::Instant::now)
}

fn now_ms() -> u64 {
    epoch().elapsed().as_millis() as u64
}

/// Sentinel for "this has not happened yet". Zero is a real timestamp -- it is
/// the epoch itself -- so it cannot be the sentinel.
const NEVER: u64 = u64::MAX;

static SUBMITTED: AtomicU64 = AtomicU64::new(0);
static COMPLETED: AtomicU64 = AtomicU64::new(0);
static OVERRAN: AtomicU64 = AtomicU64::new(0);
static SLOWEST_MS: AtomicU64 = AtomicU64::new(0);
static LAST_TURN_MS: AtomicU64 = AtomicU64::new(NEVER);

static PROBE_SENT_MS: AtomicU64 = AtomicU64::new(NEVER);
static PROBE_BACK_MS: AtomicU64 = AtomicU64::new(NEVER);
static PROBE_ROUND_TRIP_MS: AtomicU64 = AtomicU64::new(NEVER);

/// Records that work was handed to the main thread.
pub(crate) fn submitted() {
    // Touch the epoch on the submitting thread rather than inside the closure:
    // if the main thread never runs again, the epoch must still exist for the
    // ages below to mean anything.
    let _ = epoch();
    SUBMITTED.fetch_add(1, Ordering::Relaxed);
}

/// Records that a main-thread turn finished, and how long it held the thread.
pub(crate) fn completed(duration: Duration, overran: bool) {
    let millis = duration.as_millis() as u64;
    COMPLETED.fetch_add(1, Ordering::Relaxed);
    LAST_TURN_MS.store(now_ms(), Ordering::Relaxed);
    SLOWEST_MS.fetch_max(millis, Ordering::Relaxed);
    if overran {
        OVERRAN.fetch_add(1, Ordering::Relaxed);
    }
}

/// What an observer can say about the main thread without touching it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MainThreadStats {
    /// Has [`application::main`](crate::application::main) been reached?
    pub running: bool,
    /// Turns handed to the main thread over the process's life.
    pub submitted: u64,
    /// How many of those finished.
    pub completed: u64,
    /// Submitted but not yet finished: queued plus the one in flight.
    pub outstanding: u64,
    /// Turns that held the main thread longer than the crate's 10ms budget.
    pub overran: u64,
    /// The longest single turn seen.
    pub slowest_ms: u64,
    /// How long ago a turn last finished, or `None` if none ever has.
    pub since_last_turn: Option<Duration>,
    /// How long the outstanding probe has been waiting, or `None` if there
    /// isn't one. **This is the wedged-main-thread measurement.**
    pub probe_outstanding: Option<Duration>,
    /// The last completed probe's round trip, or `None` if none completed.
    pub last_probe_round_trip: Option<Duration>,
}

fn age(stamp: &AtomicU64) -> Option<Duration> {
    match stamp.load(Ordering::Relaxed) {
        NEVER => None,
        then => Some(Duration::from_millis(now_ms().saturating_sub(then))),
    }
}

/// Reads the main thread's state without asking the main thread anything.
///
/// This is three relaxed atomic loads and never waits, which is the point: a
/// wedged main thread is exactly the case where a probe that joined it would
/// wedge too, and report nothing.
pub fn stats() -> MainThreadStats {
    let submitted = SUBMITTED.load(Ordering::Relaxed);
    let completed = COMPLETED.load(Ordering::Relaxed);
    let sent = PROBE_SENT_MS.load(Ordering::Relaxed);
    let back = PROBE_BACK_MS.load(Ordering::Relaxed);
    MainThreadStats {
        running: crate::application::is_main_thread_running(),
        submitted,
        completed,
        // Saturating, not subtracting blind: the two counters are incremented
        // by different threads under `Relaxed`, so a reader can legitimately
        // see a completion whose submission it has not seen yet.
        outstanding: submitted.saturating_sub(completed),
        overran: OVERRAN.load(Ordering::Relaxed),
        slowest_ms: SLOWEST_MS.load(Ordering::Relaxed),
        since_last_turn: age(&LAST_TURN_MS),
        probe_outstanding: if sent == NEVER || (back != NEVER && back >= sent) {
            None
        } else {
            age(&PROBE_SENT_MS)
        },
        last_probe_round_trip: match PROBE_ROUND_TRIP_MS.load(Ordering::Relaxed) {
            NEVER => None,
            millis => Some(Duration::from_millis(millis)),
        },
    }
}

/// Sends a do-nothing closure to the main thread, if one is not already in
/// flight, and returns immediately.
///
/// Deliberately not a round trip. An observer that waited for the answer would
/// be blocked by exactly the condition it is trying to report, and a debug
/// server that stalls when the application stalls is no use at all. Each call
/// starts or renews the measurement; the *next* call reads it.
///
/// Does nothing before [`application::main`](crate::application::main) has been
/// reached, because submitting then panics.
pub fn probe() {
    if !crate::application::is_main_thread_running() {
        return;
    }
    let sent = PROBE_SENT_MS.load(Ordering::Relaxed);
    let back = PROBE_BACK_MS.load(Ordering::Relaxed);
    let in_flight = sent != NEVER && (back == NEVER || back < sent);
    if in_flight {
        // Leave the outstanding one alone. Replacing it would reset the age,
        // which is the only thing worth knowing while the main thread is stuck.
        return;
    }
    let sent_at = now_ms();
    PROBE_SENT_MS.store(sent_at, Ordering::Relaxed);
    crate::application::submit_to_main_thread("app_window.probe".to_string(), move || {
        let back_at = now_ms();
        PROBE_ROUND_TRIP_MS.store(back_at.saturating_sub(sent_at), Ordering::Relaxed);
        PROBE_BACK_MS.store(back_at, Ordering::Relaxed);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counters are process-global and every other test in this crate
    /// drives the main thread, so these assert *relative* movement rather than
    /// absolute values -- an absolute assertion here would be a race against
    /// whatever else is running.
    #[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn accounting_moves_with_submissions() {
        let before = stats();
        submitted();
        let queued = stats();
        assert_eq!(queued.submitted, before.submitted + 1);
        assert_eq!(
            queued.outstanding,
            before.outstanding + 1,
            "a submission that has not completed is outstanding"
        );

        completed(Duration::from_millis(25), true);
        let done = stats();
        assert_eq!(done.completed, before.completed + 1);
        assert_eq!(done.outstanding, before.outstanding);
        assert_eq!(done.overran, before.overran + 1);
        assert!(
            done.slowest_ms >= 25,
            "the slowest turn is a high-water mark"
        );
        assert!(
            done.since_last_turn.is_some(),
            "a completed turn stamps the clock"
        );
    }

    /// Before `application::main`, submitting panics -- so the probe must
    /// decline rather than take the process down. This is the state a provider
    /// is in whenever it is asked about an application that has not started
    /// its event loop.
    #[cfg_attr(target_arch = "wasm32", wasm_lite::wasm_lite_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn probing_before_the_event_loop_is_a_no_op() {
        if crate::application::is_main_thread_running() {
            // Another test in this binary started the loop first; the
            // precondition this case exists to check is gone.
            return;
        }
        probe();
        assert!(
            stats().probe_outstanding.is_none(),
            "nothing was sent, so nothing is outstanding"
        );
    }
}
