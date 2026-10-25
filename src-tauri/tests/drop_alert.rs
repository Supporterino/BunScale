//! Integration tests for the weight-drop alert and the dashboard flag (task 5.5): WT-7's derivation
//! scenarios, D1.12's creation-order tie rule, D1.13's re-derivation around an edit and a deletion,
//! and the DASH-4 flag that shares the same derivation.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/weight-tracking/spec.md` and
//! `specs/dashboard/spec.md` state, and each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] methods the commands forward to — the whole behaviour of
//! the command apart from the wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `weight_entries.rs` (task 5.4) does. `app_lib::run()` is never called (it
//! starts the event loop) and `ipc::export_bindings()` is never called (D0.9), as
//! `library_surface.rs` records.
//!
//! `get_dashboard` has **no** [`AppState`] method to forward to: its body is the blank-id check, then
//! `ReadPath::shared().serve_local(state, household_id)`, then `domain::rabbit::dashboard`. The
//! `serve_local` half is crate-private, so [`dashboard_row`] composes the two public calls the body
//! composes — [`AppState::read_collection`] with `CollectionSource::Local` resolves the same held copy,
//! and [`app_lib::domain::rabbit::dashboard`] is the body's core — and reads the one row.
//!
//! The entries reach both this device and every other member by one route: an entry write is one
//! `save_item` into the household's shared collection, and this device's copy is loaded from those
//! accepted writes ([`refresh`]). No test reads a locally composed value: the flag is asserted from
//! the copies the read commands resolve.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # Fixed instants
//!
//! Every reading timestamp is a fixed past instant. The one clock read is the entry's `created_at`,
//! minted inside the create body and not injectable (D1.12, `AbsoluteInstant::now`); no test sleeps.
//! "Entries sharing a timestamp are ordered by creation" depends on that mint, so it asserts the
//! recorded creation order as the scenario's premise rather than assuming it.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: "presented with a visual drop warning" is asserted as the derived flag
//! the view renders, never as copy or markup. The derivation's arithmetic and the strict
//! "exceeds" boundary are unit-tested beside `domain::weight::alerts`; these tests prove the
//! threshold, the entry set and the dashboard row travel through the command path and cannot
//! disagree.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, DashboardRow, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId, Sex,
    WeightEntry, WeightEntryDraft, WeightEntryId, WeightRange, WeightTrend,
};
use app_lib::state::AppState;
use app_lib::storage::AbsoluteInstant;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{Answer, ServiceReply};

/// The one household these scenarios live in.
const HOUSEHOLD: &str = "household-1";

/// The instant the scripted service dates a created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded this device's copy.
const LOADED_AT: &str = "2026-10-25T09:31:00Z";

/// The operation that creates the rabbit every test starts from.
const CREATE_RABBIT: &str = "operation-2";

/// WT-7's documented default, in percent (RAB-1/RAB-5).
const DEFAULT_THRESHOLD_PERCENT: f64 = 5.0;

/// The read clock for the unbounded range: after every fixed entry instant, so `all` is the whole
/// history and the window never truncates the derivation.
const NOW: &str = "2026-10-25T12:00:00Z";

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

/// One accepted household create answer, so the book holds a household a rabbit can be created in.
fn accepted_household() -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({
            "outcome": "applied",
            "household": {
                "id": HOUSEHOLD,
                "owner_member_id": "member-1",
                "created_at": CREATED_AT,
            },
        }),
    ))
}

/// A rabbit profile supplying the RAB-1 field set, with the default 5% drop threshold.
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
        weight_drop_threshold_percent: DEFAULT_THRESHOLD_PERCENT,
    }
}

/// A draft supplying only the value: no timestamp, no note, nothing confirmed.
fn draft(value_grams: i32) -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(value_grams),
        timestamp: None,
        note: None,
        outlier_confirmed: false,
    }
}

/// The same draft with the person's own reading instant, which may be in the past.
fn draft_at(value_grams: i32, timestamp: &str) -> WeightEntryDraft {
    WeightEntryDraft {
        timestamp: Some(at(timestamp)),
        ..draft(value_grams)
    }
}

/// A state whose book holds the household with this account as its member, so a write can name its
/// writer. The double repeats its last write answer, so every later item write is accepted.
fn held_household() -> (Arc<ScriptedService>, AppState) {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let state = AppState::new(service.clone(), Some(account()));

    block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
        .expect("the household is created and held");

    (service, state)
}

/// Every write the transport was asked to submit that addresses one item (`save_item`), in call
/// order. The household create is a different service function and is not an item write.
fn item_bodies(service: &ScriptedService) -> Vec<Value> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { request, .. } if request.rpc() == SAVE_ITEM_RPC => {
                Some(request.body().clone())
            }
            _ => None,
        })
        .collect()
}

/// The household's one shared collection, rebuilt from the item writes the service accepted.
///
/// This is what every member resolves: the service stores one row per item in the household's
/// collection, so replaying the writes in call order yields the stored version of each item. The
/// replay fails if an item write addresses another collection — that is the "shared by construction"
/// assertion.
fn shared_rows(service: &ScriptedService, household_id: &HouseholdId) -> Vec<CollectionItem> {
    let mut rows: BTreeMap<String, CollectionItem> = BTreeMap::new();

    for body in item_bodies(service) {
        assert_eq!(
            body["household_id"],
            json!(household_id.0.clone()),
            "every item write addresses the household's one shared collection"
        );

        let item_id: ItemId = serde_json::from_value(body["item_id"].clone())
            .expect("the write names the item it writes");
        let updated_at: Instant = serde_json::from_value(body["updated_at"].clone())
            .expect("the write carries its version marker");
        // D1.17: a non-removal write **omits** `deleted` and lets `save_item` default it to `false`;
        // only a tombstone states it. Absence is therefore the live-row shape, not a missing field —
        // but a value that is present must be the boolean the contract says, so a malformed one
        // refuses here instead of replaying as a live row.
        let deleted = match body.get("deleted") {
            None => false,
            Some(Value::Bool(deleted)) => *deleted,
            Some(other) => panic!("a write's `deleted` is a boolean when present, not {other:?}"),
        };
        let payload: ItemPayload = serde_json::from_value(body["payload"].clone())
            .expect("the write carries the item's content");

        rows.insert(
            item_id.0.clone(),
            CollectionItem {
                item_id,
                updated_at,
                deleted,
                payload: Some(payload),
            },
        );
    }

    rows.into_values().collect()
}

/// Load `items` as the household's copy, exactly as the read path delivers a service read (SYNC-3).
fn load(state: &AppState, household_id: &HouseholdId, items: Vec<CollectionItem>) {
    state
        .load_collection(
            household_id,
            HouseholdCollection {
                household_id: household_id.clone(),
                items,
                source: CollectionSource::Service,
                loaded_at: None,
            },
            at(LOADED_AT),
        )
        .expect("the collection is the household's own");
}

/// This device's copy, refreshed from the writes the service accepted. Every read below runs against
/// this copy, which is the version the household shares.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// A state holding the household's one active rabbit, with the copy the read path delivers after its
/// create — the state a weight write starts from, since a write resolves the rabbit from the copy.
fn household_with_a_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    let rabbit =
        block_on_ready(state.create_rabbit(one.clone(), profile(), request_id(CREATE_RABBIT)))
            .expect("the double accepted the create");
    refresh(&state, &service, &one);

    (service, state, rabbit)
}

/// One entry create through the state method the `create_weight_entry` command forwards to. Its id is
/// the operation's, so a test can name the stored item the write produced.
fn recorded(
    state: &AppState,
    rabbit: &Rabbit,
    entry: WeightEntryDraft,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.create_weight_entry(rabbit.id.clone(), entry, request_id(operation)))
        .map(|(_household, entry)| entry)
        .expect("the double accepted the entry")
}

/// One entry correction through the state method the `update_weight_entry` command forwards to.
fn revise(
    state: &AppState,
    entry_id: &WeightEntryId,
    entry: WeightEntryDraft,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.update_weight_entry(entry_id.clone(), entry, request_id(operation)))
        .map(|(_household, entry)| entry)
        .expect("the double accepted the edit")
}

/// One entry removal through the state method the `delete_weight_entry` command forwards to.
fn remove(state: &AppState, entry_id: &WeightEntryId, operation: &str) {
    block_on_ready(state.delete_weight_entry(entry_id.clone(), request_id(operation)))
        .map(|(_household, _rabbit)| ())
        .expect("the double accepted the deletion");
}

/// Every stored weight-entry write, keyed by the item id it addresses, with the whole body intact —
/// payload, version marker (`updated_at`) and `deleted` included. A rewrite of any entry changes a
/// body here, so the equality of two snapshots is the "no stored entry was rewritten" claim itself.
fn stored_entry_bodies(service: &ScriptedService) -> BTreeMap<String, Value> {
    item_bodies(service)
        .into_iter()
        .filter(|body| body["payload"]["weight_entry"].is_object())
        .map(|body| {
            let item_id = body["item_id"]
                .as_str()
                .expect("an item write names its item")
                .to_owned();

            (item_id, body)
        })
        .collect()
}

/// The version of `entry_id` the service accepted last, decoded through the contract's own payload
/// shape — the version every member resolves.
fn stored_entry(service: &ScriptedService, entry_id: &WeightEntryId) -> WeightEntry {
    let bodies = stored_entry_bodies(service);
    let body = bodies
        .get(&entry_id.0)
        .unwrap_or_else(|| panic!("{entry_id:?} was never written"));

    match serde_json::from_value::<ItemPayload>(body["payload"].clone())
        .expect("the write's payload is an item payload the contract models")
    {
        ItemPayload::WeightEntry { weight_entry } => weight_entry,
        other => panic!("the write carries a weight entry, not {other:?}"),
    }
}

/// One entry's derived flag through the state method the `get_weight_entry` command forwards to.
fn flagged(state: &AppState, entry_id: &WeightEntryId) -> bool {
    state
        .weight_entry(entry_id)
        .expect("the entry is in this device's copy")
        .has_weight_drop_alert
}

/// The rabbit's whole series with the threshold it is judged against, through the state method the
/// `get_weight_entries` command forwards to.
fn trend_of(state: &AppState, rabbit: &Rabbit) -> WeightTrend {
    state
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the rabbit's history is readable")
}

/// One rabbit's dashboard row, through the two public halves of the `get_dashboard` body: the held
/// local copy, then the body's derivation over it.
fn dashboard_row(state: &AppState, rabbit: &Rabbit) -> DashboardRow {
    let collection = state
        .read_collection(&household(HOUSEHOLD), CollectionSource::Local)
        .expect("the device holds the collection it loaded");

    app_lib::domain::rabbit::dashboard(&collection)
        .expect("every row's content is known")
        .into_iter()
        .find(|row| row.rabbit.id == rabbit.id)
        .expect("the active rabbit has a dashboard row")
}

/// Save a new threshold through the state method the `update_rabbit` command forwards to. The edit is
/// a whole-profile replacement (RAB-5), so the profile carries every field the create did.
fn change_threshold(state: &AppState, rabbit: &Rabbit, percent: f64, operation: &str) -> Rabbit {
    let profile = RabbitProfile {
        weight_drop_threshold_percent: percent,
        ..profile()
    };

    block_on_ready(state.update_rabbit(rabbit.id.clone(), profile, request_id(operation)))
        .expect("the double accepted the threshold edit")
}

/// DASH-4's invariant: the row's flag is the same derivation as the entries', so it is exactly
/// "any entry of the rabbit is flagged" — never a second rule that could drift.
fn assert_dashboard_flag_is_the_entry_derivation(state: &AppState, rabbit: &Rabbit) {
    let any_entry_flagged = trend_of(state, rabbit)
        .points
        .iter()
        .any(|point| point.has_weight_drop_alert);

    assert_eq!(
        dashboard_row(state, rabbit).has_weight_drop_alert,
        any_entry_flagged,
        "the dashboard flag is the entry derivation, not a separate rule (D1.3, DASH-4)"
    );
}

/// WT-7, "A drop exceeding the threshold is flagged".
///
/// **WHEN** a rabbit's threshold is 5%, its previous entry is 2000 g and a new entry of 1800 g is
/// saved **THEN** the new entry is presented with a visual drop warning.
#[test]
fn a_drop_exceeding_the_threshold_is_flagged() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    assert_eq!(
        rabbit.weight_drop_threshold_percent,
        DEFAULT_THRESHOLD_PERCENT
    );

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let dropped = recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    // 2000 g to 1800 g is a drop of 10%, beyond the stored 5% threshold: the detail's derived flag is
    // the warning the entry view presents (WT-8 renders it).
    assert!(
        flagged(&state, &dropped.id),
        "a 10% drop beyond a 5% threshold is flagged"
    );

    // The trend point carries the same flag, and the series carries the threshold it was judged
    // against rather than a number read separately (WT-7, I2).
    let trend = trend_of(&state, &rabbit);
    assert_eq!(trend.drop_threshold_percent, DEFAULT_THRESHOLD_PERCENT);
    let point = trend
        .points
        .iter()
        .find(|point| point.id == dropped.id)
        .expect("the dropped entry is a point of the full history");
    assert!(
        point.has_weight_drop_alert,
        "the trend point carries the derived alert"
    );

    // DASH-4: the rabbit is flagged on the dashboard for the same entry.
    assert!(
        dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "a rabbit with a flagged drop is flagged on the dashboard (DASH-4)"
    );
}

/// WT-7, "A drop within the threshold is not flagged".
///
/// **WHEN** a rabbit's threshold is 5%, its previous entry is 2000 g and a new entry of 1950 g is
/// saved **THEN** the new entry is presented without a drop warning.
#[test]
fn a_drop_within_the_threshold_is_not_flagged() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let inside = recorded(
        &state,
        &rabbit,
        draft_at(1_950, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    // 2000 g to 1950 g is a 2.5% drop, inside the 5% threshold, so no warning is presented.
    assert!(
        !flagged(&state, &inside.id),
        "a 2.5% drop inside a 5% threshold is not flagged"
    );

    let trend = trend_of(&state, &rabbit);
    let point = trend
        .points
        .iter()
        .find(|point| point.id == inside.id)
        .expect("the entry is a point of the full history");
    assert!(
        !point.has_weight_drop_alert,
        "the trend point is not flagged either"
    );

    // DASH-4: no entry of the rabbit is flagged, so its row is not.
    assert!(
        !dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "a rabbit without a flagged drop is not flagged on the dashboard (DASH-4)"
    );
}

/// WT-7, "Raising the threshold clears an existing flag".
///
/// **WHEN** a drop of 10% is flagged at a threshold of 5% and the rabbit's threshold is raised to
/// 15% **THEN** the warning is no longer presented for that entry.
#[test]
fn raising_the_threshold_clears_an_existing_flag() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let dropped = recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    assert!(
        flagged(&state, &dropped.id),
        "the 10% drop is flagged at 5% before the threshold moves"
    );

    let raised = change_threshold(&state, &rabbit, 15.0, "operation-5");
    assert_eq!(raised.weight_drop_threshold_percent, 15.0);
    refresh(&state, &service, &one);

    // The same stored entry, judged against the new stored threshold: 10% no longer exceeds 15%.
    assert!(
        !flagged(&state, &dropped.id),
        "the warning is no longer presented for that entry"
    );

    let trend = trend_of(&state, &rabbit);
    assert_eq!(
        trend.drop_threshold_percent, 15.0,
        "the series carries the raised threshold"
    );
    assert!(
        trend
            .points
            .iter()
            .all(|point| !point.has_weight_drop_alert),
        "no point of the series is flagged at the raised threshold"
    );

    // DASH-4's third scenario: the dashboard flag clears with the entry flag.
    assert!(
        !dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "the warning indicator is no longer presented for that rabbit (DASH-4)"
    );
}

/// WT-7, "Lowering the threshold raises a flag".
///
/// **WHEN** a drop of 2.5% carries no warning at a threshold of 5% and the rabbit's threshold is
/// lowered to 2% **THEN** the warning is presented for that entry.
#[test]
fn lowering_the_threshold_raises_a_flag() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let inside = recorded(
        &state,
        &rabbit,
        draft_at(1_950, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    assert!(
        !flagged(&state, &inside.id),
        "the 2.5% drop is inside 5% before the threshold moves"
    );

    let lowered = change_threshold(&state, &rabbit, 2.0, "operation-5");
    assert_eq!(lowered.weight_drop_threshold_percent, 2.0);
    refresh(&state, &service, &one);

    // The same stored entry, now beyond the lowered threshold.
    assert!(
        flagged(&state, &inside.id),
        "the warning is presented for that entry once the threshold is lowered"
    );

    let trend = trend_of(&state, &rabbit);
    assert_eq!(trend.drop_threshold_percent, 2.0);
    let point = trend
        .points
        .iter()
        .find(|point| point.id == inside.id)
        .expect("the entry is a point of the full history");
    assert!(
        point.has_weight_drop_alert,
        "the trend point carries the raised flag"
    );

    assert!(
        dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "the dashboard follows the same derivation (DASH-4)"
    );
}

/// WT-7/D1.12, "Entries sharing a timestamp are ordered by creation".
///
/// **WHEN** two weight entries of a rabbit carry the same timestamp and the later-created one is
/// otherwise lower **THEN** the earlier-created entry is the previous entry for the later-created one
/// **AND** the drop of the later-created entry is measured against it.
#[test]
fn entries_sharing_a_timestamp_are_ordered_by_creation() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    // The operation ids deliberately order the *other* way: "operation-10" sorts before
    // "operation-9", so an implementation that fell back to the id (or to insertion into a sorted
    // container) instead of the recorded creation instant would pick the opposite predecessor and
    // answer "no drop".
    let earlier_created = recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-21T09:00:00Z"),
        "operation-9",
    );
    let later_created = recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-10",
    );
    refresh(&state, &service, &one);

    // The scenario's premise, asserted rather than assumed: one timestamp, and a recorded creation
    // order that is the order the entries were created (D1.12 stores it, since nothing else records
    // it). The two creates are sequential and the clock is read once per operation, so the first is
    // the earlier-created entry.
    let earlier_stored = stored_entry(&service, &earlier_created.id);
    let later_stored = stored_entry(&service, &later_created.id);

    assert_eq!(
        earlier_stored.timestamp, later_stored.timestamp,
        "the two entries share their timestamp"
    );
    assert!(
        AbsoluteInstant::parse(&earlier_stored.created_at).expect("a stored creation instant")
            < AbsoluteInstant::parse(&later_stored.created_at).expect("a stored creation instant"),
        "the first create is the earlier-created entry"
    );
    assert!(
        later_created.id.0 < earlier_created.id.0,
        "the ids would order the later-created entry first, so only the recorded creation order \
         makes the earlier-created entry its predecessor"
    );

    // The later-created, lower entry is measured against the earlier-created one: 2000 g to 1800 g.
    assert!(
        flagged(&state, &later_created.id),
        "the drop of the later-created entry is measured against the earlier-created one"
    );
    assert!(
        !flagged(&state, &earlier_created.id),
        "the earlier-created entry has no predecessor and is never flagged"
    );

    let ids: Vec<String> = trend_of(&state, &rabbit)
        .points
        .iter()
        .map(|point| point.id.0.clone())
        .collect();
    assert_eq!(
        ids,
        vec![earlier_created.id.0.clone(), later_created.id.0.clone()],
        "the series is ordered by creation within the shared timestamp"
    );

    assert!(
        dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "the flagged entry flags the rabbit on the dashboard (DASH-4)"
    );
}

/// WT-7/D1.13, "Deleting an entry re-derives the following entry's alert".
///
/// **WHEN** a rabbit has three entries in timestamp order and the middle one is deleted **THEN** the
/// remaining later entry is compared against the entry that now precedes it **AND** its flag is
/// presented according to that comparison.
#[test]
fn deleting_an_entry_re_derives_the_following_entry_s_alert() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let first = recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let middle = recorded(
        &state,
        &rabbit,
        draft_at(2_100, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    let last = recorded(
        &state,
        &rabbit,
        draft_at(1_950, "2026-10-22T09:00:00Z"),
        "operation-5",
    );
    refresh(&state, &service, &one);

    // Before the deletion the later entry is judged against the 2100 g middle entry: 1950 g is a
    // 7.1% drop, beyond 5%, so it is flagged. The middle entry gains on the first, so it is not.
    assert!(!flagged(&state, &middle.id));
    assert!(
        flagged(&state, &last.id),
        "before the deletion the later entry is judged against the middle one"
    );
    assert!(!flagged(&state, &first.id));
    assert!(dashboard_row(&state, &rabbit).has_weight_drop_alert);

    let before = stored_entry_bodies(&service);
    let first_before = before
        .get(&first.id.0)
        .expect("the first entry was stored")
        .clone();
    let last_before = before
        .get(&last.id.0)
        .expect("the later entry was stored")
        .clone();

    remove(&state, &middle.id, "operation-6");
    refresh(&state, &service, &one);

    // The removal is one tombstone of the middle entry's own item; the neighbours are untouched, so
    // their stored bodies — payload, version marker and tombstone flag — are byte-identical.
    let after = stored_entry_bodies(&service);
    let middle_body = after
        .get(&middle.id.0)
        .expect("the middle entry was written");
    assert_eq!(middle_body["deleted"], json!(true));
    assert_eq!(
        after.get(&first.id.0),
        Some(&first_before),
        "deleting the middle entry rewrites no neighbour's stored body"
    );
    assert_eq!(
        after.get(&last.id.0),
        Some(&last_before),
        "deleting the middle entry rewrites no neighbour's stored body"
    );

    // After the deletion the later entry is compared against the first: 2000 g to 1950 g is a 2.5%
    // drop inside the threshold, so its flag is no longer presented although it was never edited.
    assert!(
        !flagged(&state, &last.id),
        "the remaining later entry is compared against the entry that now precedes it"
    );
    assert_eq!(
        trend_of(&state, &rabbit).points.len(),
        2,
        "the history holds the two surviving entries"
    );

    // DASH-4: the flag disappears once no entry is flagged any more.
    assert!(
        !dashboard_row(&state, &rabbit).has_weight_drop_alert,
        "the dashboard flag disappears once no entry is flagged any more (DASH-4)"
    );
}

/// WT-7/D1.13, "Editing an entry re-derives the alerts around it".
///
/// **WHEN** the value of a past weight entry is edited and saved **THEN** the flag of the edited entry
/// itself is re-derived **AND** the flag of the entry that follows it is re-derived.
#[test]
fn editing_an_entry_re_derives_the_alerts_around_it() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let edited = recorded(
        &state,
        &rabbit,
        draft_at(2_050, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    let following = recorded(
        &state,
        &rabbit,
        draft_at(1_900, "2026-10-22T09:00:00Z"),
        "operation-5",
    );
    refresh(&state, &service, &one);

    // Before the edit: the middle entry gains on its predecessor, so it is not flagged; the following
    // entry drops 7.3% against it, so it is.
    assert!(!flagged(&state, &edited.id));
    assert!(flagged(&state, &following.id));

    // An edit supplies only the value: the entry keeps its timestamp (and its creation instant).
    let revised = revise(&state, &edited.id, draft(1_700), "operation-6");
    assert_eq!(revised.id, edited.id);
    assert_eq!(
        revised.timestamp, edited.timestamp,
        "an omitted timestamp keeps the past entry's own instant"
    );
    refresh(&state, &service, &one);

    // The edited entry is now a 15% drop against the first entry: its own flag is re-derived.
    assert!(
        flagged(&state, &edited.id),
        "the flag of the edited entry itself is re-derived"
    );

    // And the entry that follows gains on the edited one, so its flag is re-derived too although it
    // was never itself edited.
    assert!(
        !flagged(&state, &following.id),
        "the flag of the entry that follows it is re-derived"
    );

    assert_eq!(
        trend_of(&state, &rabbit).points.len(),
        3,
        "an edit replaces the entry rather than appending one"
    );
    assert_dashboard_flag_is_the_entry_derivation(&state, &rabbit);
}

/// D1.3/D1.13, "changing the threshold rewrites no stored entry".
///
/// The alert is a derivation, not content: raising the threshold changes the answer for the same
/// stored entries and writes no entry back. The assertion is on the **whole stored bodies**, version
/// markers included, so a rewrite that left the value alone would still be caught.
#[test]
fn changing_the_threshold_rewrites_no_stored_entry() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let dropped = recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);
    assert!(flagged(&state, &dropped.id));

    let before = stored_entry_bodies(&service);
    assert_eq!(before.len(), 2, "both entries were stored");
    assert!(
        before
            .values()
            .all(|body| body["payload"]["weight_entry"]["has_weight_drop_alert"] == json!(false)),
        "every stored entry writes the constant `false` (D1.16)"
    );

    change_threshold(&state, &rabbit, 15.0, "operation-5");
    refresh(&state, &service, &one);

    let after = stored_entry_bodies(&service);
    assert_eq!(
        after, before,
        "a threshold change rewrites no stored entry — not its payload, not its version marker"
    );

    // The derivation did change, so the equality above is not the equality of "nothing happened":
    // the same stored entries now answer unflagged, and the rabbit's own row carries the new number.
    assert!(
        !flagged(&state, &dropped.id),
        "the same stored entry is now judged against 15%"
    );
    let rabbit_body = item_bodies(&service)
        .into_iter()
        .rfind(|body| body["payload"]["rabbit"].is_object())
        .expect("the threshold edit wrote the rabbit's item");
    assert_eq!(
        rabbit_body["payload"]["rabbit"]["weight_drop_threshold_percent"],
        json!(15.0),
        "the edited threshold is the rabbit's stored profile field"
    );
}

/// D1.3/DASH-4: "the dashboard flag is derived from the same result as the entry flag".
///
/// Across three states of the same rabbit — nothing flagged, one entry flagged, nothing flagged
/// again after an edit — the row's flag equals "any entry is flagged" from the same read, so the two
/// surfaces cannot disagree.
#[test]
fn the_dashboard_flag_is_the_same_derivation_as_the_entry_flag() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    recorded(
        &state,
        &rabbit,
        draft_at(1_950, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    assert!(!dashboard_row(&state, &rabbit).has_weight_drop_alert);
    assert_dashboard_flag_is_the_entry_derivation(&state, &rabbit);

    let dropped = recorded(
        &state,
        &rabbit,
        draft_at(1_700, "2026-10-22T09:00:00Z"),
        "operation-5",
    );
    refresh(&state, &service, &one);

    assert!(
        flagged(&state, &dropped.id),
        "the third entry is a 12.8% drop beyond the threshold"
    );
    assert_dashboard_flag_is_the_entry_derivation(&state, &rabbit);
    assert!(dashboard_row(&state, &rabbit).has_weight_drop_alert);

    revise(&state, &dropped.id, draft(1_900), "operation-6");
    refresh(&state, &service, &one);

    assert!(
        !flagged(&state, &dropped.id),
        "the edit brings the entry back inside the threshold"
    );
    assert_dashboard_flag_is_the_entry_derivation(&state, &rabbit);
    assert!(!dashboard_row(&state, &rabbit).has_weight_drop_alert);
}
