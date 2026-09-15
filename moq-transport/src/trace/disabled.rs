// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! No-op stand-ins for the `moq-trace` types used at instrumentation sites.
//!
//! These mirror the real signatures, borrows included, so instrumentation code
//! compiles the same way whether or not the `trace` feature is enabled.

#![allow(dead_code)]

use std::marker::PhantomData;

/// Stand-in for the process-global tracing handle.
#[derive(Clone, Default)]
pub(crate) struct Handle {
    _private: (),
}

/// Direction of an object across the relay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Direction {
    /// An object arriving from the network.
    Rx,
    /// An object leaving for the network.
    Tx,
}

/// Stand-in for the process-unique logical object identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LogicalId {
    _private: (),
}

impl LogicalId {
    /// Create a logical identity from a group instance and frame ordinal.
    pub(crate) fn new(_group: u64, _frame: u64) -> Self {
        Self { _private: () }
    }
}

/// Stand-in for the wire identity of one object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObjectIdentity {
    _private: (),
}

impl ObjectIdentity {
    /// Create a wire identity from its track alias, group ID, and object ID.
    pub(crate) fn new(_track_alias: u64, _group_id: u64, _object_id: u64) -> Self {
        Self { _private: () }
    }
}

/// Stand-in for the metadata known before an object trace starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObjectContext {
    _private: (),
}

impl ObjectContext {
    /// Create object metadata from its wire identity and logical relay identity.
    pub(crate) fn new(
        _direction: Direction,
        _identity: ObjectIdentity,
        _logical_id: LogicalId,
    ) -> Self {
        Self { _private: () }
    }

    /// Attach the transport stream identifier.
    pub(crate) fn with_stream_id(self, _stream_id: u64) -> Self {
        self
    }

    /// Attach the inclusive stream byte offset where this object starts.
    pub(crate) fn with_stream_offset_start(self, _offset_start: u64) -> Self {
        self
    }

    /// Attach a payload size known before tracing starts.
    pub(crate) fn with_payload_bytes(self, _payload_bytes: u64) -> Self {
        self
    }
}

/// A measured step in the object lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObjectPhase {
    /// Parse an inbound object header.
    HeaderParse,
    /// Create an inbound object in the relay model.
    Create,
    /// Read an inbound object payload.
    PayloadRead,
    /// Commit an inbound frame to the relay model.
    FrameCommit,
    /// Clone or select an outbound object from the relay model.
    Clone,
    /// Encode an outbound object header.
    HeaderEncode,
    /// Write an outbound object payload.
    PayloadWrite,
}

/// Result of an object lifecycle or phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObjectOutcome {
    /// Processing completed successfully.
    Success,
    /// Processing completed with an error.
    Failed,
}

/// Stand-in for an object trace.
#[must_use]
pub(crate) struct ObjectTrace {
    _private: (),
}

impl ObjectTrace {
    /// Update the object payload size once it is known.
    pub(crate) fn set_payload_bytes(&mut self, _payload_bytes: u64) {}

    /// Update the exclusive stream byte offset reached by this object.
    pub(crate) fn set_stream_offset_end(&mut self, _stream_offset_end: u64) {}

    /// Start a measured object lifecycle phase.
    pub(crate) fn phase(&mut self, _phase: ObjectPhase) -> ObjectPhaseTrace<'_> {
        ObjectPhaseTrace {
            _object: PhantomData,
        }
    }

    /// Finish the object interval with the latest metadata and result.
    pub(crate) fn finish(self, _outcome: ObjectOutcome) {}
}

/// Stand-in for a scoped object phase.
#[must_use]
pub(crate) struct ObjectPhaseTrace<'a> {
    _object: PhantomData<&'a mut ()>,
}

impl ObjectPhaseTrace<'_> {
    /// Update the object payload size once it is known.
    pub(crate) fn set_payload_bytes(&mut self, _payload_bytes: u64) {}

    /// Update the exclusive stream byte offset reached by this object.
    pub(crate) fn set_stream_offset_end(&mut self, _stream_offset_end: u64) {}

    /// Finish the phase with an explicit result.
    pub(crate) fn finish(self, _outcome: ObjectOutcome) {}
}

impl Handle {
    /// Return the process-global handle shared by every instrumented session.
    pub(crate) fn global() -> Self {
        Self::default()
    }

    /// Return a clone that stamps object events with the next process-local session ID.
    pub(crate) fn with_new_session_id(self) -> Self {
        self
    }

    /// Return a clone that stamps object events with this transport connection ID.
    pub(crate) fn with_connection_id(self, _connection_id: u64) -> Self {
        self
    }

    /// Start a MoQ object trace.
    pub(crate) fn object(&self, _context: ObjectContext) -> ObjectTrace {
        ObjectTrace { _private: () }
    }
}

/// Return the process-global handle shared by every instrumented session.
pub(crate) fn global() -> Handle {
    Handle::default()
}

/// Allocate a process-unique identity for one group of ingested objects.
pub(crate) fn next_group_instance() -> u64 {
    0
}
