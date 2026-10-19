//! The bounded, in-memory diagnostics sink and the process-wide instance the command reads.
//!
//! [`DiagnosticSink`] is a ring: it holds at most its capacity, and recording into a full ring drops
//! the oldest record. [`SINK_CAPACITY`] fixes the capacity the process-wide sink uses, and the module
//! documentation of [`super`] states why it is 64 and what that bound buys.
//!
//! **One sink per process, and it is the only one.** [`record`] and [`summary`] are the whole
//! production API: every recorder writes into the same instance and `get_diagnostics_summary`
//! (task 2.12) reads it. The static is private, so no caller can install a second sink or hold a
//! reference to this one past a call — which is what keeps "the sink is bounded" a property of the
//! process rather than of one caller's discipline. The sink opens no file, holds no socket and has no
//! transport of any kind, so a record cannot leave the device (NFR-3).
//!
//! **The lock is never held across an `.await`.** The sink lives behind a `std::sync::Mutex` and both
//! entry points take the guard, do one in-memory operation and drop it, which is exactly the use of a
//! synchronous mutex `rust-conventions` allows. A poisoned lock — a panic in another thread while the
//! guard was held — is reported as [`SinkError::Poisoned`] rather than silently recovered from:
//! diagnostics is a value the code vouches for, and a lock a panic abandoned is exactly the state in
//! which it cannot.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{LazyLock, Mutex};

use crate::ipc::types::DiagnosticsSummary;

use super::records::DiagnosticRecord;

/// How many records the process-wide sink retains.
///
/// The bound is a deliberate choice, not an incidental array size. Sixty-four is far more than a
/// session's ordinary handful of save failures, probes and loads, so the summary describes a recent
/// window rather than a single event; and it is small enough that the whole sink is a few kilobytes —
/// nothing that needs persisting, ageing out, or evicting by anything but the ring itself. Recording
/// into a full sink drops its **oldest** record, so the newest outcome — the one a support read most
/// wants — is never the one lost.
pub const SINK_CAPACITY: usize = 64;

/// A bounded, in-memory ring of redacted records (NFR-3).
///
/// The ring keeps insertion order and holds at most [`DiagnosticSink::capacity`] records; see
/// [`DiagnosticSink::record`] for the eviction rule and the module documentation of [`super`] for why
/// the process-wide capacity is [`SINK_CAPACITY`]. The record list is private so that recording is
/// the only way in: a caller cannot install a record out of order and cannot reach past the bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSink {
    /// The held records, oldest first. A `VecDeque` because the bound is enforced by dropping the
    /// front, which a deque does without shifting the rest.
    records: VecDeque<DiagnosticRecord>,
    /// The most records the sink may hold. Zero means the sink holds nothing.
    capacity: usize,
}

impl DiagnosticSink {
    /// A sink holding at most `capacity` records.
    ///
    /// A capacity of zero is a sink that holds nothing: every record handed to it is dropped. That is
    /// allowed rather than rejected so a caller can disable the sink without special-casing it, and
    /// it keeps [`DiagnosticSink::summary`] total in every state.
    pub fn new(capacity: usize) -> Self {
        Self {
            records: VecDeque::new(),
            capacity,
        }
    }

    /// How many records the sink can hold before it starts dropping the oldest.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many records the sink currently holds.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the sink holds no record.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Record one redacted record, dropping the oldest when the sink is full.
    ///
    /// The ring is the bound: a full sink loses its oldest record rather than growing, so the newest
    /// outcome — the one a support read most wants — is never the one dropped. A sink built with a
    /// capacity of zero drops every record.
    pub fn record(&mut self, record: DiagnosticRecord) {
        if self.capacity == 0 {
            return;
        }

        if self.records.len() == self.capacity {
            self.records.pop_front();
        }

        self.records.push_back(record);
    }

    /// The frozen summary of what the sink holds (NFR-3).
    ///
    /// Records are appended as they happen and the oldest is dropped first, so the front of the ring
    /// is the oldest held record and its back the newest. That is how the two instants are read
    /// without ordering them — comparing two [`Instant`](crate::ipc::types::Instant) values is task
    /// 2.8's concern, not this one's. An empty sink is reported honestly: no records, and `None` for
    /// both instants rather than a defaulted one.
    pub fn summary(&self) -> DiagnosticsSummary {
        DiagnosticsSummary {
            record_count: self.records.len() as u32,
            oldest_at: self.records.front().map(|record| record.at().clone()),
            newest_at: self.records.back().map(|record| record.at().clone()),
        }
    }

    /// Every held record, oldest first.
    pub fn records(&self) -> impl Iterator<Item = &DiagnosticRecord> {
        self.records.iter()
    }
}

impl Default for DiagnosticSink {
    fn default() -> Self {
        Self::new(SINK_CAPACITY)
    }
}

/// The process-wide sink every recorder writes into and `get_diagnostics_summary` reads.
static SINK: LazyLock<Mutex<DiagnosticSink>> =
    LazyLock::new(|| Mutex::new(DiagnosticSink::default()));

/// Record one redacted record into the process-wide sink (NFR-3).
///
/// This is the only way a recorder reaches the sink: the static is private, so there is one sink per
/// process and no caller can replace it or reach past its bound. A record that could not be recorded
/// — the lock was poisoned by a panic in another thread — is reported rather than swallowed; the
/// caller decides whether that matters, and diagnostics is never worth failing a product operation
/// over.
pub fn record(diagnostic: DiagnosticRecord) -> Result<(), SinkError> {
    let mut sink = SINK.lock().map_err(|_| SinkError::Poisoned)?;
    sink.record(diagnostic);

    Ok(())
}

/// The redacted summary of the process-wide sink (NFR-3).
///
/// Counts and the two timestamps only; the records themselves never leave the process. An empty sink
/// reports zero records and `None` for both instants, so "nothing has been recorded" is distinct from
/// any recorded instant — the same distinction `ServiceStatus` draws with its optional fields (R3).
pub fn summary() -> Result<DiagnosticsSummary, SinkError> {
    let sink = SINK.lock().map_err(|_| SinkError::Poisoned)?;

    Ok(sink.summary())
}

/// Why the process-wide sink could not be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkError {
    /// A thread panicked while holding the sink's lock, so the records it held cannot be vouched for.
    Poisoned,
}

impl fmt::Display for SinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned => formatter.write_str("the diagnostics sink lock was poisoned"),
        }
    }
}

impl std::error::Error for SinkError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::records::{DiagnosticOutcome, SAVE_FAILED_KEYS, record_keys};
    use crate::ipc::types::{CommandError, Instant, RequestId, ServiceState};

    /// The instants are opaque to this module — ordering two of them is task 2.8's — so a test only
    /// ever compares a summary's instant with the value it supplied.
    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn probed(value: &str, state: ServiceState) -> DiagnosticRecord {
        DiagnosticRecord::reachability_probed(at(value), state)
    }

    #[test]
    fn an_empty_sink_summarises_to_no_records_and_no_timestamps() {
        let sink = DiagnosticSink::new(4);

        let summary = sink.summary();

        assert_eq!(summary.record_count, 0);
        assert_eq!(summary.oldest_at, None);
        assert_eq!(summary.newest_at, None);
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn the_default_sink_uses_the_documented_bound() {
        let sink = DiagnosticSink::default();

        assert_eq!(sink.capacity(), SINK_CAPACITY);
        assert_eq!(sink.capacity(), 64);
    }

    #[test]
    fn the_summary_names_the_oldest_and_newest_held_record() {
        let mut sink = DiagnosticSink::new(8);
        let older = probed("2026-10-19T10:00:00Z", ServiceState::Unknown);
        let newer = probed("2026-10-19T11:00:00Z", ServiceState::Reachable);

        sink.record(older);
        sink.record(newer);

        let summary = sink.summary();

        assert_eq!(summary.record_count, 2);
        assert_eq!(summary.oldest_at, Some(at("2026-10-19T10:00:00Z")));
        assert_eq!(summary.newest_at, Some(at("2026-10-19T11:00:00Z")));
        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());
    }

    #[test]
    fn a_full_sink_drops_its_oldest_record_rather_than_growing() {
        let mut sink = DiagnosticSink::new(2);
        let dropped = probed("2026-10-19T10:00:00Z", ServiceState::Unknown);
        let kept = probed("2026-10-19T11:00:00Z", ServiceState::Reachable);
        let newest = probed("2026-10-19T12:00:00Z", ServiceState::Unreachable);

        sink.record(dropped);
        sink.record(kept);
        sink.record(newest);

        let summary = sink.summary();

        assert_eq!(sink.len(), 2);
        assert_eq!(summary.record_count, 2);
        assert_eq!(summary.oldest_at, Some(at("2026-10-19T11:00:00Z")));
        assert_eq!(summary.newest_at, Some(at("2026-10-19T12:00:00Z")));
    }

    #[test]
    fn a_sink_of_capacity_zero_holds_nothing() {
        let mut sink = DiagnosticSink::new(0);
        let entry = probed("2026-10-19T10:00:00Z", ServiceState::Reachable);

        sink.record(entry);

        let summary = sink.summary();

        assert!(sink.is_empty());
        assert_eq!(summary.record_count, 0);
        assert_eq!(summary.oldest_at, None);
    }

    #[test]
    fn a_save_failure_records_the_declared_keys_and_its_cause() {
        // What this test is, stated exactly: a **regression guard on the recorded shape**, not a
        // demonstration that a payload was redacted. The P1 health payload a save would carry — a
        // rabbit's name, a note's text, a photo's bytes and a weight — is named here to fix the
        // scenario NFR-3 forbids, and it is deliberately never handed to the code under test, because
        // there is nowhere to hand it: the sink's only entry point takes a `DiagnosticRecord`, and the
        // only save-failure constructor takes the moment, the operation's identifier and the failure's
        // cause. An assertion that a value with no path in is absent from the result would be
        // unfalsifiable, so this test does not make one. What it does instead has power: it fails the
        // moment the recorded value grows a key a payload could sit in, or stops reporting the cause it
        // was given.
        let failure = CommandError::connectivity(Some(RequestId("request-1".to_owned())));
        let entry = DiagnosticRecord::save_failed(
            at("2026-10-19T10:00:00Z"),
            RequestId("request-1".to_owned()),
            failure.cause,
        );

        let mut sink = DiagnosticSink::new(4);
        sink.record(entry);

        let held = sink.records().next().expect("the record is held");

        // The recorded value's key vocabulary is exactly the declared one for a save failure...
        assert_eq!(record_keys(held), SAVE_FAILED_KEYS);

        // ...and its outcome is `SaveFailed { cause }`, carrying the cause that was reported.
        let cause = match held.outcome() {
            DiagnosticOutcome::SaveFailed { cause } => cause,
            other => panic!("a save failure recorded {other:?}"),
        };

        assert_eq!(cause, &failure.cause);

        // Redacted, not empty: the operation's own identifier is what the record carries, and the sink
        // holds exactly the one record that was recorded into it.
        assert_eq!(sink.len(), 1);
        assert!(format!("{sink:?}").contains("request-1"));
    }

    #[test]
    fn the_process_wide_sink_reports_what_was_recorded_into_it() {
        let entry = probed("2026-10-19T10:00:00Z", ServiceState::Reachable);

        record(entry).expect("the process-wide sink is writable");

        let summary = summary().expect("the process-wide sink is readable");

        assert!(summary.record_count >= 1);
        assert!(summary.oldest_at.is_some());
        assert!(summary.newest_at.is_some());
    }
}
