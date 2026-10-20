//! Integration tests for timestamps (task 5.8): DM-6's three scenarios and the refusals the
//! contract fixes.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/data-integrity/spec.md` states for
//! DM-6, and each test is named after the scenario it covers; the refusals have their own test
//! beside them.
//!
//! # The boundary these tests drive
//!
//! [`AbsoluteInstant`] is the storage boundary's only interpretation of a timestamp's content. The
//! read path is its one caller in P0 for the creation default: a load records
//! `AbsoluteInstant::now` as the copy's marker. The other timestamped product path, household
//! creation, records the service's `created_at` text verbatim (and the create request itself
//! carries no timestamp at all), so the boundary never sees that value. The tests drive both paths
//! plus the parse entry points, so the behaviour is asserted where the product uses it rather than
//! only inside the module.
//!
//! **The device offset is not the process timezone.** The module never reads the device's offset
//! (the pinned `time` crate's `local-offset` feature is deliberately off), so changing the
//! machine's TZ would prove nothing here and no test does it. The equivalent fact is asserted from
//! the wire form: one moment written with different offsets is one stored value, ordering compares
//! moments rather than text, and the persisted file gives the value back unchanged.
//!
//! **"Unchanged" has two spellings, and both are asserted.** A value the service supplies is
//! recorded with its own text (the domain copies it); a value handed to the boundary's creation
//! entry point is recorded as the same *moment* in canonical UTC, because D0.5 normalises the
//! writer's offset away on store. Neither path clamps to the present or rounds.
//!
//! # What is deliberately not asserted here
//!
//! The module's own unit tests cover the parser's case table; these tests restate the refusals the
//! contract names through the public entry points (`parse` and `at_creation`), not through private
//! internals, and add the product-level window and round trip. Rendering is not involved, and
//! nothing here is platform-specific (TOOL-1: iOS and macOS).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use app_lib::ipc::types::{
    AccountUserId, CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant,
    ItemId, RequestId,
};
use app_lib::state::AppState;
use app_lib::storage::{
    AbsoluteInstant, CollectionStore, DeviceSnapshot, GuardedStore, InstantError,
};
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::{Answer, ReadPath, ScriptedService, ServiceReply};

/// The fixed default a "defaults to now" implementation must not fall back to.
const EPOCH: &str = "1970-01-01T00:00:00Z";

/// A supplied instant in the past, with nanosecond precision.
const PAST: &str = "2001-02-03T04:05:06.123456789Z";

/// The same past moment, written with a positive offset.
const PAST_OFFSET: &str = "2001-02-03T05:05:06.123456789+01:00";

/// A supplied instant in the past, used as a load marker, written with an offset.
const LOADED_PAST: &str = "2000-01-01T00:59:59.999999999+01:00";

/// One moment, written in UTC.
const MOMENT: &str = "2026-10-19T10:00:00Z";

/// The same moment, written with a positive offset.
const MOMENT_PLUS_TWO: &str = "2026-10-19T12:00:00+02:00";

/// The same moment, written with a negative offset.
const MOMENT_MINUS_FIVE: &str = "2026-10-19T05:00:00-05:00";

/// One second after [`MOMENT`], whose text sorts before [`MOMENT_PLUS_TWO`]'s.
const MOMENT_LATER: &str = "2026-10-19T10:00:01Z";

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn item_id(value: &str) -> ItemId {
    ItemId(value.to_owned())
}

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

fn account() -> AccountUserId {
    AccountUserId("account-1".to_owned())
}

/// One accepted create answer carrying `created_at`, in the schema the create path reads.
fn accepted_answer(created_at: &str) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({
            "outcome": "applied",
            "household": {
                "id": "household-1",
                "owner_member_id": "member-1",
                "created_at": created_at,
            },
        }),
    ))
}

/// One readable read answer for `household`, listing no items.
fn feed(household: &HouseholdId) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "household_id": household, "items": [] }),
    ))
}

/// One household's wire collection with no rows, as a guarded load takes it.
fn collection(household: &HouseholdId) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items: Vec::new(),
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// One loaded wire collection, as the persisted file holds it.
fn loaded_collection(
    household: &HouseholdId,
    items: Vec<CollectionItem>,
    loaded_at: &str,
) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: Some(at(loaded_at)),
    }
}

/// One live wire row with `updated_at` as its version.
fn wire(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: false,
    }
}

/// A fresh directory under the process's temporary area, unique per call.
fn temp_data_dir(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    let name = format!("bunscale-timestamps-{process}-{sequence}-{label}");
    let directory = std::env::temp_dir().join(name);

    fs::create_dir_all(&directory).expect("a test directory");

    directory
}

fn remove_temp_data_dir(directory: &Path) {
    let _cleanup = fs::remove_dir_all(directory);
}

/// DM-6, "A timestamp defaults to now".
///
/// **WHEN** a timestamped item is created without a person-supplied time **THEN** the item records
/// the moment of its creation.
#[test]
fn a_timestamp_defaults_to_now() {
    let household = household_id("household-1");
    let service = Arc::new(ScriptedService::new().on_read([feed(&household)]));
    let state = AppState::new(service, None);

    // The load marker is a timestamp the product creates without a supplied instant: the device
    // that performed the load records its own moment (D0.5's creation default).
    let before = AbsoluteInstant::now();
    let read = block_on_ready(ReadPath::new().load(&state, &household)).expect("the read resolves");
    let after = AbsoluteInstant::now();

    let marker = read.loaded_at.as_ref().expect("a load records its instant");
    let recorded =
        AbsoluteInstant::parse(marker).expect("the recorded instant is an absolute instant");

    // A window, not an equality: the clock is read inside the call, so the value lies between the
    // two reads taken around it.
    assert!(
        recorded >= before,
        "the default is not earlier than the call"
    );
    assert!(recorded <= after, "the default is not later than the call");

    // And it is the creation moment, not a fixed default such as the epoch.
    let epoch = AbsoluteInstant::parse(&at(EPOCH)).expect("the epoch is an absolute instant");
    assert!(recorded > epoch, "the default must not be the epoch");
}

/// DM-6, "A supplied instant is stored unchanged".
///
/// **WHEN** the storage boundary is given an explicit instant, including one in the past **THEN**
/// that instant is stored and returned unchanged.
#[test]
fn a_supplied_instant_is_stored_unchanged() {
    let household = household_id("household-1");
    let service = Arc::new(ScriptedService::new().on_write([accepted_answer(PAST)]));
    let state = AppState::new(service, Some(account()));

    // A created household records the supplied instant as given — a past value with nanoseconds.
    // A clamp to the present or a rounding to whole seconds would change it.
    let created =
        block_on_ready(state.create_household("Home".to_owned(), request_id("operation-1")))
            .expect("the create completed");

    assert_eq!(
        created.created_at.0, PAST,
        "the created household keeps the supplied instant"
    );
    let stored = {
        let book = state.households();
        let record = book.get(&created.id).expect("the household is held");

        record.created_at().0.clone()
    };
    assert_eq!(stored, PAST, "the stored record keeps the supplied instant");
    let created_at =
        AbsoluteInstant::parse(&created.created_at).expect("the supplied instant parses");
    assert!(
        created_at < AbsoluteInstant::now(),
        "a past instant is not clamped"
    );

    // The other timestamped write is the load marker, and it is asserted with an offset-spelled
    // value on purpose: the guard records the marker text as the caller gives it, so a normalising
    // layer here would change the text and fail this assertion. (The read path's own marker is
    // already UTC, because `AbsoluteInstant::to_wire` is what produces it.)
    state
        .load_collection(&household, collection(&household), at(LOADED_PAST))
        .expect("the collection is addressed correctly");
    let marker = state
        .read_collection(&household, CollectionSource::Local)
        .expect("the copy is held")
        .loaded_at;
    assert_eq!(
        marker,
        Some(at(LOADED_PAST)),
        "the marker keeps the supplied spelling"
    );

    // At the boundary, "unchanged" means the same moment: a supplied value whose text carries an
    // offset is recorded in canonical UTC — normalised, never clamped to the present or rounded.
    let supplied = at(PAST_OFFSET);
    let recorded =
        AbsoluteInstant::at_creation(Some(&supplied)).expect("the supplied instant is valid");

    assert_eq!(
        recorded.to_wire(),
        at(PAST),
        "the same moment, in canonical UTC"
    );
    assert_eq!(
        recorded,
        AbsoluteInstant::parse(&supplied).expect("the supplied text names the same instant")
    );
}

/// DM-6, "Stored instants do not shift with the device offset".
///
/// **WHEN** the device's timezone or UTC offset changes **THEN** the meaning of a stored timestamp
/// is unchanged, and it is not re-interpreted as a local time.
#[test]
fn stored_instants_do_not_shift_with_the_device_offset() {
    let utc = AbsoluteInstant::parse(&at(MOMENT)).expect("Z is an absolute instant");

    // One moment, three spellings: the stored value is the moment, normalised to UTC on the way in.
    for spelled in [MOMENT_PLUS_TWO, MOMENT_MINUS_FIVE] {
        let parsed =
            AbsoluteInstant::parse(&at(spelled)).expect("an offset is an absolute instant");

        assert_eq!(parsed, utc, "{spelled} names the same moment");
        assert_eq!(
            parsed.to_wire(),
            at(MOMENT),
            "{spelled} stores as one UTC value"
        );
    }

    // Ordering compares moments, not text. The offset spelling's text sorts after the later UTC
    // instant's, so a text comparison would order these two the other way round.
    let earlier = AbsoluteInstant::parse(&at(MOMENT_PLUS_TWO)).expect("the value is valid");
    let later = AbsoluteInstant::parse(&at(MOMENT_LATER)).expect("the value is valid");
    assert!(
        at(MOMENT_PLUS_TWO).0 > at(MOMENT_LATER).0,
        "the texts order the moments wrongly"
    );
    assert!(earlier < later);

    // The persisted form does not move a stored instant: the file keeps the wire text, and loading
    // it back yields the same text and the same moment. The device's offset is never consulted, so
    // there is no process-wide TZ change to make — the wire form is the equivalent fact.
    let directory = temp_data_dir("instant-round-trip");
    let store = CollectionStore::in_dir(&directory);
    let persisted = loaded_collection(
        &household_id("household-1"),
        vec![wire("item-1", MOMENT_PLUS_TWO)],
        MOMENT_PLUS_TWO,
    );
    store
        .save(&DeviceSnapshot {
            collections: vec![persisted.clone()],
            ..DeviceSnapshot::default()
        })
        .expect("the snapshot is written");

    // The codec half: the file is a fixed point, so no pass rewrote the text on the way through.
    let restored = store.load().expect("the snapshot is readable");
    assert_eq!(restored.collections.len(), 1);
    let loaded = &restored.collections[0];
    assert_eq!(
        loaded.loaded_at, persisted.loaded_at,
        "the marker text did not move"
    );
    assert_eq!(
        loaded.items, persisted.items,
        "the row version did not move"
    );
    assert_eq!(
        AbsoluteInstant::parse(
            loaded
                .loaded_at
                .as_ref()
                .expect("a loaded copy has a marker")
        )
        .expect("the marker is an absolute instant"),
        utc,
        "the marker still names the same moment"
    );

    // The guarded-load half: the file's collection goes through the same boundary the product's
    // restore uses — the file's own marker is the one handed to the guard — and the copy it reads
    // back must carry the same marker text, the same marker moment and the same row version.
    // `AppState`'s own snapshot save is not reachable from an integration test
    // (`with_collection_store` is `pub(crate)`), so that layer is the read module's unit tests.
    let marker = loaded
        .loaded_at
        .clone()
        .expect("a loaded copy has a marker");
    let mut boundary = GuardedStore::new();
    boundary
        .load_wire_collection(&loaded.household_id, loaded.clone(), marker)
        .expect("the restored collection is addressed correctly");
    let copy = boundary
        .collection(&loaded.household_id)
        .expect("the restored copy is held")
        .to_wire(CollectionSource::Local);

    assert_eq!(
        copy.loaded_at, persisted.loaded_at,
        "the guard moved the marker"
    );
    assert_eq!(copy.items, persisted.items, "the guard moved a row version");
    assert_eq!(
        AbsoluteInstant::parse(copy.loaded_at.as_ref().expect("a loaded copy has a marker"))
            .expect("the marker is an absolute instant"),
        utc,
        "the guarded copy still names the same moment"
    );

    // Writing back the value the file just yielded produces identical bytes: no pass rewrote an
    // instant into another spelling.
    let first = fs::read(store.path()).expect("the file is readable");
    store
        .save(&restored)
        .expect("the snapshot is written again");
    let second = fs::read(store.path()).expect("the file is readable");
    assert_eq!(first, second, "a round trip rewrote a stored instant");

    remove_temp_data_dir(&directory);
}

/// DM-6's refusals: text that names no absolute instant, and an instant outside RFC 3339's
/// four-digit year range, are refused rather than replaced with now.
///
/// This is a restatement of the contract through its public entry points — the module's own unit
/// tests carry the full case table. It stays because the no-offset refusal is what "not
/// re-interpreted as a local time" (the device-offset scenario) depends on, and the year range is
/// the other half of "an absolute instant".
#[test]
fn text_that_is_not_an_absolute_instant_is_refused_rather_than_defaulted() {
    // A value with no offset names no moment: it is refused rather than read as a local time, and
    // `at_creation` refuses it rather than replacing it with the creation moment.
    let no_offset = "2026-10-19T10:00:00";
    assert_eq!(
        AbsoluteInstant::parse(&at(no_offset)),
        Err(InstantError::NotAnAbsoluteInstant),
        "a value with no offset must be refused"
    );
    assert_eq!(
        AbsoluteInstant::at_creation(Some(&at(no_offset))),
        Err(InstantError::NotAnAbsoluteInstant),
        "an unparseable supplied value must not become now"
    );

    // A syntactically valid instant whose UTC value leaves RFC 3339's four-digit year range is
    // refused as well: the offset moves it out of the range a stored instant can be written back
    // in, so accepting it would create a value the formatter cannot represent.
    for text in ["9999-12-31T23:59:59-05:00", "0000-01-01T00:00:00+05:00"] {
        assert_eq!(
            AbsoluteInstant::parse(&at(text)),
            Err(InstantError::NotAnAbsoluteInstant),
            "{text:?} normalises outside the storable year range"
        );
    }
}
