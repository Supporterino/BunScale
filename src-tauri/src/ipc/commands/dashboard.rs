//! The dashboard read: one household's rabbit rows (DASH-1, DASH-2, DASH-3, DASH-4, DASH-6, DASH-9).
//!
//! One command, frozen by task 1.1: [`get_dashboard`], implemented by task 2.14 as a thin adapter
//! over [`crate::domain::rabbit::dashboard`].
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

/// The `get_dashboard` body over the state alone.
///
/// It reads this device's copy and never the service: `get_dashboard` is the local read the
/// frontend renders from, while [`load_household`](super::sync::load_household) owns the refresh
/// (SYNC-3). The never-loaded copy is the empty list, which is not a claim that the household holds
/// no rabbit.
async fn dashboard_rows(
    state: &crate::state::AppState,
    household_id: HouseholdId,
) -> Result<Vec<DashboardRow>, CommandError> {
    // A blank id names no collection, so it is refused like `load_household` and `list_rabbits`
    // refuse one rather than answered with an empty list a caller could mistake for a loaded-and-empty
    // household (SYNC-3).
    if household_id.0.trim().is_empty() {
        return Err(CommandError::validation(None));
    }

    let collection = crate::sync::ReadPath::shared().serve_local(state, &household_id);

    crate::domain::rabbit::dashboard(&collection)
}

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
/// `validation` for a blank household id, which names no collection. `unknown` with the
/// unknown-content key when a live row of the collection has content this device cannot describe:
/// omitting the row would present a partial list as the household's own, so the read refuses instead
/// (D1.16). `unknown` when a stored timestamp cannot be ordered (DM-6). Unreachability is not an
/// error: the read resolves against the locally kept copy, so it is a status fact rather than a read
/// failure (SYNC-3).
#[tauri::command]
#[specta::specta]
pub async fn get_dashboard(
    household_id: HouseholdId,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<DashboardRow>, CommandError> {
    dashboard_rows(state.inner(), household_id).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::domain::RabbitRecord;
    use crate::ipc::types::{
        CollectionItem, CollectionSource, Grams, HouseholdCollection, ItemId, ItemPayload,
        NeuterStatus, RabbitProfile, RequestId, SaveCause, Sex, WeightEntry, WeightEntryId,
    };
    use crate::state::AppState;
    use crate::sync::ScriptedService;
    use crate::sync::double::test_support::block_on_ready;

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn profile() -> RabbitProfile {
        RabbitProfile {
            name: "Pebble".to_owned(),
            photo_ref: Some("photo-1".to_owned()),
            breed: None,
            dob: None,
            dob_unknown: false,
            sex: Sex::Female,
            neuter_status: NeuterStatus::Spayed,
            adoption_date: None,
            weight_drop_threshold_percent: 5.0,
        }
    }

    fn stored_rabbit(id: &str) -> crate::ipc::types::Rabbit {
        stored_rabbit_with(id, "Pebble", 5.0)
    }

    fn stored_rabbit_with(id: &str, name: &str, threshold: f64) -> crate::ipc::types::Rabbit {
        RabbitRecord::created(
            household("household-1"),
            RabbitProfile {
                name: name.to_owned(),
                weight_drop_threshold_percent: threshold,
                ..profile()
            },
            &RequestId(id.to_owned()),
        )
        .expect("the profile is valid")
        .to_wire()
    }

    fn rabbit_row(rabbit: &crate::ipc::types::Rabbit) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(rabbit.id.0.clone()),
            updated_at: at("2026-10-22T09:30:00Z"),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    fn weighed_entry(
        item_id: &str,
        rabbit: &crate::ipc::types::Rabbit,
        value: i32,
        timestamp: &str,
    ) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(timestamp),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId(item_id.to_owned()),
                    rabbit_id: rabbit.id.clone(),
                    value_grams: Grams(value),
                    timestamp: at(timestamp),
                    created_at: at(timestamp),
                    note: None,
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    fn entry_row(rabbit: &crate::ipc::types::Rabbit) -> CollectionItem {
        CollectionItem {
            item_id: ItemId("entry-1".to_owned()),
            updated_at: at("2026-10-21T09:00:00Z"),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId("entry-1".to_owned()),
                    rabbit_id: rabbit.id.clone(),
                    value_grams: Grams(1_800),
                    timestamp: at("2026-10-21T09:00:00Z"),
                    created_at: at("2026-10-21T09:00:00Z"),
                    note: None,
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    /// SYNC-3: a blank id names no collection, so it is `validation` — never an empty list a caller
    /// could mistake for a loaded household with no rabbits.
    #[test]
    fn a_blank_household_id_is_refused_as_validation_rather_than_answered_with_an_empty_list() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);

        for blank in ["", "   "] {
            let error = block_on_ready(dashboard_rows(&state, household(blank)))
                .expect_err("a blank id names no collection");

            assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
            assert!(!error.retryable, "{blank:?}");
            assert_eq!(error.request_id, None, "{blank:?}");
        }
    }

    /// SYNC-3: the read is served from this device's copy and asks the service nothing, so the rows
    /// a previous load stored are the rows the dashboard presents.
    #[test]
    fn the_dashboard_serves_the_locally_held_rows_without_asking_the_service() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let one = household("household-1");
        let rabbit = stored_rabbit("rabbit-1");

        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&rabbit), entry_row(&rabbit)],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        let rows = block_on_ready(dashboard_rows(&state, one)).expect("every content is known");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rabbit.name, "Pebble");
        assert_eq!(rows[0].last_weighed_at, Some(at("2026-10-21T09:00:00Z")));
        assert!(!rows[0].never_weighed);
        assert!(
            service.calls().is_empty(),
            "the read resolves against the copy, not the transport"
        );
    }

    /// DASH-2/3/4 through the command body: three rows sort by most recent weigh-in with the
    /// never-weighed rabbit last, the flagged drop raises its row's flag, and raising the threshold
    /// clears it on the next read.
    #[test]
    fn the_dashboard_rows_sort_and_flag_through_the_command_body() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let one = household("household-1");
        let flagged = stored_rabbit_with("rabbit-1", "Flagged", 5.0);
        let quiet = stored_rabbit_with("rabbit-2", "Quiet", 5.0);
        let never = stored_rabbit_with("rabbit-3", "Never", 5.0);

        let load = |state: &AppState, flagged: &crate::ipc::types::Rabbit| {
            state
                .load_collection(
                    &one,
                    HouseholdCollection {
                        household_id: one.clone(),
                        items: vec![
                            rabbit_row(&never),
                            rabbit_row(&quiet),
                            rabbit_row(flagged),
                            weighed_entry("entry-1", flagged, 2_000, "2026-10-01T08:00:00Z"),
                            weighed_entry("entry-2", flagged, 1_800, "2026-10-02T08:00:00Z"),
                            weighed_entry("entry-3", &quiet, 2_000, "2026-10-01T08:00:00Z"),
                            weighed_entry("entry-4", &quiet, 1_950, "2026-10-02T08:00:00Z"),
                        ],
                        source: CollectionSource::Service,
                        loaded_at: None,
                    },
                    at("2026-10-22T10:00:00Z"),
                )
                .expect("the collection is the household's own");
        };

        load(&state, &flagged);

        let rows =
            block_on_ready(dashboard_rows(&state, one.clone())).expect("every content is known");
        assert_eq!(
            rows.iter()
                .map(|row| row.rabbit.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Flagged", "Quiet", "Never"],
            "most recent weigh-in first, never-weighed last"
        );
        assert!(rows[0].has_weight_drop_alert, "the 10% drop flags the row");
        assert!(!rows[1].has_weight_drop_alert, "the 2.5% drop does not");
        assert!(rows[2].never_weighed);
        assert_eq!(rows[2].last_weighed_at, None);

        let raised = crate::ipc::types::Rabbit {
            weight_drop_threshold_percent: 15.0,
            ..flagged.clone()
        };
        load(&state, &raised);

        let rows = block_on_ready(dashboard_rows(&state, one)).expect("every content is known");
        assert!(
            rows.iter().all(|row| !row.has_weight_drop_alert),
            "raising the threshold clears the dashboard flag"
        );
        assert!(
            service.calls().is_empty(),
            "both reads resolve against the copy, not the transport"
        );
    }
}
