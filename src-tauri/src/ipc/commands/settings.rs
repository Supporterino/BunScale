//! Settings-area commands: the per-device display weight unit and its setter (SET-4).
//!
//! Task 2.10 owns both bodies. The pair is local to the device and changes no stored value (D0.4):
//! stored weights stay in grams and conversion happens where a weight is entered or displayed, so
//! `set_display_weight_unit` is state-changing — it carries a `RequestId` (D0.14) — without touching
//! the shared service. Its failures are therefore local, not the write path's causes.
//!
//! **Both bodies are implemented, and no failure here is the placeholder.** Each resolves the app's
//! own data directory, reads or writes the device's preference file
//! ([`crate::settings::DisplayUnitStore`]) off the thread that serves the command, and reports a
//! fault through [`CommandError::unknown`]: `unknown` is the honest cause for a local file that
//! cannot answer — the frozen cause list has no local-storage variant, the shared service is not
//! involved, and the person's input is not what failed — and it names the cause's own key
//! `error.unknown`, never the placeholder `error.not_implemented` of a body that is not written yet.
//!
//! **What the last-resort key says here, and whose copy it is.** A local failure reports `unknown`,
//! so the surface renders `error.unknown`'s copy. For this pair that copy has to read as *the device
//! could not store or read the choice* — a failure of this device's own store — and never as a
//! service, account or connectivity failure: the shared service is not involved in these commands at
//! all, so service-shaped copy would send the person looking in the wrong place. Task 3.7 owns the
//! copy behind every message key and must word `error.unknown` so it serves this pair as well as a
//! genuinely unclassified failure elsewhere.
//!
//! **The handle Tauri injects is invisible to the contract.** Both commands take an additional
//! `app: tauri::AppHandle` parameter, because a body that persists a per-device preference has to
//! reach `AppHandle::path().app_data_dir()`, and Tauri's resolver is the only dependency-free route
//! to the device's own data area (`rust-conventions`: paths come from the resolver, never from a
//! literal). Tauri fills that parameter from the invocation rather than from the frontend, and
//! `tauri` 2.12.4's specta integration implements `FunctionArg` for `AppHandle` by returning no
//! datatype, so the parameter appears in no generated binding: `src/bindings.ts` declares both
//! commands with exactly the frozen parameters and no more.
//!
//! **The two doc comments below are contract copy, so their wording is reviewed.** A command's doc
//! comment is emitted verbatim into the generated bindings' JSDoc (D0.9), which is why they describe
//! the implemented behaviour rather than the placeholder body they replaced: that change is
//! `src/bindings.ts`'s deliberate doc-only diff (D0.18), never an edit made in passing.

use std::path::PathBuf;

use tauri::Manager;

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;
use crate::settings::{DisplayUnitStore, PreferenceError};

/// The failure a local device-store fault reports (Q42).
///
/// This is the pair's one mapping from "this device could not answer" to the failure surface, and
/// both commands reach it: [`data_dir`] when the app's data directory cannot be resolved, and
/// [`off_thread`] when the store itself failed or the blocking task that ran it could not run. It
/// names the cause through the constructor rather than building a [`CommandError`] field by field,
/// so the key and the retry flag stay the constructor's — the reason `unknown` is the honest cause
/// here is the module documentation's. `request_id` is the operation's identity as the command
/// received it, so a failure of the setter carries the id a retry re-issues (D0.14).
fn local_fault_error(request_id: Option<RequestId>) -> CommandError {
    CommandError::unknown(request_id)
}

/// The app's own data directory, or the local-storage failure the command reports.
///
/// `request_id` is the operation's identity when the command has one — the setter — so a failure to
/// resolve the path carries the same identity as a failure to use the file (D0.14).
fn data_dir(
    app: &tauri::AppHandle,
    request_id: Option<RequestId>,
) -> Result<PathBuf, CommandError> {
    let resolved = app.path().app_data_dir();

    resolved.map_err(|_| local_fault_error(request_id))
}

/// Run one device-store operation on the blocking pool, and report its outcome through the one
/// failure surface.
///
/// The store touches a file, so the work belongs off the async thread that serves the command. A
/// join failure (the task could not run at all) and the store's own two classifications are all
/// `unknown`: see the module documentation for why that cause, and not the placeholder key, is what
/// a local-storage fault reports. The mapping itself is [`local_fault_error`]'s, so both this path
/// and an unresolvable data directory report the same failure.
async fn off_thread<T, F>(request_id: Option<RequestId>, operation: F) -> Result<T, CommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PreferenceError> + Send + 'static,
{
    let outcome = tauri::async_runtime::spawn_blocking(operation).await;

    match outcome {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) | Err(_) => Err(local_fault_error(request_id)),
    }
}

/// The effective per-device display unit (SET-4).
///
/// The person's own choice when there is one, otherwise the unit derived from the device's regional
/// settings. It is a display preference only — no stored value is read or written in that unit
/// (D0.4) — and it belongs to the device rather than to a household, so this read touches no
/// household collection (D0.2) and does not fail because the shared service is unreachable.
///
/// # Errors
///
/// `unknown` for a local-storage failure: the preference file cannot be read, or it holds content
/// this version cannot interpret. A device on which no unit has been chosen yet is not a failure —
/// it answers the unit the region states.
#[tauri::command]
#[specta::specta]
pub async fn get_display_weight_unit(app: tauri::AppHandle) -> Result<WeightUnit, CommandError> {
    let store = DisplayUnitStore::in_dir(data_dir(&app, None)?);
    let operation = move || store.effective_for_this_device();

    off_thread(None, operation).await
}

/// Store the per-device display unit and return the effective unit (SET-4).
///
/// The change is local to this device and never rewrites a stored value (D0.4): conversion happens
/// only where a weight is entered or displayed, so what is stored here is a unit and never a
/// weight, and per-device preferences never enter a household collection (D0.2). `request_id` is the
/// operation's identity, re-used if the surface offers a retry (D0.14).
///
/// The returned unit is read back from what the change left on the device rather than echoed from
/// `unit`, so a write that did not land is never reported as stored.
///
/// # Errors
///
/// `unknown` for a local-storage failure: the preference file cannot be written, or what it now
/// holds cannot be read back.
#[tauri::command]
#[specta::specta]
pub async fn set_display_weight_unit(
    unit: WeightUnit,
    request_id: RequestId,
    app: tauri::AppHandle,
) -> Result<WeightUnit, CommandError> {
    let path = data_dir(&app, Some(request_id.clone()))?;
    let store = DisplayUnitStore::in_dir(path);

    let operation = move || {
        store.store_choice(unit)?;

        // Read back what is now stored instead of echoing the argument: the returned unit is the
        // effective one after the change (SET-4) and it comes from the same file the next launch
        // reads, so a write that did not land cannot be reported as stored.
        store.effective_for_this_device()
    };

    off_thread(Some(request_id), operation).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair's mapping has to land on the classified `unknown` failure (Q42): the cause the
    /// frozen list has for a local file that cannot answer, its own `error.unknown` key rather than
    /// the placeholder key of an unimplemented body, and no retry — nothing about a failed local
    /// store is known well enough to promise that a second attempt succeeds.
    #[test]
    fn a_local_fault_reports_the_unknown_cause_under_its_own_key_without_a_retry() {
        let fault = local_fault_error(None);
        let classified_unknown = CommandError::unknown(None);

        assert_eq!(fault.cause, SaveCause::Unknown);
        assert_eq!(fault.cause, classified_unknown.cause);
        assert_eq!(fault.message_key, classified_unknown.message_key);
        assert_eq!(fault.message_key, "error.unknown");
        assert!(!fault.retryable);
        assert_eq!(fault.retryable, classified_unknown.retryable);
    }

    /// The read takes no operation identity and the setter has one (D0.14); the mapping passes
    /// whichever it was given straight through to the constructor.
    #[test]
    fn a_local_fault_carries_the_operation_identity_it_was_given() {
        let request_id = RequestId("request-7.2".to_owned());

        let fault = local_fault_error(Some(request_id.clone()));

        assert_eq!(fault, CommandError::unknown(Some(request_id)));
        assert_eq!(local_fault_error(None), CommandError::unknown(None));
    }
}
