//! The reachability-gated save path (SYNC-2, NFR-1; D0.1, D0.6, D0.12, D0.13, D0.14).
//!
//! This module is the one place a write to the shared service is admitted, and the one place a
//! write's outcome becomes either a [`SaveReceipt`] or a [`CommandError`]. Three rules are its whole
//! reason to exist:
//!
//! 1. **A write is attempted only while the service is reachable and the account is valid**, from a
//!    probe taken at the attempt ([`probe`]), never from a status remembered from an earlier session
//!    (D0.2, R3). When the probe refuses the attempt, the transport is never asked to submit
//!    anything: "nothing is written to the shared collection" is therefore true of the *request*, not
//!    only of the outcome (SYNC-2's first scenario).
//! 2. **A failed write is never a saved one, and a saved one is never a silent failure** (SYNC-2,
//!    NFR-1). Success exists as exactly one value — [`SaveReceipt`] — whose only constructor is
//!    reached from exactly one answer: a `2xx` whose envelope reports the write as completed. Every
//!    other outcome, including a `2xx` this boundary cannot read, is a [`CommandError`].
//! 3. **Each condition is named as itself** (D0.6, D0.12): an exhausted owner allowance is the
//!    service's own report and maps to `allowance_exceeded`, never to a generic write error; a `429`
//!    is `rate_limited`; a `401` is `no_account`; no answer at all is `connectivity`; no answer inside
//!    [`SAVE_ATTEMPT_TIMEOUT`] is `timeout`; a refused input is `validation`.
//!
//! **No queue, no optimism, no deferred reconciliation** (D0.1). The gate holds a transport and
//! nothing else — no state, no storage handle, no status tracker — so there is nowhere for a refused
//! write to be parked and nothing that could complete later. The failure is the final word until the
//! person retries, and a retry re-issues the same [`RequestId`] so the service deduplicates it
//! (D0.14).
//!
//! **The failure surface is not built here.** Every refusal routes through [`CommandError`]'s own
//! per-cause constructors, so the message key and the retry flag come from `ipc::failure`'s policy
//! (NFR-8) and this module never invents either.

use serde_json::Value;

use super::SAVE_ATTEMPT_TIMEOUT;
use super::probe::probe;
use super::service::{
    HTTP_BAD_REQUEST, HTTP_TOO_MANY_REQUESTS, HTTP_UNAUTHORIZED, HTTP_UNPROCESSABLE_CONTENT,
    OUTCOME_ALLOWANCE_EXCEEDED, OUTCOME_APPLIED, OUTCOME_DEDUPLICATED, SaveRequest, ServiceReply,
    SharedService,
};
use crate::ipc::types::{CommandError, RequestId, SaveCause};

/// The reachability gate: the one place a write to the shared service is admitted (SYNC-2, NFR-1).
///
/// It borrows the transport and holds nothing else. That is deliberate: a gate with no queue, no
/// cache and no storage handle has nowhere to write anything locally as if it were saved (D0.1), and
/// because its decision comes from the probe it takes itself, no remembered status can authorize a
/// write (D0.2, R3).
pub struct SaveGate<'a> {
    service: &'a dyn SharedService,
}

impl<'a> SaveGate<'a> {
    /// A gate over one transport.
    pub fn new(service: &'a dyn SharedService) -> Self {
        Self { service }
    }

    /// Submit one write: probe fresh, refuse the attempt if the probe says it cannot complete, and
    /// otherwise attempt it (SYNC-2, NFR-1).
    ///
    /// # Errors
    ///
    /// A [`CommandError`] naming the one condition that stopped the write — the probe's cause when it
    /// refused the attempt, the attempt's own cause otherwise. `Ok` is returned **only** for an
    /// answer whose envelope reported the write as completed, so a write is never reported as saved
    /// without the service having said so.
    pub async fn save(&self, request: SaveRequest) -> Result<SaveReceipt, CommandError> {
        let outcome = probe(self.service).await;

        if let Some(cause) = outcome.refusal_cause() {
            // The transport is not asked to submit anything: the write never leaves the device, which
            // is the observable half of "nothing is written to the shared collection" (SYNC-2).
            return Err(refusal(cause, &request));
        }

        // The bound is passed to the transport, which is the only place it can be enforced — this
        // gate starts no timer of its own (`SAVE_ATTEMPT_TIMEOUT` records where enforcement lives).
        let reply = self.service.save(&request, SAVE_ATTEMPT_TIMEOUT).await;

        classify_write(reply, &request)
    }
}

/// A write the shared service accepted (SYNC-2).
///
/// The type is the structural half of "never report success for a write that did not complete": its
/// fields are private and [`SaveReceipt::completed`] is reached only from [`classify_write`]'s one
/// acceptance arm, so no caller can build one from a refusal, a transport failure or a `2xx` whose
/// envelope it could not read (SYNC-2, NFR-1).
#[derive(Debug, Clone, PartialEq)]
pub struct SaveReceipt {
    /// The operation the service completed, carried unchanged from the request (D0.14).
    request_id: RequestId,
    /// The service's own answer body, kept opaque for the operation's schema to interpret.
    body: Value,
}

impl SaveReceipt {
    /// The receipt of one completed write — private, and reached only from the acceptance arm.
    fn completed(request: &SaveRequest, body: Value) -> Self {
        Self {
            request_id: request.request_id().clone(),
            body,
        }
    }

    /// The operation the service completed (D0.14).
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// The service's own answer body.
    ///
    /// It is opaque here on purpose: the operation that submitted the write owns its schema — 2.1's
    /// household creation reads the service-assigned household id out of it — and this boundary must
    /// not invent a schema it does not own.
    pub fn body(&self) -> &Value {
        &self.body
    }
}

/// How this boundary reads one write's envelope (D0.7 criteria 3 and 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteOutcome {
    /// The service reported the write as completed: it applied the operation, or returned the stored
    /// outcome of a re-issued one, which is the same completed write (D0.14).
    Completed,
    /// The service's own report that the household owner's allowance is exhausted (D0.12).
    AllowanceExceeded,
}

/// Read the service's write envelope: the `outcome` token is the whole signal.
///
/// Only the tokens this boundary knows are read, and an unrecognised one — a typo, a condition a
/// newer service reports, a body that is not the envelope at all — yields `None`. It is never
/// accepted by default, because "the write was saved" may not be the fallback for "I did not
/// understand the answer" (SYNC-2, NFR-1).
fn read_outcome(body: &Value) -> Option<WriteOutcome> {
    match body.get("outcome")?.as_str()? {
        OUTCOME_APPLIED | OUTCOME_DEDUPLICATED => Some(WriteOutcome::Completed),
        OUTCOME_ALLOWANCE_EXCEEDED => Some(WriteOutcome::AllowanceExceeded),
        _ => None,
    }
}

/// Classify one write attempt's reply (SYNC-2, D0.6, D0.12).
///
/// The transport's failures are decided first, then a non-success status, and last the envelope — the
/// order matters because a `2xx` means only that the RPC completed (D0.7 criterion 5): the service
/// reports an exhausted allowance at `200`, and that envelope has to be read before the status could
/// be mistaken for acceptance.
fn classify_write(reply: ServiceReply, request: &SaveRequest) -> Result<SaveReceipt, CommandError> {
    match reply {
        ServiceReply::NoResponse => Err(refusal(SaveCause::Connectivity, request)),
        ServiceReply::NoOutcome => Err(refusal(SaveCause::Timeout, request)),
        ServiceReply::Answered(answer) => {
            if !answer.is_success() {
                return Err(refusal(cause_of_status(answer.status()), request));
            }

            let Some(body) = answer.into_body() else {
                // A success status with no readable envelope is not a completed write: nothing in it
                // says the service accepted anything, so it is reported as unclassifiable.
                return Err(refusal(SaveCause::Unknown, request));
            };

            match read_outcome(&body) {
                Some(WriteOutcome::Completed) => Ok(SaveReceipt::completed(request, body)),
                Some(WriteOutcome::AllowanceExceeded) => {
                    Err(refusal(SaveCause::AllowanceExceeded, request))
                }
                None => Err(refusal(SaveCause::Unknown, request)),
            }
        }
    }
}

/// The cause an answered write's HTTP status names (D0.7 criterion 4).
///
/// A `401` is the account being missing or invalid, a `429` the gateway's rate limiting, and
/// `400`/`422` the input being refused. Everything else is `unknown`: a cause the product does not
/// know is not invented from a status it cannot classify, and the honest remainder is still a named
/// cause with its own copy.
fn cause_of_status(status: u16) -> SaveCause {
    match status {
        HTTP_UNAUTHORIZED => SaveCause::NoAccount,
        HTTP_TOO_MANY_REQUESTS => SaveCause::RateLimited,
        HTTP_BAD_REQUEST | HTTP_UNPROCESSABLE_CONTENT => SaveCause::Validation,
        _ => SaveCause::Unknown,
    }
}

/// The one place this path turns a cause into the failure surface (D0.6, NFR-8).
///
/// Every arm calls [`CommandError`]'s constructor for that cause, so the message key and the retry
/// flag are `ipc::failure`'s policy rather than a choice made here. The operation identity is the
/// request's, unchanged (D0.14): the surface offers a retry of the *same* `request_id`, which is what
/// makes that retry safe.
fn refusal(cause: SaveCause, request: &SaveRequest) -> CommandError {
    let request_id = Some(request.request_id().clone());

    match cause {
        SaveCause::Connectivity => CommandError::connectivity(request_id),
        SaveCause::NoAccount => CommandError::no_account(request_id),
        SaveCause::AllowanceExceeded => CommandError::allowance_exceeded(request_id),
        SaveCause::RateLimited => CommandError::rate_limited(request_id),
        SaveCause::Timeout => CommandError::timeout(request_id),
        SaveCause::Validation => CommandError::validation(request_id),
        SaveCause::Unknown => CommandError::unknown(request_id),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::sync::REACHABILITY_PROBE_TIMEOUT;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::service::Answer;

    fn operation() -> RequestId {
        RequestId("operation-1".to_owned())
    }

    fn write_body() -> Value {
        let mut body = serde_json::Map::new();
        body.insert("name".to_owned(), Value::String("Home".to_owned()));

        Value::Object(body)
    }

    fn write() -> SaveRequest {
        SaveRequest::new(operation(), "create_household", write_body())
            .expect("a well-formed write")
    }

    /// One write envelope, as the service's own report carries it.
    ///
    /// The tokens are literals here rather than [`super::OUTCOME_APPLIED`] and friends, so the test
    /// pins the wire contract independently of the constants the code reads.
    fn envelope(outcome: &str) -> Value {
        let mut envelope = serde_json::Map::new();
        envelope.insert("outcome".to_owned(), Value::String(outcome.to_owned()));

        Value::Object(envelope)
    }

    /// The reply of an answered write whose envelope reports `outcome`.
    fn write_answer(outcome: &str) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(200, envelope(outcome)))
    }

    /// Run one gated save against `service`.
    fn save_with(service: &ScriptedService) -> Result<SaveReceipt, CommandError> {
        let gate = SaveGate::new(service);

        block_on_ready(gate.save(write()))
    }

    /// The failure of one gated save, which must not have completed.
    fn refusal_of(result: Result<SaveReceipt, CommandError>) -> CommandError {
        result.expect_err("the write did not complete")
    }

    /// Every bound the transport was given for a write, in order.
    fn write_bounds(service: &ScriptedService) -> Vec<Duration> {
        service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { bound, .. } => Some(bound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_save_without_a_reachable_service_is_refused_with_connectivity() {
        let service = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::Connectivity);
        assert_eq!(error.message_key, "error.connectivity");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(operation()));

        // Nothing was written and nothing was sent: the only call is the probe's service check.
        assert_eq!(service.write_count(), 0);
        assert_eq!(
            service.calls(),
            vec![RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            }]
        );
    }

    #[test]
    fn a_save_whose_probe_times_out_is_refused_with_timeout() {
        let service = ScriptedService::new().on_service_check([ServiceReply::NoOutcome]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::Timeout);
        assert_eq!(error.message_key, "error.timeout");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(operation()));

        // The deadline that passed was the probe's, and it was the probe's bound that carried it: no
        // write was sent, so the attempt's own bound never came into play.
        assert_eq!(service.write_count(), 0);
        assert_eq!(
            service.calls(),
            vec![RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            }]
        );
    }

    #[test]
    fn a_save_without_a_valid_account_is_refused_with_no_account() {
        let service = ScriptedService::new()
            .on_account_check([ServiceReply::Answered(Answer::status_only(401))]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert_eq!(error.message_key, "error.no_account");
        assert!(error.retryable);
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn a_rate_limited_account_check_refuses_the_write_before_it_is_sent() {
        let service = ScriptedService::new()
            .on_account_check([ServiceReply::Answered(Answer::status_only(429))]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::RateLimited);
        assert_eq!(error.message_key, "error.rate_limited");
        assert!(error.retryable);
        assert_eq!(service.write_count(), 0);
    }

    #[test]
    fn a_save_the_service_accepted_reports_the_completed_write() {
        let service = ScriptedService::new().on_write([write_answer("applied")]);

        let receipt = save_with(&service).expect("the service accepted the write");

        assert_eq!(receipt.request_id(), &operation());
        assert_eq!(receipt.body(), &envelope("applied"));

        // The attempt the transport was asked to make carried D0.13's 15-second bound, and it was one
        // attempt with the command's own identity (D0.14).
        assert_eq!(write_bounds(&service), vec![SAVE_ATTEMPT_TIMEOUT]);
        assert_eq!(service.write_count(), 1);
    }

    #[test]
    fn a_reissued_write_is_a_completed_write_and_not_a_second_one() {
        let service = ScriptedService::new().on_write([write_answer("deduplicated")]);

        let receipt = save_with(&service).expect("the stored outcome is a completed write");

        assert_eq!(receipt.request_id(), &operation());
        assert_eq!(service.write_count(), 1);
    }

    #[test]
    fn an_exhausted_allowance_is_its_own_cause_never_a_generic_write_error() {
        let service = ScriptedService::new().on_write([write_answer("allowance_exceeded")]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::AllowanceExceeded);
        assert_eq!(error.message_key, "error.allowance_exceeded");
        assert!(error.retryable);

        // The condition is neither the generic write error nor the honest remainder: the service
        // reported it as itself, and it is reported as itself (D0.12).
        assert_ne!(error.message_key, "error.unknown");
        assert_ne!(error, CommandError::unknown(Some(operation())));
    }

    #[test]
    fn a_2xx_the_boundary_cannot_read_is_never_a_completed_write() {
        let replies = [
            ServiceReply::Answered(Answer::with_body(200, envelope("something-else"))),
            ServiceReply::Answered(Answer::with_body(200, Value::Null)),
            ServiceReply::Answered(Answer::status_only(200)),
        ];

        for reply in replies {
            let service = ScriptedService::new().on_write([reply]);
            let error = refusal_of(save_with(&service));

            assert_eq!(error.cause, SaveCause::Unknown);
            assert_eq!(error.message_key, "error.unknown");
            assert!(!error.retryable);
        }
    }

    #[test]
    fn a_write_that_gets_no_outcome_in_the_bound_is_refused_with_timeout() {
        let service = ScriptedService::new().on_write([ServiceReply::NoOutcome]);

        let error = refusal_of(save_with(&service));

        assert_eq!(error.cause, SaveCause::Timeout);
        assert_eq!(error.message_key, "error.timeout");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(operation()));

        // The timeout cause is not a placeholder: the attempt the transport was asked to make carried
        // D0.13's real 15-second bound, and re-issuing it is safe because the identity is unchanged.
        assert_eq!(write_bounds(&service), vec![SAVE_ATTEMPT_TIMEOUT]);
    }

    #[test]
    fn an_unaccepted_status_names_its_own_cause() {
        let cases = [
            (401, SaveCause::NoAccount, "error.no_account"),
            (429, SaveCause::RateLimited, "error.rate_limited"),
            (400, SaveCause::Validation, "error.validation"),
            (422, SaveCause::Validation, "error.validation"),
            (500, SaveCause::Unknown, "error.unknown"),
        ];

        for (status, cause, key) in cases {
            let reply = ServiceReply::Answered(Answer::status_only(status));
            let service = ScriptedService::new().on_write([reply]);
            let error = refusal_of(save_with(&service));

            assert_eq!(error.cause, cause);
            assert_eq!(error.message_key, key);
        }
    }

    #[test]
    fn only_a_completed_envelope_is_reported_as_a_success() {
        let completed = [write_answer("applied"), write_answer("deduplicated")];
        let refused = [
            write_answer("allowance_exceeded"),
            write_answer("something-else"),
            ServiceReply::Answered(Answer::status_only(200)),
            ServiceReply::Answered(Answer::status_only(401)),
            ServiceReply::Answered(Answer::status_only(429)),
            ServiceReply::Answered(Answer::status_only(400)),
            ServiceReply::Answered(Answer::status_only(500)),
            ServiceReply::NoResponse,
            ServiceReply::NoOutcome,
        ];

        for reply in completed {
            let service = ScriptedService::new().on_write([reply]);

            assert!(
                save_with(&service).is_ok(),
                "the service completed the write"
            );
        }

        for reply in refused {
            let service = ScriptedService::new().on_write([reply]);

            assert!(save_with(&service).is_err(), "the write did not complete");
        }
    }
}
