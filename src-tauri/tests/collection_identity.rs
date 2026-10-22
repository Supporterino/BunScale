//! Integration tests for the household collection's identity (task 5.3): SYNC-1's two scenarios and
//! the re-issued write D0.14 makes safe.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones
//! `openspec/changes/add-shared-household-foundation/specs/sync-and-offline/spec.md` states for
//! SYNC-1, and D0.14's rule that a re-issued operation adds nothing. Each test is named after the
//! scenario it covers; the D0.14 clause of the second one gets its own test.
//!
//! # The boundary these tests drive
//!
//! The command adapters in `ipc::commands` are thin forwarders over [`AppState`], a `tauri::State`
//! cannot be constructed outside a running app, and `app_lib::run()`/`ipc::export_bindings()` are
//! never called (see `library_surface.rs`). So the tests call the `AppState` methods the commands
//! forward to and drive the read path ([`ReadPath`]) and the write gate ([`SaveGate`]) themselves,
//! with the published [`ScriptedService`] as the transport — the construction style
//! `household_creation.rs` and `multiple_households.rs` use. Both platforms (TOOL-1): nothing here
//! is platform-specific.
//!
//! # What "two members" means in P0
//!
//! A member's view is that device's copy of the household's shared collection (SYNC-1). P0 has one
//! household collection shared through the service and no multi-device transport in the gate, so
//! the test models two members as two independent device states, each refreshing the same
//! household from its own transport scripted with the same shared-collection answer. What is
//! asserted is the resolution rule both of them apply: identity is the item id together with its
//! version, never the order an answer happened to list the rows in.
//!
//! # Why the re-issued write is composed with the merge path
//!
//! D0.14's guarantee is about the service's ledger, and P0 has no item-write command: the gate
//! returns a [`SaveReceipt`] and nothing else, with the body kept opaque for the operation that
//! submitted the write (2.1's create reads its own schema out of it the same way). So the D0.14
//! test performs the one step the future operation performs — record the item the receipt names
//! into the household's copy through [`MergePath`] — and asserts the two facts the scenario names:
//! the re-issue is the same completed operation, and the collection still holds exactly one item.
//!
//! # What is deliberately not asserted here
//!
//! Rendering is not involved: "each item is presented exactly once" is asserted as exactly one row
//! per item id in the resolved collection, which is the fact a screen renders from. Last-write-wins
//! convergence between two different versions is SYNC-5's, task 5.6's; this file only ever replays
//! the same version, because that is what a re-issued operation and a repeated load produce.

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::{Value, json};

use app_lib::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant, ItemId, RequestId,
};
use app_lib::state::AppState;
use app_lib::sync::double::test_support::block_on_ready;
use app_lib::sync::double::{RecordedCall, ScriptedService};
use app_lib::sync::{
    Answer, MergePath, ReadPath, SaveGate, SaveReceipt, SaveRequest, ServiceReply,
};

/// The instant the household's collection is loaded at.
const LOADED_AT: &str = "2026-10-19T11:00:00Z";

/// A later load instant, for the second load of the same batch.
const RELOADED_AT: &str = "2026-10-19T12:00:00Z";

/// The version of `item-1`, shared by both members' answers.
const FIRST_VERSION: &str = "2026-10-19T09:00:00Z";

/// The version of `item-2`, shared by both members' answers.
const SECOND_VERSION: &str = "2026-10-19T10:00:00Z";

fn request_id(value: &str) -> RequestId {
    RequestId(value.to_owned())
}

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

/// One household's wire collection holding `items`, as a guarded load or merge takes it.
fn collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// One readable whole-collection answer (the shape the read path reads).
fn feed(household: &HouseholdId, items: &[CollectionItem]) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "household_id": household, "items": items }),
    ))
}

/// Each item's identity — id and version — sorted, so two resolutions can be compared without
/// depending on the order their answers listed the rows in.
fn identities(items: &[CollectionItem]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = items
        .iter()
        .map(|row| (row.item_id.0.clone(), row.updated_at.0.clone()))
        .collect();
    pairs.sort();

    pairs
}

/// The ids of a set of rows, for a compact assertion message.
fn ids(items: &[ItemId]) -> Vec<&str> {
    items.iter().map(|id| id.0.as_str()).collect()
}

/// Assert the collection holds exactly one row per item id, `expected` rows in all.
///
/// "Exactly one copy of every item" (SYNC-1) is a property of the collected rows: a second copy
/// is a repeated id, whatever order the rows come back in.
fn assert_one_copy_per_item(collection: &HouseholdCollection, expected: usize) {
    let item_ids: Vec<&str> = collection
        .items
        .iter()
        .map(|row| row.item_id.0.as_str())
        .collect();
    let distinct: HashSet<&str> = item_ids.iter().copied().collect();

    assert_eq!(collection.items.len(), expected, "{item_ids:?}");
    assert_eq!(
        item_ids.len(),
        distinct.len(),
        "an item appears more than once: {item_ids:?}"
    );
}

/// The item the operation saves, and the service answers with.
fn saved_item() -> CollectionItem {
    item("item-1", FIRST_VERSION)
}

/// One write answer: the outcome the gate classifies, and the item the operation records.
fn completed_answer(outcome: &str, item: &CollectionItem) -> ServiceReply {
    ServiceReply::Answered(Answer::with_body(
        200,
        json!({ "outcome": outcome, "item": item }),
    ))
}

/// One write of `item`, carrying the operation identity a retry re-uses unchanged (D0.14).
fn write_of(item: &CollectionItem) -> SaveRequest {
    SaveRequest::new(
        request_id("operation-1"),
        "save_item",
        json!({ "household_id": "household-1", "item": item }),
    )
    .expect("a well-formed write")
}

/// The item a completed write's receipt names, in the operation's own schema.
///
/// The gate keeps the body opaque (it owns no schema), so reading it is the operation's step — the
/// same step `domain::create_household` performs for a household create.
fn item_of(receipt: &SaveReceipt) -> CollectionItem {
    serde_json::from_value(
        receipt
            .body()
            .get("item")
            .cloned()
            .expect("the answer names the item"),
    )
    .expect("the answer's item is a collection row")
}

/// Every write the transport was asked to submit, as the operation identity it carried (D0.14).
fn save_identities(service: &ScriptedService) -> Vec<RequestId> {
    service
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            RecordedCall::Save { request, .. } => Some(request.request_id().clone()),
            _ => None,
        })
        .collect()
}

/// SYNC-1, "Every member resolves the same items".
///
/// **WHEN** two members of the same household each view the household's data **THEN** both see the
/// same set of items, and each item is presented exactly once for each of them.
///
/// The two answers list the same two items in opposite orders on purpose: a member's resolution
/// must be the item id plus its version, not the local order one device happened to receive.
#[test]
fn every_member_resolves_the_same_items() {
    let household = household_id("household-1");
    let shared = vec![
        item("item-1", FIRST_VERSION),
        item("item-2", SECOND_VERSION),
    ];
    let reversed = vec![shared[1].clone(), shared[0].clone()];

    // Two members, each on their own device, refreshing the same household's shared collection.
    let first = AppState::new(
        Arc::new(ScriptedService::new().on_read([feed(&household, &shared)])),
        None,
    );
    let second = AppState::new(
        Arc::new(ScriptedService::new().on_read([feed(&household, &reversed)])),
        None,
    );

    let first_view = block_on_ready(ReadPath::new().load(&first, &household))
        .expect("the first member's read resolves");
    let second_view = block_on_ready(ReadPath::new().load(&second, &household))
        .expect("the second member's read resolves");

    // Both refreshed the same household from the shared service, so neither view is the vacuous
    // never-loaded state that would make an equality assertion pass for the wrong reason.
    assert_eq!(first_view.household_id, household);
    assert_eq!(second_view.household_id, household);
    assert_eq!(first_view.source, CollectionSource::Service);
    assert_eq!(second_view.source, CollectionSource::Service);
    assert!(first_view.loaded_at.is_some());
    assert!(second_view.loaded_at.is_some());

    // THEN both see the same set of items — the same ids at the same versions ...
    assert_eq!(
        identities(&first_view.items),
        identities(&second_view.items)
    );
    assert_eq!(
        identities(&first_view.items),
        identities(&shared),
        "both resolve the household's own items"
    );

    // ... and each item is presented exactly once for each of them.
    assert_one_copy_per_item(&first_view, shared.len());
    assert_one_copy_per_item(&second_view, shared.len());
}

/// SYNC-1, "Refreshing does not duplicate an item".
///
/// **WHEN** a household's collection is loaded again after it was already loaded, or a refresh
/// returns the items it already holds **THEN** items that were already present appear once, not
/// twice, and the collection holds exactly one copy of every item.
#[test]
fn refreshing_does_not_duplicate_an_item() {
    let household = household_id("household-1");
    let batch = vec![
        item("item-1", FIRST_VERSION),
        item("item-2", SECOND_VERSION),
    ];
    let service = Arc::new(ScriptedService::new().on_read([feed(&household, &batch)]));
    let state = AppState::new(service, None);

    // The collection is loaded on this device, and loading it again is a load of the same
    // collection: the copy is replaced, never appended to.
    for (instant, label) in [(LOADED_AT, "the first load"), (RELOADED_AT, "the reload")] {
        state
            .load_collection(
                &household,
                collection(&household, batch.clone()),
                at(instant),
            )
            .expect("the collection is addressed correctly");

        let held = state
            .read_collection(&household, CollectionSource::Local)
            .expect("this device holds the copy");

        assert_one_copy_per_item(&held, batch.len());
        assert_eq!(
            held.items.len(),
            batch.len(),
            "{label}: the item count is stable"
        );
        assert_eq!(
            identities(&held.items),
            identities(&batch),
            "{label}: every item is the one already present"
        );
    }

    // A refresh that returns the same items replaces the copy with itself: repeated twice, the
    // collection still holds one row per item and its item count does not grow.
    for _ in 0..2 {
        let refreshed = block_on_ready(ReadPath::new().load(&state, &household))
            .expect("the refresh read resolves");

        assert_eq!(refreshed.source, CollectionSource::Service);
        assert!(refreshed.loaded_at.is_some());

        assert_one_copy_per_item(&refreshed, batch.len());
        assert_eq!(
            refreshed.items.len(),
            batch.len(),
            "the refresh did not grow the collection"
        );
        assert_eq!(identities(&refreshed.items), identities(&batch));

        // The stored copy is exactly what the read returned: no second row lives beside it.
        let held = state
            .read_collection(&household, CollectionSource::Local)
            .expect("the loaded copy is held");
        assert_one_copy_per_item(&held, batch.len());
        assert_eq!(identities(&held.items), identities(&refreshed.items));
    }
}

/// SYNC-1 / D0.14 — a write the service already accepted, re-issued with the same request identity,
/// adds no second item.
///
/// **WHEN** a write the service already accepted is re-issued with the same client-supplied request
/// identity **THEN** the item already present appears once, not twice, and the collection holds
/// exactly one copy of it.
///
/// Two facts together are what make the retry safe, and both are asserted: the re-issue carries the
/// same operation identity (so the service answers with the stored outcome, `deduplicated`, rather
/// than applying a second write), and the item it names has the identity the collection already
/// holds (so recording it again cannot add a row).
#[test]
fn a_reissued_write_adds_no_second_item() {
    let household = household_id("household-1");
    let item = saved_item();
    let operation = request_id("operation-1");
    let service = Arc::new(ScriptedService::new().on_write([
        completed_answer("applied", &item),
        completed_answer("deduplicated", &item),
    ]));
    let state = AppState::new(service.clone(), None);

    // The household's copy is loaded, so the item a completed write names has a collection to land
    // in (P0 has no item-write command; see the module documentation).
    state
        .load_collection(
            &household,
            collection(&household, Vec::new()),
            at(LOADED_AT),
        )
        .expect("the copy is addressed correctly");
    let merge = MergePath::new();
    let gate = SaveGate::new(service.as_ref());

    // The first attempt: the service applies the write and reports it completed.
    let first = block_on_ready(gate.save(write_of(&item))).expect("the service accepted the write");
    assert_eq!(
        first.request_id(),
        &operation,
        "the receipt names the operation (D0.14)"
    );

    // The operation records the item its receipt names; that is the one row this device holds.
    let delivered = item_of(&first);
    assert_eq!(delivered, item);
    let applied = merge.merge(&state, &household, collection(&household, vec![delivered]));
    assert_eq!(ids(applied.applied()), ["item-1"]);
    assert!(applied.ignored().is_empty());

    // The retry the failure surface offers re-issues the same operation identity unchanged.
    let reissued = block_on_ready(gate.save(write_of(&item)))
        .expect("the stored outcome is a completed write");
    assert_eq!(
        reissued.request_id(),
        &operation,
        "the re-issue is the same operation, not a second one"
    );
    assert_eq!(
        reissued.body().get("outcome").and_then(Value::as_str),
        Some("deduplicated"),
        "the service returned the operation's stored outcome, not a second application"
    );

    // Recording what the replay names changes nothing: it is the item identity already stored.
    let replayed = item_of(&reissued);
    assert_eq!(
        replayed, item,
        "the replay names the item and version the operation already stored"
    );
    let ignored = merge.merge(&state, &household, collection(&household, vec![replayed]));
    assert!(ignored.applied().is_empty(), "no second row is written");
    assert_eq!(ids(ignored.ignored()), ["item-1"]);

    // THEN the collection holds one item, at the version the service stored ...
    let held = state
        .read_collection(&household, CollectionSource::Local)
        .expect("the copy is held");
    assert_one_copy_per_item(&held, 1);
    assert_eq!(held.items, vec![item], "one item, not two");

    // ... and the two attempts carried one operation identity, so the service deduplicated the
    // second rather than performing another write.
    let expected = vec![operation.clone(), operation];
    assert_eq!(
        save_identities(&service),
        expected,
        "one operation, sent twice"
    );
    assert_eq!(
        service.write_count(),
        2,
        "two attempts of the same operation"
    );
}
