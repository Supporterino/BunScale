//! Integration tests for the dashboard read model (task 5.6): DASH-1's two list scenarios, DASH-2's
//! never-weighed row, and DASH-3's two sort scenarios.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/dashboard/spec.md` states, and each test is
//! named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `rabbit` and `weight` command adapters are private bodies over [`AppState`]:
//! the `#[tauri::command]` wrapper takes `tauri::State<'_, AppState>`, which has no public
//! constructor outside a running app, so these tests drive the same [`AppState`] methods the
//! commands forward to — with the published [`ScriptedService`] double as the transport, exactly as
//! `drop_alert.rs` (task 5.5) and `weight_entries.rs` (task 5.4) do.
//!
//! `get_dashboard` has **no** [`AppState`] method to forward to: its body is the blank-id check, then
//! `ReadPath::shared().serve_local(state, household_id)`, then `domain::rabbit::dashboard`. The
//! `serve_local` half is crate-private, so [`dashboard`] composes the two public calls the body
//! composes — [`AppState::read_collection`] with `CollectionSource::Local` resolves the same held
//! copy, and [`app_lib::domain::rabbit::dashboard`] is the body's core — and the whole row sequence is
//! read back. No private symbol is named, and nothing of the body apart from `serve_local` and the
//! blank-id guard is left out (the guard's refusals are asserted by the command's own unit tests).
//!
//! Every rabbit and entry reaches the read through the write path: a create is one `save_item` into
//! the household's shared collection, and this device's copy is rebuilt from the accepted writes
//! ([`refresh`]). No row is composed by the test.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # Fixed instants
//!
//! Every entry timestamp is a fixed past instant and is an **input** to the read: DASH-3 orders on the
//! entry's timestamp. The entries' `created_at` is minted inside the create body and is not
//! injectable, so no test depends on it and none sleeps.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: DASH-2's "localized never-weighed indication" is asserted as the
//! derived `never_weighed` flag the frontend localizes (with `last_weighed_at: None` beside it), never
//! as copy. DASH-1's "thumbnail" is the row's `thumbnail_ref`, the reference the view resolves; no
//! image is loaded. DASH-4's flag and the blank-id refusal are covered by `drop_alert.rs` and the
//! command's own unit tests, and are not restated.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, DashboardRow, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId, Sex,
    WeightEntry, WeightEntryDraft,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{Answer, ServiceReply};

/// The one household these scenarios live in.
const HOUSEHOLD: &str = "household-1";

/// The instant the scripted service dates a created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded this device's copy.
const LOADED_AT: &str = "2026-10-25T09:31:00Z";

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

/// A rabbit profile supplying the RAB-1 field set, with the name and photo the row presents.
fn profile(name: &str, photo_ref: Option<&str>) -> RabbitProfile {
    RabbitProfile {
        name: name.to_owned(),
        photo_ref: photo_ref.map(str::to_owned),
        breed: Some("Mini Lop".to_owned()),
        dob: Some(at("2026-03-01T00:00:00Z")),
        dob_unknown: false,
        sex: Sex::Female,
        neuter_status: NeuterStatus::Spayed,
        adoption_date: None,
        weight_drop_threshold_percent: 5.0,
    }
}

/// A draft with the person's own reading instant, which may be in the past (WT-1).
fn draft_at(value_grams: i32, timestamp: &str) -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(value_grams),
        timestamp: Some(at(timestamp)),
        note: None,
        outlier_confirmed: false,
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
/// Replaying the writes in call order yields the stored version of each item, which is what this
/// device's copy is loaded from. The replay fails if an item write addresses another collection.
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
        let deleted = body["deleted"].as_bool().unwrap_or(false);
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

/// This device's copy, refreshed from the writes the service accepted.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// One rabbit create through the state method the `create_rabbit` command forwards to.
fn create_rabbit(state: &AppState, name: &str, photo_ref: Option<&str>, operation: &str) -> Rabbit {
    block_on_ready(state.create_rabbit(
        household(HOUSEHOLD),
        profile(name, photo_ref),
        request_id(operation),
    ))
    .expect("the double accepted the create")
}

/// One entry create through the state method the `create_weight_entry` command forwards to.
fn recorded(
    state: &AppState,
    rabbit: &Rabbit,
    value_grams: i32,
    timestamp: &str,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.create_weight_entry(
        rabbit.id.clone(),
        draft_at(value_grams, timestamp),
        request_id(operation),
    ))
    .map(|(_household, entry)| entry)
    .expect("the double accepted the entry")
}

/// One archive through the state method the `archive_rabbit` command forwards to.
fn archived(state: &AppState, rabbit: &Rabbit, operation: &str) -> Rabbit {
    block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(operation)))
        .expect("the double accepted the archive")
}

/// The dashboard rows, through the two public halves of the `get_dashboard` body: the held local
/// copy, then the body's derivation over it.
fn dashboard(state: &AppState) -> Vec<DashboardRow> {
    let collection = state
        .read_collection(&household(HOUSEHOLD), CollectionSource::Local)
        .expect("the device holds the collection it loaded");

    app_lib::domain::rabbit::dashboard(&collection).expect("every row's content is known")
}

/// The names of the rows in the order the dashboard returns them.
fn names(rows: &[DashboardRow]) -> Vec<String> {
    rows.iter().map(|row| row.rabbit.name.clone()).collect()
}

/// One rabbit's row.
fn row_for<'a>(rows: &'a [DashboardRow], rabbit: &Rabbit) -> &'a DashboardRow {
    rows.iter()
        .find(|row| row.rabbit.id == rabbit.id)
        .expect("the rabbit has a dashboard row")
}

/// DASH-1, "Both active rabbits are listed with their details".
///
/// **WHEN** the active household holds two non-archived rabbits and the product is opened **THEN**
/// both rabbits are listed **AND** each row shows the rabbit's name, its thumbnail and its
/// last-weighed date.
#[test]
fn both_active_rabbits_are_listed_with_their_details() {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    let pebble = create_rabbit(&state, "Pebble", Some("photo-1"), "operation-2");
    let clover = create_rabbit(&state, "Clover", Some("photo-2"), "operation-3");
    refresh(&state, &service, &one);

    recorded(
        &state,
        &pebble,
        1_800,
        "2026-10-24T08:00:00Z",
        "operation-4",
    );
    recorded(
        &state,
        &clover,
        1_600,
        "2026-10-22T08:00:00Z",
        "operation-5",
    );
    refresh(&state, &service, &one);

    let rows = dashboard(&state);

    assert_eq!(
        names(&rows),
        vec!["Pebble", "Clover"],
        "both non-archived rabbits are listed"
    );

    let pebble_row = row_for(&rows, &pebble);
    assert_eq!(pebble_row.rabbit.name, "Pebble");
    assert_eq!(
        pebble_row.rabbit.thumbnail_ref,
        Some("photo-1".to_owned()),
        "the row carries the rabbit's thumbnail reference"
    );
    assert_eq!(
        pebble_row.last_weighed_at,
        Some(at("2026-10-24T08:00:00Z")),
        "the row carries the rabbit's last-weighed date"
    );
    assert!(!pebble_row.never_weighed);

    let clover_row = row_for(&rows, &clover);
    assert_eq!(clover_row.rabbit.name, "Clover");
    assert_eq!(
        clover_row.rabbit.thumbnail_ref,
        Some("photo-2".to_owned()),
        "each row carries its own rabbit's thumbnail, not a neighbour's"
    );
    assert_eq!(
        clover_row.last_weighed_at,
        Some(at("2026-10-22T08:00:00Z")),
        "each row carries its own rabbit's last-weighed date"
    );
    assert!(!clover_row.never_weighed);
}

/// DASH-1, "An archived rabbit is not listed".
///
/// **WHEN** the active household holds one archived rabbit and one non-archived rabbit **THEN** the
/// dashboard list contains the non-archived rabbit **AND** it does not contain the archived one.
#[test]
fn an_archived_rabbit_is_not_listed() {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    let active = create_rabbit(&state, "Pebble", Some("photo-1"), "operation-2");
    let leaving = create_rabbit(&state, "Clover", Some("photo-2"), "operation-3");
    refresh(&state, &service, &one);

    recorded(
        &state,
        &leaving,
        1_700,
        "2026-10-23T08:00:00Z",
        "operation-4",
    );
    refresh(&state, &service, &one);

    // The archive is one whole-item write into the shared collection, so the copy the dashboard
    // resolves is the archived version every member sees.
    let archived_row = archived(&state, &leaving, "operation-5");
    assert!(archived_row.is_archived);
    refresh(&state, &service, &one);

    let rows = dashboard(&state);

    assert_eq!(
        names(&rows),
        vec!["Pebble".to_owned()],
        "the list contains the non-archived rabbit and not the archived one"
    );
    assert_eq!(rows[0].rabbit.id, active.id);
    assert!(
        rows.iter().all(|row| row.rabbit.id != leaving.id),
        "the archived rabbit is absent from the list"
    );
    assert!(
        rows.iter().all(|row| !row.rabbit.is_archived),
        "no archived rabbit is presented as a dashboard row"
    );
}

/// DASH-2, "A rabbit with no entries is marked as never weighed".
///
/// **WHEN** a rabbit has no weight entries **THEN** its dashboard row presents a "never weighed"
/// indication **AND** it presents no date.
#[test]
fn a_rabbit_with_no_entries_is_marked_as_never_weighed() {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    let never = create_rabbit(&state, "Pebble", Some("photo-1"), "operation-2");
    refresh(&state, &service, &one);

    // The premise, asserted rather than assumed: no entry addresses the rabbit.
    assert!(
        item_bodies(&service)
            .iter()
            .all(|body| body["payload"]["kind"] != json!("weight_entry")),
        "the rabbit has no weight entry"
    );

    let rows = dashboard(&state);
    let row = row_for(&rows, &never);

    assert!(
        row.never_weighed,
        "the row presents the never-weighed indication"
    );
    assert_eq!(row.last_weighed_at, None, "the row presents no date");
    assert_eq!(
        row.rabbit.name, "Pebble",
        "the rabbit is still listed with its name"
    );
}

/// DASH-3, "The most recently weighed rabbit sorts first".
///
/// **WHEN** one rabbit was weighed today and another a week ago **THEN** the rabbit weighed today
/// appears above the other one.
#[test]
fn the_most_recently_weighed_rabbit_sorts_first() {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    // The week-ago rabbit is created first, so creation order would put it first; the scenario's
    // sort key is the weigh-in, not the create.
    let week_ago = create_rabbit(&state, "Clover", Some("photo-1"), "operation-2");
    let today = create_rabbit(&state, "Pebble", Some("photo-2"), "operation-3");
    refresh(&state, &service, &one);

    recorded(
        &state,
        &week_ago,
        1_650,
        "2026-10-18T08:00:00Z",
        "operation-4",
    );
    recorded(&state, &today, 1_800, "2026-10-25T08:00:00Z", "operation-5");
    refresh(&state, &service, &one);

    let rows = dashboard(&state);

    assert_eq!(
        names(&rows),
        vec!["Pebble", "Clover"],
        "the rabbit weighed today appears above the one weighed a week ago"
    );
    assert_eq!(
        row_for(&rows, &today).last_weighed_at,
        Some(at("2026-10-25T08:00:00Z"))
    );
    assert_eq!(
        row_for(&rows, &week_ago).last_weighed_at,
        Some(at("2026-10-18T08:00:00Z"))
    );
}

/// DASH-3, "A never-weighed rabbit sorts last".
///
/// **WHEN** the household holds rabbits that were weighed and rabbits that never were, and a rabbit
/// that never was is created first **THEN** every never-weighed rabbit appears below every weighed
/// rabbit.
///
/// The back-dated entry is the second half of the sort key: it is written **after** the newer entry
/// of its own rabbit, so a row built from write order would claim the older reading as that rabbit's
/// last weigh-in and would sort the rabbit below the one it must precede.
#[test]
fn a_never_weighed_rabbit_sorts_last() {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);

    // Created first, never weighed (DASH-3's "regardless of the order in which they were created").
    let never = create_rabbit(&state, "Never", Some("photo-1"), "operation-2");
    let recent = create_rabbit(&state, "Recent", Some("photo-2"), "operation-3");
    let older = create_rabbit(&state, "Older", Some("photo-3"), "operation-4");
    refresh(&state, &service, &one);

    // Recent's newer reading is written first and its back-dated reading second: write order and
    // reading order disagree, and only the reading timestamp can put Recent first.
    recorded(
        &state,
        &recent,
        1_800,
        "2026-10-25T08:00:00Z",
        "operation-5",
    );
    recorded(
        &state,
        &recent,
        1_820,
        "2026-09-01T08:00:00Z",
        "operation-6",
    );
    recorded(&state, &older, 1_700, "2026-10-10T08:00:00Z", "operation-7");
    refresh(&state, &service, &one);

    let rows = dashboard(&state);

    assert_eq!(
        names(&rows),
        vec!["Recent", "Older", "Never"],
        "newest weigh-in first, with the never-weighed rabbit below every weighed one"
    );

    // The back-dated entry is dated, not written: Recent's last weigh-in is the later reading even
    // though the older one reached the collection last.
    assert_eq!(
        row_for(&rows, &recent).last_weighed_at,
        Some(at("2026-10-25T08:00:00Z")),
        "the last weigh-in is the entry's timestamp, not the order it was written in"
    );

    // AND every never-weighed rabbit appears below every weighed rabbit.
    let last = rows.last().expect("the list is not empty");
    assert_eq!(last.rabbit.id, never.id);
    assert!(last.never_weighed);
    assert_eq!(last.last_weighed_at, None);
    assert!(
        rows[..rows.len() - 1]
            .iter()
            .all(|row| !row.never_weighed && row.last_weighed_at.is_some()),
        "every row above the never-weighed one is a weighed rabbit with a date"
    );
}
