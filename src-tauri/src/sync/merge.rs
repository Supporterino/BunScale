//! The last-write-wins merge: one surviving version per item, and no failure for the loser
//! (SYNC-1, SYNC-5, NFR-2; D0.7 criterion 3).
//!
//! This module is the comparator every layer below it deliberately deferred to: the collection
//! layout holds a version and compares nothing (task 2.3), the guards upsert whatever row they are
//! handed (task 2.7), and the read path replaces a copy wholesale rather than merging (task 2.5).
//! The merge is where two versions of one item meet and exactly one of them survives.
//!
//! It is **not wired into [`ReadPath`](super::ReadPath)**: P0's refresh asks for the whole collection
//! and replaces the copy, and merging a whole answer would be a different operation with the same
//! result. This is the seam P6's incremental feed and P1's item writes use, and the reason it is
//! built now rather than when the first caller lands is that the comparison rules are part of the
//! collection layer's contract and must not be re-derived at each call site.
//!
//! # One comparison, and it is by instant (D0.5, D0.7 criterion 3)
//!
//! The frozen [`CollectionItem`] carries `updated_at`, the version marker. Versions are compared as
//! [`AbsoluteInstant`]s, never as text: `2026-10-19T12:00:00+02:00` and `2026-10-19T10:00:00Z` name
//! one version even though their text sorts differently. The rules are:
//!
//! | Stored row | Incoming row | Outcome |
//! |---|---|---|
//! | none | parseable version | applied — an item this device did not hold |
//! | none | unparseable version | **ignored** — a version that cannot be ordered is never stored |
//! | earlier version | later version | applied — the incoming row survives |
//! | later version | earlier version | **ignored** — the stored row survives |
//! | same version, same row state | same | **ignored** — a replay changes nothing |
//! | same version, different row state | same | applied — the service's tuple chose the delivered row |
//! | unparseable version | parseable version | applied — a row without a comparable version cannot be newer |
//! | any | unparseable version | **ignored** — it cannot be ordered, and is never an error |
//!
//! The last three rows are defensive rather than expected: a version arrives from the service, which
//! writes absolute instants. A value that cannot be parsed is not allowed to panic, to be compared
//! lexically (which would order some offsets wrongly) or to be reported as a failed conflict. It is
//! also not allowed to enter an otherwise empty copy, because the device could then hold a row whose
//! version no later merge can order.
//!
//! # The tie rule, and why it is safe
//!
//! D0.7 criterion 3 makes the service's `(updated_at, writer_id, request_id)` tuple the authority
//! for last-write-wins, and only the first element of that tuple is on the wire. The client
//! therefore cannot reproduce the service's decision, and must not try to: a delivered row is the
//! version the service already chose. At an equal `updated_at`:
//!
//! - if the delivered row's state is the one already stored (the same tombstone flag — P1 will add
//!   content to this comparison), the merge is a **replay** and changes nothing;
//! - if the state differs, the delivered row **replaces** the stored one, because the service could
//!   only have chosen one winner for that instant and the row it delivers is that winner. Keeping
//!   the stored row instead would let a device hold a losing same-instant version forever: every
//!   later delivery of the winner would tie again and be ignored.
//!
//! When one batch carries two differing rows for the same item at the same instant, the **later row
//! in the batch is the winner**: each row is resolved against the winner so far, so the last
//! supplied row prevails for that tie. That matches the order in which the service delivered the
//! batch and the storage layout's "last row supplied for a key" rule, and it still leaves exactly
//! one row.
//!
//! Either way the rule is deterministic, changes at most one row per item and never creates a
//! second row. It is the *service's* tuple that decides which same-instant row is delivered; the
//! client only decides, without ambiguity, whether that delivery is new information.
//!
//! # Exactly one row per item (SYNC-1)
//!
//! Every incoming row is resolved against a working view keyed by [`ItemId`], and only the distinct
//! winners are handed to [`AppState::merge_collection`]. The guard then upserts them, so:
//! duplicates inside one incoming batch collapse to their winning row; a merge that applies the
//! same batch twice reaches the guard with an empty batch the second time; and the household's copy
//! still holds exactly one row per item. A merge never appends.
//!
//! # A merge needs somewhere to merge into
//!
//! The household's collection must have been **loaded** on this device (`loaded_at: Some(..)`).
//! The merge never creates a copy: a non-empty copy without the load marker would have SYNC-3's
//! never-loaded shape while containing rows, and the persistence layer saves only loaded copies,
//! so those rows would silently vanish at the next launch. A merge into a household with no copy,
//! or with a never-loaded one, writes nothing and reports [`MergeOutcome::not_loaded`]; the
//! caller's answer is a load first, not a merge.
//!
//! # Loads and merges for one household must be serialised
//!
//! The comparison reads the copy under one lock acquisition and [`AppState::merge_collection`]
//! takes the lock again to upsert. `AppState` offers no compare-and-upsert operation, so a load
//! landing between the two would not be seen: the merge could write a winner chosen against the
//! pre-load copy over a row the load has since made newer. This is an API requirement, not an
//! implementation shortcut — a caller must not run a load and a merge for the same household
//! concurrently, and both belong to one serialised per-household operation. P0 satisfies it
//! because it has no merge caller at all — [`ReadPath::refresh`](super::ReadPath::refresh) only
//! loads — so no P0 path forms the pair. P6's incremental feed and P1's item writes must call the
//! merge from the same serialised refresh operation as the load (or wait for an atomic
//! compare-and-merge on [`AppState`]), never from a second task for that household.
//!
//! # A losing version is not a failure (NFR-2)
//!
//! [`MergePath::merge`] returns [`MergeOutcome`], not a `Result`. There is no error variant a
//! conflict could be turned into, so no caller can present "your change lost the race" as a failed
//! save. The outcome names the items whose copy changed ([`MergeOutcome::applied`]) and the items
//! that were not written ([`MergeOutcome::ignored`]), which is what a caller needs to decide whether
//! to re-read a rendered value — not what to show as an error.
//!
//! A storage-boundary refusal is carried as an **outcome field**, not as an error either
//! ([`MergeOutcome::refusal`]): a foreign-household value or a write to an immutable entity is an
//! invariant violation, but it still does not make the conflict a failed save, and the whole batch
//! is then in `ignored` because nothing was written.
//!
//! # What this module deliberately is not
//!
//! It owns no clock, opens no transport and writes no file. The guarded merge it calls is what
//! schedules the device's own best-effort persistence, and only after a batch actually applied
//! rows. It does not own the change-feed cursor (P6) or a payload schema (P1). The frozen wire
//! types are read, never extended.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::ipc::types::{
    CollectionItem, CollectionSource, HouseholdCollection, HouseholdId, ItemId,
};
use crate::state::AppState;
use crate::storage::{AbsoluteInstant, BoundaryRefusal};

/// The last-write-wins merge over one device's copy of a household collection (SYNC-5, NFR-2).
///
/// The merge holds no data and no transport: the copy is the app state's, and the rows to merge are
/// passed in. It is stateless, so a caller can construct one per operation ([`MergePath::new`])
/// without losing anything; there is no cache whose lifetime would have to be managed.
#[derive(Debug, Default)]
pub struct MergePath;

impl MergePath {
    /// A merge path.
    pub fn new() -> Self {
        Self
    }

    /// Merge `incoming` into the copy this device holds for `household_id` (SYNC-1, SYNC-5).
    ///
    /// The comparison is per item (see the module documentation): a later version replaces an
    /// earlier one, an earlier one is ignored, and an exact `updated_at` tie applies the delivered
    /// row only when it differs from the stored one. Only the items whose final state differs from
    /// the stored copy are handed to [`AppState::merge_collection`], so a replay reaches the storage
    /// boundary with nothing to write.
    ///
    /// The copy's `loaded_at` marker is not moved: a merge is not a load (SYNC-3), and the guard's
    /// merge path deliberately leaves the marker alone. The household's collection must already
    /// have been loaded — a merge into no copy, or into a never-loaded one, writes nothing and
    /// reports [`MergeOutcome::not_loaded`], because a non-empty copy with no marker would have
    /// SYNC-3's never-loaded shape and would not be persisted.
    ///
    /// The read of the copy and the guard's upsert are two lock acquisitions (see the module's
    /// serialisation rule): a caller must not run a load for the same household concurrently with
    /// this call.
    ///
    /// There is no `Result`: a losing version is reported in [`MergeOutcome`], and a boundary
    /// refusal is one of its fields. Neither is an error to show the person (NFR-2).
    pub fn merge(
        &self,
        state: &AppState,
        household_id: &HouseholdId,
        incoming: HouseholdCollection,
    ) -> MergeOutcome {
        // A value describing another household is not filed under the requested one. Checked here so
        // the refusal names the same pair the guard would, without reading the wrong copy first.
        if &incoming.household_id != household_id {
            return MergeOutcome::refused(
                BoundaryRefusal::ForeignHousehold {
                    requested: household_id.clone(),
                    owner: incoming.household_id.clone(),
                },
                distinct_item_ids(&incoming.items),
            );
        }

        let stored = state
            .read_collection(household_id, CollectionSource::Local)
            .filter(|collection| collection.loaded_at.is_some());
        let Some(stored) = stored else {
            // There is no loaded copy to merge into. Writing the rows anyway would create a
            // non-empty value with SYNC-3's never-loaded shape, and the persistence layer saves
            // only loaded copies, so the rows would be lost at the next launch.
            return MergeOutcome::skipped_not_loaded(distinct_item_ids(&incoming.items));
        };

        let original: HashMap<ItemId, CollectionItem> = stored
            .items
            .into_iter()
            .map(|row| (row.item_id.clone(), row))
            .collect();
        // The working view starts as the stored copy, so every incoming row — including a later
        // duplicate of one already won — is resolved against the winner so far, not against the
        // device's original row. The later row therefore wins an equal-instant tie within a batch.
        let mut working = original.clone();

        for row in &incoming.items {
            if incoming_wins(row, working.get(&row.item_id)) {
                working.insert(row.item_id.clone(), row.clone());
            }
        }

        // One row per item, in the order the item first appears in the batch, so the outcome and the
        // batch handed to the guard are stable rather than dependent on a map's iteration order.
        let mut seen = HashSet::new();
        let mut winners = Vec::new();
        let mut applied = Vec::new();

        for row in &incoming.items {
            if !seen.insert(row.item_id.clone()) {
                continue;
            }

            if let Some(current) = working.get(&row.item_id)
                && original.get(&row.item_id) != Some(current)
            {
                winners.push(current.clone());
                applied.push(row.item_id.clone());
            }
        }

        let applied_ids: HashSet<ItemId> = applied.iter().cloned().collect();
        let ignored: Vec<ItemId> = distinct_item_ids(&incoming.items)
            .into_iter()
            .filter(|item_id| !applied_ids.contains(item_id))
            .collect();

        if winners.is_empty() {
            // Nothing differs from the stored copy: a replay, an older batch, or a batch whose rows
            // all lost. The guard is not called, so no persistence write is scheduled for a no-op.
            return MergeOutcome::accepted(applied, ignored);
        }

        let merged = HouseholdCollection {
            household_id: household_id.clone(),
            items: winners,
            // A merge is never a load: the guard keeps these rows and ignores both fields. `local`
            // is the honest label because this is the device's own copy being changed.
            source: CollectionSource::Local,
            loaded_at: None,
        };

        match state.merge_collection(household_id, merged) {
            Ok(()) => MergeOutcome::accepted(applied, ignored),
            // The guard checks the whole batch before writing any row, so a refusal means none of
            // the winners landed: every incoming item is reported as ignored alongside the reason.
            Err(refusal) => MergeOutcome::refused(refusal, distinct_item_ids(&incoming.items)),
        }
    }
}

/// What one merge did to the device's copy (SYNC-1, SYNC-5, NFR-2).
///
/// The type has no error variant and no failed state: a version that lost is a fact about the copy,
/// not a failure of the operation. Its two id lists partition the distinct items the incoming batch
/// named — every item is in exactly one of [`MergeOutcome::applied`] and
/// [`MergeOutcome::ignored`] — so a caller can tell whether it holds something stale without
/// inventing an error to display. A merge that could not run at all is reported by
/// [`MergeOutcome::not_loaded`], not by an error either.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeOutcome {
    /// The items whose stored row changed, each once, in the order the item first appeared in the
    /// batch. Empty means the copy is exactly as it was.
    applied: Vec<ItemId>,
    /// The items that were **not** written: their incoming version was older, identical or
    /// unparseable, or — when [`MergeOutcome::refusal`] or [`MergeOutcome::not_loaded`] is set —
    /// every item of a merge that could not run.
    ignored: Vec<ItemId>,
    /// Why the storage boundary refused the batch, if it did.
    ///
    /// This is an invariant violation (a foreign household, a write to an immutable entity), not a
    /// lost conflict; it is carried as an outcome so a caller can decide what to do without treating
    /// the merge itself as a failed save. When `Some`, nothing was applied.
    refusal: Option<BoundaryRefusal>,
    /// Whether the merge was skipped because this device holds no **loaded** copy of the household.
    ///
    /// A merge only ever updates rows of a collection that was loaded; it never creates one. When
    /// this is `true`, the caller should load the collection first, and nothing was written.
    not_loaded: bool,
}

impl MergeOutcome {
    /// The outcome of a merge the storage boundary accepted.
    fn accepted(applied: Vec<ItemId>, ignored: Vec<ItemId>) -> Self {
        Self {
            applied,
            ignored,
            refusal: None,
            not_loaded: false,
        }
    }

    /// The outcome of a batch the storage boundary refused whole.
    fn refused(refusal: BoundaryRefusal, ignored: Vec<ItemId>) -> Self {
        Self {
            applied: Vec::new(),
            ignored,
            refusal: Some(refusal),
            not_loaded: false,
        }
    }

    /// The outcome of a merge that had no loaded copy to merge into.
    fn skipped_not_loaded(ignored: Vec<ItemId>) -> Self {
        Self {
            applied: Vec::new(),
            ignored,
            refusal: None,
            not_loaded: true,
        }
    }

    /// The items whose stored row changed, each once.
    pub fn applied(&self) -> &[ItemId] {
        &self.applied
    }

    /// The items that were not written, each once: a losing, replayed or unparseable incoming
    /// version, or every row of a merge that could not run.
    pub fn ignored(&self) -> &[ItemId] {
        &self.ignored
    }

    /// Why the storage boundary refused the batch, or `None` when it accepted it (or never ran).
    pub fn refusal(&self) -> Option<&BoundaryRefusal> {
        self.refusal.as_ref()
    }

    /// Whether the merge was skipped because no loaded copy of the household exists.
    ///
    /// This is not a failure: it means the caller has to load before merging. Nothing was written.
    pub fn not_loaded(&self) -> bool {
        self.not_loaded
    }

    /// Whether the device's copy changed, and so whether a caller holding a rendered value should
    /// re-read it. A refusal or a skipped merge changes nothing and answers `false`.
    pub fn changed(&self) -> bool {
        !self.applied.is_empty()
    }
}

/// Whether the incoming row should replace the stored row (or become the row for an item the
/// device does not hold).
///
/// The comparison is by absolute instant; the equal-instant rule is the tie rule documented in the
/// module. An item with no stored row is admitted only when its version parses: a malformed version
/// cannot be ordered, so storing it would leave a row no later merge could resolve. The row's only
/// state in P0 is its tombstone flag, so "the same row state" is checked as that flag; when P1 adds
/// content, this is the comparison that extends to it. A malformed incoming version is ignored
/// rather than allowed to win by accident, and a stored version that cannot be parsed cannot claim
/// to be newer than a delivered, parseable one.
fn incoming_wins(incoming: &CollectionItem, stored: Option<&CollectionItem>) -> bool {
    let incoming_version = AbsoluteInstant::parse(&incoming.updated_at);

    let Some(stored) = stored else {
        return incoming_version.is_ok();
    };

    match (incoming_version, AbsoluteInstant::parse(&stored.updated_at)) {
        (Ok(incoming_at), Ok(stored_at)) => match incoming_at.cmp(&stored_at) {
            Ordering::Greater => true,
            Ordering::Less => false,
            // The service's version tuple already chose the row it delivered; the client can only
            // decide whether this delivery says anything new. A differing tombstone flag does; an
            // identical row is a replay. When two such rows appear in one batch, the later one is
            // resolved last and therefore wins the tie.
            Ordering::Equal => incoming.deleted != stored.deleted,
        },
        (Ok(_), Err(_)) => true,
        (Err(_), _) => false,
    }
}

/// The distinct item ids of `items`, in first-appearance order.
///
/// Used for the outcome reports, so an item named twice in one batch is reported once.
fn distinct_item_ids(items: &[CollectionItem]) -> Vec<ItemId> {
    let mut seen = HashSet::new();

    items
        .iter()
        .filter(|row| seen.insert(row.item_id.clone()))
        .map(|row| row.item_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ipc::types::Instant;
    use crate::sync::ScriptedService;

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn item(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: false,
        }
    }

    fn tombstone(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: true,
        }
    }

    fn collection(household_id: &HouseholdId, items: Vec<CollectionItem>) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items,
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }

    /// A state holding `items` as `household_id`'s loaded copy.
    fn state_holding(household_id: &HouseholdId, items: Vec<CollectionItem>) -> AppState {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        state
            .load_collection(
                household_id,
                collection(household_id, items),
                at("2026-10-19T09:00:00Z"),
            )
            .expect("the copy is addressed correctly");

        state
    }

    fn held(state: &AppState, household_id: &HouseholdId) -> Vec<CollectionItem> {
        state
            .read_collection(household_id, CollectionSource::Local)
            .expect("this device holds the copy")
            .items
    }

    fn row<'a>(items: &'a [CollectionItem], item_id: &str) -> &'a CollectionItem {
        items
            .iter()
            .find(|row| row.item_id == ItemId(item_id.to_owned()))
            .expect("the item is present")
    }

    fn ids(item_ids: &[ItemId]) -> Vec<&str> {
        item_ids.iter().map(|id| id.0.as_str()).collect()
    }

    #[test]
    fn a_newer_version_replaces_the_stored_row() {
        let one = household("household-1");
        let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T11:00:00Z")]),
        );

        assert_eq!(ids(outcome.applied()), ["item-1"]);
        assert!(outcome.ignored().is_empty());
        assert_eq!(outcome.refusal(), None);
        assert!(outcome.changed());

        let rows = held(&state, &one);
        assert_eq!(rows.len(), 1, "the merge upserts, it never appends");
        assert_eq!(row(&rows, "item-1").updated_at, at("2026-10-19T11:00:00Z"));
        assert!(!row(&rows, "item-1").deleted);
    }

    #[test]
    fn an_older_version_is_ignored_and_is_not_a_failure() {
        let one = household("household-1");
        let stored = item("item-1", "2026-10-19T11:00:00Z");
        let state = state_holding(&one, vec![stored.clone()]);

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]),
        );

        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(outcome.refusal(), None, "losing a version is not an error");
        assert!(!outcome.changed());

        assert_eq!(held(&state, &one), vec![stored]);
    }

    #[test]
    fn an_equal_version_tie_is_deterministic() {
        let one = household("household-1");
        let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);
        let path = MergePath::new();

        // An identical delivered row is a replay: nothing changes, however often it arrives.
        for _ in 0..2 {
            let replay = path.merge(
                &state,
                &one,
                collection(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]),
            );

            assert!(replay.applied().is_empty());
            assert_eq!(ids(replay.ignored()), ["item-1"]);
            assert_eq!(replay.refusal(), None);
        }

        // A different row state at the exact same instant is the version the service chose, so it
        // replaces the stored row — once. Re-delivering it is then a replay.
        let first = path.merge(
            &state,
            &one,
            collection(&one, vec![tombstone("item-1", "2026-10-19T10:00:00Z")]),
        );

        assert_eq!(ids(first.applied()), ["item-1"]);
        assert!(row(&held(&state, &one), "item-1").deleted);

        let second = path.merge(
            &state,
            &one,
            collection(&one, vec![tombstone("item-1", "2026-10-19T10:00:00Z")]),
        );

        assert!(second.applied().is_empty());
        assert_eq!(ids(second.ignored()), ["item-1"]);
    }

    #[test]
    fn an_equal_instant_duplicate_in_one_batch_keeps_the_last_supplied_row() {
        let one = household("household-1");
        let path = MergePath::new();

        // Same batch order: tombstone then live. Both name the same instant, so the later row is the
        // winner and the item ends live at that instant.
        let first = {
            let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);
            let outcome = path.merge(
                &state,
                &one,
                collection(
                    &one,
                    vec![
                        tombstone("item-1", "2026-10-19T11:00:00Z"),
                        item("item-1", "2026-10-19T11:00:00Z"),
                    ],
                ),
            );

            assert_eq!(ids(outcome.applied()), ["item-1"]);
            let rows = held(&state, &one);
            assert_eq!(rows.len(), 1);
            rows[0].clone()
        };
        assert!(!first.deleted, "the later live row wins the tie");

        // The reverse order keeps the tombstone: the result follows the batch's order, and the
        // module documents that the last supplied row wins.
        let second = {
            let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);
            let outcome = path.merge(
                &state,
                &one,
                collection(
                    &one,
                    vec![
                        item("item-1", "2026-10-19T11:00:00Z"),
                        tombstone("item-1", "2026-10-19T11:00:00Z"),
                    ],
                ),
            );

            assert_eq!(ids(outcome.applied()), ["item-1"]);
            let rows = held(&state, &one);
            assert_eq!(rows.len(), 1);
            rows[0].clone()
        };
        assert!(second.deleted, "the later tombstone wins the tie");
    }

    #[test]
    fn versions_compare_as_instants_not_as_text() {
        let one = household("household-1");
        let stored = item("item-1", "2026-10-19T10:00:00Z");
        let state = state_holding(&one, vec![stored.clone()]);
        let path = MergePath::new();

        // 12:00+02:00 is the same instant as the stored 10:00Z: the same version, so a replay.
        let same = path.merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T12:00:00+02:00")]),
        );

        assert!(same.applied().is_empty());
        assert_eq!(held(&state, &one), vec![stored]);

        // One hour later, spelled with an offset: it is genuinely newer.
        let newer = path.merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T13:00:00+02:00")]),
        );

        assert_eq!(ids(newer.applied()), ["item-1"]);
        assert_eq!(
            row(&held(&state, &one), "item-1").updated_at,
            at("2026-10-19T13:00:00+02:00")
        );
    }

    #[test]
    fn an_older_instant_whose_text_sorts_higher_is_still_older() {
        let one = household("household-1");
        let stored = item("item-1", "2026-10-19T10:00:00Z");
        let incoming = item("item-1", "2026-10-19T11:00:00+02:00");
        let state = state_holding(&one, vec![stored.clone()]);

        // The comparison is by moment, not by text: 11:00+02:00 is 09:00Z, one hour *before* the
        // stored row, even though its text sorts after the stored text.
        assert!(
            incoming.updated_at.0 > stored.updated_at.0,
            "the test must exercise the case where lexical order disagrees"
        );

        let outcome = MergePath::new().merge(&state, &one, collection(&one, vec![incoming]));

        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(held(&state, &one), vec![stored]);
    }

    #[test]
    fn a_stored_unparseable_version_yields_to_a_parseable_delivered_one() {
        let one = household("household-1");
        let stored = item("item-1", "not a timestamp");
        let state = state_holding(&one, vec![stored]);
        let delivered = item("item-1", "2026-10-19T10:00:00Z");

        let outcome =
            MergePath::new().merge(&state, &one, collection(&one, vec![delivered.clone()]));

        assert_eq!(ids(outcome.applied()), ["item-1"]);
        assert!(outcome.ignored().is_empty());
        assert_eq!(outcome.refusal(), None);
        assert_eq!(held(&state, &one), vec![delivered]);
    }

    #[test]
    fn replaying_the_same_batch_adds_nothing() {
        let one = household("household-1");
        let state = state_holding(&one, Vec::new());
        let path = MergePath::new();
        let batch = vec![
            item("item-1", "2026-10-19T10:00:00Z"),
            item("item-2", "2026-10-19T10:00:00Z"),
        ];

        let first = path.merge(&state, &one, collection(&one, batch.clone()));
        assert_eq!(ids(first.applied()), ["item-1", "item-2"]);

        let second = path.merge(&state, &one, collection(&one, batch));
        assert!(second.applied().is_empty());
        assert_eq!(ids(second.ignored()), ["item-1", "item-2"]);
        assert_eq!(second.refusal(), None);

        let rows = held(&state, &one);
        assert_eq!(rows.len(), 2, "a replay adds no row");
    }

    #[test]
    fn duplicate_rows_in_one_batch_collapse_to_the_winning_version() {
        let one = household("household-1");
        let state = state_holding(&one, Vec::new());

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(
                &one,
                vec![
                    item("item-1", "2026-10-19T10:00:00Z"),
                    item("item-1", "2026-10-19T12:00:00Z"),
                    item("item-1", "2026-10-19T11:00:00Z"),
                ],
            ),
        );

        assert_eq!(ids(outcome.applied()), ["item-1"]);
        assert!(outcome.ignored().is_empty());

        let rows = held(&state, &one);
        assert_eq!(rows.len(), 1, "exactly one copy per item");
        assert_eq!(row(&rows, "item-1").updated_at, at("2026-10-19T12:00:00Z"));
    }

    #[test]
    fn tombstones_participate_in_the_version_order() {
        let one = household("household-1");
        let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);
        let path = MergePath::new();

        // A newer removal wins and is stored as the tombstone it is.
        let removed = path.merge(
            &state,
            &one,
            collection(&one, vec![tombstone("item-1", "2026-10-19T11:00:00Z")]),
        );
        assert_eq!(ids(removed.applied()), ["item-1"]);
        assert!(row(&held(&state, &one), "item-1").deleted);

        // An older live row does not resurrect it.
        let older = path.merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T09:00:00Z")]),
        );
        assert!(older.applied().is_empty());
        assert_eq!(ids(older.ignored()), ["item-1"]);
        assert!(row(&held(&state, &one), "item-1").deleted);

        // A newer live row does, and the tombstone's version is what it has to beat.
        let restored = path.merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T12:00:00Z")]),
        );
        assert_eq!(ids(restored.applied()), ["item-1"]);
        let rows = held(&state, &one);
        assert!(!row(&rows, "item-1").deleted);
        assert_eq!(row(&rows, "item-1").updated_at, at("2026-10-19T12:00:00Z"));
    }

    #[test]
    fn a_batch_that_mixes_winners_and_losers_reports_each_item_once() {
        let one = household("household-1");
        let state = state_holding(
            &one,
            vec![
                item("item-1", "2026-10-19T10:00:00Z"),
                item("item-2", "2026-10-19T12:00:00Z"),
            ],
        );

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(
                &one,
                vec![
                    item("item-1", "2026-10-19T11:00:00Z"),
                    item("item-2", "2026-10-19T11:00:00Z"),
                    item("item-3", "2026-10-19T10:00:00Z"),
                ],
            ),
        );

        assert_eq!(ids(outcome.applied()), ["item-1", "item-3"]);
        assert_eq!(ids(outcome.ignored()), ["item-2"]);

        let rows = held(&state, &one);
        assert_eq!(rows.len(), 3);
        assert_eq!(row(&rows, "item-1").updated_at, at("2026-10-19T11:00:00Z"));
        assert_eq!(row(&rows, "item-2").updated_at, at("2026-10-19T12:00:00Z"));
    }

    #[test]
    fn a_merge_leaves_the_loaded_at_marker_alone() {
        let one = household("household-1");
        let state = state_holding(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]);
        let loaded_at = at("2026-10-19T09:00:00Z");

        MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-2", "2026-10-19T10:00:00Z")]),
        );

        let copy = state
            .read_collection(&one, CollectionSource::Local)
            .expect("the copy is held");
        assert_eq!(copy.loaded_at, Some(loaded_at), "a merge is not a load");
        assert_eq!(copy.items.len(), 2);
    }

    #[test]
    fn a_merge_requires_a_loaded_copy_and_never_creates_one() {
        let one = household("household-1");
        let two = household("household-2");
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let path = MergePath::new();

        // No copy at all: there is nothing to merge into.
        let outcome = path.merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T10:00:00Z")]),
        );

        assert!(outcome.not_loaded());
        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(outcome.refusal(), None);
        assert!(!outcome.changed());
        assert!(
            state
                .read_collection(&one, CollectionSource::Local)
                .is_none(),
            "a merge never creates a copy"
        );

        // A copy with rows but no load marker is the shape the persistence layer skips. The guard
        // can produce it directly, but a merge must not treat it as loaded either.
        state
            .merge_collection(
                &two,
                collection(&two, vec![item("item-1", "2026-10-19T10:00:00Z")]),
            )
            .expect("the guarded merge creates a never-loaded copy");

        let outcome = path.merge(
            &state,
            &two,
            collection(&two, vec![item("item-2", "2026-10-19T10:00:00Z")]),
        );

        assert!(outcome.not_loaded());
        assert_eq!(ids(outcome.ignored()), ["item-2"]);
        let copy = state
            .read_collection(&two, CollectionSource::Local)
            .expect("the guard's copy exists");
        assert_eq!(copy.loaded_at, None);
        assert_eq!(
            copy.items,
            vec![item("item-1", "2026-10-19T10:00:00Z")],
            "the unloaded copy was not written"
        );
    }

    #[test]
    fn a_collection_describing_another_household_is_refused_whole() {
        let one = household("household-1");
        let two = household("household-2");
        let stored = item("item-1", "2026-10-19T10:00:00Z");
        let state = state_holding(&one, vec![stored.clone()]);

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&two, vec![item("item-9", "2026-10-19T11:00:00Z")]),
        );

        match outcome.refusal() {
            Some(BoundaryRefusal::ForeignHousehold { requested, owner }) => {
                assert_eq!(requested, &one);
                assert_eq!(owner, &two);
            }
            other => panic!("a foreign collection must be refused, got {other:?}"),
        }
        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-9"]);
        assert!(!outcome.changed());

        assert_eq!(held(&state, &one), vec![stored]);
        assert!(
            state
                .read_collection(&two, CollectionSource::Local)
                .is_none()
        );
    }

    #[test]
    fn a_write_to_an_immutable_item_is_refused_and_changes_nothing() {
        let one = household("household-1");
        let stored = item("item-1", "2026-10-19T10:00:00Z");
        let state = state_holding(&one, vec![stored.clone()]);
        state.mark_immutable(&stored.item_id);

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "2026-10-19T11:00:00Z")]),
        );

        match outcome.refusal() {
            Some(BoundaryRefusal::Immutable { item_id }) => {
                assert_eq!(item_id, &stored.item_id);
            }
            other => panic!("an immutable entity must be refused, got {other:?}"),
        }
        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(held(&state, &one), vec![stored]);
    }

    #[test]
    fn an_unparseable_incoming_version_is_ignored_rather_than_ordered() {
        let one = household("household-1");
        let stored = item("item-1", "2026-10-19T10:00:00Z");
        let state = state_holding(&one, vec![stored.clone()]);

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "not a timestamp")]),
        );

        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(outcome.refusal(), None);
        assert_eq!(held(&state, &one), vec![stored]);
    }

    #[test]
    fn an_unparseable_version_for_an_absent_item_is_ignored_and_writes_nothing() {
        let one = household("household-1");
        let state = state_holding(&one, Vec::new());

        let outcome = MergePath::new().merge(
            &state,
            &one,
            collection(&one, vec![item("item-1", "not a timestamp")]),
        );

        assert!(outcome.applied().is_empty());
        assert_eq!(ids(outcome.ignored()), ["item-1"]);
        assert_eq!(outcome.refusal(), None);
        assert!(!outcome.not_loaded(), "the empty copy is loaded");
        assert!(!outcome.changed());
        assert!(
            held(&state, &one).is_empty(),
            "a row no later merge could order is never stored"
        );
    }
}
