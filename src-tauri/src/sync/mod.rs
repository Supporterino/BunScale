//! The sync boundary: the shared service as this app calls it, and the gate every write passes
//! through (SYNC-2, NFR-1; D0.1, D0.2, D0.6, D0.12, D0.13, D0.14, ADR 0002).
//!
//! `design.md`'s "State and boundaries" gives this directory the reachability gate and the refresh
//! orchestration, and gives `storage/` the device's copy of a household's collection: the copy is a
//! cache of the shared collection, never a competing record, and the refresh that replaces it goes
//! *through* here (ADR 0002 R1). Task 2.4 implements the **write** half and task 2.5 the **read**
//! half:
//!
//! | Module | What it owns |
//! |---|---|
//! | [`service`] | The seam, [`SharedService`]: what a call asks and what the transport reported, before any of it is classified (D0.7 criterion 4). |
//! | [`http`] | The real client: the pinned `reqwest` with rustls (D0.7's dependency note, D0.8). |
//! | [`double`] | The scripted double the seam exists for: P0 has no live deployment and the gate never boots the service. |
//! | [`probe`] | One fresh probe of the service *and* the account, as two separate checks, each bounded by [`REACHABILITY_PROBE_TIMEOUT`] (D0.7 criterion 4). |
//! | [`status`] | The fresh-outcome [`ServiceStatus`] surface the two status commands read, with D0.13's offline threshold applied in Rust. |
//! | [`gate`] | The gated save: the one place a write to the shared service is admitted. |
//! | [`read`] | The read path: the local copy first, a fresh-probe refresh, and the three SYNC-3 states (task 2.5). |
//! | [`merge`] | The last-write-wins merge: one surviving version per item, with losing versions ignored rather than failed (task 2.6). |
//! | [`lifecycle`] | The platform lifecycle's re-evaluation entry points: one fresh probe, recorded and published as `service:status-changed` (task 2.13). |
//!
//! [`ServiceStatus`]: crate::ipc::types::ServiceStatus
//!
//! # A write is attempted only when it can actually complete (D0.1, SYNC-2)
//!
//! There is no local write queue, no optimistic write and no deferred reconciliation. A write is
//! sent while the service is reachable and the account is valid, and otherwise it is **refused**,
//! with its own cause, before anything is sent. Nothing is written locally as if it were saved and
//! nothing completes later, so "saved" is only ever a statement about a write the service itself
//! accepted — the receipt type in [`gate`] cannot be built from any other outcome (SYNC-2, NFR-1).
//!
//! The decision is taken on a probe made *at the attempt*, never on a status carried over from an
//! earlier session (D0.2, R3). That is why the gate does not read [`status::StatusTracker`] at all:
//! the tracker is the product's *indicator* (NFR-1), the probe is the write path's admission check,
//! and keeping them apart is what makes a stale status unable to authorize a write.
//!
//! # Every refusal names one condition, and only one (D0.6, D0.12, D0.13)
//!
//! The failure surface is the contract's single [`CommandError`](crate::ipc::types::CommandError),
//! built only through `ipc::failure`'s constructors: this module classifies an outcome into a
//! [`SaveCause`](crate::ipc::types::SaveCause) and never chooses a message key or a retry flag of
//! its own. The mapping is the one D0.7 criterion 4 fixes:
//!
//! | Observed condition | Cause |
//! |---|---|
//! | no HTTP response at all (DNS, TLS, connect, read) | `connectivity` |
//! | `401` — the account is missing or invalid | `no_account` |
//! | the service's own allowance report in a `2xx` envelope | `allowance_exceeded` (D0.12) |
//! | `429` from the gateway | `rate_limited` |
//! | no outcome inside [`SAVE_ATTEMPT_TIMEOUT`] | `timeout` (D0.13) |
//! | the input was refused (`400`/`422`, or a blank operation identity) | `validation` |
//! | anything this boundary cannot read | `unknown` |
//!
//! # What is deliberately absent, and who owns it
//!
//! The refresh orchestration that decides *when* a collection is loaded or replaced is task 2.5's;
//! the last-write-wins comparator is 2.6's; the storage invariants are 2.7's; the absolute-instant
//! clock and its formatting are 2.8's (which is why nothing here reads a clock — an instant that
//! has to be recorded is supplied by the caller, exactly as `storage::local` does); the diagnostics
//! sink is 2.12's, and nothing here records into it, because P0 ships the sink with no production
//! recorder at all (Q37, 2026-10-21); and the two status commands plus the platform lifecycle hooks
//! are 2.13's, which consume [`probe`] and [`StatusTracker`] rather than re-deriving either.
//!
//! Task 2.5 implements that read path in [`read`]: the local copy is served first, a fresh probe
//! decides whether a refresh is attempted, and the guarded load replaces the copy. Task 2.13
//! implements the status half in [`lifecycle`]: one fresh probe, recorded on the tracker and
//! published as `service:status-changed`, for a foreground return on iOS, a window regaining focus
//! on macOS and the `check_service_reachability` command alike. The write-side rules above are
//! unchanged by both.

use std::time::Duration;

pub mod double;
pub mod gate;
pub mod http;
pub mod lifecycle;
pub mod merge;
pub mod probe;
pub mod read;
pub mod service;
pub mod status;

pub use double::ScriptedService;
pub use gate::{SaveGate, SaveReceipt};
pub use http::HttpService;
pub use lifecycle::{StatusPublisher, current_status, handle_run_event, re_evaluate};
pub use merge::{MergeOutcome, MergePath};
pub use probe::{AccountOutcome, ProbeOutcome, probe};
pub use read::ReadPath;
pub use service::{
    Answer, SaveRequest, ServiceFuture, ServiceReply, SharedService, TransportFailure,
};
pub use status::StatusTracker;

/// The bound on one save attempt (D0.13): a save that receives no outcome within it is refused with
/// the `timeout` cause.
///
/// **The transport owns enforcement; the gate only passes this value.** A limit can be enacted only
/// where the call is awaited, and the only await in this path is inside
/// [`SharedService::save`] — the gate starts no timer of its own, because this crate declares no
/// runtime to take one from (project rule 6, D0.8's pins). What the write path therefore guarantees
/// is that **every attempt is made with this bound**: [`SaveGate::save`] hands it to the transport on
/// each try. [`HttpService`] gives it to `reqwest`'s per-request timeout, which covers connecting,
/// the response and the body read, and reports `NoOutcome` when it expires; the scripted double
/// ([`ScriptedService`]) deliberately does *not* enforce it — it returns `NoOutcome` when a test
/// scripts that outcome — so the `timeout` cause can be exercised without waiting 15 seconds. What
/// the gate's own tests prove is the passing of the bound and the `NoOutcome` → `timeout` mapping;
/// what only a run against the real client could prove is that the transport honours it.
///
/// [`SharedService::save`]: service::SharedService::save
/// [`SaveGate::save`]: gate::SaveGate::save
/// [`HttpService`]: http::HttpService
/// [`ScriptedService`]: double::ScriptedService
pub const SAVE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

/// The bound on one whole-collection refresh (task 2.5): the read path's `household_changes` call.
///
/// It is a **read-specific name** rather than a reused [`SAVE_ATTEMPT_TIMEOUT`], so the read path
/// does not borrow a write's bound and either can move without silently changing the other. The
/// value matches a save attempt because a refresh carries a whole household collection, the same
/// order of payload as a write. Like the other bounds it is *passed* to the transport on every
/// call rather than timed here; [`SAVE_ATTEMPT_TIMEOUT`]'s documentation records where enforcement
/// lives and why.
pub const READ_REFRESH_TIMEOUT: Duration = Duration::from_secs(15);

/// The bound on **each** of a probe's two checks (D0.13): a check that receives no answer within it
/// counts as a failed check.
///
/// It bounds the service check and the account check separately rather than their sum, because the
/// two are separate questions (D0.7 criterion 4) and one of them may be answered while the other is
/// not. Like [`SAVE_ATTEMPT_TIMEOUT`], it is *passed* to the transport on every check rather than
/// timed here; that constant's documentation records where enforcement lives and why.
pub const REACHABILITY_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// How many consecutive probes that got no answer are required before the status is reported as
/// `unreachable` (D0.13, adopting NFR-6's rule in P0 so P3 inherits it).
///
/// An intervening success resets the count: the threshold is hysteresis, not a total, and it is why
/// a single blip never turns the product's status into an outage. It does **not** soften the write
/// path: a write whose own fresh probe gets no answer is refused with that cause regardless of the
/// count, because a write that cannot ask the service cannot be completed.
pub const OFFLINE_FAILURE_THRESHOLD: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bounds_are_the_values_d0_13_fixes() {
        let save = SAVE_ATTEMPT_TIMEOUT.as_secs();
        let read = READ_REFRESH_TIMEOUT.as_secs();
        let probe = REACHABILITY_PROBE_TIMEOUT.as_secs();
        let offline = OFFLINE_FAILURE_THRESHOLD;

        assert_eq!(save, 15);
        assert_eq!(read, 15);
        assert_eq!(probe, 5);
        assert_eq!(offline, 3);
    }
}
