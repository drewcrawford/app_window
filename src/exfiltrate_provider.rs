// SPDX-License-Identifier: MPL-2.0

//! Exposes window and main-thread state through exfiltrate's `snapshot` command.
//!
//! Call [`install`] once, after `exfiltrate::begin()`. It registers two
//! subsystems:
//!
//! * `windows` -- every window this process created: what was asked for, where
//!   from, whether a surface is attached, the geometry the application last
//!   observed, and whether it is still open.
//! * `main_thread` -- whether the platform event loop is pumping, and if it is
//!   not, for how long it has not been.
//!
//! Two subsystems rather than one because they answer different questions and
//! fail independently: the window registry can be locked while the main thread
//! is fine, and the main thread can be wedged while the registry answers
//! instantly.
//!
//! # Why `main_thread` does not wait for the main thread
//!
//! The obvious implementation submits a closure and awaits it. That
//! implementation is blocked by exactly the condition it exists to report, and
//! it takes the debug server down with it -- the one thing that still has to
//! work when the application is wedged. A provider is also synchronous and
//! contractually non-blocking.
//!
//! So the probe is fire-and-forget and the measurement is its *age*. The first
//! snapshot starts one, and every snapshot reports the state of the outstanding
//! one. A wedged main thread therefore reads as a probe that has been
//! outstanding for N seconds and is climbing, which is more informative than
//! any timeout, and an idle one reads as a probe that came back in under a
//! millisecond.

use exfiltrate::provider::{Provider, ProviderResult, Row, SnapshotRequest};

use crate::instrument;
use crate::registry;

/// How long an outstanding probe has to be before this is reported as a
/// problem rather than a number.
///
/// From the crate's own semantics: not getting onto the main thread within a
/// few seconds is a bug -- somebody is holding it -- so it is reported as one.
const WEDGED_AFTER: std::time::Duration = std::time::Duration::from_secs(3);

/// Registers the `windows` and `main_thread` subsystems. Idempotent.
///
/// ```no_run
/// exfiltrate::begin();
/// app_window::exfiltrate_provider::install();
/// ```
pub fn install() {
    exfiltrate::provider::add_provider(Windows);
    exfiltrate::provider::add_provider(MainThread);
}

struct Windows;

impl Provider for Windows {
    fn subsystem(&self) -> &'static str {
        "windows"
    }

    fn description(&self) -> &'static str {
        "windows this process created: requested geometry, attached surface, last observed size and scale"
    }

    fn snapshot(&self, request: &SnapshotRequest<'_>) -> ProviderResult {
        let wanted: Option<u64> = match request.selector() {
            Some(selector) => match selector.parse::<u64>() {
                Ok(id) => Some(id),
                Err(_) => {
                    return ProviderResult::Unavailable(format!(
                        "--id must be a window id; got {selector:?}"
                    ));
                }
            },
            None => None,
        };

        let (Some(entries), Some((dropped, capacity))) = (registry::entries(), registry::stats())
        else {
            return ProviderResult::Busy;
        };

        let mut rows = Vec::new();
        for entry in entries {
            if request.should_stop() {
                return ProviderResult::Partial(rows, "deadline".to_string());
            }
            if let Some(id) = wanted
                && entry.id != id
            {
                continue;
            }

            let mut row = Row::new()
                .support("id", entry.id)
                .support("origin", entry.origin.name())
                .support("open", entry.is_open())
                .support("surface_attached", entry.surface_attached)
                .support("age_ms", entry.created_at.elapsed().as_millis() as u64);

            // A title is whatever the application chose to display, which can
            // be a filename or a customer name. Local-only.
            if !entry.title.is_empty() {
                row = row.local("title", entry.title.as_str());
            }
            if let Some(closed_at) = entry.closed_at {
                row = row.support("closed_ms_ago", closed_at.elapsed().as_millis() as u64);
            }
            if let Some((position, size)) = entry.requested {
                row = row
                    .support("requested_x", position.x())
                    .support("requested_y", position.y())
                    .support("requested_width", size.width())
                    .support("requested_height", size.height());
            }
            if let Some((size, scale, observed_at)) = entry.last_observed {
                // Reported with its age on purpose. This is the last value the
                // *application* asked for, not the platform's answer right now,
                // and a caller comparing it against a renderer's belief has to
                // know how old it is.
                row = row
                    .support("observed_width", size.width())
                    .support("observed_height", size.height())
                    .support("observed_scale", scale)
                    .support("observed_ms_ago", observed_at.elapsed().as_millis() as u64);
            }
            rows.push(row);
        }

        if dropped > 0 {
            return ProviderResult::Partial(
                rows,
                format!(
                    "{dropped} record(s) evicted at a capacity of {capacity}; \
                     raise APP_WINDOW_REGISTRY_CAPACITY"
                ),
            );
        }
        ProviderResult::Rows(rows)
    }
}

struct MainThread;

impl Provider for MainThread {
    fn subsystem(&self) -> &'static str {
        "main_thread"
    }

    fn description(&self) -> &'static str {
        "platform event loop: whether it is pumping, and how long it has not been"
    }

    fn snapshot(&self, request: &SnapshotRequest<'_>) -> ProviderResult {
        if let Some(selector) = request.selector() {
            return ProviderResult::Unavailable(format!(
                "there is one main thread; --id {selector:?} selects nothing"
            ));
        }

        // Read first, then start the next probe. Reading after would report a
        // probe that has existed for zero milliseconds every single time.
        let stats = instrument::stats();
        instrument::probe();

        if !stats.running {
            return ProviderResult::Unavailable(
                "app_window::application::main has not been called; there is no event loop"
                    .to_string(),
            );
        }

        let mut row = Row::new()
            .support("running", stats.running)
            .support("submitted", stats.submitted)
            .support("completed", stats.completed)
            .support("outstanding", stats.outstanding)
            .support("overran_10ms", stats.overran)
            .support("slowest_turn_ms", stats.slowest_ms);

        if let Some(since) = stats.since_last_turn {
            row = row.support("last_turn_ms_ago", since.as_millis() as u64);
        }
        if let Some(round_trip) = stats.last_probe_round_trip {
            row = row.support("probe_round_trip_ms", round_trip.as_millis() as u64);
        }

        match stats.probe_outstanding {
            None => {
                // Either a probe came back, or this is the first snapshot and
                // the one just started has not been answered yet. Both are
                // "nothing is stuck", and the next snapshot distinguishes them.
                row = row.support("responding", true);
                ProviderResult::Rows(vec![row])
            }
            Some(waiting) if waiting < WEDGED_AFTER => {
                row = row
                    .support("responding", true)
                    .support("probe_waiting_ms", waiting.as_millis() as u64);
                ProviderResult::Rows(vec![row])
            }
            Some(waiting) => {
                row = row
                    .support("responding", false)
                    .support("probe_waiting_ms", waiting.as_millis() as u64);
                // Reported as a defect, not a number: something is holding the
                // main thread, the UI is frozen, and the row alone would read
                // as ordinary output.
                ProviderResult::Partial(
                    vec![row],
                    format!(
                        "the main thread has not run a submitted closure for {}ms; \
                         something is holding it",
                        waiting.as_millis()
                    ),
                )
            }
        }
    }
}
