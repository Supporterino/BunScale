//! Integration tests for profile editing and the rabbit lifecycle (task 5.2). The scenarios are the
//! ones `openspec/changes/add-rabbit-weight-core-loop/specs/rabbit-profiles/spec.md` states, and each
//! test is named after the scenario whose remaining observable it covers.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). Task 5.2 audits the coverage that tasks 2.1–2.16 already added, so this file
//! contains only the gaps — it does not repeat a scenario whose guarantee is already asserted.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::rabbit` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] methods the commands forward to — the whole behaviour
//! of the command apart from its wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `rabbit_profiles.rs` does. `app_lib::run()` is never called (it starts the
//! event loop) and `ipc::export_bindings()` is never called (D0.9), as `library_surface.rs` records.
//!
//! # What the tests observe
//!
//! The read path's copy is only written by the read path (SYNC-3), so a lifecycle write is invisible
//! on this device until a refresh. Each test therefore replays every accepted `save_item` write, in
//! call order, into the one household collection the service would hold — the shared data every
//! member resolves — and loads it as another member's copy. The reads are then the read models the
//! commands use: [`AppState::rabbit`] for the profile, and [`AppState::weight_trend`] /
//! [`AppState::weight_entry`] for the history. The replay refuses any item write that addresses
//! another collection, which is the "shared by construction" assertion.
//!
//! # The scenario → coverage map, and why only four observables are added here
//!
//! * RAB-5 "A saved edit including the threshold is shared with every member": the shared write —
//!   one `save_item` of the rabbit's item, threshold included — is asserted by
//!   `state::tests::editing_a_rabbit_writes_the_one_shared_item_every_member_resolves`; the command
//!   test `ipc::commands::rabbit::tests::a_successful_edit_publishes_exactly_one_updated_for_the_rabbit`
//!   asserts the publication. **Added here:** every member *resolves* the edited threshold, which no
//!   test read back.
//! * RAB-6 "Archiving removes the rabbit from the active list": covered by
//!   `ipc::commands::rabbit::tests::the_archived_rabbit_leaves_the_active_list_and_appears_in_the_archived_list`
//!   through the command body, both lists included. Not repeated.
//! * RAB-7 "Restoring returns the rabbit to the active list": covered by
//!   `ipc::commands::rabbit::tests::the_restored_rabbit_returns_to_the_active_list_and_keeps_its_history`
//!   (both lists) and `domain::rabbit::tests::a_restored_rabbit_is_in_the_active_selection_and_its_entry_is_untouched`.
//!   Not repeated.
//! * RAB-7 "Restoration leaves the history unchanged": the stored rows are asserted by
//!   `state::tests::restoring_leaves_every_weight_entry_present_unchanged_and_readable` and the command
//!   test above. **Added here:** the restored entries are still served by the trend and entry-detail
//!   reads a member uses.
//! * RAB-10 "Confirmed deletion removes the rabbit and its data": the lists and the tombstone rows are
//!   asserted by `ipc::commands::rabbit::tests::a_confirmed_deletion_removes_the_rabbit_and_its_data`
//!   and `state::tests::permanently_deleting_an_archived_rabbit_writes_a_tombstone_for_it_and_every_entry`.
//!   **Added here:** the removed rabbit and each of its entries are no longer *retrievable* by the reads.
//! * RAB-10 "An active rabbit cannot be permanently deleted": covered by
//!   `ipc::commands::rabbit::tests::an_active_rabbit_cannot_be_permanently_deleted_and_nothing_is_written_or_published`,
//!   which also asserts the rabbit remains in the active list. Not repeated.
//! * RAB-10 "The deletion is shared with every member": proven by the shape of the write in
//!   `ipc::commands::rabbit::tests::the_deletion_is_one_set_of_tombstones_in_the_households_shared_collection`
//!   — one row per removed item in the household's one collection, naming the writing member. Not
//!   repeated.
//!
//! "No control is available" and "the person does not confirm" are rendering and interaction
//! decisions and belong to the component tasks (5.11); they are not asserted here. Both platforms
//! (TOOL-1): nothing here is platform-specific.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, CommandError, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId,
    SaveCause, Sex, WeightEntry, WeightEntryDraft, WeightEntryId, WeightRange,
};
use app_lib::state::AppState;
use app_lib::sync::double::RecordedCall;
use app_lib::sync::double::ScriptedService;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::{Answer, ServiceReply};

/// The one household these scenarios live in.
const HOUSEHOLD: &str = "household-1";

/// The instant the scripted service dates a created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded this device's copy.
const LOADED_AT: &str = "2026-10-22T09:31:00Z";

/// The moment the trend is read at; after every stored entry.
const NOW: &str = "2026-10-23T09:00:00Z";

/// The operation that creates the rabbit every lifecycle test starts from.
const CREATE_RABBIT: &str = "operation-2";

/// The two entries the history scenarios record before archiving.
const FIRST_ENTRY: &str = "operation-3";
const SECOND_ENTRY: &str = "operation-4";

/// The archive that precedes a restore or a permanent deletion.
const ARCHIVE: &str = "operation-5";

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

/// A rabbit profile supplying the RAB-1 field set, with the default 5% threshold.
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

/// A state whose book holds the household with this account as its member, so a write can name its
/// writer. The double repeats its last write answer, so every later item write is accepted.
fn held_household() -> (Arc<ScriptedService>, AppState) {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let state = AppState::new(service.clone(), Some(account()));

    block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
        .expect("the household is created and held");

    (service, state)
}

/// One rabbit create through the state method the `create_rabbit` command forwards to.
fn create_rabbit(
    state: &AppState,
    household_id: &HouseholdId,
    profile: RabbitProfile,
    operation: &str,
) -> Result<Rabbit, CommandError> {
    block_on_ready(state.create_rabbit(household_id.clone(), profile, request_id(operation)))
}

/// One weight entry through the state method the `create_weight_entry` command forwards to.
fn create_entry(
    state: &AppState,
    rabbit: &Rabbit,
    operation: &str,
    value_grams: i32,
    timestamp: &str,
    note: Option<&str>,
) -> WeightEntry {
    block_on_ready(state.create_weight_entry(
        rabbit.id.clone(),
        WeightEntryDraft {
            value_grams: Grams(value_grams),
            timestamp: Some(at(timestamp)),
            note: note.map(str::to_owned),
            outlier_confirmed: false,
        },
        request_id(operation),
    ))
    .expect("the double accepted the entry")
    .1
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

/// The refresh that follows a successful lifecycle write: the service's one collection, loaded as
/// this device's copy.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// The account of the other member whose device resolves the shared collection.
///
/// Reads are not member-scoped, so this is documentary for detection; it keeps the fixture from
/// presenting the writer as every member.
fn other_account() -> AccountUserId {
    AccountUserId("account-2".to_owned())
}

/// Another member's device: it holds no household book, only the shared collection the writes
/// produced, and reads it through the same local reads the commands use.
fn member_device(service: &Arc<ScriptedService>, household_id: &HouseholdId) -> AppState {
    let state = AppState::new(service.clone(), Some(other_account()));
    load(&state, household_id, shared_rows(service, household_id));

    state
}

/// The weight-entry rows of the household's shared collection, in item-id order.
fn entry_rows(service: &ScriptedService, household_id: &HouseholdId) -> Vec<CollectionItem> {
    shared_rows(service, household_id)
        .into_iter()
        .filter(|row| matches!(&row.payload, Some(ItemPayload::WeightEntry { .. })))
        .collect()
}

/// A state holding the household's one active rabbit, with the copy the read path delivers after its
/// create — the state an edit starts from.
fn household_with_a_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);
    let rabbit = create_rabbit(&state, &one, profile(), CREATE_RABBIT)
        .expect("the double accepted the create");

    refresh(&state, &service, &one);

    (service, state, rabbit)
}

/// A state holding the household's one archived rabbit **and the two entries it owns**, with the
/// copy the read path delivers after the archive — the state a permanent deletion starts from.
fn household_with_an_archived_rabbit_and_entries() -> (Arc<ScriptedService>, AppState, Rabbit) {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    create_entry(
        &state,
        &rabbit,
        FIRST_ENTRY,
        2_000,
        "2026-10-20T09:00:00Z",
        Some("morning"),
    );
    create_entry(
        &state,
        &rabbit,
        SECOND_ENTRY,
        1_800,
        "2026-10-21T09:00:00Z",
        Some("evening"),
    );
    refresh(&state, &service, &one);

    block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(ARCHIVE)))
        .expect("the double accepted the archive");
    refresh(&state, &service, &one);

    (service, state, rabbit)
}

/// RAB-5, "A saved edit including the threshold is shared with every member".
///
/// **WHEN** a person edits any field of an active rabbit's profile, including its weight-drop
/// threshold, and saves **THEN** the change is saved to the household's shared data **AND** every
/// member of the household resolves the edited profile.
///
/// The shared-write half is already asserted by the state and command tests the module document
/// names; what this test adds is the other member's read of the edited threshold.
#[test]
fn a_saved_edit_including_the_threshold_is_shared_with_every_member() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    block_on_ready(state.update_rabbit(
        rabbit.id.clone(),
        RabbitProfile {
            name: "Clover".to_owned(),
            breed: Some("Netherland Dwarf".to_owned()),
            weight_drop_threshold_percent: 15.0,
            ..profile()
        },
        request_id("operation-3"),
    ))
    .expect("the double accepted the edit");

    // The edit landed as one item write of the rabbit's existing item, which is the row every member
    // resolves; the replay below would refuse it if it named another collection.
    let bodies = item_bodies(&service);
    assert_eq!(bodies.len(), 2, "the create and the edit");
    assert_eq!(bodies[1]["item_id"], json!(rabbit.id.0.clone()));

    // AND every member resolves the edited profile: another member's copy is the shared collection
    // the write landed in, and the profile read answers the edited threshold and fields.
    let member = member_device(&service, &one);
    let resolved = member
        .rabbit(&rabbit.id)
        .expect("the edited profile is in the shared collection");
    assert_eq!(resolved.weight_drop_threshold_percent, 15.0);
    assert_eq!(resolved.name, "Clover");
    assert_eq!(resolved.breed.as_deref(), Some("Netherland Dwarf"));
}

/// RAB-7, "Restoration leaves the history unchanged".
///
/// **WHEN** an archived rabbit that had weight entries is restored **THEN** those entries are still
/// present and unchanged.
///
/// The stored rows are already asserted by the state and command tests the module document names;
/// what this test adds is that the restored entries are still served by the reads a member uses.
#[test]
fn restoration_leaves_the_history_unchanged() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    create_entry(
        &state,
        &rabbit,
        FIRST_ENTRY,
        2_000,
        "2026-10-20T09:00:00Z",
        Some("morning"),
    );
    let second = create_entry(
        &state,
        &rabbit,
        SECOND_ENTRY,
        1_800,
        "2026-10-21T09:00:00Z",
        Some("evening"),
    );
    let stored_before = entry_rows(&service, &one);
    assert_eq!(stored_before.len(), 2);

    refresh(&state, &service, &one);
    block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(ARCHIVE)))
        .expect("the double accepted the archive");
    refresh(&state, &service, &one);
    block_on_ready(state.restore_rabbit(rabbit.id.clone(), request_id("operation-6")))
        .expect("the double accepted the restore");
    refresh(&state, &service, &one);

    // THEN those entries are still present and unchanged: the restore wrote the rabbit's item alone,
    // so both entry rows are byte-for-byte the ones recorded before the archive.
    let stored_after = entry_rows(&service, &one);
    assert_eq!(stored_after, stored_before, "the restore names no entry");

    // The restored history is readable from another member's copy, through the reads the trend and
    // the entry detail use.
    let member = member_device(&service, &one);

    // The restore actually happened: the shared version the member resolves has the flag cleared.
    // Without this, a `restore_rabbit` that returns `Ok` without writing — or writes through a
    // non-`save_item` RPC, which the replay drops — would pass every assertion below, because the
    // entries are unchanged either way and an archived rabbit's reads already succeed.
    let restored = member
        .rabbit(&rabbit.id)
        .expect("the restored rabbit is readable");
    assert!(
        !restored.is_archived,
        "the restore cleared the archived flag"
    );

    let trend = member
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the restored rabbit's series is readable");
    assert_eq!(
        trend
            .points
            .iter()
            .map(|point| point.value_grams)
            .collect::<Vec<_>>(),
        vec![Grams(2_000), Grams(1_800)],
        "both entries are present, in timestamp order"
    );
    assert_eq!(
        trend
            .points
            .iter()
            .map(|point| point.timestamp.clone())
            .collect::<Vec<_>>(),
        vec![at("2026-10-20T09:00:00Z"), at("2026-10-21T09:00:00Z")]
    );

    let detail = member
        .weight_entry(&WeightEntryId(SECOND_ENTRY.to_owned()))
        .expect("the restored rabbit's entry detail is readable");
    assert_eq!(detail.value_grams, Grams(1_800));
    assert_eq!(detail.timestamp, at("2026-10-21T09:00:00Z"));
    assert_eq!(detail.created_at, second.created_at);
    assert_eq!(detail.note.as_deref(), Some("evening"));
}

/// RAB-10, "Confirmed deletion removes the rabbit and its data".
///
/// **WHEN** a person confirms permanent deletion of an archived rabbit **THEN** the rabbit is no
/// longer present in any rabbit list **AND** its weight entries are no longer retrievable.
///
/// The lists and the tombstone rows are already asserted by the command and state tests the module
/// document names; what this test adds is that the removed rabbit and each of its entries are no
/// longer *retrievable* by the reads a member uses.
#[test]
fn confirmed_deletion_removes_the_rabbit_and_its_data() {
    let (service, state, rabbit) = household_with_an_archived_rabbit_and_entries();
    let one = household(HOUSEHOLD);

    let deleted_household =
        block_on_ready(state.delete_rabbit(rabbit.id.clone(), request_id("operation-6")))
            .expect("the double accepted the deletion");
    assert_eq!(deleted_household, one);

    let member = member_device(&service, &one);

    // The rabbit no longer resolves, so no surface can present it.
    let error = member
        .rabbit(&rabbit.id)
        .expect_err("the removed rabbit does not resolve");
    assert_eq!(error.cause, SaveCause::Validation);

    // AND its weight entries are no longer retrievable: the tombstoned rows are refused by the entry
    // detail read, and the rabbit's series has no live rabbit to read.
    for entry in [FIRST_ENTRY, SECOND_ENTRY] {
        let error = member
            .weight_entry(&WeightEntryId(entry.to_owned()))
            .expect_err("a removed entry does not resolve");
        assert_eq!(error.cause, SaveCause::Validation, "{entry}");
    }
    let error = member
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect_err("a removed rabbit's series does not resolve");
    assert_eq!(error.cause, SaveCause::Validation);
}
