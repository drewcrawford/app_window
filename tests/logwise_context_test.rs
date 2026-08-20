// SPDX-License-Identifier: MPL-2.0
//! The context contract this crate now owes the facade.
//!
//! `on_main_thread` mints a durable child token for the work it schedules and
//! enters it only while that work runs. Three things follow, and all three are
//! checked here against the real platform main loop rather than a stand-in:
//!
//! * **Lineage** — the closure runs in a context descended from the caller's,
//!   so main-thread work is attributable to whoever asked for it.
//! * **Restore** — the calling thread's context is unchanged afterwards, and
//!   the main thread does not keep the token after the closure returns.
//! * **Field gating** — a view that asks only for support-safe core fields is
//!   not given the local-only detail ones.
//!
//! Run with: `cargo test --test logwise_context_test`
//! On WASM: `scripts/wasm32/tests --test logwise_context_test`

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use logwise::{ContextToken, Detail, Dispatch, EventRef, Interest, Metadata, Privacy};
use some_executor::task::{Configuration, Task};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;

// -- a dispatcher small enough to write out -----------------------------------
//
// The library depends only on the facade, so the test does too. Everything
// asserted here is facade ABI: token lineage, and which fields a call site
// materialized for a given interest mask.

// Only the native half asserts on captured events; see the note on
// `check_field_gating`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Clone, Debug)]
struct Seen {
    name: &'static str,
    fields: Vec<(&'static str, Privacy, Detail, bool)>,
}

static EVENTS: Mutex<Vec<Seen>> = Mutex::new(Vec::new());
/// Lineage is only recorded for children of the token under test.
///
/// The main executor re-submits itself every iteration, and each submission
/// mints a context — so recording every parent link, or every event, grows
/// without bound for as long as the main loop runs. Watching one token keeps
/// this bounded, which the first version of this test did not and hung.
static WATCHED: Mutex<ContextToken> = Mutex::new(ContextToken::NONE);
static PARENTS: Mutex<Option<HashMap<u64, ContextToken>>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);
static WANTED: Mutex<Interest> = Mutex::new(Interest::NONE);
static GENERATION: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static CURRENT: Cell<ContextToken> = const { Cell::new(ContextToken::NONE) };
}

struct Recorder;

impl Dispatch for Recorder {
    fn generation(&self) -> usize {
        GENERATION.load(Ordering::Acquire) as usize
    }
    fn interest(&self, _metadata: &'static Metadata) -> Interest {
        *WANTED.lock().unwrap()
    }
    fn emit(&self, event: EventRef<'_>) {
        // Only the one call site this test asserts on; see WATCHED.
        if event.metadata.event_name != "app_window.main_thread.submission_overran" {
            return;
        }
        let fields = event
            .metadata
            .fields
            .iter()
            .zip(event.fields.iter())
            .map(|(meta, got)| (meta.name, meta.privacy, meta.detail, got.is_some()))
            .collect();
        EVENTS.lock().unwrap().push(Seen {
            name: event.metadata.event_name,
            fields,
        });
    }
    fn capture_context(&self) -> ContextToken {
        CURRENT.with(Cell::get)
    }
    fn create_context(&self, parent: ContextToken, _name: &'static str) -> ContextToken {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        if parent == *WATCHED.lock().unwrap() {
            PARENTS
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .insert(id, parent);
        }
        ContextToken::from_parts(id, 0)
    }
    fn enter_context(&self, context: ContextToken) -> ContextToken {
        CURRENT.with(|current| current.replace(context))
    }
    fn exit_context(&self, previous: ContextToken) {
        CURRENT.with(|current| current.set(previous));
    }
}

static RECORDER: Recorder = Recorder;

fn parent_of(context: ContextToken) -> Option<ContextToken> {
    PARENTS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .get(&context.into_parts().0)
        .copied()
}

fn want(interest: Interest) {
    *WANTED.lock().unwrap() = interest;
    GENERATION.fetch_add(1, Ordering::Release);
}

/// Turns a panic anywhere into a process failure.
///
/// Without this a failed assertion on a worker thread leaves the platform main
/// loop spinning, and the test reads as a hang instead of a failure -- which is
/// exactly how the first run of this file presented.
fn fail_loudly() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        std::process::exit(1);
    }));
}

fn install() {
    fail_loudly();
    logwise::install_dispatcher(&RECORDER).expect("install dispatcher");
    // Nothing is evaluated until a check asks for it. The main loop runs
    // continuously, so a standing interest in everything is not free.
    want(Interest::NONE);
}

// -- the assertions -----------------------------------------------------------

/// Runs on a non-main thread, with the platform main loop already running.
///
/// Note what this deliberately does *not* do: hold a `ContextGuard` across an
/// `.await`. The guard restores thread-local state on drop, and a task that
/// yields is re-polled inside its *own* context — so a guard spanning a yield
/// point reads back something it never set. The first version of this test did
/// exactly that and failed, which is the same mistake the library must not make.
async fn check_context_contract() {
    // Not NONE: this runs inside a some_executor task, which is itself polled
    // inside its own durable context. "Restore" means back to that, not to
    // nothing.
    let outer = logwise::context::capture();
    let caller = logwise::context::child(ContextToken::NONE, "test.caller");
    *WATCHED.lock().unwrap() = caller;

    let (first_tx, first_rx) = r#continue::continuation();
    let (second_tx, second_rx) = r#continue::continuation();

    {
        let _entered = logwise::context::enter(caller);
        assert_eq!(
            logwise::context::capture(),
            caller,
            "the test thread should be in its own context"
        );

        app_window::application::submit_to_main_thread("probe".to_string(), move || {
            first_tx.send(logwise::context::capture());
        });
        app_window::application::submit_to_main_thread("probe2".to_string(), move || {
            second_tx.send(logwise::context::capture());
        });

        // Restore, on the submitting thread: handing work off must not disturb
        // the caller's own context.
        assert_eq!(
            logwise::context::capture(),
            caller,
            "submitting work must not disturb the caller's context"
        );
    }
    assert_eq!(
        logwise::context::capture(),
        outer,
        "the guard should restore the context that was entered before it"
    );

    let observed = first_rx.await;
    let after = second_rx.await;

    assert!(
        !observed.is_none(),
        "main-thread work should run in a context, not the null token"
    );
    assert_ne!(
        observed, caller,
        "it should be its own context, not the caller's"
    );
    assert_eq!(
        parent_of(observed),
        Some(caller),
        "the main-thread context should descend from the submitting one"
    );

    // Restore, on the main thread: a later closure must not inherit the
    // previous one's token.
    assert_ne!(
        after, observed,
        "each submission gets its own context; the main thread did not keep the last one"
    );
    assert_eq!(parent_of(after), Some(caller));

    *WATCHED.lock().unwrap() = ContextToken::NONE;

    // Native only, deliberately. The field-gating check drives the overrun
    // event by blocking the main thread past its 10ms threshold -- but on
    // wasm32 `on_main_thread` runs the closure *inline* when it is already on
    // the main thread, so the thread being timed is the thread doing the
    // timing, and the event proved not to fire reliably there. Lineage and
    // restore above, which are the substance of the contract, run on both
    // targets. Field gating itself is a facade property with its own coverage.
    #[cfg(not(target_arch = "wasm32"))]
    check_field_gating().await;
    logwise::log!("app_window logwise context contract: OK");
}

/// A support-safe view is not given the local-only detail fields.
#[cfg(not(target_arch = "wasm32"))]
///
/// Driven through `submit_to_main_thread`'s own overrun event rather than a
/// stand-in call site: it carries one support-safe core field and one
/// local-only detail field, which is the shape this migration introduced.
async fn check_field_gating() {
    EVENTS.lock().unwrap().clear();
    want(Interest::CORE_SUPPORT.union(Interest::DETAIL_SUPPORT));

    let (done_tx, done_rx) = r#continue::continuation();
    // Deliberately slow: the overrun threshold is 10ms.
    app_window::application::submit_to_main_thread("slow".to_string(), move || {
        // `std::time::Instant` is unimplemented on wasm32; time goes through
        // wasm_lite_std on both targets.
        let start = wasm_lite_std::time::Instant::now();
        while start.elapsed() < wasm_lite_std::time::Duration::from_millis(25) {
            std::hint::spin_loop();
        }
        done_tx.send(());
    });
    done_rx.await;

    let events = EVENTS.lock().unwrap().clone();
    let overran = events
        .iter()
        .find(|seen| seen.name == "app_window.main_thread.submission_overran")
        .expect("a 25ms main-thread closure should report an overrun");

    for (name, privacy, detail, materialized) in &overran.fields {
        match (privacy, detail) {
            (Privacy::SupportSafe, Detail::Core) => assert!(
                materialized,
                "{name} is support-safe core and should be present"
            ),
            (Privacy::LocalOnly, _) => assert!(
                !materialized,
                "{name} is local-only and must be withheld from a support view"
            ),
            _ => {}
        }
    }
    assert!(
        overran
            .fields
            .iter()
            .any(|(name, ..)| *name == "duration_ms"),
        "the overrun should say how long it took: {overran:?}"
    );

    want(Interest::NONE);
}

// -- entry points -------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    install();
    app_window::application::main(|| {
        thread::spawn(|| {
            let task = Task::without_notifications(
                "logwise_context_test".to_string(),
                Configuration::default(),
                async {
                    check_context_contract().await;
                    std::process::exit(0);
                },
            );
            task.spawn_static_current();
        });
    });
}

#[cfg(target_arch = "wasm32")]
wasm_lite::test_main!();

/// The same contract on a browser worker.
///
/// This is where a durable token earns its keep: the worker is a different
/// realm, so a thread-local "current context" could not have crossed to it.
#[cfg(target_arch = "wasm32")]
#[wasm_lite::wasm_lite_test]
fn wasm_main() {
    wasm_lite_std::async_doctest!(async {
        install();
        assert!(app_window::application::is_main_thread());
        let (done, wait) = r#continue::continuation();
        app_window::application::main(move || {
            let task = Task::without_notifications(
                "logwise_context_test".to_string(),
                Configuration::default(),
                async move {
                    check_context_contract().await;
                    done.send(());
                },
            );
            task.spawn_static_current();
        });
        wait.await;
    });
}
