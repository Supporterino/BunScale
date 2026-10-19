//! Household commands: creating a household, and holding and switching the active one (HS-1…HS-4,
//! DM-3).
//!
//! Tasks 2.1 (`create_household`) and 2.2 (the other three) own these bodies. `create_household` is
//! P0's only state-changing command that writes to the shared service, so the reachability-gated
//! write path — and with it every reachable `SaveCause` — sits behind it (2.4, 5.4). The two reads
//! carry D0.2's rule into the contract: an empty list and a `None` are conclusions the frontend may
//! draw only from a fresh, reachable [`ServiceStatus`], never from the read itself (SYNC-3).
//!
//! `create_household` is implemented (task 2.1): it validates the name, refuses honestly when no
//! account exists (Q25), and otherwise goes through the domain and the reachability gate. The other
//! three bodies are still **placeholders** that fail through [`CommandError::not_implemented`]: the
//! cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, carrying
//! the command's `RequestId` where it has one. That key says "this command has no implementation
//! yet"; it is not a cause in the frozen list, so a genuine unclassifiable failure is
//! [`CommandError::unknown`] and names `error.unknown` instead (see the parent module's docs). The
//! three stubs touch no `tauri::State`, the storage boundary or the shared service.

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// Create a household with a non-blank name; the creator becomes its first member and it becomes
/// the active household (HS-1, HS-2).
///
/// The write is attempted only through the reachability-gated save path, so it is refused before
/// anything leaves the device when the shared service cannot be reached or when no account is
/// available (SYNC-2, D0.1). No local-only household is created in either case. P0 ships no sign-in
/// flow, so the running app has no account yet and answers `no_account` until one is provisioned
/// (Q25).
///
/// The operation is **idempotent on `request_id`** (D0.14): re-issuing it with the same value
/// returns the household that already exists instead of creating a second one. The value is carried
/// to the shared service, whose request ledger deduplicates the call, and the domain records the
/// operation's outcome so a replay cannot add a second household locally either. That is what makes
/// the retry SYNC-2 offers safe and closes R6's duplicated-household risk.
///
/// # Errors
///
/// `validation` for an empty or whitespace-only name, which is treated as blank (HS-1), for a blank
/// operation identity, and when the service refuses the request body with `400` or `422`.
/// `no_account` when no account is available (Q25) or the service answers `401`. The write path's
/// other causes are reachable here because this is P0's only state-changing command that writes to
/// the shared service: `connectivity` when no HTTP answer arrives, `allowance_exceeded` when the
/// household owner's allowance is exhausted (D0.12), `rate_limited` for the gateway's `429`, and
/// `timeout` when no outcome arrives inside D0.13's bounds. `unknown` is the honest remainder: a
/// probe answer this version cannot classify, and an accepted answer whose household body cannot be
/// read. No failure is ever presented as a created household (SYNC-2).
#[tauri::command]
#[specta::specta]
pub async fn create_household(
    name: String,
    request_id: RequestId,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Household, CommandError> {
    state.create_household(name, request_id).await
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
