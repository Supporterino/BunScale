//! Integration tests for household creation (task 5.1): HS-1's two create scenarios, and the
//! Rust-side facts HS-2's first-run onboarding guard consumes.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/household-sharing/spec.md` states for
//! creation and for the first-run guard. Each test is named after the scenario it covers; the extra
//! tests below pin behaviour the scenarios imply — a name's edges and a replayed create.
//!
//! # The boundary these tests drive
//!
//! The command adapters in `ipc::commands::household` are thin forwarders over [`AppState`]: each
//! takes `tauri::State<'_, AppState>` and calls the matching method on the state. A `tauri::State`
//! cannot be constructed outside a running app, so these tests drive the same [`AppState`] methods
//! the commands forward to — the whole behaviour of the command apart from the
//! `#[tauri::command]` wrapper — and install the published [`ScriptedService`] double as the
//! transport, exactly as the crate's unit tests do. Both platforms (TOOL-1): nothing here is
//! platform-specific. `app_lib::run()` is never called (it starts the event loop) and
//! `ipc::export_bindings()` is never called (D0.9: `cargo test` precedes the bindings drift check),
//! as `library_surface.rs` records.
//!
//! # What is deliberately not asserted here
//!
//! HS-1's "the person is prompted to supply a name" and HS-2's "the onboarding flow is presented" /
//! "is not presented" are rendering decisions. Their Rust-observable half — which households are
//! held and which of them is active — is what these tests assert; the component and E2E tasks own
//! the presentation itself.
//!
//! # What the tests pin about names
//!
//! "Blank" is HS-1's whole definition: an empty name and a whitespace-only name are the same
//! refusal. A name with any non-whitespace character is valid, and the implementation keeps it
//! **as supplied** — leading and trailing whitespace included — rather than trimming it. Both
//! facts are asserted below, because a test that guessed the other convention would be pinning
//! behaviour the product does not have.

use std::sync::{Arc, Mutex};

use serde_json::json;

use app_lib::ipc::failure::NOT_IMPLEMENTED_MESSAGE_KEY;
use app_lib::ipc::types::{
    AccountUserId, CommandError, Household, HouseholdId, MemberId, RequestId, SaveCause,
    ServiceState, ServiceStatus,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{
    Answer, REACHABILITY_PROBE_TIMEOUT, ServiceReply, StatusPublisher, re_evaluate,
};

/// The instant the scripted service dates the households with, in the schema the create path reads.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// The account the person is identified by in these tests.
fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn member_id(value: &str) -> MemberId {
    MemberId(value.to_owned())
}

/// One create answer in the schema the create path reads: the outcome the gate classifies, and the
/// household the domain takes its identity, owner and instant from.
fn accepted_answer(outcome: &str, household: &str, owner: &str, at: &str) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({
            "outcome": outcome,
            "household": {
                "id": household,
                "owner_member_id": owner,
                "created_at": at,
            },
        }),
    ))
}

/// Run one create through the state method the `create_household` command forwards to.
fn create(state: &AppState, name: &str, operation: &str) -> Result<Household, CommandError> {
    block_on_ready(state.create_household(name.to_owned(), request_id(operation)))
}

/// The publication seam's test side: it records what the running app would have emitted.
#[derive(Debug, Default)]
struct RecordingPublisher {
    published: Mutex<Vec<ServiceStatus>>,
}

impl RecordingPublisher {
    fn published(&self) -> Vec<ServiceStatus> {
        self.published
            .lock()
            .expect("the records are readable")
            .clone()
    }
}

impl StatusPublisher for RecordingPublisher {
    fn publish(&self, status: &ServiceStatus) {
        self.published
            .lock()
            .expect("the records are writable")
            .push(status.clone());
    }
}

/// HS-1, "Create the first household".
///
/// **WHEN** a person with no household submits a valid, non-blank name **THEN** a household is
/// created with that name, the person is recorded as its first member, and that household is the
/// active household.
#[test]
fn create_the_first_household() {
    let answer = accepted_answer("applied", "household-1", "member-1", CREATED_AT);
    let service = Arc::new(ScriptedService::new().on_write([answer]));
    let state = AppState::new(service, Some(account()));

    // The person holds no household before the create: HS-2's onboarding state.
    assert!(state.list_households().is_empty());
    assert!(state.active_household().is_none());

    let created = create(&state, "Home", "operation-1").expect("the create completed");

    // THEN a household is created with that name.
    assert_eq!(created.id, household_id("household-1"));
    assert_eq!(created.name, "Home");
    assert_eq!(created.created_at.0.as_str(), CREATED_AT);

    // AND the person is recorded as its first member. P0 records exactly the creator, and the owner
    // member the service assigned is that same first entry.
    assert_eq!(created.owner_member_id, member_id("member-1"));
    let members = {
        let book = state.households();
        let record = book.get(&created.id).expect("the household is held");
        record.members().to_vec()
    };
    assert_eq!(members.len(), 1, "the creator is first");
    assert_eq!(members[0].id, created.owner_member_id);
    assert_eq!(members[0].account_user_id, account(), "the creator");

    // AND that household is the active household: the create returns it as active, the active read
    // reports it, and the row the list presents is marked active.
    assert!(created.is_active);
    assert!(created.is_owner, "the creator owns it");
    let active = state.active_household().expect("one is active");
    assert_eq!(active.id, created.id);
    assert!(active.is_active);

    let rows = state.list_households();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, created.id);
    assert_eq!(rows[0].name, "Home");
    assert!(rows[0].is_active);
    assert!(rows[0].is_owner);
    assert_eq!(rows[0].member_count, 1, "just the creator");
}

/// HS-1, "Blank name is rejected".
///
/// **WHEN** a person submits an empty name, or a name consisting only of whitespace characters
/// **THEN** no household is created, and the refusal is the one failure surface: `validation`, not
/// retryable, with that cause's own message key rather than the placeholder one.
#[test]
fn blank_name_is_rejected() {
    // Empty and whitespace-only are the same blank input (HS-1). An account is present here, so the
    // refusal is about the name and not about a missing identity.
    for blank in ["", " ", "   ", "\t", "\n", " \t\n "] {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), Some(account()));
        let operation = request_id("operation-1");

        let error = create(&state, blank, "operation-1").expect_err("a blank name is refused");

        assert_eq!(error.cause, SaveCause::Validation, "{blank:?} is blank");
        assert!(!error.retryable, "not retryable");
        assert_eq!(
            error.message_key, "error.validation",
            "{blank:?} must name its cause, not the placeholder"
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_ne!(error.message_key, NOT_IMPLEMENTED_MESSAGE_KEY);
        assert_eq!(error.request_id, Some(operation));

        // THEN no household is created ...
        assert!(state.list_households().is_empty());
        assert!(state.active_household().is_none());
        // ... and nothing was asked of the service: the refusal is a property of the input, so the
        // person is asked for a name rather than being told the service could not be reached.
        assert!(service.calls().is_empty());
    }

    // A blank name is refused before the account is consulted: with no account at all, the failure
    // still asks for a name (HS-1) rather than reporting the missing account.
    let service = Arc::new(ScriptedService::new());
    let state = AppState::new(service, None);
    let error = create(&state, "   ", "operation-1").expect_err("a blank name is refused");

    assert_eq!(error.cause, SaveCause::Validation);
    assert!(!error.retryable);
    assert!(state.list_households().is_empty());
    assert!(state.active_household().is_none());
}

/// HS-2, "Onboarding shown when no household exists".
///
/// The guard's decision is frontend code, and it reads two facts: a fresh outcome that is reachable
/// with an identified account, and a held set that is empty with no active household. This test
/// establishes both — the reachable half through [`re_evaluate`], whose probe is the same one a
/// start takes — and then asserts the empty book. The presentation itself is the component/E2E
/// tasks' (3.3/5.12).
#[test]
fn onboarding_shown_when_no_household_exists() {
    let service = Arc::new(ScriptedService::new());
    let state = AppState::new(service.clone(), Some(account()));
    let publisher = RecordingPublisher::default();

    // A fresh, healthy probe: the service answered and the account is valid, so the guard's
    // "a fresh reachable and identified outcome" half holds.
    let status = block_on_ready(re_evaluate(&state, &publisher));
    assert_eq!(status.state, ServiceState::Reachable);
    assert_eq!(status.account_valid, Some(true));
    assert_eq!(publisher.published(), vec![status]);

    // AND the person holds no household and none is active: the guard's "no household" condition.
    assert!(state.list_households().is_empty());
    assert!(state.active_household().is_none());

    // Both book reads are local: the only service calls are the probe's own two checks.
    assert_eq!(
        service.calls(),
        vec![
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
            RecordedCall::CheckAccount {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
        ],
        "the list and active reads add no call of their own"
    );
}

/// HS-2, "Onboarding not repeated once a household exists".
///
/// **WHEN** the product starts and at least one household exists **THEN** the active household's
/// dashboard is presented and no creation onboarding is. The Rust half is that a household is held
/// and marked active, so the guard's "no household" condition is false.
#[test]
fn onboarding_not_repeated_once_a_household_exists() {
    let answer = accepted_answer("applied", "household-1", "member-1", CREATED_AT);
    let service = Arc::new(ScriptedService::new().on_write([answer]));
    let state = AppState::new(service, Some(account()));

    let created = create(&state, "Home", "operation-1").expect("the create completed");

    let rows = state.list_households();
    assert_eq!(rows.len(), 1, "the person holds one");
    assert!(rows[0].is_active, "and it is the active one");
    let active = state.active_household().expect("one is active");
    assert_eq!(active.id, created.id);
    assert!(active.is_active);
}

/// HS-1's name edges: a name with surrounding whitespace is not blank, so it is created — and the
/// implementation keeps it **as supplied**, leading and trailing whitespace included, rather than
/// trimming it.
#[test]
fn a_name_with_surrounding_whitespace_is_created_and_kept_as_supplied() {
    let answer = accepted_answer("applied", "household-1", "member-1", CREATED_AT);
    let service = Arc::new(ScriptedService::new().on_write([answer]));
    let state = AppState::new(service, Some(account()));

    let created = create(&state, "  Home  ", "operation-1").expect("the create completed");

    assert_eq!(created.name, "  Home  ");
    let rows = state.list_households();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "  Home  ");
}

/// D0.14 as the create path applies it: re-issuing the same `request_id` returns the household that
/// already exists instead of creating a second one, and leaves the active household alone.
///
/// The replay's answer names a **different** household on purpose. The service's own ledger is what
/// guarantees one outcome per operation, but the local record must not grow a second household even
/// if an answer says so, so a same-id answer would let the test pass without the per-operation
/// ledger it is about.
#[test]
fn a_replayed_create_request_returns_the_existing_household() {
    let service = Arc::new(ScriptedService::new().on_write([
        accepted_answer("applied", "household-1", "member-1", CREATED_AT),
        accepted_answer("deduplicated", "household-2", "member-2", CREATED_AT),
    ]));
    let state = AppState::new(service, Some(account()));

    let first = create(&state, "Home", "operation-1").expect("the create completed");
    let replay = create(&state, "Home", "operation-1").expect("the replay completed");

    assert_eq!(
        replay.id, first.id,
        "the replay answers with the first operation's household"
    );
    assert_eq!(state.list_households().len(), 1, "no second one");
    assert!(
        state
            .households()
            .get(&household_id("household-2"))
            .is_none(),
        "the answer's second household was not recorded"
    );
    assert_eq!(
        state.active_household().expect("one is active").id,
        first.id,
        "the replay does not change the active household"
    );
}
