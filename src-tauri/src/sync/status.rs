//! The product's service/account status: the last fresh outcome, and the threshold that keeps a
//! blip from being reported as an outage (NFR-1, D0.13, R3).
//!
//! This is the surface the two status commands will read (`get_service_status` returns it,
//! `check_service_reachability` updates it and returns the result — task 2.13 wires both, not this
//! task). Two rules make it honest:
//!
//! * **`unknown` until a fresh outcome exists.** A tracker that has observed nothing reports
//!   `state: unknown` with no account claim, no cause and no instant; there is no default that could
//!   be mistaken for a current observation (R3). Nothing is carried over from a previous session
//!   because nothing is persisted: a tracker starts empty on every launch.
//! * **`unreachable` only after [`OFFLINE_FAILURE_THRESHOLD`] consecutive probes that got no
//!   answer**, with an intervening success resetting the count (D0.13). Before the threshold the
//!   tracker keeps the last fresh outcome — with the instant it was *actually* observed, not the
//!   instant of the probe that just failed — because "the last thing we observed" is the honest
//!   thing to report while a blip is not yet an outage.
//!
//! **The tracker is not the write path's gate.** [`super::gate::SaveGate`] never reads it: a write is
//! decided on its own fresh probe (D0.2, R3), so a status retained here for hysteresis can never
//! authorize a write. Keeping the two apart is deliberate; the failure surface tells the person the
//! cause of a refused write, and the status tells them what the product currently knows.
//!
//! **Nothing here reads a clock.** [`StatusTracker::observe`] takes the observation instant from its
//! caller — exactly as the local copy takes its load instant (`storage::local`) — so the clock and
//! its RFC 3339 formatting stay task 2.8's, and every rule below is a pure function of its input.
//! `observe` is deliberately synchronous: a caller holding a lock (a Tauri command's shared state)
//! never awaits while holding it.

use super::OFFLINE_FAILURE_THRESHOLD;
use super::probe::{AccountOutcome, ProbeOutcome};
use crate::ipc::types::{Instant, SaveCause, ServiceState, ServiceStatus};

/// The status surface the two status commands read (NFR-1).
///
/// It holds the last fresh outcome and the count that decides when a run of unanswered probes is
/// reported as offline (D0.13). The count is internal state, never a wire field: P0's
/// [`ServiceStatus`] deliberately carries no count, and the rule is applied here in Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTracker {
    /// How many consecutive probes have got no answer, since the last one that did (D0.13).
    consecutive_failures: u32,
    /// The status the product reports: `unknown` before the first fresh outcome, then the last
    /// outcome worth reporting (see [`StatusTracker::observe`]).
    status: ServiceStatus,
}

impl StatusTracker {
    /// A tracker that has observed nothing: the status is `unknown` (R3).
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            status: never_observed(),
        }
    }

    /// The status as it stands, exactly as the commands will return it.
    pub fn status(&self) -> ServiceStatus {
        self.status.clone()
    }

    /// How many consecutive probes have got no answer (D0.13).
    ///
    /// It is the count the offline threshold is applied to, exposed so a caller (or a test) can see
    /// the state behind the status. It is **not** a field of [`ServiceStatus`]: P0 carries no count
    /// on the wire.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Record one fresh probe's outcome, observed at `observed_at`, and return the status now.
    ///
    /// An **answered** probe is a fresh outcome: it resets the failure count and replaces the status
    /// with what the two checks established — `reachable` plus the account's own answer (D0.7
    /// criterion 4), and the condition when it was not a plain success.
    ///
    /// A probe that got **no answer** increments the count and leaves the status alone until the
    /// count reaches [`OFFLINE_FAILURE_THRESHOLD`]; then — and only then — the status becomes
    /// `unreachable`, naming the transport condition and carrying the instant of the probe that
    /// reached the threshold (D0.13). `observed_at` is therefore used only when an outcome is
    /// actually recorded, which is why a retained status keeps the time it was observed.
    pub fn observe(&mut self, outcome: ProbeOutcome, observed_at: Instant) -> ServiceStatus {
        match outcome {
            ProbeOutcome::Answered(account) => {
                self.consecutive_failures = 0;
                self.status = ServiceStatus {
                    state: ServiceState::Reachable,
                    account_valid: account_valid(account),
                    cause: status_cause(account),
                    observed_at: Some(observed_at),
                };
            }
            ProbeOutcome::Unanswered(failure) | ProbeOutcome::AccountUnanswered(failure) => {
                self.consecutive_failures += 1;

                if self.consecutive_failures >= OFFLINE_FAILURE_THRESHOLD {
                    self.status = ServiceStatus {
                        state: ServiceState::Unreachable,
                        account_valid: None,
                        cause: Some(failure.cause()),
                        observed_at: Some(observed_at),
                    };
                }
            }
        }

        self.status.clone()
    }
}

impl Default for StatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// The status before any fresh outcome exists (R3).
///
/// Every field is absent rather than defaulted: "we have not looked" is not "we looked and it was
/// fine", and only the second may drive a status indicator.
fn never_observed() -> ServiceStatus {
    ServiceStatus {
        state: ServiceState::Unknown,
        account_valid: None,
        cause: None,
        observed_at: None,
    }
}

/// The account half of one answered probe (D0.7 criterion 4).
///
/// `None` whenever the answer did not determine the account — before anything was observed, and when
/// the account check was rate limited or answered something unreadable — because a default of
/// `false` would be exactly the cached guess the field's contract forbids.
fn account_valid(outcome: AccountOutcome) -> Option<bool> {
    match outcome {
        AccountOutcome::Identified => Some(true),
        AccountOutcome::NoAccount => Some(false),
        AccountOutcome::RateLimited | AccountOutcome::Unreadable { .. } => None,
    }
}

/// The condition one answered probe names on the status, if any.
///
/// [`ServiceStatus::cause`]'s contract admits exactly `connectivity`, `no_account`, `rate_limited`
/// and `timeout`, and `None` for a plain success and for `unknown`. An unreadable answer is none of
/// the four, so it puts no cause on the wire: the failure surface is where an unclassifiable
/// condition may say `unknown` (a refused write), not the status field.
fn status_cause(outcome: AccountOutcome) -> Option<SaveCause> {
    match outcome {
        AccountOutcome::Identified | AccountOutcome::Unreadable { .. } => None,
        AccountOutcome::NoAccount => Some(SaveCause::NoAccount),
        AccountOutcome::RateLimited => Some(SaveCause::RateLimited),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::service::TransportFailure;

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    #[test]
    fn the_status_is_unknown_until_a_fresh_outcome_exists() {
        let tracker = StatusTracker::new();
        let status = tracker.status();

        assert_eq!(status.state, ServiceState::Unknown);
        assert_eq!(status.account_valid, None);
        assert_eq!(status.cause, None);
        assert_eq!(status.observed_at, None);
        assert_eq!(tracker.consecutive_failures(), 0);
    }

    #[test]
    fn two_failed_probes_are_not_yet_an_outage_and_the_third_is() {
        let mut tracker = StatusTracker::new();
        let failure = ProbeOutcome::Unanswered(TransportFailure::NoResponse);

        tracker.observe(failure, at("2026-10-19T10:00:00Z"));
        tracker.observe(failure, at("2026-10-19T10:00:01Z"));

        assert_eq!(tracker.status().state, ServiceState::Unknown);
        assert_eq!(tracker.status().observed_at, None);
        assert_eq!(tracker.consecutive_failures(), 2);

        let reached = tracker.observe(failure, at("2026-10-19T10:00:02Z"));

        assert_eq!(reached.state, ServiceState::Unreachable);
        assert_eq!(reached.account_valid, None);
        assert_eq!(reached.cause, Some(SaveCause::Connectivity));
        assert_eq!(reached.observed_at, Some(at("2026-10-19T10:00:02Z")));
        assert_eq!(tracker.consecutive_failures(), 3);
    }

    #[test]
    fn a_success_between_failures_resets_the_count() {
        let mut tracker = StatusTracker::new();
        let failure = ProbeOutcome::Unanswered(TransportFailure::NoOutcome);
        let success = ProbeOutcome::Answered(AccountOutcome::Identified);

        tracker.observe(failure, at("2026-10-19T10:00:00Z"));
        tracker.observe(failure, at("2026-10-19T10:00:01Z"));

        let fresh = tracker.observe(success, at("2026-10-19T10:00:02Z"));

        assert_eq!(fresh.state, ServiceState::Reachable);
        assert_eq!(fresh.account_valid, Some(true));
        assert_eq!(fresh.cause, None);
        assert_eq!(tracker.consecutive_failures(), 0);

        tracker.observe(failure, at("2026-10-19T10:00:03Z"));
        tracker.observe(failure, at("2026-10-19T10:00:04Z"));

        // Two failures after the success do not turn the status offline, and the retained value is
        // the observation that actually happened, with its own instant.
        assert_eq!(tracker.status().state, ServiceState::Reachable);
        assert_eq!(
            tracker.status().observed_at,
            Some(at("2026-10-19T10:00:02Z"))
        );

        let reached = tracker.observe(failure, at("2026-10-19T10:00:05Z"));

        assert_eq!(reached.state, ServiceState::Unreachable);
        assert_eq!(reached.observed_at, Some(at("2026-10-19T10:00:05Z")));
    }

    #[test]
    fn an_account_condition_is_a_fresh_reachable_outcome_not_an_outage() {
        let mut tracker = StatusTracker::new();

        let missing = tracker.observe(
            ProbeOutcome::Answered(AccountOutcome::NoAccount),
            at("2026-10-19T10:00:00Z"),
        );

        assert_eq!(missing.state, ServiceState::Reachable);
        assert_eq!(missing.account_valid, Some(false));
        assert_eq!(missing.cause, Some(SaveCause::NoAccount));
        assert_eq!(missing.observed_at, Some(at("2026-10-19T10:00:00Z")));
        assert_eq!(tracker.consecutive_failures(), 0);

        let limited = tracker.observe(
            ProbeOutcome::Answered(AccountOutcome::RateLimited),
            at("2026-10-19T10:00:01Z"),
        );

        assert_eq!(limited.state, ServiceState::Reachable);
        assert_eq!(limited.account_valid, None);
        assert_eq!(limited.cause, Some(SaveCause::RateLimited));
    }

    #[test]
    fn an_unreadable_account_answer_claims_nothing_about_the_account() {
        let mut tracker = StatusTracker::new();
        let outcome = ProbeOutcome::Answered(AccountOutcome::Unreadable { status: 503 });

        let status = tracker.observe(outcome, at("2026-10-19T10:00:00Z"));

        assert_eq!(status.state, ServiceState::Reachable);
        assert_eq!(status.account_valid, None);
        assert_eq!(status.cause, None);
        assert_eq!(status.observed_at, Some(at("2026-10-19T10:00:00Z")));
    }

    #[test]
    fn an_account_check_that_gets_no_answer_counts_towards_the_threshold() {
        let mut tracker = StatusTracker::new();
        let failure = ProbeOutcome::AccountUnanswered(TransportFailure::NoOutcome);

        tracker.observe(failure, at("2026-10-19T10:00:00Z"));
        tracker.observe(failure, at("2026-10-19T10:00:01Z"));

        let reached = tracker.observe(failure, at("2026-10-19T10:00:02Z"));

        assert_eq!(reached.state, ServiceState::Unreachable);
        assert_eq!(reached.account_valid, None);
        assert_eq!(reached.cause, Some(SaveCause::Timeout));
    }
}
