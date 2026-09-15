// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! MoQ object instrumentation for the relay and its peers.
//!
//! With the `trace` feature every call forwards to `moq-trace`, which records
//! `moq_trace:*` events under LTTng. Without the feature the same calls compile
//! to no-ops, so a default build carries no tracing code and resolves no extra
//! dependency.

#[cfg(feature = "trace")]
pub(crate) use moq_trace::{
    Direction, Handle, LogicalId, ObjectContext, ObjectIdentity, ObjectOutcome, ObjectPhase,
};

/// Allocate a process-unique identity for one group of ingested objects.
///
/// Wire fields cannot identify a logical object across the relay: the track
/// alias is scoped to a session, so the alias an object arrives under differs
/// from the alias it leaves under. The receiver allocates this before it reads
/// the object header and stores it on the object, so the inbound trace and every
/// outbound copy agree.
#[cfg(feature = "trace")]
pub(crate) fn next_group_instance() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_GROUP_INSTANCE: AtomicU64 = AtomicU64::new(1);
    NEXT_GROUP_INSTANCE.fetch_add(1, Ordering::Relaxed)
}

/// Return the process-global handle shared by every instrumented session.
#[cfg(feature = "trace")]
pub(crate) fn global() -> Handle {
    moq_trace::global()
}

#[cfg(not(feature = "trace"))]
mod disabled;
#[cfg(not(feature = "trace"))]
#[allow(unused_imports)]
pub(crate) use disabled::{
    global, next_group_instance, Direction, Handle, LogicalId, ObjectContext, ObjectIdentity,
    ObjectOutcome, ObjectPhase,
};
