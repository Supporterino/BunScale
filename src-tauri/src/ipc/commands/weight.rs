//! Weight-entry commands: recording, reading and correcting one rabbit's weights (WT-1 … WT-8,
//! DM-1).
//!
//! Five commands, frozen by task 1.1: [`create_weight_entry`], [`get_weight_entries`],
//! [`get_weight_entry`], [`update_weight_entry`] and [`delete_weight_entry`]. **[`create_weight_entry`]
//! is implemented (task 2.9); the other four are typed stubs returning
//! [`CommandError::not_implemented`]**, which section 2 replaces in the tasks that own them — their
//! signatures are what `src/bindings.ts` is generated from, so a stub is the contract rather than a
//! placeholder to be reshaped.
//!
//! The create body takes the app's state and, as every write does, a publication seam —
//! `tauri::State` is how a command reaches the shared service, the account and this device's copy, and
//! `tauri::AppHandle` is the seam [`WeightChangePublisher`] is implemented by — which are parameters
//! Tauri supplies and `src/bindings.ts` does not export, so the command the frontend calls keeps the
//! arguments it was frozen with. A successful create publishes `weight:changed` with kind `created`; a
//! refusal publishes nothing. The weight-entry edit and deletion of task 2.11 publish their own kinds
//! through the same seam when their bodies land.
//!
//! The create body records the value in canonical grams exactly as the draft carries it. WT-4's value
//! rules — a value of zero or less, and an unconfirmed value above the plausibility bound — are task
//! 2.10's and are deliberately not applied here yet.
//!
//! **Everything on this wire is grams.** No command accepts a per-entry unit, no type here carries a
//! unit field, and the display unit never appears in a payload (D1.2, DM-1). The conversion between
//! the person's preferred unit and grams lives in `src/features/weight/api.ts`, at the entry boundary
//! — see the design's IPC section for the decision and its reasoning. A consequence worth keeping in
//! mind while implementing section 2: because the unit is applied outside these commands, changing the
//! display unit cannot rewrite a stored value, and a value the person typed cannot arrive here
//! unparsed or in an ambiguous unit.
//!
//! Two rules the signatures already fix:
//!
//! * **An archived rabbit's entries are immutable at the boundary** (D1.5, RAB-8, DM-4). Create,
//!   update and delete all refuse it, and the refusal is `validation` — P0's cause list is frozen and
//!   has no "immutable target" variant, and a retry cannot clear the refusal.
//! * **The drop alert is derived, never stored** (D1.3, D1.13, WT-7). It appears only on the response
//!   types ([`WeightEntry::has_weight_drop_alert`], [`WeightEntrySummary::has_weight_drop_alert`]) and
//!   is recomputed on every read from the rabbit's current threshold and entry set, so editing or
//!   deleting an entry re-evaluates the entries around it without a stored flag to repair.
//!
//! Every body below is a stub, so each one consumes its arguments deliberately: the parameters are
//! part of the frozen signature and the bindings' argument names, and an unused-parameter warning
//! under `clippy -D warnings` would otherwise have to be silenced.

use super::super::events::{WeightChangeKind, WeightChangePublisher, WeightChanged};
use super::super::types::*;

/// The `create_weight_entry` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call: `tauri::State` has no public constructor outside a
/// running app (the same reason `commands::rabbit`'s bodies exist). The household comes back from the
/// state because the entry itself names only its rabbit, and `weight:changed` is scoped by household;
/// the change is published only after the create succeeded, so a refusal leaves the recorder empty and
/// the frontend never invalidates a rabbit's weight surfaces for an entry the service did not accept.
async fn created_weight_entry(
    state: &crate::state::AppState,
    publisher: &impl WeightChangePublisher,
    rabbit_id: RabbitId,
    entry: WeightEntryDraft,
    request_id: RequestId,
) -> Result<WeightEntry, CommandError> {
    let (household_id, created) = state
        .create_weight_entry(rabbit_id, entry, request_id)
        .await?;

    publisher.publish(&WeightChanged {
        household_id,
        rabbit_id: created.rabbit_id.clone(),
        entry_id: created.id.clone(),
        kind: WeightChangeKind::Created,
    });

    Ok(created)
}

/// Record a weight entry for a rabbit (WT-1, WT-3, WT-4, DM-1).
///
/// The value arrives in canonical grams and is stored as such; the rabbit's display unit plays no part
/// here. `entry.timestamp` is `None` to record the moment of creation, or a supplied absolute instant —
/// including one in the past — stored unchanged (WT-1, DM-6). The note is optional.
///
/// A value of zero or less is refused. A value above [`PLAUSIBILITY_BOUND_GRAMS`] is a *confirmation*
/// threshold rather than a rejection (D1.9): it is refused until `entry.outlier_confirmed` says the
/// person confirmed it, so a genuinely heavy rabbit is still recorded and an accidental extra digit is
/// still stopped at the boundary rather than only in the form.
///
/// # Errors
///
/// `validation` for a value of zero or less (WT-4), for an unconfirmed value above the plausibility
/// bound (WT-4), for an archived rabbit, unknown rabbit, blank id or blank operation identity. The
/// write path's service conditions — `connectivity`, `no_account`, `allowance_exceeded`,
/// `rate_limited`, `timeout` — are reachable because an entry is a shared write (D1.1), and `unknown`
/// is the honest remainder. No failure is ever presented as a saved entry.
#[tauri::command]
#[specta::specta]
pub async fn create_weight_entry(
    rabbit_id: RabbitId,
    entry: WeightEntryDraft,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<WeightEntry, CommandError> {
    created_weight_entry(state.inner(), &app, rabbit_id, entry, request_id).await
}

/// One rabbit's weight series for one window, with the threshold it is judged against (WT-6, WT-7).
///
/// This one read serves **both** ranged surfaces: the rabbit dashboard's trend chart and the
/// weight-only history listing. They hold their ranges independently in the route's typed search
/// parameters and each calls this command with its own [`WeightRange`], so one cannot influence the
/// other (D1.4, WT-6) — the backend holds no selected range.
///
/// The window limits which points come back, **not** how the flags are derived: the alert is computed
/// over the rabbit's full entry set, so the earliest point of a window may be flagged on the basis of
/// an entry outside it (D1.13). Points are ordered oldest first, the order the previous-entry
/// derivation uses (D1.12).
///
/// # Errors
///
/// `validation` for an unknown rabbit or a blank id. Unreachability is a status fact, not a read
/// failure (SYNC-3); a local store that cannot answer is `unknown`.
#[tauri::command]
#[specta::specta]
pub async fn get_weight_entries(
    rabbit_id: RabbitId,
    range: WeightRange,
) -> Result<WeightTrend, CommandError> {
    let _ = (rabbit_id, range);

    Err(CommandError::not_implemented(None))
}

/// One entry's detail: value, timestamp, full note, derived alert and photo references (WT-8).
///
/// The note is returned whole and never truncated by the contract; the alert flag is derived on read
/// exactly as it is for the trend and the dashboard (D1.3), so the three surfaces cannot disagree. An
/// entry of an archived rabbit is readable — RAB-8 makes the timeline viewable and read-only — and the
/// frontend withholds Edit and Delete from what this command returns rather than the read refusing.
///
/// # Errors
///
/// `validation` for an unknown entry or a blank id. Unreachability is a status fact, not a read
/// failure (SYNC-3).
#[tauri::command]
#[specta::specta]
pub async fn get_weight_entry(entry_id: WeightEntryId) -> Result<WeightEntry, CommandError> {
    let _ = entry_id;

    Err(CommandError::not_implemented(None))
}

/// Correct a past entry, sharing the change with every member (WT-5, WT-7).
///
/// The whole editable set is replaced through [`WeightEntryDraft`]: a patch would need a double-optional
/// per field to distinguish "unchanged" from "cleared", and the edit form is pre-filled anyway. The
/// same validation as [`create_weight_entry`] applies — grams only, zero and negative refused, an
/// unconfirmed outlier refused — and the change is shared with every member (WT-5, HS-8).
///
/// Editing a value is an **alert-affecting** operation, not a value correction only: the edited entry's
/// flag and the flag of the entry that follows it are re-derived from the current entry set
/// (D1.13, WT-7).
///
/// # Errors
///
/// `validation` for a value of zero or less, an unconfirmed outlier, an entry of an **archived** rabbit
/// (WT-5, DM-4), an unknown entry, a blank id or a blank operation identity. The write path's service
/// conditions are reachable as for [`create_weight_entry`].
#[tauri::command]
#[specta::specta]
pub async fn update_weight_entry(
    entry_id: WeightEntryId,
    entry: WeightEntryDraft,
    request_id: RequestId,
) -> Result<WeightEntry, CommandError> {
    let _ = (entry_id, entry);

    Err(CommandError::not_implemented(Some(request_id)))
}

/// Delete a past entry, sharing the removal with every member (WT-5, WT-8).
///
/// Deleting is also **alert-affecting**: the entry that followed the deleted one is afterwards compared
/// against the entry that now precedes it, so its flag can change although it was never edited
/// (D1.13, WT-7). The confirmation that precedes the call is the frontend's (NFR-9, WT-8); this
/// command is called only after it.
///
/// # Errors
///
/// `validation` for an entry of an **archived** rabbit (WT-5, DM-4), an unknown entry, a blank id or a
/// blank operation identity. The write path's service conditions are reachable as for
/// [`create_weight_entry`]; a removal the service did not accept leaves the entry in place and is never
/// reported as deleted.
#[tauri::command]
#[specta::specta]
pub async fn delete_weight_entry(
    entry_id: WeightEntryId,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let _ = entry_id;

    Err(CommandError::not_implemented(Some(request_id)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::ipc::events::test_support::RecordingWeightPublisher;
    use crate::ipc::types::{
        AccountUserId, CollectionItem, CollectionSource, Grams, HouseholdCollection, ItemId,
        ItemPayload, NeuterStatus, Rabbit, Sex,
    };
    use crate::state::AppState;
    use crate::storage::AbsoluteInstant;
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

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    /// A RAB-1 profile supplying every field, so the rabbit copy below can hold one.
    fn profile() -> RabbitProfile {
        RabbitProfile {
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: Some("Mini Lop".to_owned()),
            dob: Some(at("2026-03-01T00:00:00Z")),
            dob_unknown: false,
            sex: Sex::Female,
            neuter_status: NeuterStatus::Spayed,
            adoption_date: None,
            weight_drop_threshold_percent: 5.0,
        }
    }

    /// A draft supplying only the value: no timestamp, no note, nothing confirmed.
    fn draft() -> WeightEntryDraft {
        WeightEntryDraft {
            value_grams: Grams(1_800),
            timestamp: None,
            note: None,
            outlier_confirmed: false,
        }
    }

    /// One accepted create answer the domain can read, so the book can hold a household to write in.
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

    /// A state whose book holds `household-1` with this account as its member, so a weight entry can
    /// name its writer. The double's last write answer is repeated, so the entry write is accepted by
    /// the same script.
    fn state_with_a_held_household(service: Arc<ScriptedService>) -> AppState {
        let state = AppState::new(service, Some(account()));

        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the household is created and held");

        state
    }

    /// One row carrying the rabbit's content, as the read path delivers it.
    fn rabbit_row(rabbit: &Rabbit) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(rabbit.id.0.clone()),
            updated_at: at("2026-10-22T09:30:00Z"),
            deleted: false,
            payload: Some(ItemPayload::Rabbit {
                rabbit: rabbit.clone(),
            }),
        }
    }

    /// A state whose book holds `household-1` and whose copy holds one **active** rabbit — the state a
    /// weight-entry create starts from, since the write boundary resolves the rabbit from the copy.
    fn state_with_one_rabbit(service: Arc<ScriptedService>) -> (AppState, Rabbit) {
        let state = state_with_a_held_household(service);
        let one = household("household-1");
        let rabbit =
            block_on_ready(state.create_rabbit(one.clone(), profile(), request_id("operation-2")))
                .expect("the rabbit is created");

        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&rabbit)],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (state, rabbit)
    }

    /// A state whose copy holds the same rabbit **archived** — the target a weight-entry create must
    /// refuse (DM-4, RAB-8).
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
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&archived)],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (state, archived)
    }

    /// Every write the transport was asked to submit, in call order.
    fn write_bodies(service: &ScriptedService) -> Vec<serde_json::Value> {
        service
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                crate::sync::double::RecordedCall::Save { request, .. } => {
                    Some(request.body().clone())
                }
                _ => None,
            })
            .collect()
    }

    /// WT-1 through the command: an omitted timestamp records the moment of creation, the note stays
    /// absent, the value is stored in grams, and only a successful create publishes `weight:changed`
    /// with kind `created`.
    #[test]
    fn a_successful_create_publishes_exactly_one_created_for_the_entry() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();

        let before = AbsoluteInstant::now();
        let created = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            draft(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the write");
        let after = AbsoluteInstant::now();

        assert_eq!(created.rabbit_id, rabbit.id);
        assert_eq!(created.value_grams, Grams(1_800));
        assert_eq!(created.note, None, "an omitted note is saved as no note");
        assert_eq!(
            created.id,
            WeightEntryId("operation-3".to_owned()),
            "the entry's identity is its create operation's"
        );

        // WT-1's scenario is that the timestamp **is** the moment of creation, so it is bounded by
        // the clock either side of the call — a hard-coded or stale value would fall outside.
        let recorded = AbsoluteInstant::parse(&created.timestamp)
            .expect("the defaulted timestamp is an absolute instant");
        assert!(
            before <= recorded && recorded <= after,
            "the timestamp is the moment of creation, not a fixed value: {recorded:?} outside {before:?}..={after:?}"
        );

        // The write is one item write of the entry, into the household that holds the rabbit.
        let body = write_bodies(&service)
            .pop()
            .expect("the entry write reached the transport");
        assert_eq!(body["household_id"], json!("household-1"));
        assert_eq!(body["item_id"], json!("operation-3"));
        assert_eq!(body["payload"]["kind"], "weight_entry");
        assert_eq!(body["payload"]["weight_entry"]["value_grams"], json!(1_800));
        assert_eq!(
            body["payload"]["weight_entry"]["timestamp"],
            json!(created.timestamp.0),
            "the stored timestamp is the one the command returned"
        );
        assert_eq!(
            body["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false),
            "the derived alert is never carried in"
        );

        assert_eq!(
            publisher.published(),
            vec![WeightChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                entry_id: created.id,
                kind: WeightChangeKind::Created,
            }],
            "one report, with the create's kind, for the entry the command returned"
        );
    }

    /// DM-4, RAB-8: an archived rabbit's weight data is not writable. The refusal comes from the shared
    /// write-boundary check, is `validation`, carries the operation's identity, and neither writes nor
    /// publishes anything (D1.5, WT-1's create is a content write).
    #[test]
    fn a_create_for_an_archived_rabbit_is_refused_and_writes_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_an_archived_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let request_id = request_id("operation-3");

        let error = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            draft(),
            request_id.clone(),
        ))
        .expect_err("an archived rabbit's weight data is read-only");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(request_id));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a weight change"
        );
    }
}
