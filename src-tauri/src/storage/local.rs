//! The device's local copy of a household's collection (D0.2, SYNC-3).
//!
//! A copy is a **cache** of the shared collection and never a competing record: it holds the rows
//! this device last loaded, plus the marker of when that happened. Everything about *when* a copy is
//! refreshed — a start with connectivity, a foreground return, a reachable service — belongs to the
//! read path (task 2.5); everything about *which* of two versions survives belongs to the comparator
//! (task 2.6). This module holds the copy and the three things later tasks need from it: resolving
//! the stored rows, reading the loaded-at marker beside them, and recording that a load happened.
//!
//! **The loaded-at marker is what separates SYNC-3's states, and the row count is not.** Only a copy
//! that has never been loaded carries `None` *and* no rows: that is the explicit "nothing has been
//! loaded on this device", reported through [`LocalCopy::to_wire`] as `source: local` with
//! `loaded_at: None`, which the frontend renders as the connectivity-required state. A **loaded**
//! copy may legitimately hold no rows — the household is simply empty — and it is a different value:
//! `loaded_at: Some(..)` with no rows is a loaded empty collection, never the connectivity-required
//! state. The marker, not the row count, is the state, and neither state is ever collapsed into an
//! empty list presented as the household's current data.
//!
//! **The row set may come from the wire; the marker is always the local load's.** A
//! [`HouseholdCollection`] arriving at this boundary carries rows and a `loaded_at`, but that field
//! describes the *read* that produced that value, not this device's copy of it — and a value coming
//! from the service has no way to know when this device last loaded anything. The boundary therefore
//! takes the load instant from the caller that performed the load (task 2.5) and never adopts the
//! wire value's. That is what makes "rows without a marker" unconstructible: a copy is built either
//! never-loaded or by a load that carries its instant, so a refresh can never regress a loaded copy
//! to SYNC-3's never-loaded state, and no caller can produce a non-empty collection that the
//! frontend is told was never loaded.
//!
//! **A copy has no source.** [`CollectionSource`] describes where one *read* resolved its items, so
//! it is an argument of [`LocalCopy::to_wire`] and a field of the wire value, not state the copy
//! keeps: the same rows are `service` when this read refreshed them and `local` when they were
//! served from the copy instead.
//!
//! **One copy per household.** [`LocalCopies`] keys its copies on [`HouseholdId`], so loading the
//! same household twice replaces its one copy rather than accumulating a second — the
//! household-level half of SYNC-1's "a second load cannot duplicate", with the item-level half in
//! [`StoredCollection::upsert_all`](super::StoredCollection::upsert_all).

use std::collections::HashMap;

use super::collection::{StoredCollection, StoredItem};
use crate::ipc::types::{CollectionSource, HouseholdCollection, HouseholdId, Instant};

/// The device's copy of one household's collection (D0.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCopy {
    /// The stored rows, keyed and deduplicated by the layout (see [`StoredCollection`]).
    collection: StoredCollection,
    /// When this device last loaded the collection from the shared service; `None` while it never
    /// has. This is the whole difference between "loaded and empty" and "never loaded".
    loaded_at: Option<Instant>,
}

impl LocalCopy {
    /// A copy of `household_id` that has never been loaded: no rows, no marker (SYNC-3).
    pub fn never_loaded(household_id: HouseholdId) -> Self {
        Self {
            collection: StoredCollection::new(household_id),
            loaded_at: None,
        }
    }

    /// A copy holding one wire collection's rows, recorded as loaded at `at`.
    ///
    /// The rows come from the wire value; the marker is the caller's, because only the caller knows
    /// when this device loaded them (see the module documentation). The incoming
    /// [`HouseholdCollection::loaded_at`] is deliberately ignored for that reason — and it is also
    /// why this is a constructor taking an instant rather than a `From` conversion: a load instant is
    /// required to build the copy, so a wire value carrying rows without a marker cannot produce one.
    pub fn loaded_from_wire(collection: HouseholdCollection, at: Instant) -> Self {
        let mut copy = Self::never_loaded(collection.household_id);
        copy.upsert_all(collection.items.into_iter().map(StoredItem::from));
        copy.record_load(at);
        copy
    }

    /// The stored rows and their identities, exactly as the layout holds them.
    pub fn resolve(&self) -> &StoredCollection {
        &self.collection
    }

    /// When this device last loaded the collection, or `None` if it never has.
    pub fn loaded_at(&self) -> Option<&Instant> {
        self.loaded_at.as_ref()
    }

    /// Record that the collection was loaded from the shared service at `at`.
    ///
    /// Only the marker moves: which rows the load produced is written through
    /// [`LocalCopy::upsert_all`] (or [`LocalCopy::loaded_from_wire`]), and whether a load should
    /// happen at all is the read path's decision (task 2.5). The marker is a supplied absolute
    /// instant, stored unchanged (D0.5) — this module never reads a clock.
    pub fn record_load(&mut self, at: Instant) {
        self.loaded_at = Some(at);
    }

    /// Upsert one row into the copy; see [`StoredCollection::upsert`].
    pub fn upsert(&mut self, item: StoredItem) -> Option<StoredItem> {
        self.collection.upsert(item)
    }

    /// Upsert a loaded or merged batch into the copy, one row per item afterwards; see
    /// [`StoredCollection::upsert_all`]. The marker is left alone.
    pub fn upsert_all(&mut self, items: impl IntoIterator<Item = StoredItem>) {
        self.collection.upsert_all(items);
    }

    /// This copy as the frozen wire value, labelled with where the read that produced it resolved
    /// its items (SYNC-3).
    pub fn to_wire(&self, source: CollectionSource) -> HouseholdCollection {
        HouseholdCollection {
            household_id: self.collection.household_id().clone(),
            items: self.collection.wire_items(),
            source,
            loaded_at: self.loaded_at.clone(),
        }
    }
}

/// The device's copies of the household collections it has loaded, one per household (D0.2).
///
/// The map is the device-level half of SYNC-1's invariant: a household id addresses exactly one
/// copy, so a second load for a household replaces that household's copy instead of adding a second
/// one, and no copy is ever stored under an id it does not belong to. Which households are in the
/// map is the read path's business (tasks 2.2 and 2.5); this type only refuses to hold two copies of
/// one household.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalCopies {
    copies: HashMap<HouseholdId, LocalCopy>,
}

impl LocalCopies {
    /// A device holding no copies.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many households this device holds a copy of — loaded or never loaded.
    pub fn len(&self) -> usize {
        self.copies.len()
    }

    /// Whether the device holds no copy at all.
    pub fn is_empty(&self) -> bool {
        self.copies.is_empty()
    }

    /// The copy held for `household_id`, or `None` when this device holds none for it.
    ///
    /// `None` and a copy with `loaded_at: None` are different facts and stay different: the first is
    /// "this device does not hold this household", the second is "it holds it and has never loaded
    /// it". A caller that means the second prepares it through [`LocalCopies::copy_or_insert`], so
    /// SYNC-3's never-loaded state is produced by the type that owns the marker rather than by a
    /// caller remembering to pass `None`.
    pub fn get(&self, household_id: &HouseholdId) -> Option<&LocalCopy> {
        self.copies.get(household_id)
    }

    /// The copy held for `household_id`, creating the never-loaded one when this device holds none.
    pub fn copy_or_insert(&mut self, household_id: &HouseholdId) -> &mut LocalCopy {
        self.copies
            .entry(household_id.clone())
            .or_insert_with(|| LocalCopy::never_loaded(household_id.clone()))
    }

    /// Upsert a loaded or merged batch into `household_id`'s copy, returning that copy so the
    /// caller can record the load it came from.
    ///
    /// The household is addressed by argument rather than taken from the rows: a stored row carries
    /// no household of its own, so it can only ever reach the copy of the household the caller
    /// named — there is no path by which a batch lands under a second household id. The marker is
    /// untouched, so a merge that is not a load cannot invent one.
    pub fn upsert_all(
        &mut self,
        household_id: &HouseholdId,
        items: impl IntoIterator<Item = StoredItem>,
    ) -> &mut LocalCopy {
        let copy = self.copy_or_insert(household_id);
        copy.upsert_all(items);
        copy
    }

    /// Replace a household's copy wholesale with a wire collection loaded at `loaded_at`, returning
    /// the new copy.
    ///
    /// A refresh replaces the collection the feed describes (ADR 0002 R1): the rows come from the
    /// wire value, the marker from the caller that performed the load (see
    /// [`LocalCopy::loaded_from_wire`]), and whatever copy this device already held for that
    /// household is superseded rather than added to — [`LocalCopies`] holds one copy per household
    /// by construction, so "replaces" is the only thing this method can do, and a refresh can never
    /// leave the household looking never-loaded.
    pub fn replace_from_wire(
        &mut self,
        collection: HouseholdCollection,
        loaded_at: Instant,
    ) -> &mut LocalCopy {
        let copy = LocalCopy::loaded_from_wire(collection, loaded_at);
        let household_id = copy.resolve().household_id().clone();

        // Stored with `insert`, so a load for a household already held supersedes its copy; the
        // lookup that follows returns the copy just stored.
        self.copies.insert(household_id.clone(), copy);
        self.copy_or_insert(&household_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::{CollectionItem, ItemId};

    /// The instants are opaque to this module — ordering two of them is task 2.6's and parsing them
    /// task 2.8's — so a test only ever compares a marker with the value it supplied.
    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn id(value: &str) -> ItemId {
        ItemId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn live(item_id: &str, updated_at: &str) -> StoredItem {
        StoredItem::live(id(item_id), at(updated_at))
    }

    fn tombstone(item_id: &str, updated_at: &str) -> StoredItem {
        StoredItem::tombstone(id(item_id), at(updated_at))
    }

    fn wire_live(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: id(item_id),
            updated_at: at(updated_at),
            deleted: false,
        }
    }

    fn wire_tombstone(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: id(item_id),
            updated_at: at(updated_at),
            deleted: true,
        }
    }

    #[test]
    fn a_never_loaded_copy_holds_no_rows_and_no_marker() {
        let copy = LocalCopy::never_loaded(household("household-1"));
        let wire = copy.to_wire(CollectionSource::Local);

        assert_eq!(copy.loaded_at(), None);
        assert!(copy.resolve().is_empty());
        assert_eq!(wire.household_id, household("household-1"));
        assert_eq!(wire.source, CollectionSource::Local);
        assert_eq!(wire.loaded_at, None);
        assert!(wire.items.is_empty());
    }

    #[test]
    fn a_loaded_copy_with_no_rows_is_not_the_never_loaded_state() {
        let mut loaded = LocalCopy::never_loaded(household("household-1"));
        loaded.record_load(at("2026-10-19T11:00:00Z"));
        let never = LocalCopy::never_loaded(household("household-1"));

        let loaded_wire = loaded.to_wire(CollectionSource::Local);
        let never_wire = never.to_wire(CollectionSource::Local);

        assert!(loaded_wire.items.is_empty());
        assert!(never_wire.items.is_empty());
        assert_eq!(loaded_wire.loaded_at, Some(at("2026-10-19T11:00:00Z")));
        assert_eq!(never_wire.loaded_at, None);
        assert_ne!(loaded_wire, never_wire);
    }

    #[test]
    fn recording_a_load_sets_the_marker_and_leaves_the_rows_alone() {
        let mut copy = LocalCopy::never_loaded(household("household-1"));
        copy.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        copy.record_load(at("2026-10-19T11:00:00Z"));

        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
        assert_eq!(copy.resolve().len(), 1);

        copy.record_load(at("2026-10-19T12:00:00Z"));
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T12:00:00Z")));
        assert_eq!(copy.resolve().len(), 1);
    }

    #[test]
    fn a_copy_resolves_the_stored_rows_it_was_given() {
        let mut copy = LocalCopy::never_loaded(household("household-1"));
        let batch = [
            live("item-1", "2026-10-19T10:00:00Z"),
            tombstone("item-2", "2026-10-19T10:00:00Z"),
        ];
        copy.upsert_all(batch);

        let resolved = copy.resolve();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved.household_id(), &household("household-1"));
        assert!(resolved.item(&id("item-2")).expect("stored").deleted);
    }

    #[test]
    fn loading_the_same_batch_twice_leaves_one_row_per_item() {
        let mut copy = LocalCopy::never_loaded(household("household-1"));
        let batch = [
            live("item-1", "2026-10-19T10:00:00Z"),
            live("item-2", "2026-10-19T10:00:00Z"),
        ];

        copy.upsert_all(batch.clone());
        copy.record_load(at("2026-10-19T10:00:00Z"));
        copy.upsert_all(batch);
        copy.record_load(at("2026-10-19T11:00:00Z"));

        assert_eq!(copy.resolve().len(), 2);
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
    }

    #[test]
    fn the_wire_value_carries_the_source_the_read_resolved_from() {
        let mut copy = LocalCopy::never_loaded(household("household-1"));
        copy.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        copy.record_load(at("2026-10-19T11:00:00Z"));

        let from_service = copy.to_wire(CollectionSource::Service);
        let from_local = copy.to_wire(CollectionSource::Local);

        assert_eq!(from_service.source, CollectionSource::Service);
        assert_eq!(from_local.source, CollectionSource::Local);
        assert_eq!(from_service.loaded_at, Some(at("2026-10-19T11:00:00Z")));
        assert_eq!(from_local.items, from_service.items);
    }

    #[test]
    fn a_wire_collection_with_a_repeated_item_becomes_one_stored_row() {
        let duplicate = HouseholdCollection {
            household_id: household("household-1"),
            items: vec![
                wire_live("item-1", "2026-10-19T10:00:00Z"),
                wire_live("item-2", "2026-10-19T10:00:00Z"),
                wire_tombstone("item-1", "2026-10-19T11:00:00Z"),
            ],
            source: CollectionSource::Service,
            loaded_at: None,
        };

        let copy = LocalCopy::loaded_from_wire(duplicate, at("2026-10-19T11:00:00Z"));

        assert_eq!(copy.resolve().len(), 2);
        let stored = copy.resolve().item(&id("item-1")).expect("stored");
        assert_eq!(stored.updated_at, at("2026-10-19T11:00:00Z"));
        assert!(stored.deleted);
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
    }

    #[test]
    fn the_marker_is_the_local_loads_not_the_wire_values() {
        let wire = HouseholdCollection {
            household_id: household("household-1"),
            items: vec![wire_live("item-1", "2026-10-19T10:00:00Z")],
            source: CollectionSource::Service,
            loaded_at: Some(at("2026-10-19T09:00:00Z")),
        };

        let copy = LocalCopy::loaded_from_wire(wire, at("2026-10-19T11:00:00Z"));
        let reported = copy.to_wire(CollectionSource::Local);

        assert_eq!(copy.resolve().len(), 1);
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
        assert_eq!(reported.loaded_at, Some(at("2026-10-19T11:00:00Z")));
    }

    #[test]
    fn a_device_holds_one_copy_per_household() {
        let mut copies = LocalCopies::new();
        let first = HouseholdCollection {
            household_id: household("household-1"),
            items: vec![wire_live("item-1", "2026-10-19T10:00:00Z")],
            source: CollectionSource::Service,
            loaded_at: None,
        };
        let second = HouseholdCollection {
            household_id: household("household-1"),
            items: vec![wire_live("item-1", "2026-10-19T11:00:00Z")],
            source: CollectionSource::Service,
            loaded_at: None,
        };

        copies.replace_from_wire(first, at("2026-10-19T10:00:00Z"));
        copies.replace_from_wire(second, at("2026-10-19T11:00:00Z"));

        assert_eq!(copies.len(), 1);
        let copy = copies.get(&household("household-1")).expect("held");
        assert_eq!(copy.resolve().len(), 1);
        let stored = copy.resolve().item(&id("item-1")).expect("stored");
        assert_eq!(stored.updated_at, at("2026-10-19T11:00:00Z"));
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
    }

    #[test]
    fn a_refresh_never_regresses_a_loaded_copy_to_never_loaded() {
        let mut copies = LocalCopies::new();
        let empty = HouseholdCollection {
            household_id: household("household-1"),
            items: Vec::new(),
            source: CollectionSource::Service,
            loaded_at: None,
        };

        let copy = copies.replace_from_wire(empty, at("2026-10-19T11:00:00Z"));
        let reported = copy.to_wire(CollectionSource::Local);

        assert!(reported.items.is_empty());
        assert_eq!(copy.loaded_at(), Some(&at("2026-10-19T11:00:00Z")));
        assert_eq!(reported.loaded_at, Some(at("2026-10-19T11:00:00Z")));
    }

    #[test]
    fn two_households_hold_their_own_copies() {
        let mut copies = LocalCopies::new();
        let first = [live("item-1", "2026-10-19T10:00:00Z")];
        let second = [
            live("item-1", "2026-10-19T10:00:00Z"),
            live("item-2", "2026-10-19T10:00:00Z"),
        ];

        copies.upsert_all(&household("household-1"), first);
        copies.upsert_all(&household("household-2"), second);

        assert_eq!(copies.len(), 2);
        let one = copies.get(&household("household-1")).expect("held");
        let two = copies.get(&household("household-2")).expect("held");
        assert_eq!(one.resolve().len(), 1);
        assert_eq!(two.resolve().len(), 2);
        assert_eq!(one.resolve().household_id(), &household("household-1"));
        assert_eq!(two.resolve().household_id(), &household("household-2"));
    }

    #[test]
    fn a_batch_lands_only_in_the_copy_of_the_household_it_was_loaded_for() {
        let mut copies = LocalCopies::new();
        let batch = [live("item-1", "2026-10-19T10:00:00Z")];

        copies
            .upsert_all(&household("household-1"), batch)
            .record_load(at("2026-10-19T10:00:00Z"));

        assert!(copies.get(&household("household-2")).is_none());
        let held = copies.get(&household("household-1")).expect("held");
        assert_eq!(held.resolve().len(), 1);
        assert_eq!(held.loaded_at(), Some(&at("2026-10-19T10:00:00Z")));
    }

    #[test]
    fn a_merge_that_is_not_a_load_leaves_the_marker_alone() {
        let mut copies = LocalCopies::new();

        copies.copy_or_insert(&household("household-1"));
        copies.upsert_all(
            &household("household-1"),
            [live("item-1", "2026-10-19T10:00:00Z")],
        );

        let held = copies.get(&household("household-1")).expect("held");
        assert_eq!(held.resolve().len(), 1);
        assert_eq!(held.loaded_at(), None);
    }

    #[test]
    fn a_household_this_device_holds_but_has_not_loaded_reads_as_never_loaded() {
        let mut copies = LocalCopies::new();

        let held = copies.copy_or_insert(&household("household-1"));
        let wire = held.to_wire(CollectionSource::Local);

        assert_eq!(wire.loaded_at, None);
        assert!(wire.items.is_empty());
        assert_eq!(copies.len(), 1);
    }
}
