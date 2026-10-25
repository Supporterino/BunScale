//! Integration tests for recording and correcting a rabbit's weight entries (task 5.4): WT-1's three
//! recording scenarios, DM-1's three storage scenarios, WT-4's zero, negative and confirmed-outlier
//! scenarios, WT-5's shared edit and shared deletion, and D1.16/D1.3's stored-`false`,
//! derived-on-read alert.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/weight-tracking/spec.md` and `specs/data-integrity/spec.md`
//! state, and each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::weight` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] methods the commands forward to — the whole behaviour of
//! the command apart from the wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `rabbit_profiles.rs` and `rabbit_lifecycle.rs` do. `app_lib::run()` is never
//! called (it starts the event loop) and `ipc::export_bindings()` is never called (D0.9), as
//! `library_surface.rs` records.
//!
//! Two observables are asserted together, because an entry is only "saved and shared" if both hold:
//!
//! * **What reaches the shared collection.** Every entry write is one `save_item`, so the body's
//!   payload is decoded back through [`ItemPayload`] — the contract's own shape — rather than read out
//!   of a locally composed value. That is the version every member resolves.
//! * **What a member reads afterwards.** The accepted item writes are replayed into the household's
//!   one collection and loaded as another member's copy, then read through
//!   [`AppState::weight_entry`] and [`AppState::weight_trend`] — the methods `get_weight_entry` and
//!   `get_weight_entries` forward to. The replay refuses a write addressed at another collection,
//!   which is the "shared by construction" half of WT-5.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: WT-4's "the person is prompted for a valid value" is asserted as the
//! refusal's `validation` cause, message key and operation identity — the facts the form renders —
//! not as copy. DM-1's kilogram scenario stores the canonical grams directly: the display-unit
//! conversion is the frontend entry boundary's (`parseDisplayWeight` in
//! `src/features/weight/api.ts`, covered by `api.test.ts` and `WeightForm.test.tsx`), and the command
//! is handed grams only, so no display unit ever reaches a command. WT-1's "the timestamp is the
//! moment of creation" is bounded by the clock either side of the call, because the creation instant
//! is minted inside the command body and no clock is injectable there; no test sleeps. WT-7's
//! threshold scenarios are not restated: the one drop here is the vehicle for D1.16/D1.3's
//! stored-versus-derived flag, and WT-7's own derivation is covered where it lives.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, CommandError, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, PLAUSIBILITY_BOUND_GRAMS, Rabbit,
    RabbitProfile, RequestId, SaveCause, Sex, WeightEntry, WeightEntryDraft, WeightEntryId,
    WeightRange,
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

fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

/// The account of the other member whose device resolves the shared collection.
fn other_account() -> AccountUserId {
    AccountUserId("account-2".to_owned())
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
        weight_drop_threshold_percent: 5.0,
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

/// Another member's device: it holds no household book, only the shared collection the writes
/// produced, and reads it through the same local reads the commands use.
fn member_device(service: &Arc<ScriptedService>, household_id: &HouseholdId) -> AppState {
    let state = AppState::new(service.clone(), Some(other_account()));
    load(&state, household_id, shared_rows(service, household_id));

    state
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

/// One entry create through the state method the `create_weight_entry` command forwards to.
fn record(
    state: &AppState,
    rabbit: &Rabbit,
    entry: WeightEntryDraft,
    operation: &str,
) -> Result<WeightEntry, CommandError> {
    block_on_ready(state.create_weight_entry(rabbit.id.clone(), entry, request_id(operation)))
        .map(|(_household, entry)| entry)
}

/// One entry create whose outcome the scenario depends on: the double accepted it.
fn recorded(
    state: &AppState,
    rabbit: &Rabbit,
    entry: WeightEntryDraft,
    operation: &str,
) -> WeightEntry {
    record(state, rabbit, entry, operation).expect("the double accepted the entry")
}

/// One entry correction through the state method the `update_weight_entry` command forwards to.
fn revise(
    state: &AppState,
    entry_id: &WeightEntryId,
    entry: WeightEntryDraft,
    operation: &str,
) -> Result<WeightEntry, CommandError> {
    block_on_ready(state.update_weight_entry(entry_id.clone(), entry, request_id(operation)))
        .map(|(_household, entry)| entry)
}

/// One entry removal through the state method the `delete_weight_entry` command forwards to.
fn remove(state: &AppState, entry_id: &WeightEntryId, operation: &str) -> Result<(), CommandError> {
    block_on_ready(state.delete_weight_entry(entry_id.clone(), request_id(operation)))
        .map(|(_household, _rabbit)| ())
}

/// The last write the service accepted for `entry_id`: the version every member's copy is built from.
fn written(service: &ScriptedService, entry_id: &WeightEntryId) -> Value {
    item_bodies(service)
        .into_iter()
        .rfind(|body| body["item_id"] == json!(entry_id.0))
        .unwrap_or_else(|| panic!("{entry_id:?} was never written"))
}

/// The stored content of `entry_id`, decoded through the contract's own payload shape.
fn stored_entry(service: &ScriptedService, entry_id: &WeightEntryId) -> WeightEntry {
    match serde_json::from_value::<ItemPayload>(written(service, entry_id)["payload"].clone())
        .expect("the write's payload is an item payload the contract models")
    {
        ItemPayload::WeightEntry { weight_entry } => weight_entry,
        other => panic!("the write carries a weight entry, not {other:?}"),
    }
}

/// The field names of one stored entry, sorted so the assertion does not depend on serde's map order.
fn stored_fields(body: &Value) -> Vec<String> {
    let mut fields: Vec<String> = body["payload"]["weight_entry"]
        .as_object()
        .expect("the stored entry is an object")
        .keys()
        .cloned()
        .collect();
    fields.sort();

    fields
}

/// WT-1, "An entry is recorded with the current time".
///
/// **WHEN** a person saves a weight entry without changing the timestamp **THEN** the entry's
/// timestamp is the moment of its creation.
#[test]
fn an_entry_is_recorded_with_the_current_time() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let before = AbsoluteInstant::now();
    let created = recorded(&state, &rabbit, draft(1_800), "operation-3");
    let after = AbsoluteInstant::now();

    // THEN the timestamp is the moment of creation: it is bounded by the clock either side of the
    // call, so a fixed or stale value would fall outside.
    let recorded_at = AbsoluteInstant::parse(&created.timestamp)
        .expect("the defaulted timestamp is an absolute instant");
    assert!(
        before <= recorded_at && recorded_at <= after,
        "the timestamp is the moment of creation, not a fixed value: {recorded_at:?} outside {before:?}..={after:?}"
    );

    // The stored version carries that same instant, and a member reads it back through the detail.
    assert_eq!(
        stored_entry(&service, &created.id).timestamp,
        created.timestamp,
        "the shared version stores the timestamp the command answered"
    );

    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&created.id)
        .expect("the entry is in the shared collection");
    assert_eq!(detail.timestamp, created.timestamp);
}

/// WT-1, "An entry records an edited timestamp".
///
/// **WHEN** a person sets the timestamp of a weight entry to a time in the past and saves **THEN**
/// the entry is stored with that timestamp.
#[test]
fn an_entry_records_an_edited_timestamp() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);
    let past = "2026-10-01T07:30:00Z";

    let created = recorded(&state, &rabbit, draft_at(1_842, past), "operation-3");

    assert_eq!(created.timestamp, at(past), "the supplied instant is kept");
    assert_eq!(
        stored_entry(&service, &created.id).timestamp,
        at(past),
        "the shared version is stored with the past instant, never restamped to the present"
    );

    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&created.id)
        .expect("the entry is in the shared collection");
    assert_eq!(detail.timestamp, at(past));
}

/// WT-1, "The note is optional".
///
/// **WHEN** a person saves a weight entry without a note **THEN** the entry is saved **AND** it
/// carries no note.
#[test]
fn the_note_is_optional() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let created = recorded(&state, &rabbit, draft(1_800), "operation-3");

    // AND it carries no note: absent, not an empty string.
    assert_eq!(created.note, None);
    assert_eq!(
        written(&service, &created.id)["payload"]["weight_entry"]["note"],
        Value::Null,
        "the shared version records the absence as null"
    );
    assert_eq!(stored_entry(&service, &created.id).note, None);

    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&created.id)
        .expect("the entry is in the shared collection");
    assert_eq!(detail.note, None);
}

/// DM-1, "Every stored weight is in grams".
///
/// **WHEN** a weight value is recorded by any path **THEN** the stored value is expressed in grams.
#[test]
fn every_stored_weight_is_in_grams() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    // Both recording paths: the create, then a correction of the same entry. The correction resolves
    // the entry from this device's copy, so the accepted create is loaded first (SYNC-3).
    let created = recorded(&state, &rabbit, draft(1_800), "operation-3");
    refresh(&state, &service, &one);

    let edit = revise(&state, &created.id, draft(2_100), "operation-4");
    let revised = edit.expect("the double accepted the edit");

    // One row, two versions: each write the service accepted carries the same gram figure the person
    // entered, whether it came from a create or a correction.
    let entry_bodies: Vec<Value> = item_bodies(&service)
        .into_iter()
        .filter(|body| body["payload"]["kind"] == json!("weight_entry"))
        .collect();
    assert_eq!(entry_bodies.len(), 2, "the create and the correction");

    for (body, grams) in entry_bodies.iter().zip([1_800, 2_100]) {
        let value = &body["payload"]["weight_entry"]["value_grams"];

        assert_eq!(
            value,
            &json!(grams),
            "the stored value is the gram amount, not a display-unit amount"
        );
        assert!(
            value.is_i64(),
            "grams are stored as a whole number: {value}"
        );
    }

    assert_eq!(revised.value_grams, Grams(2_100));
    assert_eq!(
        stored_entry(&service, &revised.id).value_grams,
        Grams(2_100),
        "the stored version carries the gram equivalent"
    );
}

/// DM-1, "A weight entry carries no unit of its own".
///
/// **WHEN** a weight entry is stored **THEN** it carries a value in the canonical unit and no
/// per-entry unit **AND** no entry can disagree with another entry about the unit of its value.
#[test]
fn a_weight_entry_carries_no_unit_of_its_own() {
    let (service, state, rabbit) = household_with_a_rabbit();

    let first = recorded(&state, &rabbit, draft(1_800), "operation-3");
    let second = recorded(
        &state,
        &rabbit,
        draft_at(1_500, "2026-10-21T09:00:00Z"),
        "operation-4",
    );

    let first_body = written(&service, &first.id);
    let second_body = written(&service, &second.id);
    let first_fields = stored_fields(&first_body);
    let second_fields = stored_fields(&second_body);

    // THEN the entry carries a value in the canonical unit and no per-entry unit.
    assert_eq!(
        first_fields,
        vec![
            "created_at",
            "has_weight_drop_alert",
            "id",
            "note",
            "photo_refs",
            "rabbit_id",
            "timestamp",
            "value_grams",
        ],
        "the stored entry is the canonical value and its facts, with no unit field"
    );
    assert!(
        first_fields
            .iter()
            .all(|field| !field.to_lowercase().contains("unit")),
        "no field names a per-entry unit: {first_fields:?}"
    );

    // AND no entry can disagree with another about the unit of its value: both stored versions have
    // the same fields and each value is the bare gram count.
    assert_eq!(
        second_fields, first_fields,
        "every stored entry carries the same fields, so no two entries can name different units"
    );
    assert_eq!(
        first_body["payload"]["weight_entry"]["value_grams"],
        json!(1_800)
    );
    assert_eq!(
        second_body["payload"]["weight_entry"]["value_grams"],
        json!(1_500)
    );
}

/// DM-1, "A value entered in kilograms is stored as grams".
///
/// **WHEN** the display unit is kilograms and a person enters a value in that unit and saves **THEN**
/// the entered value is converted to grams **AND** the stored value is the gram equivalent.
#[test]
fn a_value_entered_in_kilograms_is_stored_as_grams() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    // The display-unit conversion is the frontend entry boundary's, not this layer's:
    // `parseDisplayWeight` in `src/features/weight/api.ts` turns the entered number into canonical
    // grams (digit-based, refusing sub-gram precision) and is covered by `api.test.ts`'s
    // `parseDisplayWeight('2.5', 'kilograms')` → 2500 and by `WeightForm.test.tsx`. The command is
    // handed the gram amount only, so 2.5 kg arrives here as 2500 g and is recorded as such.
    let created = recorded(&state, &rabbit, draft(2_500), "operation-3");

    // THEN the entered value is converted to grams, and the stored value is the gram equivalent.
    assert_eq!(created.value_grams, Grams(2_500));
    assert_eq!(
        written(&service, &created.id)["payload"]["weight_entry"]["value_grams"],
        json!(2_500),
        "2.5 kg is stored as the gram equivalent, never as the kilogram figure"
    );

    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&created.id)
        .expect("the entry is in the shared collection");
    assert_eq!(detail.value_grams, Grams(2_500));
}

/// WT-4, "A zero value is rejected".
///
/// **WHEN** a person submits a weight value of zero **THEN** the entry is not saved **AND** the
/// person is prompted for a valid value.
#[test]
fn a_zero_value_is_rejected() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);
    let operation = request_id("operation-3");

    let attempt = record(&state, &rabbit, draft(0), "operation-3");
    let error = attempt.expect_err("a zero value names no weight");

    // AND the person is prompted for a valid value: a validation refusal carrying the operation's
    // identity, which is the fact the form renders.
    assert_eq!(error.cause, SaveCause::Validation);
    assert_eq!(error.message_key, "error.validation");
    assert!(!error.retryable);
    assert_eq!(error.request_id, Some(operation));

    // THEN the entry is not saved: no item write was submitted and the rabbit's history is empty.
    assert!(
        item_bodies(&service)
            .iter()
            .all(|body| body["payload"]["kind"] != json!("weight_entry")),
        "no weight entry reached the service"
    );

    let member = member_device(&service, &one);
    let now = AbsoluteInstant::now().to_wire();
    let trend = member
        .weight_trend(&rabbit.id, WeightRange::All, &now)
        .expect("the rabbit's history is readable");
    assert!(trend.points.is_empty(), "no entry exists to present");
}

/// WT-4, "A negative value is rejected".
///
/// **WHEN** a person submits a negative weight value **THEN** the entry is not saved **AND** the
/// person is prompted for a valid value.
#[test]
fn a_negative_value_is_rejected() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);
    let operation = request_id("operation-3");

    let attempt = record(&state, &rabbit, draft(-250), "operation-3");
    let error = attempt.expect_err("a negative value names no weight");

    assert_eq!(error.cause, SaveCause::Validation);
    assert_eq!(error.message_key, "error.validation");
    assert!(!error.retryable);
    assert_eq!(error.request_id, Some(operation));

    assert!(
        item_bodies(&service)
            .iter()
            .all(|body| body["payload"]["kind"] != json!("weight_entry")),
        "no weight entry reached the service"
    );

    let member = member_device(&service, &one);
    let now = AbsoluteInstant::now().to_wire();
    let trend = member
        .weight_trend(&rabbit.id, WeightRange::All, &now)
        .expect("the rabbit's history is readable");
    assert!(trend.points.is_empty(), "no entry exists to present");
}

/// WT-4, "A confirmed outlier is saved".
///
/// **WHEN** a person confirms a value above the plausibility bound **THEN** the entry is saved with
/// that value. The refusal before the confirmation is part of the same scenario's precondition: the
/// value is not saved until the person confirms it.
#[test]
fn a_confirmed_outlier_is_saved() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);
    let outlier = Grams(PLAUSIBILITY_BOUND_GRAMS.0 + 1);

    // Until the confirmation is given, the entry is not saved.
    let attempt = record(&state, &rabbit, draft(outlier.0), "operation-3");
    let error = attempt.expect_err("an unconfirmed outlier is not saved");
    assert_eq!(error.cause, SaveCause::Validation);
    assert!(
        item_bodies(&service)
            .iter()
            .all(|body| body["payload"]["kind"] != json!("weight_entry")),
        "the unconfirmed value reached no collection"
    );

    // THEN the entry is saved with that value once the person confirms it — stored as submitted and
    // never clamped to the bound.
    let saved = recorded(
        &state,
        &rabbit,
        WeightEntryDraft {
            outlier_confirmed: true,
            ..draft(outlier.0)
        },
        "operation-4",
    );

    assert_eq!(saved.value_grams, outlier);
    assert_eq!(
        written(&service, &saved.id)["payload"]["weight_entry"]["value_grams"],
        json!(outlier.0),
        "the shared version carries the confirmed value itself"
    );

    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&saved.id)
        .expect("the confirmed entry is in the shared collection");
    assert_eq!(detail.value_grams, outlier);
}

/// WT-5, "A past entry is corrected and the change is shared".
///
/// **WHEN** a person edits the value of a past weight entry and saves **THEN** the entry is stored
/// with the edited value **AND** every member of the household resolves the edited entry.
#[test]
fn a_past_entry_is_corrected_and_the_change_is_shared() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let past = recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    let revised = revise(
        &state,
        &past.id,
        WeightEntryDraft {
            note: Some("corrected".to_owned()),
            ..draft(1_900)
        },
        "operation-5",
    )
    .expect("the double accepted the edit");

    // THEN the entry is stored with the edited value: the edit replaces the entry's existing item,
    // so there is one row, not a second entry and not one row per member.
    assert_eq!(revised.id, past.id);
    let stored = written(&service, &past.id);
    assert_eq!(stored["item_id"], json!(past.id.0.clone()));
    assert_eq!(
        stored["payload"]["weight_entry"]["value_grams"],
        json!(1_900)
    );
    assert_eq!(
        stored["payload"]["weight_entry"]["timestamp"],
        json!(past.timestamp.0),
        "an omitted timestamp keeps the past entry's own instant (WT-5)"
    );

    // AND every member resolves the edited entry: another member's copy is the shared collection
    // replayed from the accepted writes, and the detail read answers the edited value and note.
    let member = member_device(&service, &one);
    let resolved = member
        .weight_entry(&past.id)
        .expect("the edited entry is in the shared collection");
    assert_eq!(resolved.value_grams, Grams(1_900));
    assert_eq!(resolved.note.as_deref(), Some("corrected"));
    assert_eq!(resolved.timestamp, past.timestamp);
}

/// WT-5, "A past entry is deleted and the change is shared".
///
/// **WHEN** a person deletes a past weight entry and confirms **THEN** the entry is no longer present
/// for any member of the household.
#[test]
fn a_past_entry_is_deleted_and_the_change_is_shared() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let past = recorded(
        &state,
        &rabbit,
        draft_at(2_000, "2026-10-20T09:00:00Z"),
        "operation-3",
    );
    let later = recorded(
        &state,
        &rabbit,
        draft_at(1_800, "2026-10-21T09:00:00Z"),
        "operation-4",
    );
    refresh(&state, &service, &one);

    remove(&state, &past.id, "operation-5").expect("the double accepted the deletion");

    // The removal is one tombstone of the entry's own item, written into the shared collection, so it
    // is the same row every member resolves.
    let body = written(&service, &past.id);
    assert_eq!(body["item_id"], json!(past.id.0.clone()));
    assert_eq!(body["deleted"], json!(true));

    // THEN the entry is no longer present for any member: the detail read refuses it as an unknown
    // entry, and the other entry is untouched.
    let member = member_device(&service, &one);
    let error = member
        .weight_entry(&past.id)
        .expect_err("a deleted entry no longer resolves");
    assert_eq!(error.cause, SaveCause::Validation);
    assert_eq!(
        error.request_id, None,
        "a read carries no operation identity"
    );

    let remaining = member
        .weight_entry(&later.id)
        .expect("only the addressed entry was removed");
    assert_eq!(remaining.value_grams, Grams(1_800));

    let now = AbsoluteInstant::now().to_wire();
    let history = member
        .weight_trend(&rabbit.id, WeightRange::All, &now)
        .expect("the rabbit's history is readable");
    assert_eq!(
        history.points.len(),
        1,
        "the deleted entry is absent from the history every member resolves"
    );
    assert_eq!(history.points[0].id, later.id);
}

/// D1.16/D1.3: the stored payload carries `has_weight_drop_alert: false` on write, and a read
/// re-derives the flag from the entry set.
///
/// The second entry records a 10% drop against the first, so the derivation says "flagged" while the
/// written version says `false` — a read that echoed the stored flag could not answer `true`.
#[test]
fn the_stored_alert_is_written_false_and_a_read_derives_it() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let previous = recorded(
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

    // The stored payload carries the constant, never the derived flag.
    assert_eq!(
        written(&service, &dropped.id)["payload"]["weight_entry"]["has_weight_drop_alert"],
        json!(false),
        "the write states the constant value (D1.16)"
    );
    assert_eq!(
        written(&service, &previous.id)["payload"]["weight_entry"]["has_weight_drop_alert"],
        json!(false),
        "the predecessor's write carries the same constant (D1.16)"
    );
    assert!(
        !stored_entry(&service, &dropped.id).has_weight_drop_alert,
        "the shared version stores no alert"
    );

    // A read re-derives it: 2000 g to 1800 g is a 10% drop beyond the rabbit's 5% threshold, so the
    // detail is flagged although the stored entry says false.
    let member = member_device(&service, &one);
    let detail = member
        .weight_entry(&dropped.id)
        .expect("the entry is in the shared collection");
    assert!(
        detail.has_weight_drop_alert,
        "the flag is re-derived on read (D1.3)"
    );
    assert_eq!(detail.value_grams, Grams(1_800));

    let first = member
        .weight_entry(&previous.id)
        .expect("the first entry is in the shared collection");
    assert!(
        !first.has_weight_drop_alert,
        "the first entry has no predecessor and is never flagged"
    );
}
