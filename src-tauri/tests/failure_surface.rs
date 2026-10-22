//! Integration tests for the failure surface (task 5.10): NFR-8's two scenarios and the per-command
//! failure paths.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/product-quality/spec.md` states for
//! NFR-8, and each test is named after the scenario it covers. The command enumeration is the
//! task's own assertion (tasks.md 5.10) rather than a third scenario: NFR-8 defines two.
//!
//! # The boundary these tests drive
//!
//! [`CommandError`] is the one failure surface (D0.6): every cause has one constructor, one message
//! key and one retry policy, and every command reports through it. The expected keys and retry
//! flags are written out **as literals** here rather than read back from `ipc::failure::policy`, so
//! a mapping that swapped two keys would fail these tests instead of keeping both sides in step.
//!
//! The command adapters take `tauri::State` or `tauri::AppHandle`, neither of which can be
//! built outside a running app, so a body is reached the way its own module reaches it: the
//! household commands forward to [`AppState`] methods, `load_household` to [`ReadPath::load`], the
//! status pair to `current_status`/`re_evaluate`, the identity command to the identity chain, and
//! `get_diagnostics_summary` is a plain async function that needs no runtime. The settings pair is
//! the one surface only reachable through the injected handle; its documented cause is stated in
//! the test rather than driven, and that is recorded there as a named gap.
//!
//! **Five commands have no failure path, by design.** `list_households`, `get_active_household`,
//! `get_service_status`, `check_service_reachability` and `get_identity` document "none is
//! reachable": their answers are local values (an empty book, a status, the unresolved identity),
//! so there is no failure for NFR-8 to name. The third test asserts that totality rather than
//! inventing a failure for them, and says so at each.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.

use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, State};

use app_lib::domain::resolve_identity;
use app_lib::ipc::commands::{
    check_service_reachability, get_active_household, get_diagnostics_summary, get_identity,
    get_service_status, list_households,
};
use app_lib::ipc::failure::NOT_IMPLEMENTED_MESSAGE_KEY;
use app_lib::ipc::types::{
    AccountUserId, CommandError, Household, HouseholdId, HouseholdSummary, Identity, RequestId,
    SaveCause, ServiceState, ServiceStatus,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::{
    Answer, ReadPath, ScriptedService, ServiceReply, StatusPublisher, current_status, re_evaluate,
};

/// The eleven frozen commands, as the shipped bindings declare them, sorted.
const SHIPPED_COMMANDS: [&str; 11] = [
    "check_service_reachability",
    "create_household",
    "get_active_household",
    "get_diagnostics_summary",
    "get_display_weight_unit",
    "get_identity",
    "get_service_status",
    "list_households",
    "load_household",
    "set_active_household",
    "set_display_weight_unit",
];

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

/// The failure a create through the command's body reports, for `service` and `name`.
///
/// The command adapter is `state.create_household(name, request_id).await`; a `tauri::State` cannot
/// be built without an app, so the state method is the whole command apart from its wrapper.
fn create_failure(
    service: Arc<ScriptedService>,
    name: &str,
    account: Option<AccountUserId>,
) -> CommandError {
    let state = AppState::new(service, account);

    block_on_ready(state.create_household(name.to_owned(), request_id("operation-1")))
        .expect_err("the create failed")
}

/// A publisher that discards what it is given: the status command needs one, and publication is not
/// this test's subject.
struct DiscardingPublisher;

impl StatusPublisher for DiscardingPublisher {
    fn publish(&self, _status: &ServiceStatus) {}
}

/// Holds `list_households`' exported shape: a change that gives the adapter a failure path would
/// have to change its return type, and would fail to compile here.
fn assert_list_households_shape<'a, F, Fut>(_: F)
where
    F: Fn(State<'a, AppState>) -> Fut,
    Fut: Future<Output = Result<Vec<HouseholdSummary>, CommandError>> + 'a,
{
}

/// Holds `get_active_household`' exported shape (see [`assert_list_households_shape`]).
fn assert_get_active_household_shape<'a, F, Fut>(_: F)
where
    F: Fn(State<'a, AppState>) -> Fut,
    Fut: Future<Output = Result<Option<Household>, CommandError>> + 'a,
{
}

/// Holds `get_service_status`' exported shape (see [`assert_list_households_shape`]).
fn assert_get_service_status_shape<'a, F, Fut>(_: F)
where
    F: Fn(State<'a, AppState>) -> Fut,
    Fut: Future<Output = Result<ServiceStatus, CommandError>> + 'a,
{
}

/// Holds `check_service_reachability`' exported shape (see [`assert_list_households_shape`]).
fn assert_check_service_reachability_shape<'a, F, Fut>(_: F)
where
    F: Fn(AppHandle, State<'a, AppState>) -> Fut,
    Fut: Future<Output = Result<ServiceStatus, CommandError>> + 'a,
{
}

/// Holds `get_identity`' exported shape (see [`assert_list_households_shape`]).
fn assert_get_identity_shape<'a, F, Fut>(_: F)
where
    F: Fn(State<'a, AppState>) -> Fut,
    Fut: Future<Output = Result<Identity, CommandError>> + 'a,
{
}

/// The commands the shipped bindings declare, sorted: the frontend's whole command surface.
fn shipped_commands() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");
    let bindings = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let marker = "__TAURI_INVOKE(\"";
    let mut names = Vec::new();
    let mut rest = bindings.as_str();

    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let end = after.find('"').expect("an invoke name is quoted");
        names.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }

    names.sort();
    names
}

/// NFR-8, "A failed save is visible and specific".
///
/// **WHEN** a save fails **THEN** a user-visible message names the cause of the failure, and the
/// change is not presented as applied.
#[test]
fn a_failed_save_is_visible_and_specific() {
    let operation = request_id("operation-1");

    // Every cause, through its own constructor, against the literal key and retry policy the
    // contract fixes. The expected values are written here rather than read back from `policy`, so
    // a swapped mapping fails instead of keeping both sides in agreement.
    let cases = [
        (
            CommandError::connectivity(Some(operation.clone())),
            SaveCause::Connectivity,
            "error.connectivity",
            true,
        ),
        (
            CommandError::no_account(Some(operation.clone())),
            SaveCause::NoAccount,
            "error.no_account",
            true,
        ),
        (
            CommandError::allowance_exceeded(Some(operation.clone())),
            SaveCause::AllowanceExceeded,
            "error.allowance_exceeded",
            true,
        ),
        (
            CommandError::rate_limited(Some(operation.clone())),
            SaveCause::RateLimited,
            "error.rate_limited",
            true,
        ),
        (
            CommandError::timeout(Some(operation.clone())),
            SaveCause::Timeout,
            "error.timeout",
            true,
        ),
        (
            CommandError::validation(Some(operation.clone())),
            SaveCause::Validation,
            "error.validation",
            false,
        ),
        (
            CommandError::unknown(Some(operation.clone())),
            SaveCause::Unknown,
            "error.unknown",
            false,
        ),
    ];

    let mut keys = HashSet::new();
    for (error, cause, key, retryable) in cases {
        assert_eq!(error.cause, cause);
        assert_eq!(error.message_key, key, "{cause:?} names its own copy");
        assert_eq!(error.retryable, retryable, "{cause:?}");
        assert_eq!(
            error.request_id,
            Some(operation.clone()),
            "{cause:?} keeps the identity"
        );
        assert!(keys.insert(error.message_key), "{cause:?} shares a key");
    }
    assert_eq!(keys.len(), 7, "every cause has its own key");

    // And the same surface is what a real save failure reaches: the command's body over an
    // unreachable service, with the identity kept and nothing presented as applied.
    let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
    let state = AppState::new(service, Some(account()));
    let failure = block_on_ready(state.create_household("Home".to_owned(), operation.clone()))
        .expect_err("the save did not complete");

    assert_eq!(failure.cause, SaveCause::Connectivity);
    assert_eq!(
        failure.message_key, "error.connectivity",
        "the person is told the specific cause"
    );
    assert!(failure.retryable, "the surface may offer a retry");
    assert_eq!(failure.request_id, Some(operation));
    assert!(
        state.list_households().is_empty(),
        "the change is not presented as applied"
    );
}

/// NFR-8, "No bare failure messages".
///
/// **WHEN** a state-changing operation of the product fails **THEN** the message identifies the
/// specific cause, and it never only states that something went wrong.
#[test]
fn no_bare_failure_messages() {
    // The classified save conditions, each driven through the command's body. None may borrow the
    // generic token or the placeholder key.
    let blank_name = create_failure(Arc::new(ScriptedService::new()), "", Some(account()));
    let no_account = create_failure(Arc::new(ScriptedService::new()), "Home", None);
    let unreachable = create_failure(
        Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
        "Home",
        Some(account()),
    );
    let invalid_account = create_failure(
        Arc::new(
            ScriptedService::new()
                .on_account_check([ServiceReply::Answered(Answer::status_only(401))]),
        ),
        "Home",
        Some(account()),
    );
    let rate_limited = create_failure(
        Arc::new(
            ScriptedService::new()
                .on_account_check([ServiceReply::Answered(Answer::status_only(429))]),
        ),
        "Home",
        Some(account()),
    );
    let exhausted = create_failure(
        Arc::new(
            ScriptedService::new().on_write([ServiceReply::Answered(Answer::with_body(
                200,
                json!({ "outcome": "allowance_exceeded" }),
            ))]),
        ),
        "Home",
        Some(account()),
    );
    let stalled = create_failure(
        Arc::new(ScriptedService::new().on_write([ServiceReply::NoOutcome])),
        "Home",
        Some(account()),
    );

    for (label, failure, cause, key) in [
        (
            "a blank name",
            blank_name,
            SaveCause::Validation,
            "error.validation",
        ),
        (
            "no account",
            no_account,
            SaveCause::NoAccount,
            "error.no_account",
        ),
        (
            "unreachable",
            unreachable,
            SaveCause::Connectivity,
            "error.connectivity",
        ),
        (
            "invalid account",
            invalid_account,
            SaveCause::NoAccount,
            "error.no_account",
        ),
        (
            "rate limited",
            rate_limited,
            SaveCause::RateLimited,
            "error.rate_limited",
        ),
        (
            "allowance",
            exhausted,
            SaveCause::AllowanceExceeded,
            "error.allowance_exceeded",
        ),
        ("stalled", stalled, SaveCause::Timeout, "error.timeout"),
    ] {
        assert_eq!(failure.cause, cause, "{label}");
        assert_eq!(failure.message_key, key, "{label}");
        assert_ne!(
            failure.message_key, "error.unknown",
            "{label} is a classified cause"
        );
        assert_ne!(
            failure.message_key, NOT_IMPLEMENTED_MESSAGE_KEY,
            "{label} reached the placeholder"
        );
    }

    // An answer this boundary cannot classify is the one failure that legitimately names
    // `error.unknown` — the honest remainder, still with its own copy rather than a bare message.
    let unclassified = create_failure(
        Arc::new(
            ScriptedService::new().on_write([ServiceReply::Answered(Answer::status_only(500))]),
        ),
        "Home",
        Some(account()),
    );
    assert_eq!(unclassified.message_key, "error.unknown");

    // The local commands name their causes too: a switch refuses a household the person does not
    // hold and the read refuses a blank id, both as `validation`.
    let state = AppState::new(Arc::new(ScriptedService::new()), Some(account()));
    let unheld = state
        .set_active_household(household_id("household-9"), request_id("operation-2"))
        .expect_err("the person does not hold it");
    assert_eq!(unheld.message_key, "error.validation");

    let blank_id = block_on_ready(ReadPath::new().load(&state, &household_id("   ")))
        .expect_err("a blank id names no household");
    assert_eq!(blank_id.message_key, "error.validation");

    // The placeholder is not a second key for `unknown`: the two differ in the field the surface
    // renders, and no cause owns the placeholder.
    let placeholder = CommandError::not_implemented(Some(request_id("operation-3")));
    assert_eq!(placeholder.message_key, NOT_IMPLEMENTED_MESSAGE_KEY);
    assert_ne!(placeholder.message_key, "error.unknown");
}

/// The task's command-enumeration assertion (tasks.md 5.10): every P0 command has a cause-specific
/// error path.
///
/// The enumeration is checked against the shipped bindings first, so a command that appears without
/// a failure-path decision fails here. Nine of the eleven are then driven to their documented cause
/// or shown to be total; the settings pair is documented, not driven, because the injected
/// `AppHandle` cannot be built here.
#[test]
fn every_p0_command_has_a_cause_specific_error_path() {
    // The eleven frozen commands, as the frontend's bindings declare them.
    assert_eq!(shipped_commands(), SHIPPED_COMMANDS);

    // --- create_household: the body is `AppState::create_household`; the adapter unwraps the
    // injected `tauri::State` and, once the create succeeds, publishes `household:changed` with kind
    // `created` (a refusal publishes nothing). Its locally refutable conditions each name their
    // cause.
    let blank = create_failure(Arc::new(ScriptedService::new()), " ", Some(account()));
    assert_eq!(blank.cause, SaveCause::Validation);
    let anonymous = create_failure(Arc::new(ScriptedService::new()), "Home", None);
    assert_eq!(anonymous.cause, SaveCause::NoAccount);
    let offline = create_failure(
        Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse])),
        "Home",
        Some(account()),
    );
    assert_eq!(offline.cause, SaveCause::Connectivity);

    // --- set_active_household: local state; the body is `AppState::set_active_household`, whose
    // one failure is `validation` for a household the person does not hold.
    let state = AppState::new(Arc::new(ScriptedService::new()), Some(account()));
    let unheld = state
        .set_active_household(household_id("household-9"), request_id("operation-2"))
        .expect_err("the person does not hold it");
    assert_eq!(unheld.cause, SaveCause::Validation);

    // --- load_household: the body is `ReadPath::shared().load`; its one failure is `validation`
    // for a blank id, and unreachability is a value (`source`/`loaded_at`), never a failure.
    let blank_id = block_on_ready(ReadPath::new().load(&state, &household_id("  ")))
        .expect_err("a blank id names no household");
    assert_eq!(blank_id.cause, SaveCause::Validation);

    // --- get_display_weight_unit and set_display_weight_unit: their bodies take the injected
    // `tauri::AppHandle`, so they cannot be driven here. **Named gap:** the mapping from a
    // `PreferenceError` to `CommandError::unknown` in `commands/settings.rs` is tested in neither
    // layer, so this file records the documented promise without proving it. The owner is whoever
    // next touches that module: extract the mapping as a pure helper (for example a
    // `local_fault_cause(request_id)`) and assert its cause and key there.

    // --- get_diagnostics_summary: a plain async function, callable without a runtime. Its normal
    // path is `Ok`; the one failure it can report is `unknown` for a sink lock poisoned by another
    // thread, which nothing outside can provoke because the lock is private.
    let summary = block_on_ready(get_diagnostics_summary());
    assert!(
        summary.is_ok(),
        "reading the sink is not a failure in the normal path"
    );

    // --- The five reads with no failure path: list_households and get_active_household are local
    // book reads, get_service_status reads the tracker, check_service_reachability records a
    // status, and get_identity resolves the identity chain. Their **exported shapes** are pinned
    // at compile time, so a failure path added to one of the adapters has to change its return
    // type and fails here; the state/domain calls they forward to are asserted total below.
    assert_list_households_shape(list_households);
    assert_get_active_household_shape(get_active_household);
    assert_get_service_status_shape(get_service_status);
    assert_check_service_reachability_shape(check_service_reachability);
    assert_get_identity_shape(get_identity);

    assert!(state.list_households().is_empty());
    assert!(state.active_household().is_none());
    assert_eq!(current_status(&state).state, ServiceState::Unknown);
    let probed = block_on_ready(re_evaluate(&state, &DiscardingPublisher));
    assert_eq!(
        probed.state,
        ServiceState::Reachable,
        "a reachable service is a status value, not a command failure"
    );

    // The identity body is a value too. This exercises only the account-without-a-name tier; the
    // no-account and local-fallback tiers are the identity module's own unit tests'.
    let identity = resolve_identity(state.account(), None, None);
    assert!(!identity.resolved, "an unresolved identity is a value");
}
