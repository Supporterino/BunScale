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
//! All three bodies are implemented. `load_household` resolves the locally kept copy first and
//! refreshes it from the shared service when a fresh probe allows, reporting the three SYNC-3 states
//! through `source` and `loaded_at`. The status pair reads and refreshes the tracker the platform
//! lifecycle also feeds (`sync::lifecycle`), and neither can fail on reachability: an unreachable
//! service is a value, never a `CommandError`.

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;
use crate::state::AppState;

/// The `get_service_status` body over the state alone.
///
/// It is the outermost layer a unit test can call: `tauri::State` has no public constructor outside a
/// running app, and the mock runtime that could supply one is behind `tauri`'s `test` feature, which
/// this crate does not enable ([`get_service_status`] is the one-line adapter over this function).
fn observed_status(state: &AppState) -> Result<ServiceStatus, CommandError> {
    Ok(crate::sync::current_status(state))
}

/// The `check_service_reachability` body over the state and a publication seam.
///
/// [`crate::sync::re_evaluate`] is the same sequence the platform lifecycle runs, so the command
/// cannot drift from it. The app's [`tauri::AppHandle`] implements
/// [`crate::sync::StatusPublisher`]; a unit test supplies a recorder.
async fn checked_status(
    state: &AppState,
    publisher: &impl crate::sync::StatusPublisher,
) -> Result<ServiceStatus, CommandError> {
    Ok(crate::sync::re_evaluate(state, publisher).await)
}

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

/// The last observed service/account status, with the time it was observed (NFR-1, R3).
///
/// Returns the tracker's last fresh outcome. Before any probe has completed in this session it is
/// `state: unknown` with `account_valid`, `cause` and `observed_at` all absent — never a defaulted
/// observation and never a value carried over from an earlier session (R3). A foreground return on
/// iOS, a window regaining focus on macOS, and `check_service_reachability` all record a fresh
/// outcome on the same tracker; this command itself makes no network call, so reading it cannot
/// change what it reports.
///
/// The frontend may treat the status as a current observation only while `observed_at` is present;
/// while the state is `unknown` the household reads prove nothing on their own (SYNC-3).
///
/// # Errors
///
/// None is reachable: the status is local state, so this is always `Ok`. The `Result` is the
/// contract's uniform shape rather than a failure this command can produce.
#[tauri::command]
#[specta::specta]
pub async fn get_service_status(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<ServiceStatus, CommandError> {
    observed_status(state.inner())
}

/// Run a fresh reachability probe, record it, publish it, and return it (SYNC-2, NFR-1).
///
/// The probe checks the service and the account separately (D0.7 criterion 4), and each check is
/// bounded by `REACHABILITY_PROBE_TIMEOUT` = 5 s: a check that gets no answer counts as a failed
/// probe (D0.13). The outcome is recorded on the status tracker and published as
/// `service:status-changed` before it returns, so a subscriber (task 3.6) sees exactly the status
/// this command answers with.
///
/// The offline threshold is applied in Rust: an answered probe — including a `401` or `429`, which
/// prove the service is reachable even though the account is not usable — resets the
/// consecutive-failure count, and `state: unreachable` is reported only after
/// `OFFLINE_FAILURE_THRESHOLD` = 3 consecutive failed probes without an intervening success (D0.13).
/// This is the same sequence a foreground return on iOS and a window regaining focus on macOS run
/// (`sync::lifecycle`).
///
/// # Errors
///
/// None is reachable: an unreachable service is a *status*, not a command failure (SYNC-3), and the
/// probe is idempotent so it carries no `request_id`. The `Result` is the contract's uniform shape.
#[tauri::command]
#[specta::specta]
pub async fn check_service_reachability(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<ServiceStatus, CommandError> {
    checked_status(state.inner(), &app).await
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::ipc::types::{SaveCause, ServiceState};
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::{Answer, ScriptedService, ServiceReply, StatusPublisher};

    /// The publication seam's test side: it records what the app would have emitted.
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

    #[test]
    fn get_service_status_answers_the_trackers_value_as_ok() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);

        let status = observed_status(&state).expect("a local status read is always Ok");

        assert_eq!(status, state.status().status());
        assert_eq!(status.state, ServiceState::Unknown);
        assert_eq!(status.account_valid, None);
        assert_eq!(status.cause, None);
        assert_eq!(
            status.observed_at, None,
            "nothing is defaulted or carried over"
        );
    }

    #[test]
    fn check_service_reachability_returns_and_publishes_the_trackers_new_status() {
        // A `401` is an answer: the status is reachable with no valid account, not a failure.
        let service = Arc::new(
            ScriptedService::new()
                .on_account_check([ServiceReply::Answered(Answer::status_only(401))]),
        );
        let state = AppState::new(service, None);
        let publisher = RecordingPublisher::default();

        let status = block_on_ready(checked_status(&state, &publisher))
            .expect("an unreachable or unauthorised service is a status, not a failure");

        assert_eq!(status.state, ServiceState::Reachable);
        assert_eq!(status.account_valid, Some(false));
        assert_eq!(status.cause, Some(SaveCause::NoAccount));
        assert_eq!(
            status,
            state.status().status(),
            "the command returns the value the tracker then holds"
        );
        assert_eq!(
            publisher.published(),
            vec![status],
            "and it published that same value"
        );
    }

    /// Holds the exported `get_service_status` shape at compile time.
    fn assert_get_status_shape<'a, F, Fut>(_: F)
    where
        F: Fn(tauri::State<'a, AppState>) -> Fut,
        Fut: std::future::Future<Output = Result<ServiceStatus, CommandError>> + 'a,
    {
    }

    /// Holds the exported `check_service_reachability` shape at compile time.
    fn assert_check_status_shape<'a, F, Fut>(_: F)
    where
        F: Fn(tauri::AppHandle, tauri::State<'a, AppState>) -> Fut,
        Fut: std::future::Future<Output = Result<ServiceStatus, CommandError>> + 'a,
    {
    }

    #[test]
    fn the_exported_status_commands_are_the_tested_bodies_behind_tauri_state() {
        assert_get_status_shape(get_service_status);
        assert_check_status_shape(check_service_reachability);
    }
}
