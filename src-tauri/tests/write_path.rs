//! Integration tests for the write path (task 5.4): SYNC-2's three save scenarios, NFR-1's
//! reachability model, and the two constraints the design adds — D0.12's allowance cause and
//! D0.13's bounds.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/sync-and-offline/spec.md` states for
//! SYNC-2 and `specs/product-quality/spec.md` for NFR-1, and each scenario has a test named after
//! it; the two rules the task names separately (the stalled attempt's bound, the offline threshold)
//! have their own tests beside them.
//!
//! # The boundary these tests drive
//!
//! A save command is a thin adapter over [`SaveGate`]: the gate takes the transport and one
//! [`SaveRequest`], probes the service and the account fresh, and either submits the write or
//! refuses it — nothing else, no queue and no storage handle (D0.1). So these tests drive the gate
//! itself with the published [`ScriptedService`] as the transport, and drive the status through
//! [`re_evaluate`] with a recording [`StatusPublisher`], exactly as the running app's two triggers
//! do. The read half of NFR-1 goes through [`ReadPath`] and an [`AppState`] holding the same
//! transport. `app_lib::run()` and `ipc::export_bindings()` are never called, as
//! `library_surface.rs` records. Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # The 15-second bound, and what this layer can prove about it
//!
//! The gate takes no timeout parameter: [`SAVE_ATTEMPT_TIMEOUT`] is a constant it hands to the
//! transport on every attempt, because a limit can only be enforced where the call is awaited
//! (`sync`'s own documentation of the constant). There is therefore no shorter bound to inject, so
//! this file does not wait 15 seconds: the scripted double reports the no-outcome condition
//! immediately, which is exactly the state a passed bound produces, and the stalled attempt's test
//! asserts the cause, the constant and the bound the attempt was given. **What no test here proves
//! is that the real client honours it** — a transport-level test with a stalling listener would,
//! and none is built.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: "the person is told the cause" and "a retry is offered" are asserted
//! as the failure surface's cause, message key and `retryable` flag — the facts the UI renders —
//! and the retry itself is asserted as the same operation re-issued (D0.14).
//!
//! SYNC-2's "the change is part of the household's shared collection for every member" is not
//! asserted either: that is the service's own write path, and the client-side half — one collection
//! per household — is 5.3's while convergence is 5.6's. At this boundary a save is a request and an
//! outcome, so nothing here can observe what another member resolves.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use app_lib::ipc::types::{
    CollectionItem, CollectionSource, CommandError, HouseholdCollection, HouseholdId, Instant,
    ItemId, RequestId, SaveCause, ServiceState, ServiceStatus,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{
    Answer, REACHABILITY_PROBE_TIMEOUT, ReadPath, SAVE_ATTEMPT_TIMEOUT, SaveGate, SaveReceipt,
    SaveRequest, ServiceReply, StatusPublisher, current_status, re_evaluate,
};

/// The instant the household's collection is loaded at, before the service goes away.
const LOADED_AT: &str = "2026-10-19T11:00:00Z";

/// The version of the one loaded item.
const VERSION: &str = "2026-10-19T10:00:00Z";

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One row of a household's collection, for the read half of NFR-1.
fn item(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: ItemId(id.to_owned()),
        updated_at: at(updated_at),
        deleted: false,
    }
}

/// One household's wire collection, as a guarded load takes it.
fn collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// The write one save command submits: a new or modified item, under the operation's identity.
fn write_of(operation: &RequestId) -> SaveRequest {
    SaveRequest::new(
        operation.clone(),
        "save_item",
        json!({ "item_id": "item-1" }),
    )
    .expect("a well-formed write")
}

/// Run one gated save of `operation` through `service`.
///
/// The gate is the whole behaviour of a save command apart from its `#[tauri::command]` wrapper: it
/// holds the transport and nothing else. `&Arc<ScriptedService>` reaches it by the same deref
/// coercion `&ScriptedService` does.
fn save_with(
    service: &ScriptedService,
    operation: &RequestId,
) -> Result<SaveReceipt, CommandError> {
    block_on_ready(SaveGate::new(service).save(write_of(operation)))
}

/// Every write the transport was asked to submit, as the bound it was given (D0.13).
fn save_bounds(service: &ScriptedService) -> Vec<Duration> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { bound, .. } => Some(bound),
            _ => None,
        })
        .collect()
}

/// Every write the transport was asked to submit, as the operation identity it carried (D0.14).
fn save_identities(service: &ScriptedService) -> Vec<RequestId> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { request, .. } => Some(request.request_id().clone()),
            _ => None,
        })
        .collect()
}

/// An answered check with no body: the service is up, or the account is valid.
fn healthy() -> ServiceReply {
    ServiceReply::Answered(Answer::status_only(200))
}

/// One readable write envelope: the outcome token the gate classifies (D0.7 criterion 5).
fn envelope(outcome: &str) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(200, json!({ "outcome": outcome })))
}

/// A double whose probe answers `service` and `account` and whose write answers `write`.
fn reporting(service: ServiceReply, account: ServiceReply, write: ServiceReply) -> ScriptedService {
    ScriptedService::new()
        .on_service_check([service])
        .on_account_check([account])
        .on_write([write])
}

/// Assert one gated save is refused with `cause`, its own `key` and the retry policy `retryable`.
///
/// `sent` is how many writes reached the transport: a refusal decided by the probe sends none, and
/// that is what "nothing is written to the shared collection" means at the request level.
///
/// The key is passed as a literal rather than read back from `ipc::failure::policy`: a mapping that
/// returned one key for two causes would otherwise keep both sides of the equality in step, and the
/// swapped key would pass.
fn assert_refused(
    service: &ScriptedService,
    operation: &RequestId,
    cause: SaveCause,
    key: &str,
    retryable: bool,
    sent: usize,
) -> CommandError {
    let error = save_with(service, operation).expect_err("the write did not complete");

    assert_eq!(error.cause, cause);
    assert_eq!(error.retryable, retryable, "{cause:?}");
    assert_eq!(error.message_key, key, "{cause:?} names its own copy");
    assert_eq!(
        error.request_id.as_ref(),
        Some(operation),
        "{cause:?} keeps the operation identity"
    );
    assert_eq!(service.write_count(), sent, "{cause:?}: writes sent");

    error
}

/// The publication seam's test side: it records what the running app would have emitted.
#[derive(Debug, Default)]
struct Recorder {
    published: Mutex<Vec<ServiceStatus>>,
}

impl Recorder {
    fn published(&self) -> Vec<ServiceStatus> {
        self.published
            .lock()
            .expect("the records are readable")
            .clone()
    }
}

impl StatusPublisher for Recorder {
    fn publish(&self, status: &ServiceStatus) {
        self.published
            .lock()
            .expect("the records are writable")
            .push(status.clone());
    }
}

/// SYNC-2, "Save refused without connectivity".
///
/// **WHEN** the shared service is unreachable and a person attempts to save new or modified data
/// **THEN** nothing is written to the shared collection, the person is told that connectivity is
/// required, and the attempted data is not presented as saved.
#[test]
fn save_refused_without_connectivity() {
    let operation = request_id("operation-1");
    let service = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);

    // THEN nothing is written to the shared collection — the transport was never asked to submit a
    // write, and the account check was not spent on a service that did not answer — and the person
    // is told that connectivity is required, in this cause's own words.
    assert_refused(
        &service,
        &operation,
        SaveCause::Connectivity,
        "error.connectivity",
        true,
        0,
    );
    assert_eq!(
        service.calls(),
        vec![RecordedCall::CheckService {
            bound: REACHABILITY_PROBE_TIMEOUT,
        }]
    );
}

/// SYNC-2, "Save succeeds when the service is reachable".
///
/// **WHEN** the shared service is reachable and the account is valid and a person saves new or
/// modified data **THEN** the save completes, and the change is part of the household's shared
/// collection for every member.
///
/// The completion exists as exactly one shape: a `2xx` whose envelope reports the write as
/// completed (SYNC-2, NFR-1). Every other `2xx` — including one this boundary cannot read — is
/// refused, so a success is never inferred from the status code.
#[test]
fn save_succeeds_when_the_service_is_reachable() {
    let operation = request_id("operation-1");

    // `deduplicated` is the stored outcome of a re-issued operation: the same completed write
    // (D0.14), so both tokens complete a save.
    for outcome in ["applied", "deduplicated"] {
        let service = ScriptedService::new().on_write([envelope(outcome)]);

        let receipt = save_with(&service, &operation).expect("the service completed the write");

        assert_eq!(
            receipt.request_id(),
            &operation,
            "the receipt names the operation"
        );
        assert_eq!(
            receipt.body().get("outcome").and_then(Value::as_str),
            Some(outcome)
        );

        // The one attempt was made with D0.13's save bound, after a probe of the service and of the
        // account each with the probe bound, and it carried the caller's own write.
        assert_eq!(
            service.calls(),
            vec![
                RecordedCall::CheckService {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
                RecordedCall::CheckAccount {
                    bound: REACHABILITY_PROBE_TIMEOUT,
                },
                RecordedCall::Save {
                    bound: SAVE_ATTEMPT_TIMEOUT,
                    request: write_of(&operation),
                },
            ]
        );
    }

    // A `2xx` is not a completed write by itself. An unreadable envelope — no body, an unknown
    // outcome token, a body that is not an envelope — is refused, and no retry is promised for it
    // because nothing is known about the failure (NFR-8).
    let unreadable = [
        ServiceReply::Answered(Answer::status_only(200)),
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({ "outcome": "something-else" }),
        )),
        ServiceReply::Answered(Answer::with_body(200, Value::Null)),
    ];

    for reply in unreadable {
        let service = ScriptedService::new().on_write([reply]);

        assert_refused(
            &service,
            &operation,
            SaveCause::Unknown,
            "error.unknown",
            false,
            1,
        );
    }
}

/// SYNC-2, "Save failure names its cause and offers a retry".
///
/// **WHEN** a save fails because the account is unavailable, the storage allowance is exceeded, or
/// the service is temporarily rate limiting **THEN** the message identifies that specific cause, an
/// exhausted owner allowance is named as its own cause and never as a generic write failure, and a
/// retry of the save is offered.
#[test]
fn save_failure_names_its_cause_and_offers_a_retry() {
    let operation = request_id("operation-1");

    // Each service-reported condition names itself, offers the retry, keeps the operation identity,
    // and is refused before a write is described to the transport when the probe refused it. The
    // key each case expects is a literal, not the mapping the gate reads.
    let unreachable = ScriptedService::new().on_service_check([ServiceReply::NoResponse]);
    let unidentified = reporting(
        healthy(),
        ServiceReply::Answered(Answer::status_only(401)),
        envelope("applied"),
    );
    let limited = reporting(
        healthy(),
        ServiceReply::Answered(Answer::status_only(429)),
        envelope("applied"),
    );
    let stalled = reporting(healthy(), healthy(), ServiceReply::NoOutcome);

    let cases = [
        (
            unreachable,
            SaveCause::Connectivity,
            "error.connectivity",
            0,
        ),
        (unidentified, SaveCause::NoAccount, "error.no_account", 0),
        (limited, SaveCause::RateLimited, "error.rate_limited", 0),
        (stalled, SaveCause::Timeout, "error.timeout", 1),
    ];

    for (service, cause, key, sent) in cases {
        assert_refused(&service, &operation, cause, key, true, sent);
    }

    // D0.12: the exhausted allowance is the service's own report, named as itself. It is neither
    // the generic write error nor the placeholder, and it still offers the retry SYNC-2 promises.
    let exhausted = assert_refused(
        &reporting(healthy(), healthy(), envelope("allowance_exceeded")),
        &operation,
        SaveCause::AllowanceExceeded,
        "error.allowance_exceeded",
        true,
        1,
    );
    assert_ne!(
        exhausted,
        CommandError::unknown(Some(operation.clone())),
        "an exhausted allowance is never a generic write failure"
    );

    // A refused input is the one condition a retry cannot clear — the same request would be refused
    // again — so the surface must not offer one.
    let invalid = reporting(
        healthy(),
        healthy(),
        ServiceReply::Answered(Answer::status_only(400)),
    );
    assert_refused(
        &invalid,
        &operation,
        SaveCause::Validation,
        "error.validation",
        false,
        1,
    );

    // The retry the surface offers is the refusal's own operation re-issued: the allowance clears
    // and the second attempt completes under the same request identity (D0.14).
    let service =
        ScriptedService::new().on_write([envelope("allowance_exceeded"), envelope("applied")]);
    let offered = assert_refused(
        &service,
        &operation,
        SaveCause::AllowanceExceeded,
        "error.allowance_exceeded",
        true,
        1,
    );
    assert!(offered.retryable, "a retry is offered");

    let retried = save_with(&service, &operation).expect("the retry completes");
    assert_eq!(
        retried.request_id(),
        &operation,
        "the retry re-issues the same operation"
    );
    assert_eq!(
        save_identities(&service),
        vec![operation.clone(), operation],
        "one operation, sent twice"
    );
}

/// NFR-1, "Writes are accepted only while reachable and identified".
///
/// **WHEN** the shared service is reachable and the account is valid **THEN** new and modified data
/// can be saved.
///
/// The "only" half is asserted too: a service that answers without identifying the account refuses
/// the write before it is described to the transport, so reachability alone never authorizes one.
#[test]
fn writes_are_accepted_only_while_reachable_and_identified() {
    let operation = request_id("operation-1");

    let service = ScriptedService::new().on_write([envelope("applied")]);
    let receipt = save_with(&service, &operation).expect("the service accepted the write");
    assert_eq!(receipt.request_id(), &operation);
    assert_eq!(service.write_count(), 1, "the write was sent");

    // The account check answered, but not with an identified account. A `401` is the account,
    // `429` the gateway and an unreadable status neither — and each refuses the attempt with its
    // own cause and its own literal key.
    let unidentified = [
        (401, SaveCause::NoAccount, "error.no_account", true),
        (429, SaveCause::RateLimited, "error.rate_limited", true),
        (500, SaveCause::Unknown, "error.unknown", false),
    ];

    for (status, cause, key, retryable) in unidentified {
        let service = reporting(
            healthy(),
            ServiceReply::Answered(Answer::status_only(status)),
            envelope("applied"),
        );

        assert_refused(&service, &operation, cause, key, retryable, 0);

        // The probe's two checks and nothing else: the transport was never asked to submit a write.
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
}

/// NFR-1, "Writes are refused while unreachable, reads are not".
///
/// **WHEN** the shared service is unreachable **THEN** no new or modified data is accepted as
/// saved, data that was already loaded remains readable, the cause is stated to the person, and the
/// status is reported as offline only after three consecutive reachability checks fail without an
/// intervening success.
#[test]
fn writes_are_refused_while_unreachable_reads_are_not() {
    let household = household_id("household-1");
    let operation = request_id("operation-1");
    let loaded = vec![item("item-1", VERSION)];
    let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
    let state = AppState::new(service.clone(), None);
    state
        .load_collection(
            &household,
            collection(&household, loaded.clone()),
            at(LOADED_AT),
        )
        .expect("the copy is addressed correctly");

    // THEN no new data is accepted as saved, and the cause is stated to the person.
    assert_refused(
        &service,
        &operation,
        SaveCause::Connectivity,
        "error.connectivity",
        true,
        0,
    );

    // AND data that was already loaded remains readable: the read is served from this device's
    // copy, and only the refresh it would have attempted is skipped (NFR-1, D0.2).
    let read = block_on_ready(ReadPath::new().load(&state, &household))
        .expect("a read is not refused while unreachable");
    assert_eq!(read.source, CollectionSource::Local);
    assert_eq!(read.loaded_at, Some(at(LOADED_AT)));
    assert_eq!(read.items, loaded);
    assert_eq!(service.read_count(), 0, "no refresh was attempted");

    // Both attempts stopped at the probe: no account check (the service did not answer), and no
    // write or read call at all. That is the asymmetry — the write is refused, the read is served.
    assert_eq!(
        service.calls(),
        vec![
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
        ]
    );

    // AND a refused write is not itself a status observation: the tracker is the probe's surface,
    // not the gate's (D0.2). The count is what tells a never-observed tracker from one that has
    // already recorded a failure.
    assert_eq!(current_status(&state).state, ServiceState::Unknown);
    assert_eq!(state.status().consecutive_failures(), 0);

    // AND the status is reported offline only after three consecutive reachability checks fail.
    let recorder = Recorder::default();
    let published: Vec<ServiceStatus> = (0..3)
        .map(|_| block_on_ready(re_evaluate(&state, &recorder)))
        .collect();

    assert_eq!(
        published[0].state,
        ServiceState::Unknown,
        "one failed check is a blip, not an outage"
    );
    assert_eq!(published[1].state, ServiceState::Unknown);
    assert_eq!(
        published[2].state,
        ServiceState::Unreachable,
        "the third consecutive failure is offline"
    );
    assert_eq!(published[2].cause, Some(SaveCause::Connectivity));
    assert!(published[2].observed_at.is_some(), "the outage is timed");
    assert_eq!(state.status().consecutive_failures(), 3);
    assert_eq!(
        recorder.published(),
        published,
        "every fresh outcome is reported as it is recorded"
    );
}

/// D0.13: a stalled attempt is refused with `timeout` and carries the documented 15-second bound.
///
/// There is no injectable shorter bound (see the module documentation), so this asserts the cause,
/// the constant and the bound the attempt was given — not that the gate itself waited: resolution
/// time is the transport's to enforce, and the double reports the no-outcome condition immediately.
#[test]
fn a_stalled_attempt_is_refused_with_timeout_and_carries_the_documented_bound() {
    let operation = request_id("operation-1");
    let service = ScriptedService::new().on_write([ServiceReply::NoOutcome]);

    assert_refused(
        &service,
        &operation,
        SaveCause::Timeout,
        "error.timeout",
        true,
        1,
    );

    assert_eq!(SAVE_ATTEMPT_TIMEOUT, Duration::from_secs(15));
    assert_eq!(
        save_bounds(&service),
        vec![SAVE_ATTEMPT_TIMEOUT],
        "the attempt carried the documented bound"
    );
}

/// D0.13: the offline status needs three consecutive failed checks, and an answered check in
/// between resets the run rather than extending it.
#[test]
fn an_intervening_success_resets_the_offline_count() {
    let service = Arc::new(ScriptedService::new().on_service_check([
        ServiceReply::NoResponse,
        ServiceReply::NoResponse,
        healthy(),
        ServiceReply::NoResponse,
        ServiceReply::NoResponse,
        ServiceReply::NoResponse,
    ]));
    let state = AppState::new(service, None);
    let recorder = Recorder::default();

    let published: Vec<ServiceStatus> = (0..6)
        .map(|_| block_on_ready(re_evaluate(&state, &recorder)))
        .collect();

    assert_eq!(published[0].state, ServiceState::Unknown);
    assert_eq!(published[1].state, ServiceState::Unknown);
    assert_eq!(
        published[2].state,
        ServiceState::Reachable,
        "an answer resets the run"
    );
    assert_eq!(published[2].account_valid, Some(true));

    // The two failures after the success start a new run. The status stays the last thing actually
    // observed, including the instant it was observed at.
    let observed = published[2].observed_at.clone();
    assert_eq!(published[3].state, ServiceState::Reachable);
    assert_eq!(published[4].state, ServiceState::Reachable);
    assert_eq!(
        published[4].observed_at, observed,
        "a retained status keeps its own observation instant"
    );

    assert_eq!(
        published[5].state,
        ServiceState::Unreachable,
        "three failures after the success are an outage"
    );
    assert_eq!(published[5].cause, Some(SaveCause::Connectivity));
    assert_eq!(state.status().consecutive_failures(), 3);
    assert_eq!(recorder.published(), published);
}
