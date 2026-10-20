//! Integration tests for conflict convergence (task 5.6): SYNC-5's two scenarios and NFR-2's two,
//! plus the equal-instant rule the implementation applies.
//!
//! `src-tauri/tests/` is where a test that links `app_lib` lives; unit tests stay beside their code
//! (`rust-conventions`). The scenarios are the ones two delta specs state: SYNC-5's in
//! `openspec/changes/add-shared-household-foundation/specs/sync-and-offline/spec.md` and NFR-2's
//! in `openspec/changes/add-shared-household-foundation/specs/product-quality/spec.md`. Each test
//! is named after the scenario it covers; the equal-instant case has its own test beside them.
//!
//! # The boundary these tests drive
//!
//! A conflict is resolved by [`MergePath::merge`], which compares the rows a household's copy
//! already holds with the rows the shared service delivered and hands only the winners to the
//! guarded merge. It is the client half of D0.5's last-write-wins: the service's
//! `(updated_at, writer_id, request_id)` tuple is the authority, but only the first element is on
//! the wire, so this layer cannot reproduce the service's choice — it applies the row the service
//! delivered and decides only whether that delivery says anything new. `ReadPath` does not call it
//! in P0 (a whole-collection refresh replaces the copy); it is the seam P6's incremental feed and
//! P1's item writes use.
//!
//! **A conflict cannot be reported as a failure, structurally.** [`MergePath::merge`] returns
//! [`MergeOutcome`], not a `Result`: there is no error variant a lost version could be turned into,
//! and a storage refusal is an outcome field rather than an error. The tests assert
//! `MergeOutcome::refusal` is `None` and the merge was not skipped, which is the observable half of
//! NFR-2's "neither member is shown a failure".
//!
//! # What "two members" means here
//!
//! The merge is a pure function of one device's copy and one delivered batch, so two members are
//! modelled as two device states holding the same household's collection. Both edits reach the
//! shared service, its tuple picks the later instant, and each member's copy then receives that
//! delivered row: the member whose row won sees a replay, the member whose row lost sees it
//! replaced, and both end with exactly the service's row.
//!
//! # The equal-instant rule, and the one order dependence
//!
//! Versions compare as [`AbsoluteInstant`]s, never as text: `2026-10-19T12:00:00+02:00` and
//! `2026-10-19T10:00:00Z` are one version. At an equal instant, a delivered row whose state is the
//! one already stored is a **replay** and changes nothing; a delivered row whose state differs
//! **wins**, because the service's tuple already chose it — the rule is symmetric, so the delivered
//! row wins whichever of the two it is. When one batch carries two same-instant rows that differ,
//! the **later-supplied** row is the delivered winner: that is the single documented order
//! dependence, it is deterministic for a given delivery, and the equal-instant test pins both
//! directions of it rather than claiming order independence the implementation does not have.
//!
//! # Two limits worth naming
//!
//! `CollectionStore` cannot be handed to a state from here — `AppState::with_collection_store` is
//! `pub(crate)` — so a relaunch is not modelled; the read path's own unit tests cover the store.
//! What this file proves is the merge over the in-memory copy the command path holds.
//!
//! **A row carries no content yet.** `CollectionItem` is an item id, an `updated_at` version and a
//! `deleted` flag, so "reflects the change saved last" can only be asserted through those fields:
//! P1's item content arrives with the first domain type, and the comparison extends there. Both
//! platforms (TOOL-1): nothing here is platform-specific.

use std::collections::HashSet;
use std::sync::Arc;

use app_lib::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, Instant, ItemId,
};
use app_lib::state::AppState;
use app_lib::storage::AbsoluteInstant;
use app_lib::sync::{MergeOutcome, MergePath, ScriptedService};

/// The instant the seeded copies are loaded at, before any edit.
const SEEDED_AT: &str = "2026-10-19T09:00:00Z";

/// The base version both members start from.
const BASE_VERSION: &str = "2026-10-19T10:00:00Z";

/// The earlier edit: the version that loses when both edits reach the service.
const EARLIER_EDIT: &str = "2026-10-19T12:00:00Z";

/// The later edit: the winning version, and the one the service delivers.
const LATER_EDIT: &str = "2026-10-19T13:00:00Z";

/// The same instant as [`BASE_VERSION`], spelled with an offset instead of `Z`.
const BASE_VERSION_OFFSET: &str = "2026-10-19T12:00:00+02:00";

fn household_id(value: &str) -> HouseholdId {
    HouseholdId(value.to_owned())
}

fn item_id(value: &str) -> ItemId {
    ItemId(value.to_owned())
}

fn at(value: &str) -> Instant {
    Instant(value.to_owned())
}

/// One live row of a household's collection.
fn item(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: false,
    }
}

/// One removed row, still carrying its version: a conflict can be about a removal too.
fn tombstone(id: &str, updated_at: &str) -> CollectionItem {
    CollectionItem {
        item_id: item_id(id),
        updated_at: at(updated_at),
        deleted: true,
    }
}

/// One household's wire collection, as the guarded merge takes it.
fn collection(household: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
    HouseholdCollection {
        household_id: household.clone(),
        items,
        source: CollectionSource::Service,
        loaded_at: None,
    }
}

/// A device holding `items` as `household`'s loaded collection.
fn state_holding(household: &HouseholdId, items: Vec<CollectionItem>) -> AppState {
    let state = AppState::new(Arc::new(ScriptedService::new()), None);
    state
        .load_collection(household, collection(household, items), at(SEEDED_AT))
        .expect("the collection is addressed through its own household");

    state
}

/// Merge `items` into `household`'s copy on this device.
fn merge(state: &AppState, household: &HouseholdId, items: Vec<CollectionItem>) -> MergeOutcome {
    MergePath::new().merge(state, household, collection(household, items))
}

/// The rows this device holds for `household`.
fn held(state: &AppState, household: &HouseholdId) -> Vec<CollectionItem> {
    state
        .read_collection(household, CollectionSource::Local)
        .expect("this device holds the copy")
        .items
}

/// The row for `id`, which a resolved collection must carry exactly once.
fn row<'a>(items: &'a [CollectionItem], id: &str) -> &'a CollectionItem {
    items
        .iter()
        .find(|row| row.item_id == item_id(id))
        .expect("the item is present")
}

/// The ids of an outcome's item lists, for a compact assertion message.
fn ids(items: &[ItemId]) -> Vec<&str> {
    items.iter().map(|id| id.0.as_str()).collect()
}

/// Each row's identity — id, version and tombstone flag — sorted, so two copies can be compared
/// without depending on the order a map handed them back in.
fn sorted_rows(items: &[CollectionItem]) -> Vec<(String, String, bool)> {
    let mut rows: Vec<(String, String, bool)> = items
        .iter()
        .map(|row| (row.item_id.0.clone(), row.updated_at.0.clone(), row.deleted))
        .collect();
    rows.sort();

    rows
}

/// SYNC-5, "Two members edit the same item".
///
/// **WHEN** two members save a change to the same item and both changes reach the shared service
/// **THEN** exactly one version of the item remains, and it reflects the change that reached the
/// shared service last.
#[test]
fn two_members_edit_the_same_item() {
    let household = household_id("household-1");
    let base = vec![item("item-1", BASE_VERSION)];

    // Two members, each holding the same household's copy of the same item.
    let first = state_holding(&household, base.clone());
    let second = state_holding(&household, base);

    // Each member's edit lands on their own copy. Both reached the service, whose tuple made the
    // later instant the winning version — the row it now delivers.
    let first_edit = merge(&first, &household, vec![item("item-1", EARLIER_EDIT)]);
    let second_edit = merge(&second, &household, vec![item("item-1", LATER_EDIT)]);
    assert_eq!(ids(first_edit.applied()), ["item-1"]);
    assert_eq!(ids(second_edit.applied()), ["item-1"]);

    // THEN the delivered row resolves to exactly one surviving version on both copies: it replaces
    // the losing edit for the member whose change lost, and it is a replay for the member whose
    // change won.
    let replaced = merge(&first, &household, vec![item("item-1", LATER_EDIT)]);
    let replay = merge(&second, &household, vec![item("item-1", LATER_EDIT)]);
    assert_eq!(ids(replaced.applied()), ["item-1"]);
    assert!(
        replay.applied().is_empty(),
        "the winner is already this member's row"
    );
    assert_eq!(ids(replay.ignored()), ["item-1"]);

    for state in [&first, &second] {
        let rows = held(state, &household);
        assert_eq!(rows.len(), 1, "one surviving row, never a duplicate");
        assert_eq!(row(&rows, "item-1").updated_at, at(LATER_EDIT));
    }
    assert_eq!(
        sorted_rows(&held(&first, &household)),
        sorted_rows(&held(&second, &household)),
        "both members converge on the same row"
    );

    // AND a late re-delivery of the losing edit does not undo the winner: last-write-wins is by the
    // version, not by which row arrived most recently.
    let stale = merge(&first, &household, vec![item("item-1", EARLIER_EDIT)]);
    assert!(
        stale.applied().is_empty(),
        "an older version does not replace the winner"
    );
    assert_eq!(ids(stale.ignored()), ["item-1"]);
    assert_eq!(
        row(&held(&first, &household), "item-1").updated_at,
        at(LATER_EDIT)
    );

    // Never a surfaced failure: the merge has no error variant, and no outcome here carries a
    // refusal or a skipped merge.
    for outcome in [&first_edit, &second_edit, &replaced, &replay, &stale] {
        assert_eq!(outcome.refusal(), None);
        assert!(!outcome.not_loaded());
    }
}

/// SYNC-5, "Convergence does not duplicate or fail".
///
/// **WHEN** concurrent edits to the same item are resolved **THEN** no second copy of the item is
/// created, and neither member is shown a failure caused by the conflict.
///
/// The batch is settled three times: a resolution is idempotent, and a re-delivery must leave the
/// copy exactly as it was, not merely equal after a rewrite.
///
/// The "no second copy" half is asserted here as one row per item for a batch with distinct ids;
/// the same item carried twice in one batch is the duplicate-batch case, which the "Resolution
/// leaves no duplicate" test owns.
#[test]
fn convergence_does_not_duplicate_or_fail() {
    let household = household_id("household-1");
    let stored = vec![
        item("item-1", BASE_VERSION),
        // This device already holds the later version of item-2.
        item("item-2", LATER_EDIT),
    ];
    let state = state_holding(&household, stored);

    // The batch a settling member receives: item-1's later version, and item-2 at a version older
    // than the one this device holds.
    let batch = vec![item("item-1", LATER_EDIT), item("item-2", BASE_VERSION)];
    let settled = merge(&state, &household, batch.clone());
    assert_eq!(ids(settled.applied()), ["item-1"]);
    assert_eq!(ids(settled.ignored()), ["item-2"]);
    assert!(settled.changed());

    let after_first = held(&state, &household);
    assert_eq!(after_first.len(), 2, "one row per item");

    // THEN replaying the batch adds no row and changes none ...
    for _ in 0..2 {
        let replay = merge(&state, &household, batch.clone());

        assert!(replay.applied().is_empty(), "a replayed batch adds nothing");
        assert_eq!(ids(replay.ignored()), ["item-1", "item-2"]);
        assert!(!replay.changed());
        assert_eq!(
            held(&state, &household),
            after_first,
            "the rows are untouched"
        );

        // ... and the conflict is not a failure: no refusal, no skipped merge, no error value to
        // show either member.
        assert_eq!(replay.refusal(), None);
        assert!(!replay.not_loaded());
    }
}

/// NFR-2, "Concurrent edits resolve without failing".
///
/// **WHEN** two members edit the same item without seeing each other's change **THEN** one version
/// of the item remains, reflecting the edit saved last, and neither member's edit is reported as
/// failed because of the conflict.
#[test]
fn concurrent_edits_resolve_without_failing() {
    let household = household_id("household-1");
    let state = state_holding(
        &household,
        vec![
            item("item-1", BASE_VERSION),
            // The later version of item-2 is already here, so the batch's row for it loses.
            item("item-2", LATER_EDIT),
            item("item-3", BASE_VERSION),
        ],
    );

    // One batch of concurrent edits: a winning later version, a losing older one, an identical
    // re-delivery, and an item this device has never seen.
    let outcome = merge(
        &state,
        &household,
        vec![
            item("item-1", LATER_EDIT),
            item("item-2", BASE_VERSION),
            item("item-3", BASE_VERSION),
            item("item-4", BASE_VERSION),
        ],
    );

    // THEN each item is resolved exactly once and reported exactly once.
    assert_eq!(ids(outcome.applied()), ["item-1", "item-4"]);
    assert_eq!(ids(outcome.ignored()), ["item-2", "item-3"]);

    let mut reported: Vec<&str> = ids(outcome.applied());
    reported.extend(ids(outcome.ignored()));
    let distinct: HashSet<&str> = reported.iter().copied().collect();
    assert_eq!(reported.len(), distinct.len(), "no item is reported twice");
    assert_eq!(
        distinct.len(),
        4,
        "every item of the batch is reported once"
    );

    // AND neither member is shown a failure: the conflict has no error to show.
    assert_eq!(outcome.refusal(), None);
    assert!(!outcome.not_loaded());

    // ... and the copy holds each item once, at the version that won its comparison.
    let rows = held(&state, &household);
    assert_eq!(rows.len(), 4);
    assert_eq!(row(&rows, "item-1").updated_at, at(LATER_EDIT));
    assert_eq!(
        row(&rows, "item-2").updated_at,
        at(LATER_EDIT),
        "the losing edit did not replace the later version"
    );
    assert_eq!(row(&rows, "item-3").updated_at, at(BASE_VERSION));
    assert_eq!(row(&rows, "item-4").updated_at, at(BASE_VERSION));
}

/// NFR-2, "Resolution leaves no duplicate".
///
/// **WHEN** concurrent edits are resolved **THEN** the affected item exists exactly once for every
/// member.
///
/// The same batch is applied in both orders to two members' copies, and the two copies start with
/// `item-2` at different versions — the concurrent-edit shape. The batch carries `item-2` twice at
/// different versions too, so its surviving row is the comparison's winner rather than whichever
/// row the batch happened to list last; the two members therefore end with the same row.
#[test]
fn resolution_leaves_no_duplicate() {
    let household = household_id("household-1");
    let batch = vec![
        item("item-1", LATER_EDIT),
        item("item-2", BASE_VERSION),
        item("item-2", LATER_EDIT),
        item("item-3", BASE_VERSION),
    ];

    let forward = state_holding(&household, vec![item("item-2", BASE_VERSION)]);
    let backward = state_holding(&household, vec![item("item-2", LATER_EDIT)]);
    merge(&forward, &household, batch.clone());

    let mut reversed = batch.clone();
    reversed.reverse();
    merge(&backward, &household, reversed);

    // THEN the affected items exist exactly once for every member: one row per distinct id, and the
    // count is the input *set*, not the batch length.
    for state in [&forward, &backward] {
        let rows = held(state, &household);
        let row_ids: Vec<&str> = rows.iter().map(|row| row.item_id.0.as_str()).collect();
        let distinct: HashSet<&str> = row_ids.iter().copied().collect();

        assert_eq!(rows.len(), 3, "the batch names three items, held once each");
        assert_eq!(row_ids.len(), distinct.len(), "no item has two rows");
        assert_eq!(distinct.len(), 3);
    }

    // AND the resolution does not depend on delivery order: both members hold the same rows, with
    // item-2 at the later version in both, because the comparison is by version rather than by the
    // row that arrived last.
    assert_eq!(
        sorted_rows(&held(&forward, &household)),
        sorted_rows(&held(&backward, &household)),
        "the same rows, whatever order the batch arrived in"
    );
    for state in [&forward, &backward] {
        assert_eq!(
            row(&held(state, &household), "item-2").updated_at,
            at(LATER_EDIT)
        );
    }
}

/// The equal-instant rule the implementation applies (D0.5, D0.7 criterion 3).
///
/// Versions compare as instants, not text; an identical row at the same instant is a replay; a row
/// whose state differs **wins**, because the service's tuple already chose it, and the rule is
/// symmetric — whichever same-instant row is delivered is the one that wins. Within one batch that
/// carries both, the later-supplied row wins, which is the documented order dependence: the test
/// pins both directions instead of claiming an order independence the merge does not have.
#[test]
fn two_versions_with_the_same_instant_resolve_deterministically() {
    let household = household_id("household-1");

    // The two spellings name one instant, so a delivered row at that instant is the same version as
    // the stored one — not a later one. The precondition below pins that a lexical comparison would
    // order them the other way round, so the test would catch a text comparison.
    assert!(at(BASE_VERSION_OFFSET).0 > at(BASE_VERSION).0);
    assert_eq!(
        AbsoluteInstant::parse(&at(BASE_VERSION_OFFSET)),
        AbsoluteInstant::parse(&at(BASE_VERSION))
    );

    let state = state_holding(&household, vec![item("item-1", BASE_VERSION)]);
    let replay = merge(
        &state,
        &household,
        vec![item("item-1", BASE_VERSION_OFFSET)],
    );
    assert!(
        replay.applied().is_empty(),
        "the same instant is the same version"
    );
    assert_eq!(ids(replay.ignored()), ["item-1"]);
    assert_eq!(
        row(&held(&state, &household), "item-1").updated_at,
        at(BASE_VERSION)
    );

    // A row whose state differs at the same instant wins, because the service's tuple chose it: the
    // removal replaces the live row ...
    let removed = merge(
        &state,
        &household,
        vec![tombstone("item-1", BASE_VERSION_OFFSET)],
    );
    assert_eq!(
        ids(removed.applied()),
        ["item-1"],
        "the delivered row wins the tie"
    );
    assert!(row(&held(&state, &household), "item-1").deleted);

    // ... and delivering that same row again is a replay, not a second change.
    let settled = merge(&state, &household, vec![tombstone("item-1", BASE_VERSION)]);
    assert!(settled.applied().is_empty());
    assert_eq!(ids(settled.ignored()), ["item-1"]);

    // The rule is symmetric: a member whose copy holds the removal receives the live row at the
    // same instant, and the delivered row wins there too.
    let other = state_holding(&household, vec![tombstone("item-1", BASE_VERSION)]);
    let restored = merge(
        &other,
        &household,
        vec![item("item-1", BASE_VERSION_OFFSET)],
    );
    assert_eq!(ids(restored.applied()), ["item-1"]);
    assert!(!row(&held(&other, &household), "item-1").deleted);

    // Two differing same-instant rows in one batch: the later-supplied row is the delivered winner.
    // The reverse order produces the reverse winner, which is exactly the documented behaviour.
    let forward = state_holding(&household, Vec::new());
    let applied = merge(
        &forward,
        &household,
        vec![
            tombstone("item-1", BASE_VERSION),
            item("item-1", BASE_VERSION_OFFSET),
        ],
    );
    assert_eq!(ids(applied.applied()), ["item-1"]);
    let rows = held(&forward, &household);
    assert_eq!(rows.len(), 1, "the batch's two rows are one item");
    assert!(!row(&rows, "item-1").deleted);

    let backward = state_holding(&household, Vec::new());
    let applied = merge(
        &backward,
        &household,
        vec![
            item("item-1", BASE_VERSION),
            tombstone("item-1", BASE_VERSION_OFFSET),
        ],
    );
    assert_eq!(ids(applied.applied()), ["item-1"]);
    let rows = held(&backward, &household);
    assert_eq!(rows.len(), 1, "the batch's two rows are one item");
    assert!(row(&rows, "item-1").deleted);
}
