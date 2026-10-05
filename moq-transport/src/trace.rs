// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! MoQ object instrumentation for the relay and its peers.
//!
//! Every call forwards to `moq-trace`. With the `trace` feature it records
//! `moq_trace:*` events under LTTng. Without it the toolkit's handles are
//! disabled, so the same calls do no work. The toolkit is always linked, which
//! keeps this crate on the real API rather than a stand-in that could drift.

pub(crate) use moq_trace::{
    global, now_ns, Direction, Handle, LogicalId, ObjectContext, ObjectIdentity, ObjectOutcome,
    ObjectPhase, ObjectTrace,
};

/// Run a step that makes received data readable in the relay model.
///
/// The model wakes waiting readers inside the step, when a modified state is
/// released or a writer half drops. Each wake is recorded as
/// [`ObjectPhase::Notify`], and the rest of the step as `phase`, so the step
/// becomes alternating occurrences that never overlap. The last occurrence
/// carries the step's outcome. Phases are emitted after the step returns, so
/// emission adds nothing to the measured intervals.
#[cfg(feature = "trace")]
pub(crate) fn publish<T, E>(
    object: &mut ObjectTrace,
    phase: ObjectPhase,
    step: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    if !object.records_phases() {
        return step();
    }
    let start_ns = now_ns();
    let (result, wakes) = crate::watch::probe::observe(now_ns, step);
    let end_ns = now_ns();
    let mut cursor = start_ns;
    for &(wake_start, wake_end) in wakes.intervals() {
        object
            .phase_at(phase, cursor)
            .finish_at(ObjectOutcome::Success, wake_start);
        object
            .phase_at(ObjectPhase::Notify, wake_start)
            .finish_at(ObjectOutcome::Success, wake_end);
        cursor = wake_end;
    }
    let outcome = match result {
        Ok(_) => ObjectOutcome::Success,
        Err(_) => ObjectOutcome::Failed,
    };
    object.phase_at(phase, cursor).finish_at(outcome, end_ns);
    result
}

/// Run a step that makes received data readable; untraced builds record nothing.
#[cfg(not(feature = "trace"))]
pub(crate) fn publish<T, E>(
    _object: &mut ObjectTrace,
    _phase: ObjectPhase,
    step: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    step()
}

/// The trace clock reading at which an inbound object became readable.
///
/// Outbound copies start their delivery wait here. Only traced builds record
/// the instant, so untraced ones never read the clock on the per-object path.
#[cfg(feature = "trace")]
pub(crate) fn readable_ns() -> u64 {
    now_ns()
}

/// Allocate a process-unique identity for one group of ingested objects.
///
/// Wire fields cannot identify a logical object across the relay: the track
/// alias is scoped to a session, so the alias an object arrives under differs
/// from the alias it leaves under. The receiver allocates this before it reads
/// the object header and stores it on the object, so the inbound trace and every
/// outbound copy agree. The toolkit owns the counter, so the group stays unique
/// even alongside other instrumented code in the process.
///
/// Without the `trace` feature nothing reads the identity, so this skips the
/// shared counter on the per-object path.
pub(crate) fn next_group_instance() -> u64 {
    #[cfg(feature = "trace")]
    {
        moq_trace::next_logical_group()
    }
    #[cfg(not(feature = "trace"))]
    {
        0
    }
}
