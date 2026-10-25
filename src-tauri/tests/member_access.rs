//! Integration tests for equal member access (task 5.9): HS-8's three scenarios.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/household-sharing/spec.md` states for HS-8,
//! and each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `rabbit`, `weight` and `dashboard` adapters are private bodies over [`AppState`]: the
//! `#[tauri::command]` wrapper takes `tauri::State<'_, AppState>`, which has no public constructor
//! outside a running app, so these tests drive the same [`AppState`] methods the commands forward
//! to — with the published [`ScriptedService`] double as the transport, exactly as
//! `weight_entries.rs` (task 5.4) and `dashboard_rows.rs` (task 5.6) do. `app_lib::run()` is never
//! called (it starts the event loop) and `ipc::export_bindings()` is never called (D0.9), as
//! `library_surface.rs` records.
//!
//! Two members are two **accounts on two devices of one household**: each [`AppState`] has its own
//! account, its own household book and its own loaded copy of the same collection, while both write
//! through one [`ScriptedService`] double. A write is therefore attributed by the book (`writer_id`)
//! and lands in the one shared collection; a read is resolved from the copy that device loaded.
//!
//! # How "a member who joined rather than created" is arranged
//!
//! P1 has no command that joins a member to an existing household: HS-1's create records the
//! creator, and member management that grows the list is P3's (`add-household-sharing-hardening`,
//! HS-5 … HS-7; `domain::household` records the same split). The join itself is therefore out of
//! this change's surface, and what HS-8 owns is the *consequence*: two member identities of one
//! household with the same rights. The precondition is arranged the way a completed join leaves the
//! device — the household record carries the member list, which
//! [`HouseholdRecord::restore`](app_lib::domain::household::HouseholdRecord::restore) admits with
//! the creator first and owner (its documented invariant), and which the book records as one
//! household. Nothing about the write or read path is faked: after the member exists, every
//! assertion goes through the command bodies.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: "the same read and write capabilities" is asserted as the two
//! members' reads being *equal* (the same rows, the same payloads, the same derived flags) and as
//! both members' writes being accepted, not as which controls a screen shows. `is_owner` on the
//! household row legitimately differs between the two members (HS-10, P3), so the list of
//! *households* is not compared; the household's **data** is, which is what HS-8 is about. The
//! permission model has no representation to assert the absence of — the product has no tiers —
//! so "no tier hides it" is asserted as visibility to the other member rather than as a flag.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::domain::household::HouseholdRecord;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, DashboardRow, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, Member, MemberId, NeuterStatus, Rabbit,
    RabbitProfile, RabbitSummary, RequestId, Sex, WeightEntry, WeightEntryDraft, WeightRange,
};
use app_lib::state::AppState;
use app_lib::storage::AbsoluteInstant;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{Answer, ServiceReply};

/// The one household these scenarios live in.
const HOUSEHOLD: &str = "household-1";

/// The instant the scripted service dates the created household with.
const CREATED_AT: &str = "2026-10-19T10:00:00Z";

/// When the read path loaded each device's copy.
const LOADED_AT: &str = "2026-10-25T09:31:00Z";

/// The operation that created the household in the fixture.
const CREATE_HOUSEHOLD: &str = "operation-1";

/// The operation under which the second member's membership is recorded.
const MEMBERSHIP: &str = "membership-1";

/// The account of the person who created the household.
fn creator_account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

/// The account of the person who joined it.
fn joined_account() -> AccountUserId {
    AccountUserId("account-2".to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn household(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn member_id(value: &str) -> MemberId {
    MemberId(value.to_owned())
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
fn profile(name: &str) -> RabbitProfile {
    RabbitProfile {
        name: name.to_owned(),
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

/// A draft with the person's own reading instant, which may be in the past (WT-1).
fn draft_at(value_grams: i32, timestamp: &str, note: Option<&str>) -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(value_grams),
        timestamp: Some(at(timestamp)),
        note: note.map(str::to_owned),
        outlier_confirmed: false,
    }
}

/// The household record a completed join leaves behind: the creator first and owner, the joined
/// member second (the invariant `HouseholdRecord::restore` enforces).
fn two_member_record() -> HouseholdRecord {
    HouseholdRecord::restore(
        household(HOUSEHOLD),
        "Home".to_owned(),
        at(CREATED_AT),
        member_id("member-1"),
        vec![
            Member {
                id: member_id("member-1"),
                account_user_id: creator_account(),
                display_name: Some("Ada".to_owned()),
            },
            Member {
                id: member_id("member-2"),
                account_user_id: joined_account(),
                display_name: Some("Robin".to_owned()),
            },
        ],
    )
    .expect("the two-member record satisfies the household invariants")
}

/// Record the household as it stands after the join, on the device that is to act as one member.
fn admit_membership(state: &AppState, record: HouseholdRecord) {
    let mut book = state.households();
    book.record_created(request_id(MEMBERSHIP), record);
}

/// The member ids the device's household record carries, in order.
fn member_ids(state: &AppState) -> Vec<MemberId> {
    let book = state.households();
    book.get(&household(HOUSEHOLD))
        .expect("the device holds the household")
        .members()
        .iter()
        .map(|member| member.id.clone())
        .collect()
}

/// A device for the household's creator: it creates the household through the write path, so the
/// household and its first member come from a command rather than from the test.
fn creator_device(service: &Arc<ScriptedService>) -> AppState {
    let state = AppState::new(service.clone(), Some(creator_account()));

    block_on_ready(state.create_household("Home".to_owned(), request_id(CREATE_HOUSEHOLD)))
        .expect("the household is created and held");

    state
}

/// The creator's device after the second member joined: the membership is recorded, which is all a
/// join leaves on a device (see the module documentation).
fn creator_with_a_joined_member(service: &Arc<ScriptedService>) -> AppState {
    let state = creator_device(service);
    admit_membership(&state, two_member_record());

    assert_eq!(
        member_ids(&state),
        [member_id("member-1"), member_id("member-2")],
        "the household now holds both member identities, the creator first"
    );

    state
}

/// The joined member's device: a different account, the same household membership, and its own copy
/// of whatever the shared collection holds.
fn joined_device(service: &Arc<ScriptedService>) -> AppState {
    let state = AppState::new(service.clone(), Some(joined_account()));
    admit_membership(&state, two_member_record());

    state
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

/// The last write the service accepted for `item_id`: the version every member's copy is built from.
fn written(service: &ScriptedService, item_id: &str) -> Value {
    item_bodies(service)
        .into_iter()
        .rfind(|body| body["item_id"] == json!(item_id))
        .unwrap_or_else(|| panic!("{item_id:?} was never written"))
}

/// The household's one shared collection, rebuilt from the item writes the service accepted.
///
/// This is what every member resolves: the service stores one row per item in the household's
/// collection, so replaying the writes in call order yields the stored version of each item. The
/// replay fails if an item write addresses another collection — the "one shared household data"
/// half of HS-8.
fn shared_rows(service: &ScriptedService, household_id: &HouseholdId) -> Vec<CollectionItem> {
    let mut rows: BTreeMap<String, CollectionItem> = BTreeMap::new();

    for body in item_bodies(service) {
        assert_eq!(body["household_id"], json!(household_id.0.clone()));

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

/// Load `items` as the household's copy on one device, exactly as the read path delivers a service
/// read (SYNC-3).
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

/// Refresh one device's copy from every write the service accepted, including the other member's.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// One rabbit create through the state method the `create_rabbit` command forwards to.
fn create_rabbit(state: &AppState, name: &str, operation: &str) -> Rabbit {
    block_on_ready(state.create_rabbit(household(HOUSEHOLD), profile(name), request_id(operation)))
        .expect("the double accepted the create")
}

/// One rabbit edit through the state method the `update_rabbit` command forwards to.
fn revise_rabbit(state: &AppState, rabbit: &Rabbit, name: &str, operation: &str) -> Rabbit {
    block_on_ready(state.update_rabbit(rabbit.id.clone(), profile(name), request_id(operation)))
        .expect("the double accepted the edit")
}

/// One entry create through the state method the `create_weight_entry` command forwards to.
fn record(
    state: &AppState,
    rabbit: &Rabbit,
    value_grams: i32,
    timestamp: &str,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.create_weight_entry(
        rabbit.id.clone(),
        draft_at(value_grams, timestamp, None),
        request_id(operation),
    ))
    .map(|(_household, entry)| entry)
    .expect("the double accepted the entry")
}

/// One entry correction through the state method the `update_weight_entry` command forwards to.
fn revise_entry(
    state: &AppState,
    entry: &WeightEntry,
    value_grams: i32,
    note: Option<&str>,
    operation: &str,
) -> WeightEntry {
    block_on_ready(state.update_weight_entry(
        entry.id.clone(),
        draft_at(value_grams, &entry.timestamp.0, note),
        request_id(operation),
    ))
    .map(|(_household, entry)| entry)
    .expect("the double accepted the correction")
}

/// One device's held copy of the household collection.
fn collection(state: &AppState) -> HouseholdCollection {
    state
        .read_collection(&household(HOUSEHOLD), CollectionSource::Local)
        .expect("the device holds the collection it loaded")
}

/// The dashboard rows, through the two public halves of the `get_dashboard` body: the held local
/// copy, then the body's derivation over it.
fn dashboard(state: &AppState) -> Vec<DashboardRow> {
    app_lib::domain::rabbit::dashboard(&collection(state)).expect("every row's content is known")
}

/// The active rabbit list, through the two public halves of the `list_rabbits` body.
fn rabbit_list(state: &AppState) -> Vec<RabbitSummary> {
    app_lib::domain::rabbit::summaries(&collection(state), false)
        .expect("every row's content is known")
}

/// The names the rows carry, sorted: the assertion is about *which* rabbits are present.
fn names(rows: &[DashboardRow]) -> Vec<String> {
    let mut names: Vec<String> = rows.iter().map(|row| row.rabbit.name.clone()).collect();
    names.sort();

    names
}

/// HS-8, "A member edits data created by another member".
///
/// **WHEN** two members share a household and one of them edits a rabbit or a weight entry created
/// by the other and saves **THEN** the edit is saved **AND** every member of the household resolves
/// the edited data.
///
/// The second clause is why the edit must land on the item that already exists: a second row (or a
/// per-member copy) would let one member's read resolve the edit while the other's resolved the
/// original. Both the rabbit and the entry are edited, because the scenario names both.
#[test]
fn a_member_edits_data_created_by_another_member() {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let creator = creator_with_a_joined_member(&service);
    let joined = joined_device(&service);
    let one = household(HOUSEHOLD);

    // Two distinct member identities on the same household, with the creator's data created first.
    assert_ne!(creator_account(), joined_account());
    let rabbit = create_rabbit(&creator, "Pebble", "operation-2");
    // The entry write resolves its rabbit from this device's copy, so the create is loaded first
    // (SYNC-3), exactly as the weight tests record.
    refresh(&creator, &service, &one);
    let entry = record(
        &creator,
        &rabbit,
        2_100,
        "2026-10-24T09:00:00Z",
        "operation-3",
    );

    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    // WHEN the joined member edits the other member's rabbit and entry and saves ...
    let edited_rabbit = revise_rabbit(&joined, &rabbit, "Pebble II", "operation-4");
    let edited_entry = revise_entry(
        &joined,
        &entry,
        1_900,
        Some("corrected by the other member"),
        "operation-5",
    );

    // THEN the edit is saved: each write succeeded against the existing item, not by creating one.
    assert_eq!(edited_rabbit.id, rabbit.id);
    assert_eq!(edited_entry.id, entry.id);

    // ... and the writes name the joined member as their writer, carrying the edited values.
    let rabbit_body = written(&service, &rabbit.id.0);
    assert_eq!(rabbit_body["writer_id"], json!("member-2"));
    assert_eq!(rabbit_body["payload"]["rabbit"]["name"], json!("Pebble II"));

    let entry_body = written(&service, &entry.id.0);
    assert_eq!(entry_body["writer_id"], json!("member-2"));
    assert_eq!(
        entry_body["payload"]["weight_entry"]["value_grams"],
        json!(1_900)
    );
    assert_eq!(
        entry_body["payload"]["weight_entry"]["note"],
        json!("corrected by the other member")
    );

    // AND there is no duplicate and no per-member copy: the household's collection still holds one
    // row for the rabbit and one for the entry, under the same item ids.
    let rows = shared_rows(&service, &one);
    let item_ids: BTreeSet<&str> = rows.iter().map(|row| row.item_id.0.as_str()).collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        item_ids,
        BTreeSet::from([rabbit.id.0.as_str(), entry.id.0.as_str()])
    );

    // AND every member resolves the edited data — on the device that wrote it and on the other's.
    for device in [&creator, &joined] {
        refresh(device, &service, &one);

        let resolved_rabbit = device
            .rabbit(&rabbit.id)
            .expect("the edited rabbit is in the shared collection");
        assert_eq!(resolved_rabbit.name, "Pebble II");

        let resolved_entry = device
            .weight_entry(&entry.id)
            .expect("the edited entry is in the shared collection");
        assert_eq!(resolved_entry.value_grams, Grams(1_900));
        assert_eq!(
            resolved_entry.note.as_deref(),
            Some("corrected by the other member")
        );
        // The correction keeps the entry's own instant rather than restamping it.
        assert_eq!(resolved_entry.timestamp, entry.timestamp);
    }
}

/// HS-8, "A joining member writes into the same shared data".
///
/// **WHEN** a member who joined the household rather than created it creates a rabbit or records a
/// weight entry **THEN** the created data belongs to the same shared household data as that of every
/// other member **AND** no permission tier hides it from or restricts it for the other members.
///
/// The join happens after the household exists, and the member then writes into data the creator
/// already made as well as creating its own — both directions are the scenario's precondition.
#[test]
fn a_joining_member_writes_into_the_same_shared_data() {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let one = household(HOUSEHOLD);

    // The household exists first, and P1 records exactly its creator (HS-1).
    let creator = creator_device(&service);
    assert_eq!(member_ids(&creator), [member_id("member-1")]);

    // The other member joins afterwards: a second member identity on the same household.
    admit_membership(&creator, two_member_record());
    let joined = joined_device(&service);
    assert_eq!(
        member_ids(&joined),
        [member_id("member-1"), member_id("member-2")],
        "the joined device holds the same two members"
    );

    // The household holds the creator's data before the joined member writes.
    let pebble = create_rabbit(&creator, "Pebble", "operation-2");
    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    // WHEN the joined member creates a rabbit and records a weight entry ...
    let clover = create_rabbit(&joined, "Clover", "operation-3");
    let entry = record(
        &joined,
        &pebble,
        1_950,
        "2026-10-24T08:00:00Z",
        "operation-4",
    );

    assert_eq!(clover.household_id, one);
    // THEN the created data belongs to the same shared household data: each write addresses the
    // household's one collection, is attributed to the joined member, and carries its own kind.
    for (item_id, expected_kind) in [
        (clover.id.0.as_str(), "rabbit"),
        (entry.id.0.as_str(), "weight_entry"),
    ] {
        let body = written(&service, item_id);
        assert_eq!(body["household_id"], json!(HOUSEHOLD));
        assert_eq!(body["writer_id"], json!("member-2"));
        assert_eq!(body["payload"]["kind"], json!(expected_kind));
    }

    // AND no permission tier hides it from the other member: the creator's own reads of the shared
    // collection carry the joined member's rabbit and entry.
    refresh(&creator, &service, &one);

    let row_names = names(&dashboard(&creator));
    assert!(row_names.contains(&"Clover".to_owned()));

    let now = AbsoluteInstant::now().to_wire();
    let trend = creator
        .weight_trend(&pebble.id, WeightRange::All, &now)
        .expect("the rabbit's history is readable");
    assert!(trend.points.iter().any(|point| point.id == entry.id));

    // AND nor does a tier restrict it: the creator can edit the rabbit the joined member created,
    // through the same command path.
    let revised = revise_rabbit(&creator, &clover, "Clover II", "operation-5");
    assert_eq!(revised.id, clover.id);
    assert_eq!(
        written(&service, &clover.id.0)["writer_id"],
        json!("member-1")
    );

    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);
    for device in [&creator, &joined] {
        let resolved = device
            .rabbit(&clover.id)
            .expect("the joined member's rabbit is shared");
        assert_eq!(resolved.name, "Clover II");
    }
}

/// HS-8, "No member is offered a reduced view of household data".
///
/// **WHEN** any member of a household opens a rabbit or a weight entry **THEN** the data is
/// presented with the same read and write capabilities as for every other member **AND** no
/// capability is withheld because of the member's role in the household.
///
/// The comparison is between the two members' **reads of the same household**, and equality — not
/// merely non-empty — is the assertion: an implementation that withheld a row, a field or a derived
/// flag from the non-owner would answer two different views of one collection.
#[test]
fn no_member_is_offered_a_reduced_view_of_household_data() {
    let service = Arc::new(ScriptedService::new().on_write([accepted_household()]));
    let creator = creator_with_a_joined_member(&service);
    let joined = joined_device(&service);
    let one = household(HOUSEHOLD);

    // One rabbit and one entry made by each member, so the household's data is not one member's.
    let pebble = create_rabbit(&creator, "Pebble", "operation-2");
    refresh(&creator, &service, &one);
    let pebble_entry = record(
        &creator,
        &pebble,
        2_000,
        "2026-10-22T09:00:00Z",
        "operation-3",
    );
    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    let clover = create_rabbit(&joined, "Clover", "operation-4");
    refresh(&joined, &service, &one);
    let clover_entry = record(
        &joined,
        &clover,
        1_800,
        "2026-10-23T09:00:00Z",
        "operation-5",
    );
    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    // The two copies of the household are the same collection — same rows, same payloads — so no
    // member is served a collection the other is not.
    let creator_copy = collection(&creator);
    let joined_copy = collection(&joined);
    assert_eq!(creator_copy, joined_copy);

    // THEN the dashboard is the same read model for both, and it is not empty: two rabbits.
    let creator_rows = dashboard(&creator);
    let joined_rows = dashboard(&joined);
    assert_eq!(
        names(&creator_rows),
        ["Clover".to_owned(), "Pebble".to_owned()]
    );
    assert_eq!(creator_rows, joined_rows);

    // ... and the active rabbit list is the same list, holding both rabbits.
    let creator_list = rabbit_list(&creator);
    let joined_list = rabbit_list(&joined);
    assert_eq!(creator_list.len(), 2);
    assert_eq!(creator_list, joined_list);

    // ... and each rabbit and entry opens to the same payload for both, derived flags included.
    let now = AbsoluteInstant::now().to_wire();
    for rabbit in [&pebble, &clover] {
        let creator_profile = creator.rabbit(&rabbit.id).expect("shared rabbit");
        let joined_profile = joined.rabbit(&rabbit.id).expect("shared rabbit");
        assert_eq!(creator_profile, joined_profile);

        let creator_trend = creator
            .weight_trend(&rabbit.id, WeightRange::All, &now)
            .expect("the rabbit's history is readable");
        let joined_trend = joined
            .weight_trend(&rabbit.id, WeightRange::All, &now)
            .expect("the rabbit's history is readable");
        assert_eq!(creator_trend, joined_trend);
    }
    for entry in [&pebble_entry, &clover_entry] {
        let creator_detail = creator.weight_entry(&entry.id).expect("shared entry");
        let joined_detail = joined.weight_entry(&entry.id).expect("shared entry");
        assert_eq!(creator_detail, joined_detail);
    }

    // AND no capability is withheld because of the member's role: each member can write the data the
    // other created, and both then resolve both edits identically.
    let revised_by_creator = revise_rabbit(&creator, &clover, "Clover II", "operation-6");
    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    let revised_by_joined = revise_rabbit(&joined, &pebble, "Pebble II", "operation-7");
    refresh(&creator, &service, &one);
    refresh(&joined, &service, &one);

    assert_eq!(revised_by_creator.id, clover.id);
    assert_eq!(revised_by_joined.id, pebble.id);

    for device in [&creator, &joined] {
        let clover_now = device.rabbit(&clover.id).expect("shared rabbit");
        let pebble_now = device.rabbit(&pebble.id).expect("shared rabbit");
        assert_eq!(clover_now.name, "Clover II");
        assert_eq!(pebble_now.name, "Pebble II");
    }

    // ... and the two views are still equal after both wrote.
    let creator_rows = dashboard(&creator);
    let joined_rows = dashboard(&joined);
    assert_eq!(creator_rows, joined_rows);
}
