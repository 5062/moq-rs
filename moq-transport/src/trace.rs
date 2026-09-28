// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! MoQ object instrumentation for the relay and its peers.
//!
//! Every call forwards to `moq-trace`. With the `trace` feature it records
//! `moq_trace:*` events under LTTng. Without it the toolkit's handles are
//! disabled, so the same calls do no work. The toolkit is always linked, which
//! keeps this crate on the real API rather than a stand-in that could drift.

pub(crate) use moq_trace::{
    global, Direction, Handle, LogicalId, ObjectContext, ObjectIdentity, ObjectOutcome, ObjectPhase,
};

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
