//! Household commands: creating a household, and holding and switching the active one (HS-1…HS-4,
//! DM-3).
//!
//! Tasks 2.1 (`create_household`) and 2.2 (the other three) own these bodies. `create_household` is
//! P0's only state-changing command that writes to the shared service, so the reachability-gated
//! write path — and with it every reachable `SaveCause` — sits behind it (2.4, 5.4). The two reads
//! carry D0.2's rule into the contract: an empty list and a `None` are conclusions the frontend may
//! draw only from a fresh, reachable [`ServiceStatus`], never from the read itself (SYNC-3).
//!
//! Every body here is still a **placeholder** that fails through [`CommandError::not_implemented`]:
//! the cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, carrying
//! the command's `RequestId` where it has one. That key says "this command has no implementation
//! yet"; it is not a cause in the frozen list, so a genuine unclassifiable failure is
//! [`CommandError::unknown`] and names `error.unknown` instead (see the parent module's docs). No
//! stub touches `tauri::State`, the storage boundary or the shared service.

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

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
    Err(CommandError::not_implemented(Some(request_id)))
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
    Err(CommandError::not_implemented(None))
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
    Err(CommandError::not_implemented(None))
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
    Err(CommandError::not_implemented(Some(request_id)))
}
