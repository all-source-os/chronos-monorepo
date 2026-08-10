//! Test-only probe that counts how many `Event`s a call materialized.
//!
//! Some invariants in this crate are about *work*, not about results: a bounded
//! `/events/query` page must not clone an entity's whole history just to return
//! one event (issue #251). Result-shaped assertions cannot see that — `count ==
//! 1` holds both when the store clones one event and when it clones ten
//! thousand and throws 9 999 away. Core's `query_results_total` metric has the
//! same blind spot: it is incremented with `results.len()`, i.e. rows
//! *returned*, so it reads 1 either way.
//!
//! Under `cfg(test)` `Event` gets a hand-written `Clone` that calls
//! [`record`], and [`measure`] reports the clones a closure caused on the
//! calling thread. Because the counter lives in `Event::clone` — not inside the
//! code under test — a guard written against it goes red the moment
//! `EventStore::query_window` goes back to cloning every match and truncating.
//! The counter is thread-local, so tests running in parallel (and background
//! WAL/flush threads) cannot pollute a measurement.

use std::cell::Cell;

thread_local! {
    /// `Event::clone` calls on this thread.
    static CLONES: Cell<u64> = const { Cell::new(0) };
}

/// Called by `Event::clone` (test builds only).
#[inline]
pub(crate) fn record() {
    // `try_with` (not `with`) so a clone during thread-local teardown cannot
    // panic.
    let _ = CLONES.try_with(|c| c.set(c.get().saturating_add(1)));
}

/// Number of `Event` clones recorded on this thread so far.
pub(crate) fn count() -> u64 {
    CLONES.get()
}

/// Run `f`, returning its value and how many `Event`s it cloned on this thread.
pub(crate) fn measure<T>(f: impl FnOnce() -> T) -> (T, u64) {
    let before = count();
    let out = f();
    (out, count() - before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;

    fn event() -> Event {
        Event::from_strings(
            "probe.checked".to_string(),
            "e-1".to_string(),
            "default".to_string(),
            serde_json::json!({}),
            None,
        )
        .unwrap()
    }

    #[test]
    fn measure_counts_event_clones() {
        let e = event();
        let (_, none) = measure(|| e.id);
        assert_eq!(none, 0, "not cloning must record nothing");

        let (_, three) = measure(|| vec![e.clone(), e.clone(), e.clone()]);
        assert_eq!(three, 3, "one increment per Event::clone");

        // Cloning a Vec<Event> clones each element.
        let batch = vec![event(), event(), event(), event()];
        let (_, four) = measure(|| batch.clone());
        assert_eq!(four, 4);
    }

    #[test]
    fn cloned_events_are_faithful_copies() {
        // The hand-written test-only Clone must be a real clone, not a
        // near-copy that silently drops a field.
        let e = event();
        let copy = e.clone();
        assert_eq!(copy, e);
    }
}
