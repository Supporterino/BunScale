//! The service client boundary: the trait the real HTTPS client and the scripted double both
//! implement, and the vocabulary one call and one write are described in (D0.7 criterion 4, D0.13).
//!
//! # Why this seam exists
//!
//! P0 has no live deployment and the gate never boots the shared service (D0.7's "the cost,
//! recorded rather than hidden"), so the only way the write path can be tested is for the transport
//! to be a substitutable seam: [`SharedService`] is implemented by the real `reqwest` client
//! ([`super::http::HttpService`]) and by a scripted double ([`super::double::ScriptedService`]), and
//! everything above the seam — the probe, the gate, the refusal mapping — is ordinary, testable
//! code. The seam is the *transport*, not the protocol: a call is asked, a write is described, and
//! a reply is reported, and no part of the reply is a cause yet.
//!
//! # Why a boxed future rather than `async fn` in the trait
//!
//! An `async fn` in a public trait returns an opaque future whose `Send` bound cannot be stated at
//! the trait, so a caller that has to spawn the call — the Tauri command runtime does — would be
//! relying on an inference it cannot see, and the returned type is not `dyn`-compatible. The app
//! holds one transport and its tests substitute another, which is exactly the `dyn` case, so
//! [`ServiceFuture`] states the bound and keeps the trait object-safe at the cost of one allocation
//! per call: nothing beside the network round trip the call describes.
//!
//! # What a reply is, and what it is not
//!
//! [`ServiceReply`] carries the transport's facts — which status the service answered with, and the
//! body the call read — and **no cause**. Classification is deliberately one step up ([`super::probe`]
//! for the two checks, [`super::gate`] for the write), because that step is the part the tests can
//! exercise: the real client cannot be reached by the gate, so any mapping baked into it would ship
//! untested.
//!
//! # The bound is a parameter, not an assumption
//!
//! Every call takes the caller's time limit (D0.13). A limit can only be enforced where the call is
//! awaited, so the seam carries it: the real client hands it to `reqwest`'s per-request timeout and
//! the double records it, which is what lets a test assert that the probe used
//! [`super::REACHABILITY_PROBE_TIMEOUT`] and that a save attempt used
//! [`super::SAVE_ATTEMPT_TIMEOUT`] rather than a number invented at the call site.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde_json::Value;

use crate::ipc::types::{CommandError, RequestId, SaveCause};

/// The future every call on the seam returns (see the module documentation for why it is boxed).
pub type ServiceFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// How a call failed to produce an outcome at all (D0.7 criterion 4).
///
/// The two variants are the two transport conditions the cause list distinguishes: a call that got
/// no HTTP response — DNS, TLS, connect or read — and a call that got no outcome inside the bound
/// the caller set. They are distinct causes because they are distinct conditions for the person:
/// one says the device cannot reach the service, the other says the service did not answer in time,
/// and D0.14's ledger is what makes the second safe to retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailure {
    /// No HTTP response at all: DNS, TLS, connect or read failure.
    NoResponse,
    /// No outcome arrived inside the caller's bound (D0.13).
    NoOutcome,
}

impl TransportFailure {
    /// The cause this transport condition reports (D0.7 criterion 4).
    pub fn cause(self) -> SaveCause {
        match self {
            Self::NoResponse => SaveCause::Connectivity,
            Self::NoOutcome => SaveCause::Timeout,
        }
    }
}

/// One answered call: the status the service returned, and whatever body the call read.
///
/// The two ways to build it state what happened rather than how the call was made: a call that
/// asked only for the status reads no body, and a call that expected one and could not read it as
/// JSON reports no body either — which is deliberate, because "the service answered `200` with
/// something this boundary cannot read" must never be turned into an acceptance anywhere above
/// ([`super::gate`] reads it as `unknown`).
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// The HTTP status the service answered with.
    status: u16,
    /// The body the call read, when it read one.
    body: Option<Value>,
}

impl Answer {
    /// An answered call that read no body: the calls whose only question is the status (the two
    /// probe checks), and a write whose body could not be read as JSON.
    pub fn status_only(status: u16) -> Self {
        Self { status, body: None }
    }

    /// An answered call carrying the body it read.
    pub fn with_body(status: u16, body: Value) -> Self {
        Self {
            status,
            body: Some(body),
        }
    }

    /// The HTTP status the service answered with.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Whether the status is a success (`2xx`).
    ///
    /// It is the *status* half of an answer, never the whole answer: `2xx` means the call completed,
    /// which for a write is not the same as the write being accepted (D0.7 criterion 5).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The body this answer carries, when it carries one.
    pub fn body(&self) -> Option<&Value> {
        self.body.as_ref()
    }

    /// This answer's body by value, for the caller that keeps it (the gate's receipt).
    pub fn into_body(self) -> Option<Value> {
        self.body
    }
}

/// What one call to the shared service produced, as the transport saw it (D0.7 criterion 4).
///
/// Nothing here is a [`SaveCause`] yet and nothing here is a failure: an unanswered call and an
/// answered one are both ordinary outcomes, and which cause (if any) an answer implies is decided
/// one step up, where the tests can reach it.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceReply {
    /// The service answered this call.
    Answered(Answer),
    /// No HTTP response at all: DNS, TLS, connect or read failure.
    NoResponse,
    /// No outcome arrived inside the bound this call was given (D0.13).
    NoOutcome,
}

impl ServiceReply {
    /// The transport failure this reply reports, or `None` when the service answered.
    ///
    /// It is the one place the reply's shape is read for its transport meaning, so a caller never
    /// has to reconstruct "did this call get an answer" from the variants.
    pub fn transport_failure(&self) -> Option<TransportFailure> {
        match self {
            Self::Answered(_) => None,
            Self::NoResponse => Some(TransportFailure::NoResponse),
            Self::NoOutcome => Some(TransportFailure::NoOutcome),
        }
    }

    /// The answer this reply carries, or `None` when the call produced none.
    pub fn answer(&self) -> Option<&Answer> {
        match self {
            Self::Answered(answer) => Some(answer),
            Self::NoResponse | Self::NoOutcome => None,
        }
    }
}

/// One write, as the transport is asked to submit it (SYNC-2, D0.14).
///
/// It carries the three things a submission needs and nothing else: the operation's identity, the
/// service function to call, and the body. The body stays an opaque JSON value on purpose — the
/// operation's *schema* belongs to the task that owns the operation (2.1's household creation, P1's
/// item commands), and a boundary that invented a schema it does not own would have to be rewritten
/// by every later one.
///
/// **Its only constructor refuses input that must never be sent**, which is what makes the
/// `validation` cause reachable from this path without borrowing the domain's own validation: a
/// blank operation identity would break the service's deduplication (D0.14 keys its ledger on
/// `(account_user_id, request_id)`, so a blank key could collide with another blank-keyed
/// operation), and a blank service function names nothing to call.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveRequest {
    /// The operation's identity — semantic, not diagnostic (D0.14).
    request_id: RequestId,
    /// The service function this write calls.
    rpc: String,
    /// The body the service's function receives.
    body: Value,
}

impl SaveRequest {
    /// Build one write.
    ///
    /// # Errors
    ///
    /// [`CommandError`] with the `validation` cause — not retryable, because a retry resends the
    /// same input — when the operation identity or the service function is blank.
    pub fn new(
        request_id: RequestId,
        rpc: impl Into<String>,
        body: Value,
    ) -> Result<Self, CommandError> {
        let rpc = rpc.into();

        if request_id.0.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id.clone())));
        }

        if rpc.trim().is_empty() {
            return Err(CommandError::validation(Some(request_id)));
        }

        Ok(Self {
            request_id,
            rpc,
            body,
        })
    }

    /// The operation's identity, carried unchanged into every failure this write reports (D0.14).
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// The service function this write calls.
    pub fn rpc(&self) -> &str {
        &self.rpc
    }

    /// The body the service's function receives.
    pub fn body(&self) -> &Value {
        &self.body
    }
}

/// The shared service, as the sync boundary uses it: two separate questions about availability and
/// one way to submit a write (D0.7 criterion 4).
///
/// The trait requires `Send + Sync` because the app holds one transport for the whole process and
/// probes and writes from different tasks; a double that could not be shared could not stand in for
/// it. It has no method for reading a collection: the read path is task 2.5's, and it arrives as its
/// own call on this seam rather than as something invented here.
pub trait SharedService: Send + Sync {
    /// Ask whether the service itself is answering — the `health` check of D0.7 criterion 4.
    ///
    /// Any HTTP answer means the service answered; only a transport failure means it did not.
    /// `bound` is the caller's limit on the call (see the module documentation for why it is a
    /// parameter): [`super::REACHABILITY_PROBE_TIMEOUT`] for a probe.
    fn check_service(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply>;

    /// Ask what the account looks like — the account service's own check of D0.7 criterion 4.
    ///
    /// It is a **separate** call from [`SharedService::check_service`], which is what makes
    /// `ServiceStatus.account_valid` a fresh outcome rather than a guess inferred from the service
    /// being up. A status of `2xx` means the account is valid, `401` that it is missing or invalid,
    /// and `429` that the service is temporarily rate limiting; anything else is answered but not
    /// classified here.
    fn check_account(&self, bound: Duration) -> ServiceFuture<'_, ServiceReply>;

    /// Submit one write.
    ///
    /// The implementation sends what [`SaveRequest`] describes and reports what came back. It does
    /// **not** decide whether the write may be attempted — that is [`super::gate`]'s, on a fresh
    /// probe — and it does not interpret its own answer: an `allowance_exceeded` report inside a
    /// `200`, for instance, is read one step up, where D0.12's rule that it is never a generic write
    /// error is tested.
    ///
    /// The lifetime is explicit because the returned future borrows both the service and the
    /// request; the shorthand form would tie it to the receiver alone and refuse to compile.
    fn save<'a>(
        &'a self,
        request: &'a SaveRequest,
        bound: Duration,
    ) -> ServiceFuture<'a, ServiceReply>;
}

/// The HTTP status the service answers when the account is missing or invalid (D0.7 criterion 4).
pub const HTTP_UNAUTHORIZED: u16 = 401;

/// The HTTP status the gateway answers when it is rate limiting the service (D0.7 criterion 4).
pub const HTTP_TOO_MANY_REQUESTS: u16 = 429;

/// The HTTP status a write surface answers when it refuses the request body.
pub const HTTP_BAD_REQUEST: u16 = 400;

/// The HTTP status a write surface answers when the body is well-formed but refused as invalid.
pub const HTTP_UNPROCESSABLE_CONTENT: u16 = 422;

/// The envelope token a completed write carries (D0.7 criterion 3): the service applied it.
pub const OUTCOME_APPLIED: &str = "applied";

/// The envelope token the request ledger returns for a re-issued operation (D0.14): the stored
/// outcome of the first attempt, which is a completed write and not a second one.
pub const OUTCOME_DEDUPLICATED: &str = "deduplicated";

/// The envelope token of the service's own report that the household owner's allowance is
/// exhausted (D0.7 criterion 5, D0.12).
///
/// The wire token is the same one [`SaveCause::AllowanceExceeded`] serialises to, which is what
/// keeps it from being collapsed into a generic write error by a translation table.
pub const OUTCOME_ALLOWANCE_EXCEEDED: &str = "allowance_exceeded";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::failure::policy;

    #[test]
    fn each_transport_condition_names_its_own_cause_and_is_retryable() {
        assert_eq!(
            TransportFailure::NoResponse.cause(),
            SaveCause::Connectivity
        );
        assert_eq!(TransportFailure::NoOutcome.cause(), SaveCause::Timeout);

        // Both are conditions a later attempt can clear, which is the policy `ipc::failure` owns:
        // the seam does not restate the flag, it relies on the cause.
        for failure in [TransportFailure::NoResponse, TransportFailure::NoOutcome] {
            assert!(policy(failure.cause()).retryable);
        }
    }

    #[test]
    fn only_an_answered_call_has_an_answer() {
        let answered = ServiceReply::Answered(Answer::status_only(200));

        assert_eq!(answered.transport_failure(), None);
        assert!(answered.answer().is_some());

        for reply in [ServiceReply::NoResponse, ServiceReply::NoOutcome] {
            assert!(reply.transport_failure().is_some());
            assert!(reply.answer().is_none());
        }
    }

    #[test]
    fn a_success_is_a_2xx_status_and_nothing_else() {
        assert!(Answer::status_only(200).is_success());
        assert!(Answer::status_only(204).is_success());
        assert!(Answer::status_only(299).is_success());
        assert!(!Answer::status_only(199).is_success());
        assert!(!Answer::status_only(300).is_success());

        for status in [HTTP_UNAUTHORIZED, HTTP_TOO_MANY_REQUESTS, 500] {
            assert!(!Answer::status_only(status).is_success());
        }
    }

    #[test]
    fn the_wire_codes_and_tokens_are_the_ones_the_decision_names() {
        assert_eq!(HTTP_UNAUTHORIZED, 401);
        assert_eq!(HTTP_TOO_MANY_REQUESTS, 429);
        assert_eq!(HTTP_BAD_REQUEST, 400);
        assert_eq!(HTTP_UNPROCESSABLE_CONTENT, 422);
        assert_eq!(OUTCOME_APPLIED, "applied");
        assert_eq!(OUTCOME_DEDUPLICATED, "deduplicated");

        // The allowance token is the cause's own wire token (D0.7 criterion 5), so no translation
        // table can collapse the condition into a generic write error.
        let cause = SaveCause::AllowanceExceeded;
        let serialised = serde_json::to_value(cause).expect("a cause serialises");
        assert_eq!(
            serialised,
            Value::String(OUTCOME_ALLOWANCE_EXCEEDED.to_owned())
        );
    }

    #[test]
    fn a_write_with_a_blank_identity_or_function_is_refused_as_invalid_input() {
        let identity = RequestId("   ".to_owned());
        let blank_identity = SaveRequest::new(identity, "create_household", Value::Null);
        let blank_rpc = SaveRequest::new(RequestId("operation-1".to_owned()), "  ", Value::Null);

        for refused in [blank_identity, blank_rpc] {
            let error = refused.expect_err("the input is refused");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.message_key, policy(SaveCause::Validation).message_key);
            assert!(!error.retryable);
        }
    }

    #[test]
    fn a_well_formed_write_keeps_its_identity_function_and_body() {
        let request_id = RequestId("operation-1".to_owned());
        let body = Value::String("Home".to_owned());
        let request = SaveRequest::new(request_id.clone(), "create_household", body.clone())
            .expect("a well-formed write");

        assert_eq!(request.request_id(), &request_id);
        assert_eq!(request.rpc(), "create_household");
        assert_eq!(request.body(), &body);
    }
}
