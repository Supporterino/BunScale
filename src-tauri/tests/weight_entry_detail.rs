//! Integration tests for the weight entry detail read (task 5.8): WT-8's two read scenarios — the
//! value and timestamp with the whole untruncated note, and the derived drop-alert flag.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-rabbit-weight-core-loop/specs/weight-tracking/spec.md` (WT-8) states, and
//! each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::weight` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so these tests drive the same [`AppState`] methods the commands forward to — the whole behaviour of
//! the command apart from the wrapper — with the published [`ScriptedService`] double as the
//! transport, exactly as `weight_entries.rs` (task 5.4) and `drop_alert.rs` (task 5.5) do.
//! `get_weight_entry` forwards to [`AppState::weight_entry`], which is the read seam every assertion
//! below goes through. `app_lib::run()` is never called (it starts the event loop) and
//! `ipc::export_bindings()` is never called (D0.9), as `library_surface.rs` records.
//!
//! The entry reaches this device the way it reaches every member: the create is one `save_item` into
//! the household's shared collection, and this device's copy is loaded from those accepted writes
//! ([`refresh`]). No test asserts a locally composed value — the note and the flag are read back from
//! the copy the read path resolves.
//!
//! Both platforms (TOOL-1): nothing here is platform-specific.
//!
//! # Fixed instants
//!
//! Every reading timestamp is a fixed past instant, so the ordering the derivation uses is fixed. The
//! one clock read is each entry's `created_at`, minted inside the create body and not injectable
//! (D1.12, `AbsoluteInstant::now`). No test sleeps and no test asserts wall-clock time.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved. WT-8's "shown in the person's display unit" is a frontend formatting
//! concern: the read returns canonical grams and no command applies a unit (D1.2, I1), and
//! `src/features/weight/api.ts` formats those grams at the boundary. The Rust half — which is what
//! this file can observe — is that the detail hands that formatter the exact stored grams, the stored
//! timestamp and the note whole, so a conversion cannot be applied twice or fed a rounded value.
//! The drop-flag scenario asserts the flag the view renders, never copy or markup; WT-7's arithmetic
//! and its "exceeds" boundary are covered where they live (`drop_alert.rs`, 5.5).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, Grams, HouseholdCollection, HouseholdId,
    Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitProfile, RequestId, Sex, WeightEntry,
    WeightEntryDraft, WeightEntryId,
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

/// The operation that creates the rabbit every test starts from.
const CREATE_RABBIT: &str = "operation-2";

/// WT-7's documented default, in percent (RAB-1/RAB-5): the threshold the detail derives against.
const DEFAULT_THRESHOLD_PERCENT: f64 = 5.0;

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

/// The same draft with the person's own reading instant, which may be in the past.
fn draft_at(value_grams: i32, timestamp: &str) -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(value_grams),
        timestamp: Some(at(timestamp)),
        note: None,
        outlier_confirmed: false,
    }
}

/// The same draft with a note, as the form submits the whole editable set.
fn draft_noted(value_grams: i32, timestamp: &str, note: &str) -> WeightEntryDraft {
    WeightEntryDraft {
        note: Some(note.to_owned()),
        ..draft_at(value_grams, timestamp)
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
        // D1.17: a non-removal write omits `deleted` and lets `save_item` default it to `false`.
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

/// This device's copy, refreshed from the writes the service accepted — the version every member
/// resolves, and the copy every read below runs against.
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

/// The last write the service accepted for `entry_id`: the version every member's copy is built from.
fn written(service: &ScriptedService, entry_id: &WeightEntryId) -> Value {
    item_bodies(service)
        .into_iter()
        .rfind(|body| body["item_id"] == json!(entry_id.0))
        .unwrap_or_else(|| panic!("{entry_id:?} was never written"))
}

/// One entry's detail through the state method the `get_weight_entry` command forwards to, resolved
/// from `state`'s own copy.
fn detail_of(state: &AppState, entry_id: &WeightEntryId) -> WeightEntry {
    state
        .weight_entry(entry_id)
        .expect("the entry is in this device's copy")
}

/// WT-8, "The detail shows the value in the display unit and the untruncated note".
///
/// **WHEN** a weight entry with a long note is opened in detail **THEN** the value is shown in the
/// person's display unit **AND** the timestamp is shown **AND** the whole note is shown without
/// truncation.
///
/// The display unit is applied by `src/features/weight/api.ts` (D1.2, I1), so the facts this read
/// owns are the ones asserted: the exact stored grams — not a rounded or unit-scaled figure — the
/// stored timestamp, and the note whole. The note is deliberately far longer than any preview or
/// one-line clamp, carries several line breaks, doubled spaces and punctuation, and ends with a
/// sentence only the complete note has.
#[test]
fn the_detail_shows_the_value_in_the_display_unit_and_the_untruncated_note() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let note = "Weighed after the long run, before breakfast.\n\
                The vet asked us to keep a weekly log, so this note is deliberately long: it names \
                the pellets, the hay, the water and the hour of every meal, because one short line \
                could not tell the two of us apart when we read it back.\n\
                Punctuation survives too — commas, semicolons; colons: parentheses (like this), an \
                em dash, an ellipsis… and a pair of \"quotes\".\n\
                Spacing is kept:  two spaces here,   three here.\n\
                Where the two of us differ is everything else: the time of the weighing, the mood of \
                the rabbit, whether the pellets were finished and whether the run was long or short, \
                all of which we agreed to write down rather than guess from a chart.\n\
                The last line of the note must still be present, so any truncation — a line clamp, a \
                character budget or a preview cut — is visible in the assertion below.";

    // The scenario's premise, asserted rather than assumed: the note is far longer than a list
    // preview or a single clamped line, so an equality below cannot pass by accident.
    assert!(
        note.chars().count() > 700,
        "the note is long enough for a truncation to be visible, was {}",
        note.chars().count()
    );

    let created = recorded(
        &state,
        &rabbit,
        draft_noted(1_843, "2026-10-21T09:00:00Z", note),
        "operation-3",
    );
    refresh(&state, &service, &one);

    let detail = detail_of(&state, &created.id);

    // The value is the stored grams: 1843 g, not a rounded 1.8 kg, not a re-scaled figure. That is the
    // exact number `api.ts` formats into the person's display unit, so the conversion cannot be
    // applied twice or fed a value the entry never held.
    assert_eq!(
        detail.value_grams,
        Grams(1_843),
        "the detail carries the exact stored grams the display unit formats"
    );
    assert_eq!(
        written(&service, &created.id)["payload"]["weight_entry"]["value_grams"],
        json!(1_843),
        "the shared collection stores the same gram figure the detail answers"
    );

    assert_eq!(
        detail.timestamp,
        at("2026-10-21T09:00:00Z"),
        "the detail carries the entry's stored timestamp"
    );

    // AND the whole note is shown without truncation: byte-for-byte equal, line breaks, doubled
    // spaces and punctuation included, and its last sentence still present.
    assert_eq!(
        detail.note.as_deref(),
        Some(note),
        "the note is returned whole, with its line breaks, spacing and full length"
    );
    assert_eq!(
        detail.note.as_deref().map(|note| note.chars().count()),
        Some(note.chars().count()),
        "no character of the note is dropped on the way through the read"
    );
    assert_eq!(
        detail
            .note
            .as_deref()
            .map(|note| note.matches('\n').count()),
        Some(5),
        "every line break of the note survives"
    );
    assert!(
        detail
            .note
            .as_deref()
            .is_some_and(|note| note.contains("Spacing is kept:  two spaces here,   three here.")),
        "the note's doubled spacing is not collapsed"
    );
    assert!(
        detail
            .note
            .as_deref()
            .is_some_and(|note| note.ends_with("visible in the assertion below.")),
        "the last line of the note survives every truncation shorter than the whole note"
    );
}

/// WT-8, "The drop-alert flag appears in the detail when it applies".
///
/// **WHEN** a weight entry that is flagged by a drop beyond the rabbit's threshold is opened in
/// detail **THEN** the drop-alert flag is presented in the detail.
///
/// The same composition `drop_alert.rs` (5.5) proves: a 5% threshold, a previous entry of 2000 g and
/// a new entry of 1800 g — a 10% drop. The stored payloads write the constant `false` (D1.16), so a
/// detail answering `true` can only be the derivation over the rabbit's entry set (D1.3, D1.13).
/// The entry with no predecessor is asserted too: the flag is absent when it does not apply.
#[test]
fn the_drop_alert_flag_appears_in_the_detail_when_it_applies() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    assert_eq!(
        rabbit.weight_drop_threshold_percent, DEFAULT_THRESHOLD_PERCENT,
        "the rabbit is judged against the default 5% threshold"
    );

    let before = recorded(
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

    // D1.16/D1.3: the alert is not content. Both stored versions write `false`, so the flag below is
    // derived on read rather than echoed from the shared collection.
    assert_eq!(
        written(&service, &dropped.id)["payload"]["weight_entry"]["has_weight_drop_alert"],
        json!(false),
        "the stored entry writes the constant `false`"
    );
    assert_eq!(
        written(&service, &before.id)["payload"]["weight_entry"]["has_weight_drop_alert"],
        json!(false),
        "the stored entry writes the constant `false`"
    );

    // THEN the drop-alert flag is presented in the detail: 2000 g to 1800 g is a 10% drop, beyond 5%.
    assert!(
        detail_of(&state, &dropped.id).has_weight_drop_alert,
        "a 10% drop beyond the 5% threshold is flagged in the entry's detail"
    );

    // AND the flag is absent for an entry the derivation does not flag: the first entry has no
    // predecessor to drop from, so its stored `false` is also the derived answer.
    assert!(
        !detail_of(&state, &before.id).has_weight_drop_alert,
        "an entry with no predecessor is not flagged in its detail"
    );
}
