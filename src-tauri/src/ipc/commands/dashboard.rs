//! The dashboard read: one household's rabbit rows (DASH-1, DASH-2, DASH-3, DASH-4, DASH-6, DASH-9).
//!
//! One command, frozen by task 1.1: [`get_dashboard`], whose body is a typed stub returning
//! [`CommandError::not_implemented`] until section 2 replaces it (task 2.14).
//!
//! It is a **read model**, not a ninth table: every field of a [`DashboardRow`] that is not stored data
//! — `never_weighed` and `has_weight_drop_alert` — is derived on read (D1.3), and the sort order
//! (DASH-3) is a property of the returned sequence rather than a stored rank. Nothing derived is
//! persisted, so a threshold edit, an entry edit or a deletion reaches the dashboard by invalidating
//! this query, never by a repair pass.
//!
//! The row deliberately carries `RabbitSummary` rather than the whole profile: the dashboard needs a
//! name, a thumbnail and flags, and loading every profile field for every row would make the list pay
//! for data it does not render. The row also carries the rabbit's id, which is what DASH-6's selection
//! and DASH-9's per-row quick-weight control bind.
//!
//! The History section and the severe-health flag are other phases' (D1.6): this command returns no
//! placeholder field for them, because a contract field nothing may fill would outlive the phase that
//! invented it.

use super::super::types::*;

/// One household's dashboard rows (DASH-1, DASH-2, DASH-3, DASH-4).
///
/// The household comes from the argument, never from this device's active-household selection: which
/// household is active is per-device local state the backend must not infer (see
/// [`create_rabbit`](super::rabbit::create_rabbit)).
///
/// Every non-archived rabbit of the household, each with its name, its thumbnail reference, its
/// last-weighed instant or the never-weighed indication, and the derived weight-drop flag. Archived
/// rabbits are absent (DASH-1). Rows are sorted by most recent weigh-in, newest first, with
/// never-weighed rabbits last regardless of creation order (DASH-3) — sorted on the entry timestamp
/// rather than on creation order, because WT-1 allows a past timestamp and WT-5 allows an edit (R6).
///
/// An empty result is not proof that the household holds no rabbit: as with
/// [`list_rabbits`](super::rabbit::list_rabbits), a device that never loaded the collection and cannot
/// reach the service answers the same way (SYNC-3). DASH-8's empty state — the prompt to add the first
/// rabbit — is reachable only from a fresh reachable
/// [`ServiceStatus`](crate::ipc::types::ServiceStatus).
///
/// # Errors
///
/// None is expected: the rows resolve against the locally kept collection, so unreachability is a status
/// fact rather than a read failure (SYNC-3). A local store that cannot answer is `unknown`.
#[tauri::command]
#[specta::specta]
pub async fn get_dashboard(household_id: HouseholdId) -> Result<Vec<DashboardRow>, CommandError> {
    let _ = household_id;

    Err(CommandError::not_implemented(None))
}
