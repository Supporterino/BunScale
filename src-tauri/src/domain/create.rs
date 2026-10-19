//! Create one household through the reachability-gated write path (HS-1, SYNC-2; D0.1, D0.14).
//!
//! This is the operation task 2.1 owns, and it is deliberately the only place a household is
//! written. It does four things in order, and the order is part of the behaviour:
//!
//! 1. **Take a validated name.** A [`HouseholdName`] cannot be blank — its constructor refuses
//!    empty and whitespace-only input — so the command validates first, before the account is
//!    consulted and before any probe is made, and a refused input never depends on the service's
//!    state (HS-1).
//! 2. **Describe the write.** The [`SaveRequest`] carries the operation's identity and the body the
//!    service function receives: the name and the same `request_id`. The identity is in the body as
//!    well as on the request because the service's ledger keys on `(account_user_id, request_id)`
//!    (D0.7 criterion 6, D0.14); the transport sends the body verbatim, so this is where the
//!    service can read it.
//! 3. **Go through [`SaveGate`].** No reachability or no valid account refuses the attempt before a
//!    write is sent, and every refusal is the gate's mapped [`CommandError`] — this module invents
//!    no cause, no key and no second write path (SYNC-2).
//! 4. **Read the service-assigned household out of the accepted answer.** The gate guarantees the
//!    envelope reports a completed write; it does not own the operation's schema. This module does,
//!    and an answer it cannot read is `unknown` rather than a household: a `2xx` is never accepted
//!    as a successful create by this boundary (SYNC-2).
//!
//! The accepted answer's schema is the one the service function returns:
//!
//! ```json
//! {
//!   "outcome": "applied" | "deduplicated",
//!   "household": {
//!     "id": "<opaque household id>",
//!     "owner_member_id": "<the creator's member id>",
//!     "created_at": "<RFC 3339 instant>"
//!   }
//! }
//! ```
//!
//! `outcome` is read by the gate; this module reads only the assigned identity and instant. The
//! name is the validated input, the owner is the creator, and the member's account is the account
//! that made the request — so the domain constructs the record from facts it already holds and the
//! service assigns only what it alone can assign (D0.3, D0.5). A replay returns the stored outcome
//! of the first call, which is the same household body (D0.14).

use serde_json::{Map, Value};

use super::household::{HouseholdName, HouseholdRecord};
use crate::ipc::types::{AccountUserId, CommandError, HouseholdId, Instant, MemberId, RequestId};
use crate::sync::{SaveGate, SaveRequest, SharedService};

/// The shared-service function that creates a household.
///
/// It is the operation's schema, owned here rather than by the gate: the transport uses it as the
/// RPC path (`/rest/v1/rpc/<rpc>`) and the gate keeps it opaque (D0.7).
pub const CREATE_HOUSEHOLD_RPC: &str = "create_household";

/// Create one household and return the domain record the service confirmed.
///
/// The caller is the account that will be recorded as the creator; there is no identity argument
/// on `request_id` because the request identity is a separate fact. The [`RequestId`] is carried to
/// the service unchanged, which is what makes a retry safe (D0.14).
///
/// # Errors
///
/// The gate's own causes for the attempt: `connectivity`, `no_account`, `allowance_exceeded`,
/// `rate_limited`, `timeout`, `validation` and `unknown`. An accepted answer whose household body is
/// missing or unreadable is `unknown`; it is never a created household (SYNC-2).
pub async fn create_household(
    service: &dyn SharedService,
    account: &AccountUserId,
    name: HouseholdName,
    request_id: RequestId,
) -> Result<HouseholdRecord, CommandError> {
    let body = create_body(&name, &request_id);
    let request = SaveRequest::new(request_id.clone(), CREATE_HOUSEHOLD_RPC, body)?;

    // The one write path: the gate probes fresh, refuses the attempt when it cannot complete, and
    // returns a receipt only for an answer whose envelope reported the write as completed.
    let receipt = SaveGate::new(service).save(request).await?;
    let assigned = assigned_household(receipt.body(), &request_id)?;

    Ok(HouseholdRecord::created(
        assigned.id,
        name,
        assigned.created_at,
        assigned.owner_member_id,
        account.clone(),
    ))
}

/// The body the service function receives: the name and the operation's identity.
///
/// The identity is repeated here because the service's ledger must see it (D0.14); the gate keeps
/// the same value on the [`SaveRequest`] for the transport and for every failure it reports.
fn create_body(name: &HouseholdName, request_id: &RequestId) -> Value {
    let mut body = Map::new();
    body.insert("request_id".to_owned(), Value::String(request_id.0.clone()));
    body.insert("name".to_owned(), Value::String(name.as_str().to_owned()));

    Value::Object(body)
}

/// The facts the service assigned to a created household.
struct AssignedHousehold {
    /// The opaque household id, which also addresses the household's collection.
    id: HouseholdId,
    /// The member id the service assigned to the creator.
    owner_member_id: MemberId,
    /// When the service recorded the creation (D0.5).
    created_at: Instant,
}

/// Read the service-assigned facts out of an accepted answer's household body.
///
/// Every field is required and must be a non-blank string. A missing, blank or wrongly typed field
/// is an answer this boundary cannot read — `unknown`, not a partially built household — because
/// the one thing that must never happen is presenting a write as created when its identity is not
/// known (SYNC-2).
fn assigned_household(
    body: &Value,
    request_id: &RequestId,
) -> Result<AssignedHousehold, CommandError> {
    let household = body
        .get("household")
        .and_then(Value::as_object)
        .ok_or_else(|| unreadable(request_id))?;

    Ok(AssignedHousehold {
        id: HouseholdId(field(household, "id", request_id)?.to_owned()),
        owner_member_id: MemberId(field(household, "owner_member_id", request_id)?.to_owned()),
        created_at: Instant(field(household, "created_at", request_id)?.to_owned()),
    })
}

/// One required, non-blank string field of the household body.
fn field<'a>(
    household: &'a Map<String, Value>,
    name: &str,
    request_id: &RequestId,
) -> Result<&'a str, CommandError> {
    household
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| unreadable(request_id))
}

/// The one failure for an accepted answer this boundary cannot interpret.
///
/// `unknown` is the honest cause — the request was accepted, so the input was not refused, but the
/// product cannot say which household was created — and it is built through the contract's
/// constructor rather than assembled here (D0.6).
fn unreadable(request_id: &RequestId) -> CommandError {
    CommandError::unknown(Some(request_id.clone()))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::domain::HouseholdBook;
    use crate::domain::household::validate_name;
    use crate::ipc::types::SaveCause;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::double::{RecordedCall, ScriptedService};
    use crate::sync::{Answer, ServiceReply};

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn member(value: &str) -> MemberId {
        MemberId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    /// One accepted create answer, in the schema this module reads.
    fn accepted_answer(
        outcome: &str,
        household_id: &str,
        owner_member_id: &str,
        created_at: &str,
    ) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "outcome": outcome,
                "household": {
                    "id": household_id,
                    "owner_member_id": owner_member_id,
                    "created_at": created_at,
                },
            }),
        ))
    }

    /// Run one create against `service`, validating `name` exactly as the command does.
    fn create_with(
        service: &ScriptedService,
        name: &str,
        request_id: RequestId,
    ) -> Result<HouseholdRecord, CommandError> {
        let name = validate_name(name, &request_id)?;

        block_on_ready(create_household(service, &account(), name, request_id))
    }

    /// Every write the transport was asked to submit, in order.
    fn writes(service: &ScriptedService) -> Vec<SaveRequest> {
        service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RecordedCall::Save { request, .. } => Some(request),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_blank_name_is_refused_before_the_gate_is_called() {
        for blank in ["", "   ", "\t\n"] {
            let service = ScriptedService::new();
            let request_id = request_id("operation-1");

            let error = create_with(&service, blank, request_id.clone())
                .expect_err("a blank name is refused");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.message_key, "error.validation");
            assert!(!error.retryable);
            assert_eq!(error.request_id, Some(request_id));

            // Nothing was probed and nothing was sent: the refusal is a property of the input.
            assert!(service.calls().is_empty());
            assert_eq!(service.write_count(), 0);
        }
    }

    #[test]
    fn the_writer_carries_the_identity_and_the_name_to_the_service() {
        let service = ScriptedService::new().on_write([accepted_answer(
            "applied",
            "household-1",
            "member-1",
            "2026-10-19T10:00:00Z",
        )]);

        let record = create_with(&service, "Home", request_id("operation-1"))
            .expect("the service accepted the write");
        let writes = writes(&service);

        assert_eq!(record.id(), &household("household-1"));
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].rpc(), CREATE_HOUSEHOLD_RPC);
        assert_eq!(writes[0].request_id(), &request_id("operation-1"));

        let body = writes[0].body();
        assert_eq!(body.get("name").and_then(Value::as_str), Some("Home"));
        assert_eq!(
            body.get("request_id").and_then(Value::as_str),
            Some("operation-1")
        );
    }

    #[test]
    fn the_creator_is_recorded_as_the_first_member_and_owner() {
        let service = ScriptedService::new().on_write([accepted_answer(
            "applied",
            "household-1",
            "member-1",
            "2026-10-19T10:00:00Z",
        )]);

        let record = create_with(&service, "Home", request_id("operation-1"))
            .expect("the service accepted the write");
        let owner = record.owner().expect("the creator is a member");

        assert_eq!(record.members().len(), 1);
        assert_eq!(record.members()[0], *owner);
        assert_eq!(owner.id, member("member-1"));
        assert_eq!(owner.account_user_id, account());
        assert_eq!(record.owner_member_id(), &member("member-1"));
        assert_eq!(record.created_at(), &at("2026-10-19T10:00:00Z"));

        let wire = record.to_wire(&account(), true);
        assert!(wire.is_owner);
        assert!(wire.is_active);
    }

    #[test]
    fn a_created_household_is_the_only_active_one() {
        let service = ScriptedService::new().on_write([
            accepted_answer("applied", "household-1", "member-1", "2026-10-19T10:00:00Z"),
            accepted_answer("applied", "household-2", "member-2", "2026-10-19T10:01:00Z"),
        ]);
        let mut book = HouseholdBook::new();

        let first = create_with(&service, "First", request_id("operation-1"))
            .expect("the first create completed");
        let first = book.record_created(request_id("operation-1"), first);
        let second = create_with(&service, "Second", request_id("operation-2"))
            .expect("the second create completed");
        let second = book.record_created(request_id("operation-2"), second);

        assert_eq!(book.len(), 2);
        assert_eq!(book.active().expect("one is active").id(), second.id());
        assert!(!book.is_active(first.id()));
        assert!(book.is_active(second.id()));

        let first_wire = first.to_wire(&account(), book.is_active(first.id()));
        let second_wire = second.to_wire(&account(), book.is_active(second.id()));
        assert!(!first_wire.is_active);
        assert!(second_wire.is_active);
    }

    #[test]
    fn a_replayed_request_id_returns_the_existing_household_and_never_a_second_one() {
        let service = ScriptedService::new().on_write([
            accepted_answer("applied", "household-1", "member-1", "2026-10-19T10:00:00Z"),
            // The service's stored outcome for the re-issued operation (D0.14).
            accepted_answer(
                "deduplicated",
                "household-1",
                "member-1",
                "2026-10-19T10:00:00Z",
            ),
        ]);
        let mut book = HouseholdBook::new();
        let operation = request_id("operation-1");

        let first =
            create_with(&service, "Home", operation.clone()).expect("the first create completed");
        let first = book.record_created(operation.clone(), first);
        let replay = create_with(&service, "Home", operation.clone())
            .expect("the replay returns the stored outcome");
        let replay = book.record_created(operation.clone(), replay);

        assert_eq!(first.id(), replay.id());
        assert_eq!(book.len(), 1);
        assert_eq!(
            book.active().expect("one is active").id(),
            &household("household-1")
        );

        // The replay is still a request carrying the same semantic identity: that is what the
        // service's ledger deduplicates, and it is not replaced by a fresh id here.
        let writes = writes(&service);
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].request_id(), &operation);
        assert_eq!(writes[1].request_id(), &operation);
        assert_eq!(writes[0].body(), writes[1].body());
    }

    #[test]
    fn an_accepted_answer_without_a_readable_household_is_never_a_success() {
        let service = ScriptedService::new().on_write([
            ServiceReply::Answered(Answer::with_body(200, json!({ "outcome": "applied" }))),
            ServiceReply::Answered(Answer::with_body(
                200,
                json!({
                    "outcome": "applied",
                    "household": { "id": "household-1" },
                }),
            )),
            ServiceReply::Answered(Answer::with_body(
                200,
                json!({
                    "outcome": "applied",
                    "household": {
                        "id": " ",
                        "owner_member_id": "member-1",
                        "created_at": "2026-10-19T10:00:00Z",
                    },
                }),
            )),
        ]);

        for _ in 0..3 {
            let error = create_with(&service, "Home", request_id("operation-1"))
                .expect_err("the answer cannot be read as a household");

            assert_eq!(error.cause, SaveCause::Unknown);
            assert_eq!(error.message_key, "error.unknown");
            assert!(!error.retryable);
            assert_eq!(error.request_id, Some(request_id("operation-1")));
        }
    }

    #[test]
    fn a_gate_refusal_is_reported_unchanged() {
        let service = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);

        let error = create_with(&service, "Home", request_id("operation-1"))
            .expect_err("the gate refuses an unreachable service");

        assert_eq!(error.cause, SaveCause::Connectivity);
        assert_eq!(error.message_key, "error.connectivity");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id("operation-1")));
        assert_eq!(service.write_count(), 0);
    }
}
