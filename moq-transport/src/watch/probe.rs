// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Timing of the reader wakes that state modifications trigger, for tracing.
//!
//! A tracer wraps a call in [`observe`] to learn when, inside it, a [`State`]
//! woke the readers waiting on it. Outside [`observe`] the probe reads no clock,
//! so an untraced modification costs one thread-local check.
//!
//! [`State`]: super::State

use std::cell::RefCell;

/// Wake intervals one [`Wakes`] holds before merging later ones into its last.
const CAPACITY: usize = 4;

/// The wake intervals recorded during one [`observe`] call.
///
/// Each interval is a `(start, end)` pair of readings from the clock the caller
/// passed, in the order the wakes ran. Wakes beyond the capacity extend the last
/// interval to the end of the latest one, so the intervals always cover every
/// wake and never overlap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Wakes {
    intervals: [(u64, u64); CAPACITY],
    len: usize,
}

impl Wakes {
    /// The recorded intervals, in the order the wakes ran.
    pub(crate) fn intervals(&self) -> &[(u64, u64)] {
        &self.intervals[..self.len]
    }

    fn push(&mut self, start: u64, end: u64) {
        if self.len < CAPACITY {
            self.intervals[self.len] = (start, end);
            self.len += 1;
        } else {
            self.intervals[CAPACITY - 1].1 = end;
        }
    }
}

struct Recording {
    clock: fn() -> u64,
    wakes: Wakes,
}

thread_local! {
    static RECORDING: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

/// Restores the enclosing recording, even when the observed call unwinds.
struct Restore(Option<Option<Recording>>);

impl Drop for Restore {
    fn drop(&mut self) {
        if let Some(previous) = self.0.take() {
            RECORDING.with(|recording| *recording.borrow_mut() = previous);
        }
    }
}

/// Run `f`, recording the wakes it triggers on this thread with `clock`.
///
/// Only wakes that run synchronously inside `f` on the calling thread are
/// recorded. A nested call records into its own [`Wakes`], and the enclosing one
/// resumes afterwards without them.
pub(crate) fn observe<R>(clock: fn() -> u64, f: impl FnOnce() -> R) -> (R, Wakes) {
    let fresh = Recording {
        clock,
        wakes: Wakes::default(),
    };
    let mut restore = Restore(Some(
        RECORDING.with(|recording| recording.replace(Some(fresh))),
    ));
    let result = f();
    let previous = restore.0.take().expect("restored once");
    let recorded = RECORDING.with(|recording| recording.replace(previous));
    (
        result,
        recorded
            .map(|recording| recording.wakes)
            .unwrap_or_default(),
    )
}

/// Read the clock before a wake, when a recording is active on this thread.
pub(super) fn start() -> Option<u64> {
    RECORDING.with(|recording| {
        recording
            .borrow()
            .as_ref()
            .map(|recording| (recording.clock)())
    })
}

/// Record the wake that began at `start`, if [`start`] found a recording.
pub(super) fn finish(start: Option<u64>) {
    let Some(start) = start else { return };
    RECORDING.with(|recording| {
        if let Some(recording) = recording.borrow_mut().as_mut() {
            let end = (recording.clock)();
            recording.wakes.push(start, end);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TICKS: AtomicU64 = AtomicU64::new(0);

    fn tick() -> u64 {
        TICKS.fetch_add(1, Ordering::Relaxed) + 1
    }

    #[test]
    fn records_only_inside_observe() {
        finish(start());
        let ((), wakes) = observe(tick, || finish(start()));
        assert_eq!(wakes.intervals().len(), 1);
        let (began, ended) = wakes.intervals()[0];
        assert!(began < ended);
        finish(start());
        assert!(RECORDING.with(|recording| recording.borrow().is_none()));
    }

    #[test]
    fn extra_wakes_extend_the_last_interval() {
        let ((), wakes) = observe(tick, || {
            for _ in 0..CAPACITY + 2 {
                finish(start());
            }
        });
        assert_eq!(wakes.intervals().len(), CAPACITY);
        let last = wakes.intervals()[CAPACITY - 1];
        assert!(last.1 > last.0 + 1, "merged wakes span several readings");
    }

    #[test]
    fn modifying_state_records_its_wake() {
        let (writer, _reader) = super::super::State::<u32>::default().split();
        let ((), wakes) = observe(tick, || *writer.lock_mut().expect("open") += 1);
        assert_eq!(wakes.intervals().len(), 1);
    }
}
