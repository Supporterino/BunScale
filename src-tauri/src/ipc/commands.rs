//! The P0 command surface: one typed stub per command the frontend needs in this phase.
//!
//! **Every body here is a placeholder.** Each one returns `Err` through the contract's single
//! failure surface ([`CommandError`]) with the cause `unknown`, `retryable: false` and the message
//! key `error.not_implemented`. Task 1.3 freezes the shapes; the behaviour belongs to section 2
//! (tasks 2.1–2.13 replace the bodies one by one). No domain logic lives here, and no stub touches
//! `tauri::State`, the storage boundary or the shared service.
//!
//! **The frontend reaches these commands only through `src/features/*/api.ts`** — wrappers over the
//! generated `src/bindings.ts` (project hard rule 2). No route, component or hook imports the
//! bindings or calls `invoke` directly, and `api.ts` is also where a state-changing command's
//! [`RequestId`] is generated and re-used on retry (task 3.1, D0.14).
//!
//! Every command is `async`: what sits behind it does network and storage I/O, so the async
//! signature is part of the contract rather than a detail of the placeholder. Every command that
//! changes product data takes a `RequestId`; the reads and the reachability probe take none, so
//! their failure carries `request_id: None` (they are idempotent — see
//! [`CommandError::request_id`]).

use super::types::{
    CommandError, DiagnosticsSummary, Household, HouseholdCollection, HouseholdId,
    HouseholdSummary, Identity, RequestId, SaveCause, ServiceStatus, WeightUnit,
};

/// Create a household with a non-blank name; the creator becomes its first member and it becomes
/// the active household (HS-1, HS-2).
///
/// The operation is **idempotent on `request_id`** (D0.14, task 2.1): re-issuing it with the same
/// value returns the household that already exists instead of creating a second one. That is what
/// makes the retry SYNC-2 offers safe and closes R6's duplicated-household risk.
///
/// # Errors
///
/// `validation` for an empty or whitespace-only name, which is treated as blank (HS-1). A valid
/// name reaches the write path, so its causes are also reachable here — `connectivity`,
/// `no_account`, `allowance_exceeded`, `rate_limited` and `timeout` (D0.12, D0.13) — because this
/// is P0's only state-changing command that writes to the shared service. The stub returns
/// `unknown` until task 2.1 implements the body.
#[tauri::command]
#[specta::specta]
pub async fn create_household(
    name: String,
    request_id: RequestId,
) -> Result<Household, CommandError> {
    // Named for the generated bindings; task 2.1 validates the value.
    let _ = name;
    Err(not_implemented(Some(request_id)))
}

/// Every household the person holds, with the active one indicated (HS-3, HS-4, DM-3).
///
/// Served from the locally kept data first (D0.2). An empty list is **not** proof that the person
/// holds no household: it cannot distinguish "no household" from "nothing was ever loaded and the
/// service is unreachable" (SYNC-3). The list is what HS-2's route guard reads at launch, so the
/// guard may start first-run onboarding only when a fresh [`ServiceStatus`] reports
/// `state: reachable` and `account_valid: true`; while that status is `unknown` or `unreachable`,
/// the guard shows the connectivity-required state and does not start onboarding (task 3.3
/// consumes this rule; task 3.4 holds the status).
///
/// # Errors
///
/// The read path's causes (`validation`, `unknown`). Unreachability is a status fact
/// (`get_service_status`), not a read failure. The stub returns `unknown` until task 2.2.
#[tauri::command]
#[specta::specta]
pub async fn list_households() -> Result<Vec<HouseholdSummary>, CommandError> {
    Err(not_implemented(None))
}

/// The person's active household, or `None` when they hold none (HS-2, HS-3, DM-3).
///
/// `None` is the onboarding trigger — HS-2 presents household creation in that state and never once
/// a household exists — but it is a *conclusion*, not an absence of data: `None` also covers a
/// device with no local copy and an unreachable service. It may be read as "no household" only when
/// a fresh [`ServiceStatus`] reports `state: reachable` and `account_valid: true`; while that status
/// is `unknown` or `unreachable`, the guard shows the connectivity-required state instead of
/// onboarding (SYNC-3, D0.2). At most one household is active (DM-3).
///
/// # Errors
///
/// The read path's causes (`validation`, `unknown`); "no household" is `Ok(None)`. The stub returns
/// `unknown` until task 2.2.
#[tauri::command]
#[specta::specta]
pub async fn get_active_household() -> Result<Option<Household>, CommandError> {
    Err(not_implemented(None))
}

/// Switch the active household and return the household that is active afterwards (HS-3, HS-4,
/// DM-3).
///
/// The result is the post-state rather than an echo of the argument: DM-3 re-elects an active
/// household when the previous one is no longer held, so what the screens must show is the active
/// household after the call. A re-issued call with the same `request_id` is deduplicated rather
/// than applied twice (D0.14).
///
/// # Errors
///
/// `validation` for a household the person does not hold, plus the write path's causes (D0.7
/// criterion 4). The stub returns `unknown` until task 2.2.
#[tauri::command]
#[specta::specta]
pub async fn set_active_household(
    household_id: HouseholdId,
    request_id: RequestId,
) -> Result<Household, CommandError> {
    let _ = household_id;
    Err(not_implemented(Some(request_id)))
}

/// One household's collection, resolved against the locally kept copy first (D0.2, SYNC-1,
/// SYNC-3).
///
/// Unreachability is deliberately not an error here: [`HouseholdCollection`]'s `source` and
/// `loaded_at` report the three SYNC-3 states, so "nothing was ever loaded and the service is
/// unreachable" keeps its own surface and is never shown as an empty collection.
///
/// # Errors
///
/// `validation` for an unknown household id, `unknown` for an unclassifiable internal failure. The
/// stub returns `unknown` until task 2.5.
#[tauri::command]
#[specta::specta]
pub async fn load_household(
    household_id: HouseholdId,
) -> Result<HouseholdCollection, CommandError> {
    let _ = household_id;
    Err(not_implemented(None))
}

/// The last observed service/account status, with the time it was observed (NFR-1).
///
/// Returns `state: unknown` with no observation instead of a value carried over from an earlier
/// session (R3); the status store keeps that state until a fresh outcome arrives (task 3.4).
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure. The stub returns `unknown` until task 2.13.
#[tauri::command]
#[specta::specta]
pub async fn get_service_status() -> Result<ServiceStatus, CommandError> {
    Err(not_implemented(None))
}

/// Run a fresh reachability probe and return its outcome (SYNC-2, NFR-1).
///
/// The probe checks the service and the account separately (D0.7 criterion 4), is bounded by
/// `REACHABILITY_PROBE_TIMEOUT` = 5 s, and reports `unreachable` only after
/// `OFFLINE_FAILURE_THRESHOLD` = 3 consecutive failures without an intervening success (D0.13). A
/// fresh outcome is what `service:status-changed` publishes.
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure; an unreachable service is a *status*, not an
/// error. The stub returns `unknown` until task 2.13.
#[tauri::command]
#[specta::specta]
pub async fn check_service_reachability() -> Result<ServiceStatus, CommandError> {
    Err(not_implemented(None))
}

/// The current person's account id and name, resolved with SET-2's fallback chain (task 2.9).
///
/// An unresolved identity returns normally (`resolved: false`, `display_name: None`): it must not
/// fail the command or block the rest of Settings (SET-2's scenario).
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure. The stub returns `unknown` until task 2.9.
#[tauri::command]
#[specta::specta]
pub async fn get_identity() -> Result<Identity, CommandError> {
    Err(not_implemented(None))
}

/// The effective per-device display unit (SET-4).
///
/// The person's own choice when there is one, otherwise the unit derived from the device's regional
/// settings (task 2.10). It is a display preference only: no stored value is read or written in
/// that unit (D0.4).
///
/// # Errors
///
/// `unknown` for an unclassifiable local-storage failure. The stub returns `unknown` until task
/// 2.10.
#[tauri::command]
#[specta::specta]
pub async fn get_display_weight_unit() -> Result<WeightUnit, CommandError> {
    Err(not_implemented(None))
}

/// Store the per-device display unit and return the effective unit (SET-4).
///
/// The change is local to this device and never rewrites a stored value (D0.4): conversion happens
/// only where a weight is entered or displayed. `request_id` is the operation's identity, re-used
/// if the surface offers a retry (D0.14).
///
/// # Errors
///
/// `unknown` for an unclassifiable local-storage failure. The stub returns `unknown` until task
/// 2.10.
#[tauri::command]
#[specta::specta]
pub async fn set_display_weight_unit(
    unit: WeightUnit,
    request_id: RequestId,
) -> Result<WeightUnit, CommandError> {
    let _ = unit;
    Err(not_implemented(Some(request_id)))
}

/// A redacted summary of the diagnostics sink: counts and timestamps, never content (NFR-3).
///
/// P0 defines it for support diagnostics only — no P0 screen shows it — and P3 reuses it unchanged.
///
/// # Errors
///
/// `unknown` for an unclassifiable internal failure. The stub returns `unknown` until task 2.12.
#[tauri::command]
#[specta::specta]
pub async fn get_diagnostics_summary() -> Result<DiagnosticsSummary, CommandError> {
    Err(not_implemented(None))
}

/// The message key every stub failure carries.
///
/// Section 2 replaces each body with the cause-specific keys task 3.7's scaffold resolves. The
/// placeholder is still a *specific* key rather than a bare message, so a stub failure cannot be
/// mistaken for a production failure path.
const NOT_IMPLEMENTED_MESSAGE_KEY: &str = "error.not_implemented";

/// The placeholder failure shared by every stub (see the module documentation).
///
/// `SaveCause::Unknown` is the only honest cause available: the frozen cause list (D0.6) admits no
/// "not implemented" variant, and adding one would contradict it. `retryable` is `false` because
/// retrying a missing implementation cannot succeed.
fn not_implemented(request_id: Option<RequestId>) -> CommandError {
    CommandError {
        cause: SaveCause::Unknown,
        retryable: false,
        message_key: NOT_IMPLEMENTED_MESSAGE_KEY.to_owned(),
        request_id,
    }
}
