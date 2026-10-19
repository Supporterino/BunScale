//! Sync-area commands: the household collection read, and the reachability status pair it is read
//! against (SYNC-1, SYNC-3, NFR-1).
//!
//! Task 2.5 owns `load_household`; task 2.13 owns `get_service_status` and
//! `check_service_reachability`, which share one probe and one observation and so stay one file.
//! Unreachability is a *status* here, never a command failure (SYNC-3): `load_household` reports
//! where it resolved its items through [`HouseholdCollection`]'s `source` and `loaded_at`, and the
//! probe reports the three states of [`ServiceStatus`], so "nothing was ever loaded and the service
//! is unreachable" keeps its own surface instead of arriving as an empty collection or as an error.
//!
//! Every body here is still a **placeholder** that fails through [`CommandError::not_implemented`]:
//! the cause `unknown`, `retryable: false` and the placeholder key `error.not_implemented`, with
//! `request_id: None`, because these commands take no `RequestId` and are idempotent, so their retry
//! is a fresh call rather than a re-issued one. That key says "this command has no implementation
//! yet"; a genuine unclassifiable failure is [`CommandError::unknown`] and names `error.unknown`
//! instead (see the parent module's docs).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

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
    Err(CommandError::not_implemented(None))
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
    Err(CommandError::not_implemented(None))
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
    Err(CommandError::not_implemented(None))
}
