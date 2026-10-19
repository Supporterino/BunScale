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
//! `load_household` is implemented (task 2.5): it resolves the locally kept copy first and refreshes
//! it from the shared service when a fresh probe allows, reporting the three SYNC-3 states through
//! `source` and `loaded_at`. The other two bodies are still **placeholders** that fail through
//! [`CommandError::not_implemented`]: the cause `unknown`, `retryable: false` and the placeholder key
//! `error.not_implemented`, with `request_id: None`, because these commands take no `RequestId` and
//! are idempotent, so their retry is a fresh call rather than a re-issued one. That key says "this
//! command has no implementation yet"; a genuine unclassifiable failure is
//! [`CommandError::unknown`] and names `error.unknown` instead (see the parent module's docs).

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
/// The read is served from the local copy captured before any network call. When a fresh probe
/// identifies both the service and the account, the collection is then refreshed through the guarded
/// load path, which replaces the household's copy and records this device's load instant; the
/// result is `source: service`. When the probe refuses or the refresh fails for any reason, the
/// previously loaded copy is returned unchanged (`source: local` with `loaded_at: Some(..)`), or —
/// when nothing was ever loaded — the explicit never-loaded value (`source: local` with
/// `loaded_at: None`). A refresh failure never clears, resets or empties the copy (SYNC-3).
///
/// # Errors
///
/// `validation` for a blank household id. Unreachability and a failed refresh are not errors: they
/// are reported by `source`/`loaded_at`. Unlike the placeholder this command used to return, it
/// never reports `unknown` merely because the shared service cannot be reached. Only a blank id is
/// locally refutable: P0 has no local membership list yet (task 2.2 owns the household list), so a
/// well-formed id this device holds no copy of is the never-loaded state, not a validation failure.
#[tauri::command]
#[specta::specta]
pub async fn load_household(
    household_id: HouseholdId,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<HouseholdCollection, CommandError> {
    crate::sync::ReadPath::shared()
        .load(&state, &household_id)
        .await
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
