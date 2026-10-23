//! Weight-entry commands: recording, reading and correcting one rabbit's weights (WT-1 … WT-8,
//! DM-1).
//!
//! Five commands, frozen by task 1.1: [`create_weight_entry`], [`get_weight_entries`],
//! [`get_weight_entry`], [`update_weight_entry`] and [`delete_weight_entry`].
//! **[`create_weight_entry`] (task 2.9), [`update_weight_entry`] and [`delete_weight_entry`] (task
//! 2.11) and [`get_weight_entries`] (task 2.12) are implemented; [`get_weight_entry`] is a typed stub
//! returning [`CommandError::not_implemented`]**, which the task that owns it replaces (2.15) — its
//! signature is what `src/bindings.ts` is generated from, so a stub is the contract rather than a
//! placeholder to be reshaped.
//!
//! The writing bodies take the app's state and, as every write does, a publication seam —
//! `tauri::State` is how a command reaches the shared service, the account and this device's copy, and
//! `tauri::AppHandle` is the seam [`WeightChangePublisher`] is implemented by — which are parameters
//! Tauri supplies and `src/bindings.ts` does not export, so the command the frontend calls keeps the
//! arguments it was frozen with. A successful write publishes `weight:changed` with its own kind —
//! `created`, `updated` or `deleted` — and a refusal publishes nothing. The event names only the entry
//! that changed because the alerts around it are re-derived by the read that follows the invalidation
//! (D1.13), not repaired by the write.
//!
//! The create body records the value in canonical grams exactly as the draft carries it; the edit body
//! replaces the whole editable set and keeps the stored instant when the draft omits the timestamp, so
//! a back-dated entry is never restamped (WT-5). WT-4's value rules — a value of zero or less, and an
//! unconfirmed value above the plausibility bound — are applied where the draft becomes a record, by
//! `crate::domain::weight::validate_value`, so create and edit share one rule rather than two. The
//! bound is a confirmation threshold, not a rejection (D1.9): a confirmed outlier is stored at its
//! true value.
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
//! Every body below is a thin adapter over [`crate::state::AppState`]: it forwards the frozen
//! arguments, publishes the one event a successful write produces, and returns the state's answer
//! unchanged. A read stub still consumes its arguments deliberately: the parameters are part of the
//! frozen signature and the bindings' argument names, and an unused-parameter warning under
//! `clippy -D warnings` would otherwise have to be silenced.

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

/// The `update_weight_entry` body over the state and a publication seam.
///
/// It is the outermost layer a unit test can call: `tauri::State` has no public constructor outside a
/// running app (the same reason `commands::rabbit`'s bodies exist). The change is published only after
/// the edit succeeded, so a refusal leaves the recorder empty and the frontend never invalidates a
/// rabbit's weight surfaces for an entry the service did not accept (WT-5).
async fn revised_weight_entry(
    state: &crate::state::AppState,
    publisher: &impl WeightChangePublisher,
    entry_id: WeightEntryId,
    entry: WeightEntryDraft,
    request_id: RequestId,
) -> Result<WeightEntry, CommandError> {
    let (household_id, revised) = state
        .update_weight_entry(entry_id, entry, request_id)
        .await?;

    publisher.publish(&WeightChanged {
        household_id,
        rabbit_id: revised.rabbit_id.clone(),
        entry_id: revised.id.clone(),
        kind: WeightChangeKind::Updated,
    });

    Ok(revised)
}

/// The `delete_weight_entry` body over the state and a publication seam.
///
/// The deletion publishes `deleted` with the entry id it was addressed with; the state returns the
/// rabbit the entry belonged to beside the household, because the removal itself has no value to
/// return (the contract's answer is `()`).
async fn deleted_weight_entry(
    state: &crate::state::AppState,
    publisher: &impl WeightChangePublisher,
    entry_id: WeightEntryId,
    request_id: RequestId,
) -> Result<(), CommandError> {
    let (household_id, rabbit_id) = state
        .delete_weight_entry(entry_id.clone(), request_id)
        .await?;

    publisher.publish(&WeightChanged {
        household_id,
        rabbit_id,
        entry_id,
        kind: WeightChangeKind::Deleted,
    });

    Ok(())
}

/// Record a weight entry for a rabbit (WT-1, WT-3, WT-4, DM-1).
///
/// The value arrives in canonical grams and is stored as such; the rabbit's display unit plays no part
/// here. `entry.timestamp` is `None` to record the moment of creation, or a supplied absolute instant —
/// including one in the past — stored unchanged (WT-1, DM-6). The note is optional. Beside the
/// timestamp the entry records its own `created_at`, the operation's creation instant, even when the
/// person supplied a reading moment: that is the ordering key WT-7's tie rule uses, and no draft field
/// can set or move it (D1.12).
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

/// The `get_weight_entries` body over the state alone.
///
/// It reads this device's copy and never the service, exactly as
/// [`list_rabbits`](super::rabbit::list_rabbits) does, and reads the clock once at the edge for the
/// window's upper edge (`now`); the lower edge is `now - N days`. Nothing is cached per range, so two
/// calls with different ranges over the same rabbit each compute their own window (WT-6, D1.4).
async fn read_weight_trend(
    state: &crate::state::AppState,
    rabbit_id: RabbitId,
    range: WeightRange,
) -> Result<WeightTrend, CommandError> {
    state.weight_trend(
        &rabbit_id,
        range,
        &crate::storage::AbsoluteInstant::now().to_wire(),
    )
}

/// One rabbit's weight series for one window, with the threshold it is judged against (WT-6, WT-7).
///
/// This one read serves **both** ranged surfaces: the rabbit dashboard's trend chart and the
/// weight-only history listing. They hold their ranges independently in the route's typed search
/// parameters and each calls this command with its own [`WeightRange`], so one cannot influence the
/// other (D1.4, WT-6) — the backend holds no selected range.
///
/// A bounded range is `[now - N days, now]`, both edges inclusive; `all` is the unbounded full
/// history (WT-6, D1.4). The window limits which points come back, **not** how the flags are derived:
/// the alert is computed over the rabbit's full entry set, so the earliest point of a window may be
/// flagged on the basis of an entry outside it (D1.13, I3). Points are ordered oldest first by the one
/// comparator, `(timestamp, created_at, id)`, the order the previous-entry derivation uses (D1.12).
///
/// An archived rabbit is readable: RAB-8 presents its timeline read-only, so the read does not refuse
/// an archived target. Unreachability is a status fact, not a read failure (SYNC-3).
///
/// # Errors
///
/// `validation` for an unknown rabbit, a blank id, or an id that names a non-rabbit item. `unknown`
/// with the unknown-content key when the rabbit's row or a live row of its collection has no content
/// this device can describe, and `unknown` for a local store that cannot answer (D1.16).
#[tauri::command]
#[specta::specta]
pub async fn get_weight_entries(
    rabbit_id: RabbitId,
    range: WeightRange,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<WeightTrend, CommandError> {
    read_weight_trend(state.inner(), rabbit_id, range).await
}

/// One entry's detail: value, timestamp, full note, derived alert and photo references (WT-8).
///
/// The note is returned whole and never truncated by the contract; the alert flag is derived on read
/// exactly as it is for the trend and the dashboard (D1.3), so the three surfaces cannot disagree. An
/// entry of an archived rabbit is readable — RAB-8 makes the timeline viewable and read-only — and the
/// frontend withholds Edit and Delete from what this command returns rather than the read refusing.
///
/// Still a stub (task 2.15): its body must call `crate::domain::weight::alerts` over the rabbit's full
/// entry set, never re-derive the rule.
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
/// unconfirmed outlier refused — and the change is shared with every member (WT-5, HS-8). The
/// timestamp is the supplied moment, or the entry's stored instant when omitted, so an edit never
/// restamps a back-dated entry.
///
/// Editing a value is an **alert-affecting** operation, not a value correction only: the edited entry's
/// flag and the flag of the entry that follows it are re-derived from the current entry set
/// (D1.13, WT-7). That re-derivation is the read's (tasks 2.13–2.15); this command changes one stored
/// row and writes its `has_weight_drop_alert` as `false` (D1.16).
///
/// # Errors
///
/// `validation` for a value of zero or less, an unconfirmed outlier, an entry of an **archived** rabbit
/// (WT-5, DM-4), an unknown or tombstoned entry, an id that names another kind of item, a blank id or a
/// blank operation identity. `unknown` with the unknown-content key when the entry's or its rabbit's
/// content is not on this device (D1.16). The write path's service conditions are reachable as for
/// [`create_weight_entry`].
#[tauri::command]
#[specta::specta]
pub async fn update_weight_entry(
    entry_id: WeightEntryId,
    entry: WeightEntryDraft,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<WeightEntry, CommandError> {
    revised_weight_entry(state.inner(), &app, entry_id, entry, request_id).await
}

/// Delete a past entry, sharing the removal with every member (WT-5, WT-8).
///
/// Deleting is also **alert-affecting**: the entry that followed the deleted one is afterwards compared
/// against the entry that now precedes it, so its flag can change although it was never edited
/// (D1.13, WT-7). That re-derivation is the read's (tasks 2.13–2.15); this command writes one
/// tombstone and never rewrites a neighbouring entry. The confirmation that precedes the call is the
/// frontend's (NFR-9, WT-8); this command is called only after it.
///
/// # Errors
///
/// `validation` for an entry of an **archived** rabbit (WT-5, DM-4), an unknown or tombstoned entry, an
/// id that names another kind of item, a blank id or a blank operation identity. `unknown` with the
/// unknown-content key when the entry's or its rabbit's content is not on this device (D1.16). The
/// write path's service conditions are reachable as for [`create_weight_entry`]; a removal the service
/// did not accept leaves the entry in place and is never reported as deleted.
#[tauri::command]
#[specta::specta]
pub async fn delete_weight_entry(
    entry_id: WeightEntryId,
    request_id: RequestId,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<(), CommandError> {
    deleted_weight_entry(state.inner(), &app, entry_id, request_id).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::ipc::events::test_support::RecordingWeightPublisher;
    use crate::ipc::types::{
        AccountUserId, CollectionItem, CollectionSource, Grams, HouseholdCollection, ItemId,
        ItemPayload, NeuterStatus, PLAUSIBILITY_BOUND_GRAMS, Rabbit, Sex,
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

    /// WT-4, "A zero value is rejected": the refusal is `validation`, carries the operation's identity,
    /// and neither writes nor publishes anything, so no entry is left for the person to see.
    #[test]
    fn a_zero_value_is_refused_and_writes_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let request_id = request_id("operation-3");

        let error = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            WeightEntryDraft {
                value_grams: Grams(0),
                ..draft()
            },
            request_id.clone(),
        ))
        .expect_err("a zero value names no weight");

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

    /// WT-4, "A negative value is rejected": `validation`, the operation's identity, and nothing
    /// written or published.
    #[test]
    fn a_negative_value_is_refused_and_writes_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let request_id = request_id("operation-3");

        let error = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            WeightEntryDraft {
                value_grams: Grams(-250),
                ..draft()
            },
            request_id.clone(),
        ))
        .expect_err("a negative value names no weight");

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

    /// WT-4, "A value above the plausibility bound requires confirmation": one gram above the bound is
    /// refused while the draft is unconfirmed — the bound is inclusive, so this is the first value that
    /// needs confirming — and nothing is written or published.
    #[test]
    fn one_gram_above_the_bound_is_refused_until_confirmed_and_writes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let request_id = request_id("operation-3");

        let error = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            WeightEntryDraft {
                value_grams: Grams(PLAUSIBILITY_BOUND_GRAMS.0 + 1),
                outlier_confirmed: false,
                ..draft()
            },
            request_id.clone(),
        ))
        .expect_err("an unconfirmed outlier is not saved");

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

    /// WT-4, D1.9: the bound itself is plausible — exactly 10 kg is saved without confirmation, which
    /// fixes the bound on the accepted side rather than the rejected one.
    #[test]
    fn a_value_exactly_at_the_bound_is_saved_without_confirmation() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();

        let created = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            WeightEntryDraft {
                value_grams: PLAUSIBILITY_BOUND_GRAMS,
                outlier_confirmed: false,
                ..draft()
            },
            request_id("operation-3"),
        ))
        .expect("the bound itself needs no confirmation");

        assert_eq!(created.value_grams, PLAUSIBILITY_BOUND_GRAMS);
        let body = write_bodies(&service)
            .pop()
            .expect("the entry write reached the transport");
        assert_eq!(
            body["payload"]["weight_entry"]["value_grams"],
            json!(PLAUSIBILITY_BOUND_GRAMS.0)
        );
        assert_eq!(
            publisher.published().len(),
            1,
            "the saved entry is reported exactly once"
        );
    }

    /// WT-4, "A confirmed outlier is saved": the person's confirmation lets the entry through at its
    /// true value — it is stored as submitted and never clamped to the bound or otherwise rewritten.
    #[test]
    fn a_confirmed_outlier_is_saved_with_its_true_value() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit) = state_with_one_rabbit(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let outlier = Grams(30_000);

        let created = block_on_ready(created_weight_entry(
            &state,
            &publisher,
            rabbit.id.clone(),
            WeightEntryDraft {
                value_grams: outlier,
                outlier_confirmed: true,
                ..draft()
            },
            request_id("operation-3"),
        ))
        .expect("a confirmed outlier is saved");

        assert_eq!(created.value_grams, outlier, "the value is never clamped");
        let body = write_bodies(&service)
            .pop()
            .expect("the entry write reached the transport");
        assert_eq!(
            body["payload"]["weight_entry"]["value_grams"],
            json!(outlier.0),
            "the stored payload carries the confirmed value itself"
        );
        assert_eq!(
            publisher.published().len(),
            1,
            "the saved entry is reported exactly once"
        );
    }

    /// One past entry of `rabbit`, at the value and instant given (WT-5).
    fn stored_entry(rabbit: &Rabbit, id: &str, value: i32, timestamp: &str) -> WeightEntry {
        WeightEntry {
            id: WeightEntryId(id.to_owned()),
            rabbit_id: rabbit.id.clone(),
            value_grams: Grams(value),
            timestamp: at(timestamp),
            created_at: at(timestamp),
            note: Some("morning".to_owned()),
            photo_refs: Vec::new(),
            has_weight_drop_alert: false,
        }
    }

    /// One entry row carrying the entry's content, as the read path delivers it (D1.16).
    fn entry_row(entry: &WeightEntry) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(entry.id.0.clone()),
            updated_at: entry.timestamp.clone(),
            deleted: false,
            payload: Some(ItemPayload::WeightEntry {
                weight_entry: entry.clone(),
            }),
        }
    }

    /// A state whose copy holds one active rabbit and two past entries — the state an edit or a
    /// deletion starts from, since both resolve the entry first and then check its rabbit (WT-5).
    fn state_with_one_rabbit_and_entries(
        service: Arc<ScriptedService>,
    ) -> (AppState, Rabbit, WeightEntry) {
        let (state, rabbit) = state_with_one_rabbit(service);
        let one = household("household-1");
        let entry = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-21T09:00:00Z");

        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![
                        rabbit_row(&rabbit),
                        entry_row(&entry),
                        entry_row(&stored_entry(
                            &rabbit,
                            "entry-2",
                            1_700,
                            "2026-10-22T09:00:00Z",
                        )),
                    ],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (state, rabbit, entry)
    }

    /// The same copy with the rabbit **archived** — the target the entry writes must refuse (DM-4).
    fn state_with_an_archived_rabbit_and_entries(
        service: Arc<ScriptedService>,
    ) -> (AppState, Rabbit, WeightEntry) {
        let (state, rabbit) = state_with_one_rabbit(service);
        let one = household("household-1");
        let rabbit = Rabbit {
            is_archived: true,
            ..rabbit
        };
        let entry = stored_entry(&rabbit, "entry-1", 1_800, "2026-10-21T09:00:00Z");

        state
            .load_collection(
                &one,
                HouseholdCollection {
                    household_id: one.clone(),
                    items: vec![rabbit_row(&rabbit), entry_row(&entry)],
                    source: CollectionSource::Service,
                    loaded_at: None,
                },
                at("2026-10-22T09:31:00Z"),
            )
            .expect("the collection is the household's own");

        (state, rabbit, entry)
    }

    /// WT-5, WT-7: a saved edit replaces the whole editable set of the entry's existing item — value,
    /// note, and the stored instant when the draft omits the timestamp — and publishes exactly one
    /// `updated` for that entry.
    #[test]
    fn a_successful_edit_publishes_exactly_one_updated_for_the_entry() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit, entry) = state_with_one_rabbit_and_entries(service.clone());
        let publisher = RecordingWeightPublisher::default();

        let edited = block_on_ready(revised_weight_entry(
            &state,
            &publisher,
            entry.id.clone(),
            WeightEntryDraft {
                value_grams: Grams(1_500),
                timestamp: None,
                note: Some("corrected".to_owned()),
                outlier_confirmed: false,
            },
            request_id("operation-3"),
        ))
        .expect("the double accepted the edit");

        assert_eq!(edited.id, entry.id);
        assert_eq!(edited.rabbit_id, rabbit.id);
        assert_eq!(edited.value_grams, Grams(1_500));
        assert_eq!(edited.note.as_deref(), Some("corrected"));
        assert_eq!(
            edited.timestamp, entry.timestamp,
            "an omitted timestamp keeps the stored instant"
        );

        // One item write of the entry's existing row, in the household that holds it.
        let body = write_bodies(&service)
            .pop()
            .expect("the edit write reached the transport");
        assert_eq!(body["household_id"], json!("household-1"));
        assert_eq!(body["item_id"], json!(entry.id.0));
        assert_eq!(body["payload"]["kind"], "weight_entry");
        assert_eq!(body["payload"]["weight_entry"]["value_grams"], json!(1_500));
        assert_eq!(
            body["payload"]["weight_entry"]["timestamp"],
            json!(entry.timestamp.0)
        );
        assert_eq!(
            body["payload"]["weight_entry"]["has_weight_drop_alert"],
            json!(false),
            "the derived alert is never carried in (D1.16)"
        );

        assert_eq!(
            publisher.published(),
            vec![WeightChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                entry_id: entry.id,
                kind: WeightChangeKind::Updated,
            }],
            "one report, with the edit's kind, for the entry the command returned"
        );
    }

    /// WT-5: a saved deletion writes the entry's item as a tombstone — content retained, removal flag
    /// set — and publishes exactly one `deleted`.
    #[test]
    fn a_successful_delete_publishes_exactly_one_deleted_for_the_entry() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit, entry) = state_with_one_rabbit_and_entries(service.clone());
        let publisher = RecordingWeightPublisher::default();

        block_on_ready(deleted_weight_entry(
            &state,
            &publisher,
            entry.id.clone(),
            request_id("operation-3"),
        ))
        .expect("the double accepted the deletion");

        let body = write_bodies(&service)
            .pop()
            .expect("the deletion write reached the transport");
        assert_eq!(body["household_id"], json!("household-1"));
        assert_eq!(body["item_id"], json!(entry.id.0));
        assert_eq!(body["deleted"], json!(true));
        assert_eq!(body["payload"]["kind"], "weight_entry");
        assert_eq!(
            body["payload"]["weight_entry"]["value_grams"],
            json!(1_800),
            "the removing version retains the entry's content (D1.17)"
        );

        assert_eq!(
            publisher.published(),
            vec![WeightChanged {
                household_id: household("household-1"),
                rabbit_id: rabbit.id,
                entry_id: entry.id,
                kind: WeightChangeKind::Deleted,
            }],
            "one report, with the deletion's kind, for the entry that was addressed"
        );
    }

    /// DM-4, RAB-8, WT-5: an archived rabbit's entry cannot be edited. The refusal is `validation`,
    /// carries the operation's identity, and neither writes nor publishes anything (D1.5).
    #[test]
    fn an_edit_for_an_archived_rabbits_entry_is_refused_and_writes_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, _rabbit, entry) = state_with_an_archived_rabbit_and_entries(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let operation = request_id("operation-3");

        let error = block_on_ready(revised_weight_entry(
            &state,
            &publisher,
            entry.id.clone(),
            draft(),
            operation.clone(),
        ))
        .expect_err("an archived rabbit's entries are read-only");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a weight change"
        );
    }

    /// DM-4, RAB-8, WT-5: an archived rabbit's entry cannot be deleted either, with the same refusal
    /// and nothing written or published.
    #[test]
    fn a_delete_for_an_archived_rabbits_entry_is_refused_and_writes_and_publishes_nothing() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, _rabbit, entry) = state_with_an_archived_rabbit_and_entries(service.clone());
        let publisher = RecordingWeightPublisher::default();
        let calls_before = service.calls();
        let operation = request_id("operation-3");

        let error = block_on_ready(deleted_weight_entry(
            &state,
            &publisher,
            entry.id.clone(),
            operation.clone(),
        ))
        .expect_err("an archived rabbit's entries are read-only");

        assert_eq!(error.cause, SaveCause::Validation);
        assert_eq!(error.message_key, "error.validation");
        assert!(!error.retryable);
        assert_eq!(error.request_id, Some(operation));
        assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
        assert!(
            publisher.published().is_empty(),
            "a refusal never reports a weight change"
        );
    }

    /// WT-6: the command body itself serves the held copy and answers the series — the adapter is
    /// exercised, not only `AppState::weight_trend`, and `all` needs no injected clock.
    #[test]
    fn the_command_body_answers_the_rabbits_full_series() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
        let (state, rabbit, _entry) = state_with_one_rabbit_and_entries(service);

        let trend = block_on_ready(read_weight_trend(
            &state,
            rabbit.id.clone(),
            WeightRange::All,
        ))
        .expect("the held copy is served");

        assert_eq!(trend.range, WeightRange::All);
        assert_eq!(trend.drop_threshold_percent, 5.0);
        assert_eq!(
            trend
                .points
                .iter()
                .map(|point| (point.id.0.as_str(), point.value_grams))
                .collect::<Vec<_>>(),
            vec![("entry-1", Grams(1_800)), ("entry-2", Grams(1_700)),],
            "oldest first, with the stored values"
        );
    }
}
