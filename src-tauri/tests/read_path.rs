//! Integration tests for the read path (task 5.5): SYNC-3's three states, plus the failed-refresh
//! rule that is additional coverage beyond them.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/sync-and-offline/spec.md` states for
//! SYNC-3, and each test is named after the scenario it covers. The extra test — a failed refresh
//! changing nothing — asserts SYNC-3's and D0.2's own rule; it is not a fourth scenario.
//!
//! # The boundary these tests drive
//!
//! A read command is a thin adapter over [`ReadPath::load`], which is the read path's only public
//! entry point: `serve_local` and `refresh` are `pub(crate)`, so the local copy and the refresh are
//! exercised exactly as the command exercises them — one call, and the value it resolves. The copy
//! itself belongs to the [`AppState`], and a previously loaded collection is seeded through
//! `AppState::load_collection`, the same guarded load the refresh uses. The transport is the
//! published [`ScriptedService`]; `app_lib::run()` and `ipc::export_bindings()` are never called,
//! as `library_surface.rs` records.
//!
//! # Two limits worth naming
//!
//! A **relaunch** — a second process reading the copy a previous one persisted — cannot be built
//! from `src-tauri/tests/`: `AppState::with_collection_store` is `pub(crate)` and
//! `AppState::production` needs a running `tauri::AppHandle`, so a `CollectionStore` cannot be
//! handed to a state here. That case is covered where the seam exists, by the read path's own unit
//! tests (`sync/read.rs`) over a temporary directory; what this file proves is the same
//! [`ReadPath::load`] the command calls, over the in-memory device state a test can build.
//!
//! **Rendering is not involved.** "The previously loaded data is shown" and "the refreshed data
//! replaces it" are asserted as the value the read resolves — `source`, `loaded_at` and the rows —
//! which is the fact a screen renders from. Both platforms (TOOL-1): nothing here is
//! platform-specific.
//!
//! # The three states, as the tests tell them apart
//!
//! | State | `source` | `loaded_at` | Rows |
//! |---|---|---|---|
//! | loaded, served while unreachable | `local` | the instant it was loaded | the copy's rows |
//! | refreshed from the service | `service` | this device's fresh instant | the answer's rows |
//! | never loaded, unreachable | `local` | `None` | none |
//!
//! The last row is the reason the marker exists: a loaded collection that happens to hold no rows
//! has the same empty row set, and only `loaded_at` says which of the two a device is in.

use std::sync::Arc;

use serde_json::json;

use app_lib::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant, ItemId,
    ServiceState,
};
use app_lib::state::AppState;
use app_lib::storage::AbsoluteInstant;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{
    Answer, REACHABILITY_PROBE_TIMEOUT, READ_REFRESH_TIMEOUT, ReadPath, ServiceReply,
    current_status,
};

/// The instant the seeded copy was loaded at: strictly before any instant a run produces.
const LOADED_AT: &str = "2026-10-19T10:00:00Z";

/// The version the seeded copy's rows carry.
const OLD_VERSION: &str = "2026-10-19T09:00:00Z";

/// The version the refreshing answer delivers.
const NEW_VERSION: &str = "2026-10-20T09:00:00Z";

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn item_id(value: &str) -> ItemId {
    ItemId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One row of a household's collection: an item id and its version marker (SYNC-1).
fn item(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: false,
        payload: None,
    }
}

/// One household's wire collection, as a guarded load takes it.
fn collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// One readable whole-collection answer, in the shape the read path reads.
fn feed(household: &HouseholdId, items: &[CollectionItem]) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "household_id": household, "items": items }),
    ))
}

/// Put `items` into `household`'s copy as a collection this device loaded at `loaded_at`.
fn seed(state: &AppState, household: &HouseholdId, items: Vec<CollectionItem>, loaded_at: &str) {
    state
        .load_collection(household, collection(household, items), at(loaded_at))
        .expect("the collection is addressed through its own household");
}

/// SYNC-3, "Previously loaded data is shown while unreachable".
///
/// **WHEN** the product starts while the shared service is unreachable and the household's data was
/// loaded before **THEN** the previously loaded data is shown.
#[test]
fn previously_loaded_data_is_shown_while_unreachable() {
    let household = household_id("household-1");
    let held = vec![item("item-1", OLD_VERSION)];
    let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
    let state = AppState::new(service.clone(), None);
    seed(&state, &household, held.clone(), LOADED_AT);

    let read = block_on_ready(ReadPath::new().load(&state, &household))
        .expect("a read is not refused while unreachable");

    // THEN the previously loaded data is shown, labelled as this device's own copy and keeping the
    // instant it was loaded at.
    assert_eq!(read.household_id, household);
    assert_eq!(read.source, CollectionSource::Local);
    assert_eq!(read.loaded_at, Some(at(LOADED_AT)));
    assert_eq!(read.items, held);

    // AND no refresh replaced it: the probe refused the attempt before the transport was asked for
    // the collection, and the account check was not spent on a service that did not answer.
    assert_eq!(service.read_count(), 0);
    assert_eq!(
        service.calls(),
        vec![RecordedCall::CheckService {
            bound: REACHABILITY_PROBE_TIMEOUT,
        }]
    );

    // The copy the process holds is exactly what the read returned, marker included: a read never
    // creates or rewrites one.
    let stored = state
        .read_collection(&household, CollectionSource::Local)
        .expect("the copy is held");
    assert_eq!(stored.loaded_at, Some(at(LOADED_AT)));
    assert_eq!(stored.items, held);

    // A refresh that never happened is not a status observation: the tracker belongs to the probe's
    // callers, not to the read path (D0.2). The count distinguishes a never-observed tracker from
    // one that has already recorded a failure.
    assert_eq!(current_status(&state).state, ServiceState::Unknown);
    assert_eq!(state.status().consecutive_failures(), 0);
}

/// SYNC-3, "A start with connectivity refreshes loaded data".
///
/// **WHEN** the product starts while the shared service is reachable **THEN** the household's data
/// is refreshed from the shared service, and the refreshed data replaces the previously loaded
/// data.
///
/// "Replaces" is the subject: the answer omits one of the two rows the copy held, and that row must
/// be gone afterwards rather than merged. A load replaces the copy wholesale; only a delta would
/// merge, and P0 never asks for one (it sends `since_change_seq: 0`).
#[test]
fn a_start_with_connectivity_refreshes_loaded_data() {
    let household = household_id("household-1");
    let answer = [item("item-1", NEW_VERSION)];
    let previous = vec![item("item-1", OLD_VERSION), item("item-2", OLD_VERSION)];
    let service = Arc::new(ScriptedService::new().on_read([feed(&household, &answer)]));
    let state = AppState::new(service.clone(), None);
    seed(&state, &household, previous, LOADED_AT);

    let before = AbsoluteInstant::now();
    let read = block_on_ready(ReadPath::new().load(&state, &household)).expect("the read resolves");
    let after = AbsoluteInstant::now();

    // THEN the household's data is refreshed from the shared service, with this device's own load
    // instant: it lies inside the window the read ran in, which the seeded instant cannot.
    assert_eq!(read.household_id, household);
    assert_eq!(read.source, CollectionSource::Service);
    let recorded = AbsoluteInstant::parse(read.loaded_at.as_ref().expect("a load instant"))
        .expect("the recorded instant is an absolute instant");
    assert!(recorded >= before, "the load instant precedes the read");
    assert!(recorded <= after, "the load instant follows the read");

    // AND the refreshed data replaces the previously loaded data: item-1 is the version the answer
    // delivered, and item-2 — which the answer omitted — is gone rather than kept.
    assert_eq!(read.items, answer);
    let stored_ids: Vec<&str> = read
        .items
        .iter()
        .map(|row| row.item_id.0.as_str())
        .collect();
    assert_eq!(stored_ids, ["item-1"], "the copy is replaced, not merged");

    // The stored copy is exactly the value the read returned, marker included.
    let stored = state
        .read_collection(&household, CollectionSource::Local)
        .expect("the loaded copy is held");
    assert_eq!(stored.items, read.items);
    assert_eq!(stored.loaded_at, read.loaded_at);

    // The refresh is one whole-collection read with D0.13's read bound, asked from position 0 (P0
    // has no cursor; a range from a non-zero position would be a delta, which replacing would
    // lose).
    assert_eq!(
        service.calls(),
        vec![
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
            RecordedCall::CheckAccount {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
            RecordedCall::Read {
                bound: READ_REFRESH_TIMEOUT,
                household_id: household.clone(),
                since_change_seq: 0,
            },
        ]
    );
    assert_eq!(service.write_count(), 0, "a read is never a write");
    assert_eq!(service.read_count(), 1);
}

/// SYNC-3, "Nothing loaded and unreachable".
///
/// **WHEN** the product starts while the shared service is unreachable and no household data was
/// ever loaded on that device **THEN** an explicit state stating that connectivity is required is
/// shown, and no empty or stale data is presented as the household's current data.
///
/// The distinction is the subject of the test, so the two empty states are resolved side by side: a
/// household that was never loaded, and one whose collection was loaded and happens to hold no
/// rows. They differ only in `loaded_at`, which is why that marker, not the row count, is the
/// contract's "never loaded" signal.
#[test]
fn nothing_loaded_and_unreachable() {
    let never_loaded = household_id("household-1");
    let loaded_empty = household_id("household-2");
    let service = Arc::new(ScriptedService::new().on_service_check([ServiceReply::NoResponse]));
    let state = AppState::new(service.clone(), None);
    seed(&state, &loaded_empty, Vec::new(), LOADED_AT);

    let nothing = block_on_ready(ReadPath::new().load(&state, &never_loaded))
        .expect("a read is not refused while unreachable");
    let empty = block_on_ready(ReadPath::new().load(&state, &loaded_empty))
        .expect("a read is not refused while unreachable");

    // THEN the never-loaded household is the explicit connectivity-required state: no rows and no
    // load marker at all.
    assert_eq!(nothing.household_id, never_loaded);
    assert_eq!(nothing.source, CollectionSource::Local);
    assert_eq!(nothing.loaded_at, None, "nothing was ever loaded here");
    assert!(nothing.items.is_empty());

    // AND a loaded collection that holds no rows is a different value: the same empty row set, and
    // the marker that says this device did load it.
    assert_eq!(empty.household_id, loaded_empty);
    assert_eq!(empty.source, CollectionSource::Local);
    assert_eq!(empty.loaded_at, Some(at(LOADED_AT)));
    assert!(empty.items.is_empty());

    assert_ne!(
        nothing.loaded_at, empty.loaded_at,
        "the marker tells the two states apart"
    );

    // No empty or stale data is presented as the never-loaded household's current data: the read
    // created no copy, so this device holds nothing for it — the explicit state, not a fabricated
    // collection.
    assert!(
        state
            .read_collection(&never_loaded, CollectionSource::Local)
            .is_none()
    );
    let loaded = state
        .read_collection(&loaded_empty, CollectionSource::Local)
        .expect("the loaded copy is held");
    assert_eq!(loaded.loaded_at, Some(at(LOADED_AT)));
    assert!(loaded.items.is_empty());

    // Neither read attempted a refresh, and neither asked for the account: the service did not
    // answer, so there was nothing to ask it about.
    assert_eq!(service.read_count(), 0);
    assert_eq!(
        service.calls(),
        vec![
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
            RecordedCall::CheckService {
                bound: REACHABILITY_PROBE_TIMEOUT,
            },
        ]
    );
}

/// The honesty rule the contract fixes: a refresh that fails leaves the previous copy exactly as it
/// was, and the read still resolves (SYNC-3, D0.2).
///
/// Every failure shape is one a real refresh produces — no response, no outcome inside the bound,
/// a refused status, a `2xx` this boundary cannot read, and an answer describing another household
/// (which the guarded load refuses). None may clear the copy, move `loaded_at` or turn the read
/// into a failure.
#[test]
fn a_failed_refresh_leaves_the_previous_copy_exactly_as_it_was() {
    let household = household_id("household-1");
    let other = household_id("household-2");
    let held = vec![item("item-1", OLD_VERSION)];
    let items_missing = ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "household_id": "household-1" }),
    ));
    let failures = [
        ("no response", ServiceReply::NoResponse),
        ("no outcome", ServiceReply::NoOutcome),
        (
            "a refused status",
            ServiceReply::Answered(Answer::status_only(500)),
        ),
        ("an answer without items", items_missing),
        (
            "another household's collection",
            feed(&other, &[item("item-9", NEW_VERSION)]),
        ),
    ];

    for (label, reply) in failures {
        let service = Arc::new(ScriptedService::new().on_read([reply]));
        let state = AppState::new(service.clone(), None);
        seed(&state, &household, held.clone(), LOADED_AT);

        let read = block_on_ready(ReadPath::new().load(&state, &household))
            .expect("a failed refresh does not fail the read");

        assert_eq!(read.source, CollectionSource::Local, "{label}");
        assert_eq!(
            read.loaded_at,
            Some(at(LOADED_AT)),
            "{label}: the marker moved"
        );
        assert_eq!(read.items, held, "{label}: the rows changed");
        assert_eq!(
            service.read_count(),
            1,
            "{label}: the refresh was attempted"
        );

        // The copy the process holds is still the one that was loaded before the attempt.
        let stored = state
            .read_collection(&household, CollectionSource::Local)
            .expect("the copy is still held");
        assert_eq!(stored.loaded_at, Some(at(LOADED_AT)), "{label}");
        assert_eq!(stored.items, held, "{label}");
    }
}
