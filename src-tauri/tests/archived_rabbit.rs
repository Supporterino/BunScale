//! Integration tests for archived-rabbit immutability and retention (task 5.3): RAB-8's "No new entry
//! can be added to an archived rabbit" and "Archived entries cannot be edited or deleted", RAB-9's
//! "Archived history survives an arbitrary interval" and "Archiving discards nothing", and DM-4's
//! "No weight data of an archived rabbit is mutable" and "The archive is retrievable and its data is
//! retained by default".
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). Each test is named after the scenario it covers.
//!
//! # The boundary these tests drive
//!
//! The `ipc::commands::weight` adapters are private bodies over [`AppState`]: the `#[tauri::command]`
//! wrapper takes `tauri::State<'_, AppState>`, which has no public constructor outside a running app,
//! so an integration test cannot call a command body. These tests drive the same [`AppState`] methods
//! the bodies forward to — the level those bodies' own `#[cfg(test)]` tests call, with the wrapper's
//! state extraction and change-event publisher left out — and the published [`ScriptedService`]
//! double as the transport, exactly as `rabbit_lifecycle.rs` (5.2) does:
//!
//! * create — [`AppState::create_weight_entry`];
//! * edit — [`AppState::update_weight_entry`];
//! * delete — [`AppState::delete_weight_entry`].
//!
//! # The scenarios, and what was actually missing
//!
//! The three entry-write refusals already exist, on archived rows a fixture flagged:
//! `ipc::commands::weight`'s `a_create_for_an_archived_rabbit_is_refused_and_writes_and_publishes_nothing`,
//! `an_edit_for_an_archived_rabbits_entry_is_refused_and_writes_and_publishes_nothing` and
//! `a_delete_for_an_archived_rabbits_entry_is_refused_and_writes_and_publishes_nothing`, which also
//! assert that a refusal publishes no change event; and `state.rs`'s
//! `an_archived_rabbit_refuses_the_write_that_would_add_a_weight_entry`,
//! `an_archived_rabbit_refuses_the_write_that_would_edit_or_delete_an_entry` (both the shared
//! boundary helper alone) and `an_archived_rabbits_entry_cannot_be_edited_or_deleted` (the edit and
//! the delete through their write methods). **None of them ran the three writes on a rabbit and entry
//! set the product itself archived.** That is this file's addition: the rabbit and its entries are
//! recorded by the write path, archived by [`AppState::archive_rabbit`] and refreshed, and the writes
//! are then refused on that copy.
//!
//! Retention is asserted at two layers, and the two are complements rather than repeats. `state.rs`
//! reaches archived through [`AppState::archive_rabbit`] on hand-built rows
//! (`archiving_writes_the_whole_rabbit_item_with_only_the_flag_changed`, and
//! `archiving_discards_no_entry_and_the_archived_history_survives_a_relaunch`, which survives a real
//! disk relaunch — the stronger claim about "a long time after archiving", which this file does not
//! repeat). This file adds the write-path half: entries the create write recorded are byte-identical
//! in the shared collection once the product archives their rabbit, and remain so through the reads.
//!
//! # What the tests observe
//!
//! The read path's copy is only written by the read path (SYNC-3), so a write is invisible on this
//! device until a refresh. Each test therefore replays every accepted `save_item` write, in call
//! order, into the one household collection the service would hold — the shared data every member
//! resolves — and loads it as this device's copy. "Nothing was written" is asserted by comparing that
//! shared collection, as JSON, byte for byte before and after a refused call, and by the transport
//! recording no new call at all. The publication a command wrapper performs on success is not
//! observable from here; its absence on a refusal is the command-layer test's claim, and this file's
//! "nothing was sent" is the precondition for it.
//!
//! "Byte-identical" is `serde_json`'s canonical serialisation of the rows: identity, version marker,
//! tombstone flag and payload included — not just the values that happen to be asserted.
//!
//! # Two tests share the edit/delete pair, deliberately
//!
//! `archived_entries_cannot_be_edited_or_deleted` and `no_weight_data_of_an_archived_rabbit_is_mutable`
//! both refuse an edit and a deletion on this fixture. They are kept apart because each names a
//! different scenario and each carries a distinct claim: RAB-8's test refuses both writes on **both**
//! entries and compares the whole collection and every entry's detail once at the end, while DM-4's
//! sweep adds the **create** — the "from any entry point" claim — and compares the collection after
//! **each** attempt, so an attempt that "succeeded" before a later one is still caught. Merging them
//! would leave one of the two scenarios without a test named after it.
//!
//! # What is deliberately not asserted here
//!
//! "No control that creates, edits or deletes is available" is a rendering decision, and it belongs
//! to the component tasks (5.11, 5.13); the Rust-observable guarantee behind it is the refusal this
//! file asserts. DM-4's "Archived health and medication data are not mutable" belongs to the phases
//! that own health and medication data, as the delta states. Both platforms (TOOL-1): nothing here is
//! platform-specific.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::domain::SAVE_ITEM_RPC;
use app_lib::domain::rabbit::summaries;
use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, CommandError, Grams, HouseholdCollection,
    HouseholdId, Instant, ItemId, ItemPayload, NeuterStatus, Rabbit, RabbitId, RabbitProfile,
    RequestId, SaveCause, Sex, WeightEntry, WeightEntryDraft, WeightRange,
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

/// The moment the history is read at before the archive; after every stored entry.
const NOW: &str = "2026-10-23T09:00:00Z";

/// The moment the archived history is opened at — five years after the entries were recorded.
const A_LONG_TIME_LATER: &str = "2031-10-23T09:00:00Z";

/// A clock **before** every stored entry. `WeightRange::All` is unbounded, so a read at this instant
/// still returns both entries: the assertion that the read-side clock cannot hide stored history.
const BEFORE_EVERY_ENTRY: &str = "2020-01-01T00:00:00Z";

/// The operation that creates the rabbit every retention test starts from.
const CREATE_RABBIT: &str = "operation-2";

/// The two entries recorded before archiving. An entry's create operation is also its identity and
/// the row it is stored under (WT-1, D0.14).
const FIRST_ENTRY: &str = "operation-3";
const SECOND_ENTRY: &str = "operation-4";

/// The archive that makes the rabbit read-only.
const ARCHIVE: &str = "operation-5";

/// Which of the two entry writes on an existing entry is being attempted.
#[derive(Clone, Copy)]
enum EntryWrite {
    Edit,
    Delete,
}

impl EntryWrite {
    fn name(self) -> &'static str {
        match self {
            Self::Edit => "the edit",
            Self::Delete => "the deletion",
        }
    }
}

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

/// A rabbit profile supplying the RAB-1 field set, with the default 5% threshold. Every optional field
/// carries a value, so a write that dropped one is visible in the stored row (task 5.3, finding 4).
fn profile() -> RabbitProfile {
    RabbitProfile {
        name: "Pebble".to_owned(),
        photo_ref: Some("photo-1".to_owned()),
        breed: Some("Mini Lop".to_owned()),
        dob: Some(at("2026-03-01T00:00:00Z")),
        dob_unknown: false,
        sex: Sex::Female,
        neuter_status: NeuterStatus::Spayed,
        adoption_date: Some(at("2026-05-01T00:00:00Z")),
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
) -> Rabbit {
    block_on_ready(state.create_rabbit(household_id.clone(), profile, request_id(operation)))
        .expect("the double accepted the create")
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
/// replay fails if an item write addresses another collection.
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

/// The refresh that follows an accepted write: the service's one collection, loaded as this device's
/// copy.
fn refresh(state: &AppState, service: &ScriptedService, household_id: &HouseholdId) {
    load(state, household_id, shared_rows(service, household_id));
}

/// The weight-entry rows of the household's shared collection, in item-id order.
fn entry_rows(service: &ScriptedService, household_id: &HouseholdId) -> Vec<CollectionItem> {
    shared_rows(service, household_id)
        .into_iter()
        .filter(|row| matches!(&row.payload, Some(ItemPayload::WeightEntry { .. })))
        .collect()
}

/// The stored entry rows, serialised so two readings compare byte for byte — identity, version
/// marker, tombstone flag and payload included.
fn stored_entries(service: &ScriptedService, household_id: &HouseholdId) -> Value {
    json_of(entry_rows(service, household_id))
}

/// The stored content of one rabbit's row: the profile the household's shared collection holds for it,
/// not the read path's copy.
fn stored_profile(
    service: &ScriptedService,
    household_id: &HouseholdId,
    rabbit_id: &RabbitId,
) -> Rabbit {
    let row = shared_rows(service, household_id)
        .into_iter()
        .find(|row| row.item_id == ItemId(rabbit_id.0.clone()))
        .expect("the rabbit's row is stored");
    assert!(!row.deleted, "the rabbit's row is live");

    match row.payload {
        Some(ItemPayload::Rabbit { rabbit }) => rabbit,
        _ => panic!("the rabbit's row carries its content"),
    }
}

/// One contract value in the JSON two readings are compared in. The serialisation has sorted keys,
/// so it is stable and a comparison is a byte comparison of the whole value.
fn json_of(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("the contract value serialises")
}

/// A state holding the household's one active rabbit, with the copy the read path delivers after its
/// create.
fn household_with_a_rabbit() -> (Arc<ScriptedService>, AppState, Rabbit) {
    let (service, state) = held_household();
    let one = household(HOUSEHOLD);
    let rabbit = create_rabbit(&state, &one, profile(), CREATE_RABBIT);

    refresh(&state, &service, &one);

    (service, state, rabbit)
}

/// The fixture four scenarios start from: the transport, this device, the archived rabbit, and the
/// entries it owns in creation order.
type ArchivedRabbit = (Arc<ScriptedService>, AppState, Rabbit, Vec<WeightEntry>);

/// A state holding the household's one **archived** rabbit and the two entries it owns, with the copy
/// the read path delivers after the archive.
fn household_with_an_archived_rabbit_and_entries() -> ArchivedRabbit {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let first = create_entry(
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
    refresh(&state, &service, &one);

    block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(ARCHIVE)))
        .expect("the double accepted the archive");
    refresh(&state, &service, &one);

    (service, state, rabbit, vec![first, second])
}

/// Assert one refusal is the contract's: `validation`, the operation's identity, not retryable.
fn assert_refused(error: &CommandError, operation: &RequestId, what: &str) {
    assert_eq!(error.cause, SaveCause::Validation, "{what}");
    assert_eq!(error.message_key, "error.validation", "{what}");
    assert!(!error.retryable, "{what}");
    assert_eq!(error.request_id.as_ref(), Some(operation), "{what}");
}

/// Assert one entry write on an archived rabbit's entry is refused and sends nothing.
fn assert_entry_write_refused(
    state: &AppState,
    service: &ScriptedService,
    entry: &WeightEntry,
    operation: &str,
    write: EntryWrite,
) {
    let calls_before = service.calls();
    let operation_id = request_id(operation);

    let error = match write {
        EntryWrite::Edit => {
            let edit = state.update_weight_entry(
                entry.id.clone(),
                rewritten_draft(),
                operation_id.clone(),
            );
            block_on_ready(edit).expect_err("an archived rabbit's entry is read-only")
        }
        EntryWrite::Delete => {
            let delete = state.delete_weight_entry(entry.id.clone(), operation_id.clone());
            block_on_ready(delete).expect_err("an archived rabbit's entry is read-only")
        }
    };

    assert_refused(&error, &operation_id, write.name());
    assert_eq!(service.calls(), calls_before, "nothing was probed or sent");

    // The refusal has to be the archived target, not an entry that is gone: `live_entry` refuses an
    // absent, tombstoned or wrongly-keyed row with the same cause, key and identity, so the helper
    // alone would read a vanished entry as a refusal. The entry must still read back as recorded —
    // every field but the derived flag, which the read re-derives (WT-7, D1.13).
    let read_back = state
        .weight_entry(&entry.id)
        .expect("the refused write left the entry in place");
    // The alert flag is derived on read (WT-7, D1.13), so it is the one field the recorded entry
    // cannot supply; every other field must be exactly what the create recorded.
    let mut expected = entry.clone();
    expected.has_weight_drop_alert = read_back.has_weight_drop_alert;
    assert_eq!(
        read_back, expected,
        "the entry still reads back as recorded"
    );
}

/// The correction an archived rabbit's entry must refuse: a different value, note and timestamp rule.
fn rewritten_draft() -> WeightEntryDraft {
    WeightEntryDraft {
        value_grams: Grams(1_000),
        timestamp: None,
        note: Some("rewritten".to_owned()),
        outlier_confirmed: false,
    }
}

/// RAB-8, "No new entry can be added to an archived rabbit".
///
/// **WHEN** an archived rabbit's timeline is presented **THEN** no control that creates a new weight
/// entry is available for it.
///
/// "No control" is the component tasks' half (5.11, 5.13). The guarantee behind it is asserted here
/// at the entry-write level: the create refuses with the contract's `validation` cause and the
/// operation's identity, before anything is probed or sent, and neither the stored entries nor the
/// timeline the surface renders changes.
#[test]
fn no_new_entry_can_be_added_to_an_archived_rabbit() {
    let (service, state, rabbit, _entries) = household_with_an_archived_rabbit_and_entries();
    let one = household(HOUSEHOLD);
    let stored_before = stored_entries(&service, &one);
    let calls_before = service.calls();
    let operation = request_id("operation-6");

    let error = block_on_ready(state.create_weight_entry(
        rabbit.id.clone(),
        WeightEntryDraft {
            value_grams: Grams(1_900),
            timestamp: Some(at("2026-10-22T09:00:00Z")),
            note: Some("after the archive".to_owned()),
            outlier_confirmed: false,
        },
        operation.clone(),
    ))
    .expect_err("an archived rabbit's timeline cannot grow");

    assert_refused(&error, &operation, "the create");
    assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    assert_eq!(
        stored_entries(&service, &one),
        stored_before,
        "no entry row was written"
    );

    // The refusal is not a presentation accident: the rabbit is still archived, and its timeline is
    // still the one recorded before archiving.
    let archived = state
        .rabbit(&rabbit.id)
        .expect("the archived rabbit stays readable");
    assert!(archived.is_archived);
    let trend = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the archived timeline is readable");
    assert_eq!(trend.points.len(), 2, "the created entry is not there");
}

/// RAB-8, "Archived entries cannot be edited or deleted".
///
/// **WHEN** an entry of an archived rabbit is presented **THEN** no control that edits or deletes it
/// is available.
///
/// As above, the visible absence of the controls is 5.13's; both entry writes are refused here for
/// each stored entry, and the rows are byte-identical afterwards, so nothing a person could have
/// submitted — including the edit's value and note — reached storage.
#[test]
fn archived_entries_cannot_be_edited_or_deleted() {
    let (service, state, rabbit, entries) = household_with_an_archived_rabbit_and_entries();
    let one = household(HOUSEHOLD);
    let stored_before = stored_entries(&service, &one);
    let details_before: Vec<Value> = entries
        .iter()
        .map(|entry| {
            let detail = state
                .weight_entry(&entry.id)
                .expect("the archived entry detail is readable");
            json_of(detail)
        })
        .collect();

    assert_entry_write_refused(
        &state,
        &service,
        &entries[0],
        "operation-6",
        EntryWrite::Edit,
    );
    assert_entry_write_refused(
        &state,
        &service,
        &entries[0],
        "operation-7",
        EntryWrite::Delete,
    );
    assert_entry_write_refused(
        &state,
        &service,
        &entries[1],
        "operation-8",
        EntryWrite::Edit,
    );
    assert_entry_write_refused(
        &state,
        &service,
        &entries[1],
        "operation-9",
        EntryWrite::Delete,
    );

    assert_eq!(
        stored_entries(&service, &one),
        stored_before,
        "no entry row was written"
    );
    for (entry, expected) in entries.iter().zip(details_before) {
        let detail = state
            .weight_entry(&entry.id)
            .expect("the archived entry detail is still readable");
        assert_eq!(
            json_of(detail),
            expected,
            "the entry reads back as it was recorded"
        );
    }

    let archived = state
        .rabbit(&rabbit.id)
        .expect("the archived rabbit stays readable");
    assert!(archived.is_archived);
}

/// RAB-9, "Archiving discards nothing".
///
/// **WHEN** a rabbit with weight entries is archived **THEN** every entry it had before archiving is
/// still stored afterwards.
///
/// The archive is one item write that names the rabbit alone, and the stored entry rows are
/// byte-for-byte the ones recorded before it. The profile is written back whole, so the archive
/// changes the flag and nothing else — the reason D1.16 makes the lifecycle writes whole-item writes.
#[test]
fn archiving_discards_nothing() {
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

    let stored_before = stored_entries(&service, &one);
    assert_eq!(entry_rows(&service, &one).len(), 2);
    let profile_before = stored_profile(&service, &one, &rabbit.id);
    let writes_before = item_bodies(&service).len();

    let archived = block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(ARCHIVE)))
        .expect("the double accepted the archive");
    assert!(archived.is_archived);

    // The archive is one item write, of the rabbit's item: no entry is named, so no entry can be
    // replaced by it.
    let bodies = item_bodies(&service);
    assert_eq!(bodies.len(), writes_before + 1, "the archive is one write");
    assert_eq!(bodies[writes_before]["item_id"], json!(rabbit.id.0));
    assert_eq!(
        stored_entries(&service, &one),
        stored_before,
        "every entry recorded before archiving is still stored, unchanged"
    );

    // The rabbit's own content is written back whole: every RAB-1 field — the photo reference, the
    // breed, the date of birth and its unknown marker, the sex, the neuter status, the adoption date
    // and the threshold — is the value recorded before the archive, so the flag is the only
    // difference. A partial payload that dropped any one of them fails here. The item's `updated_at`
    // is the write's version marker rather than content, and an archive legitimately advances it
    // (D1.16).
    assert_eq!(
        stored_profile(&service, &one, &rabbit.id),
        Rabbit {
            is_archived: true,
            ..profile_before
        },
        "the archive changes the flag and no other field of the stored profile"
    );
}

/// RAB-9, "Archived history survives an arbitrary interval".
///
/// **WHEN** an archived rabbit's history is opened a long time after archiving **THEN** the weight
/// entries recorded before archiving are still present.
///
/// How the interval is asserted, and what the clock can and cannot show: `WeightRange::All` is
/// `(None, None)` — `domain::weight::window` returns before it parses `now` — so the read-side clock
/// is deliberately **not** what retains anything; a later clock re-labels the read instead of reaching
/// a retention path. Presence across time is therefore asserted on the three things that can lose
/// data: the shared collection's stored rows (byte-identical before and after the archive, and again
/// after the reads), this device's copy (unchanged by the later read), and a real restart — which is
/// the stronger "a long time after", and is `state.rs`'s
/// `archiving_discards_no_entry_and_the_archived_history_survives_a_relaunch`, not repeated here. The
/// clock is made to bite where it can: the read is repeated at an instant **before** every entry, and
/// `All` still returns both, so no read-side cut-off can age out or hide a stored entry.
#[test]
fn archived_history_survives_an_arbitrary_interval() {
    let (service, state, rabbit) = household_with_a_rabbit();
    let one = household(HOUSEHOLD);

    let first = create_entry(
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
    refresh(&state, &service, &one);

    let trend_before = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the active rabbit's history is readable");

    let archived = block_on_ready(state.archive_rabbit(rabbit.id.clone(), request_id(ARCHIVE)))
        .expect("the double accepted the archive");
    // The archive must have happened — on the answer and on the copy every later read resolves. An
    // `archive_rabbit` that answered `Ok` without writing would leave the rabbit active, and an active
    // rabbit's reads answer exactly as the archived ones do, which is the trap 5.2's restore test
    // records (rabbit_lifecycle.rs).
    assert!(archived.is_archived, "the archive is reported");
    refresh(&state, &service, &one);
    let held = state
        .rabbit(&rabbit.id)
        .expect("the archived rabbit is readable");
    assert!(held.is_archived, "the archive reached the copy");

    let stored_after_archive = stored_entries(&service, &one);
    let copy_after_archive = state
        .read_collection(&one, CollectionSource::Local)
        .expect("the copy is held")
        .items;

    // The interval: the same archived history, read against a clock five years later.
    let trend_later = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(A_LONG_TIME_LATER))
        .expect("the archived history is readable a long time later");
    assert_eq!(
        json_of(&trend_later),
        json_of(&trend_before),
        "the series is the one recorded before archiving"
    );
    assert_eq!(
        trend_later
            .points
            .iter()
            .map(|point| point.value_grams)
            .collect::<Vec<_>>(),
        vec![Grams(2_000), Grams(1_800)],
        "both entries are present, in timestamp order"
    );
    assert_eq!(trend_later.drop_threshold_percent, 5.0);

    // The clock bites here: read before every entry, and `All` still serves both, because the full
    // history is the absence of a window rather than a period that could age out.
    let trend_before_the_entries = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(BEFORE_EVERY_ENTRY))
        .expect("the archived history is readable at any clock");
    assert_eq!(
        json_of(&trend_before_the_entries),
        json_of(&trend_before),
        "the read-side clock cannot hide a stored entry"
    );

    for (entry, note) in [(&first, "morning"), (&second, "evening")] {
        let detail = state
            .weight_entry(&entry.id)
            .expect("the archived entry is readable a long time later");
        assert_eq!(detail.value_grams, entry.value_grams);
        assert_eq!(detail.timestamp, entry.timestamp);
        assert_eq!(detail.created_at, entry.created_at);
        assert_eq!(detail.note.as_deref(), Some(note), "the note is kept whole");
    }

    // Opening the history is a read: the stored rows are still the ones the archive left, and no
    // read-side ageing removed anything from this device's copy either.
    assert_eq!(
        stored_entries(&service, &one),
        stored_after_archive,
        "reading the archived history writes nothing"
    );
    let copy_later = state
        .read_collection(&one, CollectionSource::Local)
        .expect("the copy is held")
        .items;
    assert_eq!(
        json_of(copy_later),
        json_of(copy_after_archive),
        "reading the archived history purges nothing"
    );
}

/// DM-4, "No weight data of an archived rabbit is mutable".
///
/// **WHEN** an archived rabbit's weight data is reached, from any entry point in the product **THEN**
/// no weight entry of that rabbit can be created, edited or deleted.
///
/// The three entry points are the three entry writes, and each is refused on the one archived copy
/// this fixture archived through the product. After every attempt the shared collection is
/// byte-for-byte the one the archive produced, so no attempt wrote anything at all — and the
/// timeline is still served afterwards, so a refusal cost the person no history.
#[test]
fn no_weight_data_of_an_archived_rabbit_is_mutable() {
    let (service, state, rabbit, entries) = household_with_an_archived_rabbit_and_entries();
    let one = household(HOUSEHOLD);
    let before = stored_entries(&service, &one);

    // Create.
    let create = request_id("operation-6");
    let calls_before = service.calls();
    let error = block_on_ready(state.create_weight_entry(
        rabbit.id.clone(),
        WeightEntryDraft {
            value_grams: Grams(1_900),
            timestamp: Some(at("2026-10-22T09:00:00Z")),
            note: None,
            outlier_confirmed: false,
        },
        create.clone(),
    ))
    .expect_err("an archived rabbit's weight data is read-only");
    assert_refused(&error, &create, "the create");
    assert_eq!(service.calls(), calls_before, "nothing was probed or sent");
    assert_eq!(
        stored_entries(&service, &one),
        before,
        "the create wrote nothing"
    );

    // Edit.
    assert_entry_write_refused(
        &state,
        &service,
        &entries[0],
        "operation-7",
        EntryWrite::Edit,
    );
    assert_eq!(
        stored_entries(&service, &one),
        before,
        "the edit wrote nothing"
    );

    // Delete.
    assert_entry_write_refused(
        &state,
        &service,
        &entries[1],
        "operation-8",
        EntryWrite::Delete,
    );
    assert_eq!(
        stored_entries(&service, &one),
        before,
        "the deletion wrote nothing"
    );

    // Nothing was lost to the refusals: the archived timeline still serves both entries.
    let trend = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the archived timeline is readable");
    assert_eq!(
        trend
            .points
            .iter()
            .map(|point| point.value_grams)
            .collect::<Vec<_>>(),
        vec![Grams(2_000), Grams(1_800)]
    );
}

/// DM-4, "The archive is retrievable and its data is retained by default".
///
/// **WHEN** a rabbit is archived **THEN** the rabbit remains retrievable from the archived list
/// **AND** none of its weight data is discarded by archiving.
///
/// The two lists are the selections the list command serves (`summaries`); the archived list holds
/// the rabbit and the active list does not. Its entries are still stored and still served by the
/// reads the surfaces use, which is "retained by default": nothing purges them, and the only removal
/// this phase has is RAB-10's confirmed deletion.
#[test]
fn the_archive_is_retrievable_and_its_data_is_retained_by_default() {
    let (service, state, rabbit, entries) = household_with_an_archived_rabbit_and_entries();
    let one = household(HOUSEHOLD);
    let collection = state
        .read_collection(&one, CollectionSource::Local)
        .expect("the read path's copy is held");

    let archived = summaries(&collection, true).expect("the archived list is describable");
    assert_eq!(archived.len(), 1, "the archived list holds the rabbit");
    assert_eq!(archived[0].id, rabbit.id);
    assert_eq!(archived[0].name, "Pebble");
    assert!(archived[0].is_archived);

    let active = summaries(&collection, false).expect("the active list is describable");
    assert!(
        active.is_empty(),
        "the archived rabbit left the active list"
    );

    // None of its weight data was discarded by archiving.
    assert_eq!(
        entry_rows(&service, &one).len(),
        2,
        "both entries are still stored"
    );
    for entry in &entries {
        let detail = state
            .weight_entry(&entry.id)
            .expect("the archived entry is still retrievable");
        assert_eq!(detail.value_grams, entry.value_grams);
        assert_eq!(detail.timestamp, entry.timestamp);
        assert_eq!(detail.rabbit_id, rabbit.id);
    }
    let trend = state
        .weight_trend(&rabbit.id, WeightRange::All, &at(NOW))
        .expect("the archived history is retrievable");
    assert_eq!(trend.points.len(), 2);
}
