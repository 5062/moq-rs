// SPDX-FileCopyrightText: 2024-2026 Cloudflare Inc., Luke Curley, Mike English and contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! MoQ object instrumentation for the relay and its peers.
//!
//! Every call forwards to `moq-trace`. With the `trace` feature it records
//! `moq_trace:*` events under LTTng. Without it the toolkit's handles are
//! disabled, so the same calls do no work. The toolkit is always linked, which
//! keeps this crate on the real API rather than a stand-in that could drift.

pub(crate) use moq_trace::{
    now_ns, transport_call, Direction, Handle, LogicalId, ObjectContext, ObjectIdentity,
    ObjectOutcome, ObjectPhase, ObjectTrace, StreamSetup,
};

/// Return the handle a session records object events through.
///
/// Object identity and stream offsets are compiled only with this crate's `trace`
/// feature, but emission follows `moq-trace/lttng`, which another crate in the
/// build can turn on alone. Without `trace` a session therefore gets a disabled
/// handle, so it never emits objects that lack the logical identity and stream
/// position the analysis joins on. This crate's feature is the one switch.
#[cfg(feature = "trace")]
pub(crate) fn session_handle(session: &web_transport::Session) -> Handle {
    let handle = moq_trace::global().with_session_id(moq_trace::next_session_id());
    // The transport connection identity is what ties an object to the packets
    // that carried it, so tag it when the transport reports one.
    match session.connection_id() {
        Some(connection_id) => handle.with_connection_id(connection_id.into_inner()),
        None => handle,
    }
}

#[cfg(not(feature = "trace"))]
pub(crate) fn session_handle(_session: &web_transport::Session) -> Handle {
    Handle::disabled()
}

/// Allocate a process-unique logical group for one group of ingested objects.
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
pub(crate) fn next_logical_group() -> u64 {
    #[cfg(feature = "trace")]
    {
        moq_trace::next_logical_group()
    }
    #[cfg(not(feature = "trace"))]
    {
        0
    }
}

/// How an object ended, given the session error that stopped it.
///
/// A read or write error on the stream itself means the peer stopped or reset
/// it, or it was already closed, and a subscription that ended under the copy
/// stops it too, so both end the object as reset. Every other error is a
/// failure. This relay has no cache eviction or delivery deadline, so it never
/// ends an object as dropped or expired.
pub(crate) fn outcome_of(err: &crate::session::SessionError) -> ObjectOutcome {
    use crate::session::SessionError;
    match err {
        SessionError::WebTransport(
            web_transport::Error::Write(_) | web_transport::Error::Read(_),
        ) => ObjectOutcome::Reset,
        SessionError::Serve(err) => serve_outcome_of(err),
        _ => ObjectOutcome::Failed,
    }
}

/// How an object ended, given the serve error that stopped it.
pub(crate) fn serve_outcome_of(err: &crate::serve::ServeError) -> ObjectOutcome {
    use crate::serve::ServeError;
    match err {
        ServeError::Cancel | ServeError::Closed(_) | ServeError::Done => ObjectOutcome::Reset,
        _ => ObjectOutcome::Failed,
    }
}

/// Ends an object with the outcome of the error that stopped it.
pub(crate) trait FinishOnError {
    /// Finish `object` from the error, if there is one, then hand the result back.
    ///
    /// Write it as `step.await.finish_on_error(&mut object)?`, so an object an
    /// early return leaves behind records why it ended instead of being abandoned.
    fn finish_on_error(self, object: &mut ObjectTrace) -> Self;
}

impl<T> FinishOnError for Result<T, crate::session::SessionError> {
    fn finish_on_error(self, object: &mut ObjectTrace) -> Self {
        if let Err(err) = &self {
            std::mem::replace(object, ObjectTrace::disabled()).finish(outcome_of(err));
        }
        self
    }
}

impl<T> FinishOnError for Result<T, crate::serve::ServeError> {
    fn finish_on_error(self, object: &mut ObjectTrace) -> Self {
        if let Err(err) = &self {
            std::mem::replace(object, ObjectTrace::disabled()).finish(serve_outcome_of(err));
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::ServeError;
    use crate::session::SessionError;

    #[test]
    fn errors_map_to_the_outcome_that_describes_them() {
        assert_eq!(serve_outcome_of(&ServeError::Cancel), ObjectOutcome::Reset);
        assert_eq!(serve_outcome_of(&ServeError::Done), ObjectOutcome::Reset);
        assert_eq!(serve_outcome_of(&ServeError::Size), ObjectOutcome::Failed);
        assert_eq!(
            outcome_of(&SessionError::Serve(ServeError::Closed(3))),
            ObjectOutcome::Reset
        );
        assert_eq!(outcome_of(&SessionError::WrongSize), ObjectOutcome::Failed);
    }
}
