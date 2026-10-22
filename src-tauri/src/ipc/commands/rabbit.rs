//! Rabbit-profile commands: the P1 lifecycle over one household's rabbits (RAB-1 … RAB-10).
//!
//! Seven commands, frozen by task 1.1: [`create_rabbit`], [`list_rabbits`], [`get_rabbit`],
//! [`update_rabbit`], [`archive_rabbit`], [`restore_rabbit`] and [`delete_rabbit`]. Each body was a
//! typed stub returning [`CommandError::not_implemented`]; section 2 replaces them one task at a
//! time, and the signatures are what `src/bindings.ts` is generated from, so a stub is the contract
//! rather than a placeholder to be reshaped.
//!
//! **[`create_rabbit`] (task 2.1), [`update_rabbit`] (task 2.4), [`list_rabbits`] and
//! [`archive_rabbit`] (task 2.5), [`restore_rabbit`] (task 2.6) and [`delete_rabbit`] (task 2.7) are
//! implemented; [`get_rabbit`] is still a stub.** Each implemented body takes the app's state and, for a
//! write, a publication seam — `tauri::State` is how a command reaches the shared service and the
//! account, and `tauri::AppHandle` is the seam [`RabbitChangePublisher`] is implemented by — which
//! are parameters Tauri supplies and `src/bindings.ts` does not export, so the command the frontend
//! calls keeps the arguments it was frozen with. A successful create publishes `rabbit:changed` with
//! kind `created`, an edit with kind `updated`, an archive with kind `archived`, a restore with kind
//! `restored`, and a deletion with kind `deleted`, as the contract's event table requires of every
//! successful rabbit write; a refusal publishes nothing. The remaining stub keeps exactly the
//! arguments it was frozen with, so its bindings do not move either.
//!
//! Three things the signatures already encode, because section 2 must not have to renegotiate them:
//!
//! * **Writes carry a [`RequestId`], reads do not.** Every state-changing command is idempotent on it
//!   (D0.14) and re-issues the same value on a retry, so its failure can carry the identity the retry
//!   needs; the three reads are idempotent already and report `request_id: None`.
//! * **`update_rabbit` replaces the whole profile** through [`RabbitProfile`] rather than a partial
//!   patch: the edit form is pre-filled, and a patch would need a double-optional per field to tell
//!   "unchanged" from "cleared" (see [`RabbitProfile`]).
//! * **Archived means immutable at the boundary** (D1.5, RAB-8). Every content write refuses an
//!   archived target — `update_rabbit` and the weight-entry writes included — and reports it through
//!   the one failure surface; restoring is the one operation that accepts an archived rabbit, because
//!   it is the transition that returns it to the active list (RAB-7). P0's cause list is frozen and
//!   has no "immutable target" variant, so the refusal is `validation`: the request itself is refused
//!   and a retry cannot clear it (D1.5, NFR-10). Archiving now produces that state (task 2.5), but the
//!   refusal on the remaining write paths — [`update_rabbit`] and the weight-entry writes — is task
//!   2.8's, so the check is the hook [`crate::state::AppState::update_rabbit`] documents.
//!
//! Every remaining stub body consumes its arguments deliberately: the parameters are part of the
//! frozen signature and the bindings' argument names, and an unused-parameter warning under
//! `clippy -D warnings` would otherwise have to be silenced.

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::events::{RabbitChangeKind, RabbitChangePublisher, RabbitChanged};
use super::super::types::*;

/// The `create_rabbit` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call: `tauri::State` has no public constructor outside a
/// running app (the same reason `commands::household`'s bodies exist). The change is published only
/// after the create succeeded, so a refusal leaves the recorder empty and the frontend's policy never
/// invalidates a household for a rabbit the service did not accept.
async fn created_rabbit(
    state: &crate::state::AppState,
    publisher: &impl RabbitChangePublisher,
    household_id: HouseholdId,
    profile: RabbitProfile,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let rabbit = state
        .create_rabbit(household_id, profile, request_id)
        .await?;

    publisher.publish(&RabbitChanged {
        household_id: rabbit.household_id.clone(),
        rabbit_id: rabbit.id.clone(),
        kind: RabbitChangeKind::Created,
    });

    Ok(rabbit)
}

/// The `update_rabbit` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call, for the same reason [`created_rabbit`] is. The
/// change is published only after the edit succeeded, so a refusal leaves the recorder empty and the
/// frontend's policy never invalidates a profile the service did not accept.
async fn updated_rabbit(
    state: &crate::state::AppState,
    publisher: &impl RabbitChangePublisher,
    rabbit_id: RabbitId,
    profile: RabbitProfile,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let rabbit = state.update_rabbit(rabbit_id, profile, request_id).await?;

    publisher.publish(&RabbitChanged {
        household_id: rabbit.household_id.clone(),
        rabbit_id: rabbit.id.clone(),
        kind: RabbitChangeKind::Updated,
    });

    Ok(rabbit)
}

/// The `list_rabbits` body over the state alone.
///
/// It reads this device's copy and never the service: `list_rabbits` is the local read the frontend
/// renders from, while [`load_household`](super::sync::load_household) owns the refresh (SYNC-3). The
/// never-loaded copy is the empty list, which is not a claim that the household holds no rabbit.
async fn listed_rabbits(
    state: &crate::state::AppState,
    household_id: HouseholdId,
    include_archived: bool,
) -> Result<Vec<RabbitSummary>, CommandError> {
    // A blank id names no collection, so it is refused like `load_household` refuses one rather than
    // answered with an empty list a caller could mistake for a loaded-and-empty household (SYNC-3).
    if household_id.0.trim().is_empty() {
        return Err(CommandError::validation(None));
    }

    let collection = crate::sync::ReadPath::shared().serve_local(state, &household_id);

    crate::domain::rabbit::summaries(&collection, include_archived)
}

/// The `archive_rabbit` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call, for the same reason [`created_rabbit`] is. The
/// change is published only after the archive succeeded, so a refusal leaves the recorder empty and
/// the frontend's policy never invalidates a rabbit the service did not accept.
async fn archived_rabbit(
    state: &crate::state::AppState,
    publisher: &impl RabbitChangePublisher,
    rabbit_id: RabbitId,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let rabbit = state.archive_rabbit(rabbit_id, request_id).await?;

    publisher.publish(&RabbitChanged {
        household_id: rabbit.household_id.clone(),
        rabbit_id: rabbit.id.clone(),
        kind: RabbitChangeKind::Archived,
    });

    Ok(rabbit)
}

/// The `restore_rabbit` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call, for the same reason [`created_rabbit`] is. The
/// change is published only after the restore succeeded, so a refusal leaves the recorder empty and
/// the frontend's policy never invalidates a rabbit the service did not accept.
async fn restored_rabbit(
    state: &crate::state::AppState,
    publisher: &impl RabbitChangePublisher,
    rabbit_id: RabbitId,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let rabbit = state.restore_rabbit(rabbit_id, request_id).await?;

    publisher.publish(&RabbitChanged {
        household_id: rabbit.household_id.clone(),
        rabbit_id: rabbit.id.clone(),
        kind: RabbitChangeKind::Restored,
    });

    Ok(rabbit)
}

/// The `delete_rabbit` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call, for the same reason [`created_rabbit`] is. The
/// change is published only after the deletion succeeded, so a refusal leaves the recorder empty and
/// the frontend's policy never invalidates anything for a removal the service did not accept. The
/// payload names the rabbit that no longer exists, which is what a listener needs to drop its cached
/// rows (the event contract).
async fn deleted_rabbit(
    state: &crate::state::AppState,
    publisher: &impl RabbitChangePublisher,
    rabbit_id: RabbitId,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let household_id = state.delete_rabbit(rabbit_id.clone(), request_id).await?;

    publisher.publish(&RabbitChanged {
        household_id,
        rabbit_id,
        kind: RabbitChangeKind::Deleted,
    });

    Ok(())
}

/// Create a rabbit profile in a household (RAB-1, RAB-2, RAB-3).
///
/// The profile is stored in the household's shared collection, so every member resolves it (HS-8).
/// `household_id` is explicit even though the person has an active household: the write belongs to
/// the collection it names, and the active household is a per-device selection rather than a scope
/// the backend may infer.
///
/// # Errors
///
/// `validation` for a blank or whitespace-only name, which is treated as blank (RAB-2), and for a
/// blank operation identity. The write path's service conditions — `connectivity`, `no_account`,
/// `allowance_exceeded`, `rate_limited`, `timeout` — are reachable because the profile is a shared
/// write (D1.1), and `unknown` is the honest remainder.
#[tauri::command]
#[specta::specta]
pub async fn create_rabbit(
    household_id: HouseholdId,
    profile: RabbitProfile,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Rabbit, CommandError> {
    created_rabbit(state.inner(), &app, household_id, profile, request_id).await
}

/// The household's rabbits, as either the active list or the archived list (DASH-1, RAB-6).
///
/// The two values select **one list each**, never a union: `include_archived: false` is DASH-1's
/// active list — every non-archived rabbit of the household, in no required order because the
/// dashboard sorts by most recent weigh-in (DASH-3) — and `include_archived: true` is the archived
/// list, RAB-6's "retrievable from an archived list", and contains no active rabbit. The name reads
/// like a union only because it began as the design's draft parameter; the contract's meaning is the
/// one stated here and in the design, and section 2 must not implement "both lists".
///
/// An empty result is **not** proof that the household holds no rabbit: as with
/// [`list_households`](super::household::list_households), a device that never loaded the collection
/// and cannot reach the service answers the same way (SYNC-3), so the empty state is a conclusion to
/// draw only from a fresh reachable [`ServiceStatus`](crate::ipc::types::ServiceStatus).
///
/// A weight entry of the same household collection is not a rabbit and appears in neither list, and a
/// tombstoned rabbit appears in neither (RAB-10).
///
/// # Errors
///
/// `validation` for a blank household id, which names no collection. `unknown` when a live row of the
/// collection has content this device cannot describe: omitting the row would present a partial list
/// as the household's own, so the read refuses instead (D1.16). Unreachability is not an error: the
/// read resolves against the locally kept copy, so it is a status fact rather than a read failure
/// (SYNC-3).
#[tauri::command]
#[specta::specta]
pub async fn list_rabbits(
    household_id: HouseholdId,
    include_archived: bool,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<RabbitSummary>, CommandError> {
    listed_rabbits(state.inner(), household_id, include_archived).await
}

/// One rabbit's profile and archived state (DASH-6, RAB-5).
///
/// # Errors
///
/// `validation` for an id this household's collection does not hold, or for a blank id. A rabbit of
/// another household is reported the same way and never resolved: RAB-3 forbids presenting a rabbit
/// of one household as a rabbit of another.
#[tauri::command]
#[specta::specta]
pub async fn get_rabbit(rabbit_id: RabbitId) -> Result<Rabbit, CommandError> {
    let _ = rabbit_id;

    Err(CommandError::not_implemented(None))
}

/// Replace the editable fields of an active rabbit's profile (RAB-5, WT-7).
///
/// Every field is editable through this command, including `weight_drop_threshold_percent`: the
/// threshold is a profile field (RAB-1/RAB-5) rather than a setting of its own, so editing it is an
/// ordinary shared write and every member receives it. Because the alert is derived on read (D1.3), a
/// threshold edit re-derives the entry flags and the dashboard flag without rewriting an entry.
///
/// The change applies only to an **active** rabbit; an archived rabbit's profile is not editable
/// (RAB-5, D1.5). **That refusal is not produced by this body yet:** no rabbit can be archived before
/// task 2.5, and task 2.8 adds the write-boundary check for this command as well as the weight-entry
/// writes — see [`crate::state::AppState::update_rabbit`], where the check belongs.
///
/// # Errors
///
/// `validation` for a blank name (RAB-2), for an unknown or already-deleted rabbit — the request is
/// refused rather than accepted and ignored — and for a blank operation identity; once tasks 2.5 and
/// 2.8 land, an archived rabbit is refused the same way. The write path's service conditions are
/// reachable as for [`create_rabbit`], and `unknown` is the honest remainder.
#[tauri::command]
#[specta::specta]
pub async fn update_rabbit(
    rabbit_id: RabbitId,
    profile: RabbitProfile,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Rabbit, CommandError> {
    updated_rabbit(state.inner(), &app, rabbit_id, profile, request_id).await
}

/// Archive a rabbit: a reversible soft delete that discards nothing (RAB-6, RAB-9).
///
/// The rabbit leaves the active list and appears in the archived one, and every weight entry it holds
/// stays stored — nothing purges archived data, and this command deletes none of it (RAB-9). The
/// operation is reversible through [`restore_rabbit`]. Archiving is **not** a destructive action under
/// NFR-9, so it carries no consequence confirmation (D1.8); the confirmation belongs to
/// [`delete_rabbit`].
///
/// The write submits the **whole item**: the rabbit's content is read from this device's copy and
/// written back unchanged except for `is_archived`, because a partial payload would replace the
/// stored version and discard the profile (RAB-9, D1.16).
///
/// # Errors
///
/// `validation` for an unknown rabbit, for one that is already archived, for a rabbit this device
/// holds only as a tombstone, and for a blank operation identity. `unknown` when the row's content is
/// not on this device: the archive refuses rather than submitting a partial payload, and a refresh
/// fills the content (D1.16). The write path's service conditions are reachable as for
/// [`create_rabbit`].
#[tauri::command]
#[specta::specta]
pub async fn archive_rabbit(
    rabbit_id: RabbitId,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Rabbit, CommandError> {
    archived_rabbit(state.inner(), &app, rabbit_id, request_id).await
}

/// Return an archived rabbit to the active list with its history intact (RAB-7).
///
/// Every weight entry recorded before archiving is still present and unchanged afterwards: restoring
/// moves the rabbit's archived state and nothing else. The write submits the **whole item**, the
/// rabbit's content read from this device's copy and written back unchanged except for `is_archived`,
/// because a partial payload would replace the stored version and discard the profile (RAB-7, D1.16).
///
/// A successful restore publishes `rabbit:changed` with kind `restored`; a refusal publishes nothing.
///
/// # Errors
///
/// `validation` for an unknown rabbit, for a rabbit this device holds only as a tombstone, for one
/// that is not archived, and for a blank operation identity. `unknown` when the row's content is not
/// on this device: the restore refuses rather than submitting a partial payload, and a refresh fills
/// the content (D1.16). The write path's service conditions are reachable as for [`create_rabbit`].
#[tauri::command]
#[specta::specta]
pub async fn restore_rabbit(
    rabbit_id: RabbitId,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Rabbit, CommandError> {
    restored_rabbit(state.inner(), &app, rabbit_id, request_id).await
}

/// Permanently delete an **archived** rabbit and every weight entry it holds (RAB-10).
///
/// Irreversible, and shared with every member: once it completes, no member resolves the rabbit or its
/// entries. Deletion is available only for an archived rabbit — an active one is refused, which is what
/// keeps archiving the recoverable step before the point of no return (D1.8, R7). The confirmation
/// that states the consequence is the frontend's (NFR-9); this command is called only after it.
///
/// The removal is written as a **tombstone per item** — the rabbit and each weight entry that names it
/// — into the household's shared collection, so the change feed reports it and every member resolves
/// the household without it (D1.16, D1.17, HS-8). Each tombstone keeps the identity, the operation's
/// version marker, the writing member and the content of the version that removed it, and carries the
/// removal as `save_item`'s own `deleted` argument. Every read excludes a tombstone before its
/// unknown-content check, so a removed item is never answered as unreadable (D1.17). The cascade stops
/// before the rabbit's tombstone if any entry's removal does not apply, so the removal is never
/// reported while a readable entry would survive it (RAB-10).
///
/// A successful deletion publishes `rabbit:changed` with kind `deleted`; a refusal publishes nothing.
/// No `weight:changed` is published per cascaded entry: the rabbit's removal names the whole subtree a
/// listener drops, and this change has no weight write that owns those entries yet (tasks 2.9–2.11).
///
/// # Errors
///
/// `validation` for an **active** rabbit — the deliberate narrowing of RAB-10 (D1.8) — for an unknown
/// rabbit, for one this device holds only as a tombstone, for a blank operation identity, and for an
/// id the derived ledger keys cannot keep distinct (D1.17). `unknown` with the unknown-content key when
/// the rabbit's row or another live row cannot be described (D1.16), and `unknown` when a tombstone did
/// not apply. The write path's service conditions are reachable as for [`create_rabbit`]; a deletion the
/// service did not accept leaves the rabbit in place and is never reported as removed.
#[tauri::command]
#[specta::specta]
pub async fn delete_rabbit(
    rabbit_id: RabbitId,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<(), CommandError> {
    deleted_rabbit(state.inner(), &app, rabbit_id, request_id).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::ipc::events::test_support::RecordingRabbitPublisher;
    use crate::ipc::types::{
        AccountUserId, CollectionItem, CollectionSource, Grams, HouseholdCollection, ItemId,
        ItemPayload, NeuterStatus, Sex, WeightEntry, WeightEntryId,
    };
    use crate::state::AppState;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::{Answer, ScriptedService, ServiceReply};

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    /// A RAB-1 profile with every field supplied.
    fn profile() -> RabbitProfile {
        RabbitProfile {
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: Some("Mini Lop".to_owned()),
            dob: Some(Instant("2026-03-01T00:00:00Z".to_owned())),
            dob_unknown: false,
            sex: Sex::Female,
            neuter_status: NeuterStatus::Spayed,
            adoption_date: None,
            weight_drop_threshold_percent: 5.0,
        }
    }

    /// One accepted create answer the domain can read, so the book can hold a household to create in.
    fn accepted_household() -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "outcome": "applied",
                "household": {
                    "id": "household-1",
                    "owner_member_id": "member-1",
                    "created_at": "2026-10-19T10:00:00Z",
                },
            }),
        ))
    }

    /// A state whose book holds `household-1` with this account as its member, so a rabbit create can
    /// name its writer. The double's last write answer is repeated, so the rabbit write is accepted
    /// by the same script.
    fn state_with_a_held_household(service: Arc<ScriptedService>) -> AppState {
        let state = AppState::new(service, Some(account()));

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        state
    }

    #[test]
    fn a_successful_create_publishes_exactly_one_created_for_the_new_rabbit() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let state = state_with_a_held_household(service);
        let publisher = RecordingRabbitPublisher::default();

        let created = block_on_ready(created_rabbit(
            &state,
            &publisher,
            household("household-1"),
            profile(),
            request_id("operation-2"),
        ))
        .expect("the double accepted the write");

        assert_eq!(created.name, "Pebble");
        assert_eq!(
            publisher.published(),
            vec![RabbitChanged {
                household_id: household("household-1"),
                rabbit_id: created.id,
                kind: RabbitChangeKind::Created,
            }],
            "one report for the rabbit the command returned"
        );
    }

    #[test]
    fn a_refused_create_publishes_nothing() {
        // P0 has no account, so the create is refused before anything is sent (Q25).
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let publisher = RecordingRabbitPublisher::default();

        let error = block_on_ready(created_rabbit(
            &state,
            &publisher,
            household("household-1"),
            profile(),
            request_id("operation-1"),
        ))
        .expect_err("P0 has no account");

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// One collection row for the rabbit, as the read path delivers the household's copy.
    fn collection_with(household_id: &HouseholdId, rabbit: &Rabbit) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items: vec![CollectionItem {
                item_id: ItemId(rabbit.id.0.clone()),
                updated_at: Instant("2026-10-22T09:30:00Z".to_owned()),
                deleted: false,
                payload: Some(ItemPayload::Rabbit {
                    rabbit: rabbit.clone(),
                }),
            }],
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }

    /// One collection row with no content: a row this device knows of but cannot describe (D1.16).
    fn collection_with_unknown(household_id: &HouseholdId, item_id: &str) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items: vec![CollectionItem {
                item_id: ItemId(item_id.to_owned()),
                updated_at: Instant("2026-10-22T09:30:00Z".to_owned()),
                deleted: false,
                payload: None,
            }],
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }

    /// One weight entry of `rabbit`, as the read path delivers it beside the rabbit's row.
    fn weight_entry_row(rabbit: &Rabbit, entry_id: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(entry_id.to_owned()),
            updated_at: Instant("2026-10-22T09:30:00Z".to_owned()),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: WeightEntry {
                    id: WeightEntryId(entry_id.to_owned()),
                    rabbit_id: rabbit.id.clone(),
                    value_grams: Grams(1_800),
                    timestamp: Instant("2026-10-21T09:00:00Z".to_owned()),
                    note: None,
                    photo_refs: Vec::new(),
                    has_weight_drop_alert: false,
                },
            }),
        }
    }

    /// One row carrying `rabbit`'s content, at `updated_at`.
    fn rabbit_row(rabbit: &Rabbit, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(rabbit.id.0.clone()),
            updated_at: Instant(updated_at.to_owned()),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    /// A state whose book holds `household-1`, whose copy holds one rabbit, and that rabbit itself —
    /// the state an edit starts from, since the edit is addressed by the rabbit alone.
    fn state_with_one_rabbit(service: Arc<ScriptedService>) -> (AppState, Rabbit) {
        let state = state_with_a_held_household(service);
        let one = household("household-1");
        let rabbit =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the rabbit is created");

        state
            .load_collection(
                &one,
                collection_with(&one, &rabbit),
                Instant("2026-10-22T09:31:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        (state, rabbit)
    }

    /// A state whose book holds `household-1` and whose copy holds one **archived** rabbit with its
    /// content — the state a restore starts from.
    fn state_with_an_archived_rabbit(service: Arc<ScriptedService>) -> (AppState, Rabbit) {
        let (state, rabbit) = state_with_one_rabbit(service);
        let one = household("household-1");
        let archived = Rabbit {
            is_archived: true,
            ..rabbit
        };

        state
            .load_collection(
                &one,
                collection_with(&one, &archived),
                Instant("2026-10-22T09:31:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        (state, archived)
    }

    #[test]
    fn a_successful_edit_publishes_exactly_one_updated_for_the_rabbit() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service);
        let publisher = RecordingRabbitPublisher::default();

        let edited = block_on_ready(updated_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            RabbitProfile {
                name: "Clover".to_owned(),
                weight_drop_threshold_percent: 15.0,
                ..profile()
            },
            request_id("operation-3"),
        ))
        .expect("the double accepted the edit");

        assert_eq!(edited.id, rabbit.id);
        assert_eq!(edited.name, "Clover");
        assert_eq!(edited.weight_drop_threshold_percent, 15.0);
        assert_eq!(
            publisher.published(),
            vec![RabbitChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                kind: RabbitChangeKind::Updated,
            }],
            "one report, with the edit's kind, for the rabbit the command returned"
        );
    }

    #[test]
    fn a_refused_edit_publishes_nothing() {
        // The device holds no copy naming the rabbit, so the edit is refused before anything is sent.
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let publisher = RecordingRabbitPublisher::default();

        let error = block_on_ready(updated_rabbit(
            &state,
            &publisher,
            RabbitId("rabbit-1".to_owned()),
            profile(),
            request_id("operation-1"),
        ))
        .expect_err("there is no rabbit to edit");

        assert_eq!(error.cause, SaveCause::Validation);
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// RAB-6, RAB-9: the archive is one item write that returns the archived rabbit, and only a
    /// successful archive publishes `rabbit:changed` with kind `archived`.
    #[test]
    fn a_successful_archive_publishes_exactly_one_archived_for_the_rabbit() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service);
        let publisher = RecordingRabbitPublisher::default();

        let archived = block_on_ready(archived_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the archive");

        assert!(archived.is_archived);
        assert_eq!(archived.id, rabbit.id);
        assert_eq!(
            archived.name, rabbit.name,
            "the profile is carried, not replaced"
        );
        assert_eq!(
            publisher.published(),
            vec![RabbitChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                kind: RabbitChangeKind::Archived,
            }],
            "one report, with the archive's kind, for the rabbit the command returned"
        );
    }

    /// D1.16: a row whose content is unknown cannot be archived — the whole item is written, so a
    /// partial payload would replace the stored version. The refusal publishes nothing.
    #[test]
    fn an_archive_of_a_row_with_unknown_content_is_refused_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let state = state_with_a_held_household(service);
        let one = household("household-1");
        state
            .load_collection(
                &one,
                collection_with_unknown(&one, "rabbit-1"),
                Instant("2026-10-22T09:31:00Z".to_owned()),
            )
            .expect("the collection is the household's own");
        let publisher = RecordingRabbitPublisher::default();
        let request_id = request_id("operation-3");

        let error = block_on_ready(archived_rabbit(
            &state,
            &publisher,
            RabbitId("rabbit-1".to_owned()),
            request_id.clone(),
        ))
        .expect_err("content this device cannot read cannot be written back");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// RAB-7: the restore is one item write that returns the restored rabbit, and only a successful
    /// restore publishes `rabbit:changed` with kind `restored`.
    #[test]
    fn a_successful_restore_publishes_exactly_one_restored_for_the_rabbit() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit(service);
        let publisher = RecordingRabbitPublisher::default();

        let restored = block_on_ready(restored_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the restore");

        assert!(!restored.is_archived);
        assert_eq!(restored.id, rabbit.id);
        assert_eq!(
            restored,
            Rabbit {
                is_archived: false,
                ..rabbit.clone()
            },
            "the profile is carried, not replaced"
        );
        assert_eq!(
            publisher.published(),
            vec![RabbitChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                kind: RabbitChangeKind::Restored,
            }],
            "one report, with the restore's kind, for the rabbit the command returned"
        );
    }

    /// RAB-7: restoring a rabbit that is not archived is refused, and the refusal publishes nothing.
    #[test]
    fn a_refused_restore_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service);
        let publisher = RecordingRabbitPublisher::default();

        let error = block_on_ready(restored_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect_err("the rabbit is already active");

        assert_eq!(error.cause, SaveCause::Validation);
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// D1.16: a row whose content is unknown cannot be restored — the whole item is written, so a
    /// partial payload would replace the stored version. The refusal publishes nothing.
    #[test]
    fn a_restore_of_a_row_with_unknown_content_is_refused_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let state = state_with_a_held_household(service);
        let one = household("household-1");
        state
            .load_collection(
                &one,
                collection_with_unknown(&one, "rabbit-1"),
                Instant("2026-10-22T09:31:00Z".to_owned()),
            )
            .expect("the collection is the household's own");
        let publisher = RecordingRabbitPublisher::default();
        let request_id = request_id("operation-3");

        let error = block_on_ready(restored_rabbit(
            &state,
            &publisher,
            RabbitId("rabbit-1".to_owned()),
            request_id.clone(),
        ))
        .expect_err("content this device cannot read cannot be written back");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// RAB-10, D1.16: a tombstone names a permanently deleted rabbit, so a restore never reaches it.
    /// The refusal is `validation` and publishes nothing.
    #[test]
    fn a_restore_of_a_tombstoned_rabbit_is_refused_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let state = state_with_a_held_household(service);
        let one = household("household-1");
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![CollectionItem {
                        item_id: ItemId("rabbit-1".to_owned()),
                        updated_at: Instant("2026-10-22T10:00:00Z".to_owned()),
                        deleted: true,
                        payload: None,
                    }],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T10:01:00Z".to_owned()),
            )
            .expect("the collection is the household's own");
        let publisher = RecordingRabbitPublisher::default();
        let request_id = request_id("operation-3");

        let error = block_on_ready(restored_rabbit(
            &state,
            &publisher,
            RabbitId("rabbit-1".to_owned()),
            request_id.clone(),
        ))
        .expect_err("a deleted rabbit is not restorable");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert_eq!(error.request_id, Some(request_id));
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// RAB-7's two scenarios through the real bodies: an archived rabbit carrying weight entries is
    /// restored, it returns to the active list and leaves the archived one, and the entries recorded
    /// before archiving are still present, unchanged and readable.
    #[test]
    fn the_restored_rabbit_returns_to_the_active_list_and_keeps_its_history() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit(service);
        let one = household("household-1");
        let publisher = RecordingRabbitPublisher::default();
        let entry = weight_entry_row(&rabbit, "entry-1");

        // The archived rabbit carries one entry recorded before archiving.
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&rabbit, "2026-10-22T09:32:00Z"), entry.clone()],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:33:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        let restored = block_on_ready(restored_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the restore");

        // The read path's next refresh stores the accepted version beside the entry it already holds;
        // the write itself never touches this device's copy (SYNC-3).
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&restored, "2026-10-22T09:34:00Z"), entry.clone()],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:35:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        let active = block_on_ready(listed_rabbits(&state, one.clone(), false))
            .expect("the active list resolves");
        assert_eq!(
            active,
            vec![RabbitSummary {
                id: rabbit.id.clone(),
                name: rabbit.name.clone(),
                thumbnail_ref: rabbit.photo_ref.clone(),
                is_archived: false,
            }],
            "the rabbit appears again in the active list"
        );

        let archived_list = block_on_ready(listed_rabbits(&state, one.clone(), true))
            .expect("the archived list resolves");
        assert!(
            archived_list.is_empty(),
            "it no longer appears in the archived list"
        );

        // RAB-7's second scenario: the entry recorded before archiving is still present, unchanged,
        // and reads back with its content.
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        let readable = held
            .items
            .iter()
            .find(|item| item.item_id == entry.item_id)
            .expect("the entry is still stored beside the restored rabbit");
        assert_eq!(readable, &entry, "the entry is unchanged");
    }

    /// RAB-6's first scenario through the real bodies: after the archive and the read path's next
    /// refresh, the rabbit is gone from the active list and present in the archived list. Its weight
    /// history is still held, which is what the archived list's detail read resolves (RAB-6, RAB-9).
    #[test]
    fn the_archived_rabbit_leaves_the_active_list_and_appears_in_the_archived_list() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service);
        let one = household("household-1");
        let publisher = RecordingRabbitPublisher::default();

        let archived = block_on_ready(archived_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the archive");

        // The read path's next refresh stores the accepted version beside the entries it already
        // holds; the write itself never touches this device's copy (SYNC-3).
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![
                        CollectionItem {
                            item_id: ItemId(rabbit.id.0.clone()),
                            updated_at: Instant("2026-10-22T09:32:00Z".to_owned()),
                            deleted: false,
                            payload: Some(ItemPayload::Rabbit {
                                rabbit: archived.clone(),
                            }),
                        },
                        weight_entry_row(&rabbit, "entry-1"),
                    ],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:33:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        let active = block_on_ready(listed_rabbits(&state, one.clone(), false))
            .expect("the active list resolves");
        assert!(active.is_empty(), "the rabbit left the active list");

        let archived_list = block_on_ready(listed_rabbits(&state, one.clone(), true))
            .expect("the archived list resolves");
        assert_eq!(
            archived_list,
            vec![RabbitSummary {
                id: rabbit.id.clone(),
                name: rabbit.name.clone(),
                thumbnail_ref: rabbit.photo_ref.clone(),
                is_archived: true,
            }],
            "the archived list contains it"
        );

        // Its history is retained: the entry is still a row of the copy the archived rabbit's reads
        // resolve (RAB-9).
        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert!(
            held.items
                .iter()
                .any(|item| item.item_id == ItemId("entry-1".to_owned())),
            "the entry recorded before archiving is still stored"
        );
    }

    /// D1.16: a set read refuses rather than omitting a row whose content it cannot describe — an
    /// omission would be indistinguishable from a complete list.
    #[test]
    fn a_list_refuses_rather_than_presenting_a_partial_one() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let one = household("household-1");
        state
            .load_collection(
                &one,
                collection_with_unknown(&one, "rabbit-1"),
                Instant("2026-10-22T09:31:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        for include_archived in [false, true] {
            let error = block_on_ready(listed_rabbits(&state, one.clone(), include_archived))
                .expect_err("the row's content is unknown");

            assert_eq!(error.cause, SaveCause::Unknown, "{include_archived}");
            assert!(!error.retryable, "{include_archived}");
            assert_eq!(
                error.message_key,
                crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY,
                "{include_archived}"
            );
            assert_ne!(error.message_key, "error.unknown", "{include_archived}");
            assert_eq!(error.request_id, None, "{include_archived}");
        }
    }

    /// SYNC-3: a blank id names no collection, so it is `validation` — never an empty list a caller
    /// could mistake for a loaded household with no rabbits.
    #[test]
    fn a_blank_household_id_is_refused_as_validation_rather_than_answered_with_an_empty_list() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);

        for blank in ["", "   "] {
            for include_archived in [false, true] {
                let error =
                    block_on_ready(listed_rabbits(&state, household(blank), include_archived))
                        .expect_err("a blank id names no collection");

                assert_eq!(error.cause, SaveCause::Validation, "{blank:?}");
                assert!(!error.retryable, "{blank:?}");
                assert_eq!(error.request_id, None, "{blank:?}");
            }
        }
    }

    /// A state whose book holds `household-1`, whose copy holds one archived rabbit and one entry it
    /// owns — the state a permanent deletion starts from (RAB-10).
    fn state_with_an_archived_rabbit_and_an_entry(
        service: Arc<ScriptedService>,
    ) -> (AppState, Rabbit) {
        let (state, rabbit) = state_with_an_archived_rabbit(service);
        let one = household("household-1");

        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![
                        rabbit_row(&rabbit, "2026-10-22T09:31:00Z"),
                        weight_entry_row(&rabbit, "entry-1"),
                    ],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:32:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        (state, rabbit)
    }

    /// RAB-10's first scenario through the real bodies: a confirmed deletion of an archived rabbit
    /// writes a tombstone for the rabbit and for its entry, publishes exactly one `deleted`, and — once
    /// the read path delivers those tombstones — the rabbit is in neither rabbit list and the entry is
    /// no longer a live row.
    #[test]
    fn a_confirmed_deletion_removes_the_rabbit_and_its_data() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit_and_an_entry(service.clone());
        let one = household("household-1");
        let publisher = RecordingRabbitPublisher::default();

        block_on_ready(deleted_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the deletion");

        assert_eq!(
            publisher.published(),
            vec![RabbitChanged {
                household_id: one.clone(),
                rabbit_id: rabbit.id.clone(),
                kind: RabbitChangeKind::Deleted,
            }],
            "one report, with the deletion's kind, for the rabbit that no longer exists"
        );

        // The entry is tombstoned before the rabbit, and both keep their identity and content.
        let writes: Vec<serde_json::Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                crate::sync::double::RecordedCall::Save { request, .. } => {
                    Some(request.body().clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            writes.len(),
            4,
            "the household, the create and the two-item deletion set"
        );
        assert_eq!(writes[2]["item_id"], json!("entry-1"));
        assert_eq!(writes[3]["item_id"], json!(rabbit.id.0));
        assert_eq!(writes[2]["deleted"], json!(true));
        assert_eq!(writes[3]["deleted"], json!(true));

        // The read path's next refresh stores the tombstones the service accepted. That device copy is
        // the one every member resolves, so the same rows are the proof of RAB-10's first scenario.
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![
                        CollectionItem {
                            item_id: ItemId(rabbit.id.0.clone()),
                            updated_at: Instant("2026-10-22T09:33:00Z".to_owned()),
                            deleted: true,
                            payload: Some(ItemPayload::Rabbit {
                                rabbit: rabbit.clone(),
                            }),
                        },
                        CollectionItem {
                            item_id: ItemId("entry-1".to_owned()),
                            updated_at: Instant("2026-10-22T09:33:00Z".to_owned()),
                            deleted: true,
                            payload: Some(ItemPayload::WeightEntry {
                                weight_entry: WeightEntry {
                                    id: WeightEntryId("entry-1".to_owned()),
                                    rabbit_id: rabbit.id.clone(),
                                    value_grams: Grams(1_800),
                                    timestamp: Instant("2026-10-21T09:00:00Z".to_owned()),
                                    note: None,
                                    photo_refs: Vec::new(),
                                    has_weight_drop_alert: false,
                                },
                            }),
                        },
                    ],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:34:00Z".to_owned()),
            )
            .expect("the collection is the household's own");

        for include_archived in [false, true] {
            assert!(
                block_on_ready(listed_rabbits(&state, one.clone(), include_archived))
                    .expect("the list resolves")
                    .is_empty(),
                "the rabbit is in neither list ({include_archived})"
            );
        }

        let held = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        let entry = held
            .items
            .iter()
            .find(|item| item.item_id == ItemId("entry-1".to_owned()))
            .expect("the entry's row is still held");
        assert!(
            entry.deleted,
            "the entry is a tombstone, so no read describes it as retrievable"
        );
    }

    /// RAB-10's second scenario: an active rabbit is refused, and the refusal writes nothing, probes
    /// nothing and publishes nothing.
    #[test]
    fn an_active_rabbit_cannot_be_permanently_deleted_and_nothing_is_written_or_published() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingRabbitPublisher::default();
        let request_id = request_id("operation-3");
        let calls_before = service.calls();

        let error = block_on_ready(deleted_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id.clone(),
        ))
        .expect_err("an active rabbit cannot be permanently deleted");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );

        // The rabbit is still active, and still the row the active list resolves.
        let one = household("household-1");
        assert_eq!(
            block_on_ready(listed_rabbits(&state, one.clone(), true)).expect("the list resolves"),
            vec![],
            "it was not moved to the archived list either"
        );
        assert_eq!(
            block_on_ready(listed_rabbits(&state, one, false)).expect("the list resolves"),
            vec![RabbitSummary {
                id: rabbit.id.clone(),
                name: rabbit.name.clone(),
                thumbnail_ref: rabbit.photo_ref.clone(),
                is_archived: false,
            }],
            "the rabbit remains active"
        );
    }

    /// D1.16: a live row whose content is unknown could be one of the rabbit's entries, so the
    /// deletion refuses with its own key, sends nothing and publishes nothing.
    #[test]
    fn a_deletion_of_a_row_with_unknown_content_is_refused_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit(service.clone());
        let one = household("household-1");
        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![
                        rabbit_row(&rabbit, "2026-10-22T09:30:00Z"),
                        CollectionItem {
                            item_id: ItemId("entry-1".to_owned()),
                            updated_at: Instant("2026-10-22T09:31:00Z".to_owned()),
                            deleted: false,
                            payload: None,
                        },
                    ],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                Instant("2026-10-22T09:32:00Z".to_owned()),
            )
            .expect("the collection is the household's own");
        let publisher = RecordingRabbitPublisher::default();
        let request_id = request_id("operation-3");
        let calls_before = service.calls();

        let error = block_on_ready(deleted_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id.clone(),
        ))
        .expect_err("a row that cannot be described cannot be removed blind");

        assert_eq!(error.cause, SaveCause::Unknown);
        assert!(!error.retryable);
        assert_eq!(
            error.message_key,
            crate::ipc::failure::HOUSEHOLD_CONTENT_UNLOADED_MESSAGE_KEY
        );
        assert_ne!(error.message_key, "error.unknown");
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a rabbit change"
        );
    }

    /// RAB-10's sharing scenario: the deletion is one set of tombstones in the household's **single**
    /// collection, naming the household and the writing member — not one removal per member — so every
    /// member resolves the same household without the rabbit or its entry (HS-8).
    #[test]
    fn the_deletion_is_one_set_of_tombstones_in_the_households_shared_collection() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit_and_an_entry(service.clone());
        let publisher = RecordingRabbitPublisher::default();

        block_on_ready(deleted_rabbit(
            &state,
            &publisher,
            rabbit.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the deletion");

        let writes: Vec<serde_json::Value> = service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                crate::sync::double::RecordedCall::Save { request, .. } => {
                    Some(request.body().clone())
                }
                _ => None,
            })
            .collect();

        // The create plus exactly one tombstone per deleted item: two writes for a rabbit and its
        // entry, into the household's one collection, with the same writing member.
        assert_eq!(writes.len(), 4);
        for body in &writes[2..] {
            assert_eq!(body["household_id"], json!("household-1"));
            assert_eq!(body["writer_id"], json!("member-1"));
            assert_eq!(body["deleted"], json!(true));
        }
        assert_eq!(writes[2]["item_id"], json!("entry-1"));
        assert_eq!(writes[3]["item_id"], json!(rabbit.id.0));

        // The device's own copy belongs to the read path: the write must not change it, or the removal
        // would exist twice — once locally and once in the collection every member reads (SYNC-3).
        let held = state
            .read_collection(&household("household-1"), CollectionSource::Local)
            .expect("the copy is held");
        assert!(
            held.items.iter().all(|item| !item.deleted),
            "the deletion reached the service, not the local copy"
        );
    }
}
