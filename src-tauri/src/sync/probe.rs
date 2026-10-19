//! One fresh probe of the shared service and the account (D0.7 criterion 4, D0.13, NFR-1).
//!
//! A probe answers a single question for the caller that makes it: *may a call be attempted right
//! now, and if not, which condition is in the way?* It is deliberately two calls and not one —
//! a health check for the service and the account service's own check for the account (D0.7
//! criterion 4) — because the two answers are different facts: the service being up says nothing
//! about whether the account is valid, and `ServiceStatus.account_valid` exists precisely so the
//! product reports a fresh outcome for the second rather than a guess inferred from the first.
//!
//! Three rules shape it:
//!
//! * **Fresh, never remembered.** A probe is taken when it is needed and returns what it found; it
//!   consults no stored status and updates none (D0.2, R3). Feeding it into
//!   [`super::status::StatusTracker`] is the *caller's* step, because the caller owns the instant
//!   the observation is recorded at (nothing here reads a clock).
//! * **Bounded, twice.** Each of the two checks is given [`REACHABILITY_PROBE_TIMEOUT`] (D0.13)
//!   rather than the pair sharing one limit, so a service that answers but an account service that
//!   hangs is reported as exactly that.
//! * **The account is not asked when the service did not answer.** Asking a service that just
//!   failed to respond is a second wasted bound; the outcome distinguishes the two failures
//!   ([`ProbeOutcome::Unanswered`] against [`ProbeOutcome::AccountUnanswered`]) so the caller can
//!   still tell them apart.

use super::REACHABILITY_PROBE_TIMEOUT;
use super::service::{
    HTTP_TOO_MANY_REQUESTS, HTTP_UNAUTHORIZED, ServiceReply, SharedService, TransportFailure,
};
use crate::ipc::types::SaveCause;

/// What the two separate checks established, when the service answered at all (D0.7 criterion 4).
///
/// The variants are the four things an answered account check can say. `Unreadable` is the honest
/// remainder rather than a fifth condition: a service that answers a status this boundary cannot
/// read has answered, so the product is not offline, but nothing is claimed about the account
/// either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountOutcome {
    /// The account is valid in this fresh outcome: a write may be attempted.
    Identified,
    /// The account is missing or invalid — the service answered `401` (D0.7 criterion 4).
    NoAccount,
    /// The service is temporarily rate limiting — the gateway answered `429` (D0.7 criterion 4).
    RateLimited,
    /// The service answered a status this boundary cannot read as an account outcome: not the
    /// account being invalid, and not one of the conditions the cause list names.
    Unreadable {
        /// The HTTP status the account check answered with.
        status: u16,
    },
}

/// One fresh probe's outcome (SYNC-2, NFR-1).
///
/// It is what the write path decides on and what the status surface records; the two uses differ in
/// exactly one place, and the difference is deliberate — see [`AccountOutcome::Unreadable`], which
/// refuses a write (`unknown` is a cause the failure surface may name) but puts no cause on the
/// status, whose `cause` field admits only connectivity, `no_account`, `rate_limited` and
/// `timeout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The service answered both checks (D0.7 criterion 4).
    Answered(AccountOutcome),
    /// The service did not answer the service check, so the account was never asked.
    Unanswered(TransportFailure),
    /// The service answered the service check but the account check produced no answer.
    AccountUnanswered(TransportFailure),
}

impl ProbeOutcome {
    /// Whether a **read** call may be attempted on the strength of this probe (NFR-1, SYNC-3).
    ///
    /// True for the same one outcome as [`ProbeOutcome::permits_write`]: the service answered and
    /// the account is identified. A read is *served* whether or not this holds — the local copy is
    /// the fallback — so a false answer only means the refresh is skipped, never that the read is
    /// refused.
    pub fn permits_call(self) -> bool {
        matches!(self, Self::Answered(AccountOutcome::Identified))
    }

    /// Whether a write may be attempted on the strength of this probe (SYNC-2, D0.1).
    ///
    /// True for exactly one outcome: the service answered *and* the account is identified. A write
    /// is attempted only while both hold, so a rate-limited or unreadable account answer refuses the
    /// attempt just as an unreachable service does — and it refuses it *before* the write is
    /// described to the transport, which is what the double's call log makes observable.
    pub fn permits_write(self) -> bool {
        self.permits_call()
    }

    /// The cause a write refused on this probe reports (D0.6, D0.7 criterion 4).
    ///
    /// `None` only for [`ProbeOutcome::Answered`] with [`AccountOutcome::Identified`] — the one
    /// outcome that is not a refusal. `Unreadable` reports `unknown`, the honest last resort:
    /// nothing could be classified, and the failure surface says exactly that rather than inventing
    /// a cause the product does not know.
    pub fn refusal_cause(self) -> Option<SaveCause> {
        match self {
            Self::Answered(AccountOutcome::Identified) => None,
            Self::Answered(AccountOutcome::NoAccount) => Some(SaveCause::NoAccount),
            Self::Answered(AccountOutcome::RateLimited) => Some(SaveCause::RateLimited),
            Self::Answered(AccountOutcome::Unreadable { .. }) => Some(SaveCause::Unknown),
            Self::Unanswered(failure) | Self::AccountUnanswered(failure) => Some(failure.cause()),
        }
    }

    /// Whether this probe counts as a failed probe for [`super::OFFLINE_FAILURE_THRESHOLD`]
    /// (D0.13).
    ///
    /// A probe fails when it got no answer — from the service, or from the account check — and not
    /// when the service answered something the product does not like: a `401` or a `429` is an
    /// answer, so the service is reachable and the threshold does not apply. That distinction is
    /// what keeps the offline status about reachability rather than about the account's condition.
    pub fn is_failed_probe(self) -> bool {
        matches!(self, Self::Unanswered(_) | Self::AccountUnanswered(_))
    }
}

/// Run one fresh probe: the service check, and then — only if the service answered — the account
/// check (D0.7 criterion 4), each bounded by [`REACHABILITY_PROBE_TIMEOUT`] (D0.13).
///
/// The caller decides what to do with the outcome: [`super::gate::SaveGate`] refuses the write it
/// was asked to make, and task 2.13 records it on the status surface with the instant it was taken
/// at. Nothing here is stored, so a stale answer cannot survive the call.
pub async fn probe(service: &dyn SharedService) -> ProbeOutcome {
    let service_reply = service.check_service(REACHABILITY_PROBE_TIMEOUT).await;

    // The account is not asked when the service did not answer: there is nothing to ask it of, and
    // the second bound would be spent on a service that just failed to respond.
    if let Some(failure) = service_reply.transport_failure() {
        return ProbeOutcome::Unanswered(failure);
    }

    match service.check_account(REACHABILITY_PROBE_TIMEOUT).await {
        ServiceReply::Answered(answer) if answer.is_success() => {
            ProbeOutcome::Answered(AccountOutcome::Identified)
        }
        ServiceReply::Answered(answer) => match answer.status() {
            HTTP_UNAUTHORIZED => ProbeOutcome::Answered(AccountOutcome::NoAccount),
            HTTP_TOO_MANY_REQUESTS => ProbeOutcome::Answered(AccountOutcome::RateLimited),
            status => ProbeOutcome::Answered(AccountOutcome::Unreadable { status }),
        },
        ServiceReply::NoResponse => ProbeOutcome::AccountUnanswered(TransportFailure::NoResponse),
        ServiceReply::NoOutcome => ProbeOutcome::AccountUnanswered(TransportFailure::NoOutcome),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::service::Answer;

    /// Every outcome a probe can produce, so a rule about them is stated once and checked on all.
    const ALL_OUTCOMES: [ProbeOutcome; 7] = [
        ProbeOutcome::Answered(AccountOutcome::Identified),
        ProbeOutcome::Answered(AccountOutcome::NoAccount),
        ProbeOutcome::Answered(AccountOutcome::RateLimited),
        ProbeOutcome::Answered(AccountOutcome::Unreadable { status: 503 }),
        ProbeOutcome::Unanswered(TransportFailure::NoResponse),
        ProbeOutcome::Unanswered(TransportFailure::NoOutcome),
        ProbeOutcome::AccountUnanswered(TransportFailure::NoResponse),
    ];

    #[test]
    fn the_probe_asks_the_service_and_then_the_account_as_two_separate_calls() {
        let service = ScriptedService::new();

        assert_eq!(
            block_on_ready(probe(&service)),
            ProbeOutcome::Answered(AccountOutcome::Identified)
        );

        // Two calls, not one, and each carries D0.13's probe bound rather than a number of its own.
        assert_eq!(
            service.calls(),
            vec![
                RecordedCall::CheckService {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
                RecordedCall::CheckAccount {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
            ]
        );
    }

    #[test]
    fn an_unanswered_service_check_never_asks_the_account() {
        let service = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);

        assert_eq!(
            block_on_ready(probe(&service)),
            ProbeOutcome::Unanswered(TransportFailure::NoResponse)
        );
        assert_eq!(
            service.calls(),
            vec![RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            }]
        );
    }

    #[test]
    fn an_account_check_that_gets_no_answer_is_reported_as_its_own_outcome() {
        let service = ScriptedService::new().on_account_check([ServiceReply::NoOutcome]);

        let outcome = block_on_ready(probe(&service));

        assert_eq!(
            outcome,
            ProbeOutcome::AccountUnanswered(TransportFailure::NoOutcome)
        );
    }

    #[test]
    fn each_account_answer_becomes_its_own_outcome() {
        assert_eq!(
            probe_answered(Answer::status_only(200)),
            ProbeOutcome::Answered(AccountOutcome::Identified)
        );
        assert_eq!(
            probe_answered(Answer::status_only(HTTP_UNAUTHORIZED)),
            ProbeOutcome::Answered(AccountOutcome::NoAccount)
        );
        assert_eq!(
            probe_answered(Answer::status_only(HTTP_TOO_MANY_REQUESTS)),
            ProbeOutcome::Answered(AccountOutcome::RateLimited)
        );
        assert_eq!(
            probe_answered(Answer::status_only(500)),
            ProbeOutcome::Answered(AccountOutcome::Unreadable { status: 500 })
        );
    }

    /// Probe a double whose account check answers exactly `answer`.
    fn probe_answered(answer: Answer) -> ProbeOutcome {
        let reply = ServiceReply::Answered(answer);
        let service = ScriptedService::new().on_account_check([reply]);

        block_on_ready(probe(&service))
    }

    #[test]
    fn only_an_identified_account_permits_a_call() {
        for outcome in ALL_OUTCOMES {
            assert_eq!(outcome.permits_write(), outcome.refusal_cause().is_none());
            assert_eq!(outcome.permits_call(), outcome.refusal_cause().is_none());
        }

        assert!(ProbeOutcome::Answered(AccountOutcome::Identified).permits_call());
        assert_eq!(
            ProbeOutcome::Answered(AccountOutcome::Unreadable { status: 503 }).refusal_cause(),
            Some(SaveCause::Unknown)
        );
    }

    #[test]
    fn only_a_probe_that_got_no_answer_counts_as_a_failed_probe() {
        for outcome in ALL_OUTCOMES {
            let answered = matches!(outcome, ProbeOutcome::Answered(_));

            assert_eq!(outcome.is_failed_probe(), !answered);
        }
    }
}
