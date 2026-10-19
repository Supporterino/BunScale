//! The stored representation of one household's collection, and its identity/dedupe rules
//! (SYNC-1, D0.7 criterion 1).
//!
//! The storage boundary's job here is the **layout**: which rows exist, what identifies them and
//! what versions them. This module decides no behaviour — it calls nothing, compares no two
//! versions and knows no clock.
//!
//! Three properties are fixed by the types rather than left to a caller's discipline:
//!
//! 1. **One collection per household.** A [`StoredCollection`] carries its [`HouseholdId`], and the
//!    row set is reachable only through that value. There is one logical collection per household,
//!    never one per member and never one per device (SYNC-1).
//! 2. **One row per item — the store's invariant, not a caller's promise.** The row set is a
//!    `HashMap<ItemId, StoredItem>`, so writing an item is an *upsert* on `(household_id, item_id)`:
//!    writing an item that is already present replaces it. A second load of a batch that contains a
//!    known item therefore cannot append a parallel copy — no code path could, which is what makes
//!    "exactly one copy of every item" a property of the store rather than a check the caller is
//!    trusted to have made (SYNC-1, ADR 0002 M1).
//! 3. **A removal is representable.** [`StoredItem::deleted`] is a tombstone: a removed item stays a
//!    row carrying the flag rather than vanishing. A vanished row cannot appear in the change feed,
//!    so a removal would never reach another device (D0.7 criterion 2, ADR 0002 consequences).
//!
//! **The version marker is held, not compared.** [`StoredItem::updated_at`] is the client-supplied
//! `updated_at` that last-write-wins compares (D0.7 criterion 3). It is deliberately distinct from
//! any user-supplied domain timestamp, which will be stored unchanged beside it once P1's domain
//! types exist (D0.5, DM-6); this module knows only the marker, because the marker is the part the
//! collection layer needs. Deciding *which* of two versions survives is the comparator policy of
//! task 2.6: [`StoredCollection::upsert`] stores the row it is handed, and the caller that resolved
//! a conflict hands over the winner. Within one batch the last row supplied for a key is the one
//! stored, which is what "a second load cannot duplicate an item" needs and no more.
//!
//! **Why storage types exist although the wire types look the same.** [`CollectionItem`] is the
//! frontend's shape and is frozen by task 1.3; [`StoredItem`] is the device's row. The conversions
//! below are the single place the two could drift, so a P1 field the store keeps but the wire does
//! not yet represent is an additive change here rather than an edit to a frozen type — and
//! re-declaring the fields on both sides is exactly the duplication a conversion avoids.

use std::collections::HashMap;

use crate::ipc::types::{CollectionItem, HouseholdId, Instant, ItemId};

/// One item as the device stores it: identity, version marker, tombstone (SYNC-1, SYNC-5).
///
/// The three fields are exactly the part of an item every member must resolve identically; the
/// item's *content* arrives with the first domain type (P1's rabbits and weight entries) and is an
/// additive change to this type. The two named constructors below are the only way to write a row,
/// so a row can never be missing its identity or its version marker, and no construction site has to
/// spell a bare tombstone flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredItem {
    /// The item's opaque, stable identifier within its household. It is unique for the life of the
    /// item and shared verbatim across the household's devices, which is what makes the row set
    /// keyable on it.
    pub item_id: ItemId,
    /// The item's **version marker**: the client-supplied `updated_at` last-write-wins compares
    /// (D0.7 criterion 3). Held as an absolute instant, never reinterpreted, and never compared
    /// here — the comparator is task 2.6's.
    pub updated_at: Instant,
    /// Whether this row is a tombstone: the item was removed and the removal is reported rather
    /// than the row silently vanishing. The row keeps its identity and version, so the removal
    /// itself is ordered and resumable like any other change.
    pub deleted: bool,
}

impl StoredItem {
    /// A live row: the item exists, at version `updated_at`.
    pub fn live(item_id: ItemId, updated_at: Instant) -> Self {
        Self {
            item_id,
            updated_at,
            deleted: false,
        }
    }

    /// A **tombstone** row: the item was removed, at version `updated_at`.
    ///
    /// The removal is reported rather than the row silently vanishing (D0.7 criterion 2): the row
    /// keeps its identity and version, so the removal is ordered and resumable like any other
    /// change. Two named constructors rather than one flag parameter, so a call site states which of
    /// the two states it writes instead of ending in a bare `true`.
    pub fn tombstone(item_id: ItemId, updated_at: Instant) -> Self {
        Self {
            item_id,
            updated_at,
            deleted: true,
        }
    }
}

/// One household's collection as the device stores it (SYNC-1).
///
/// The row set is private on purpose: [`StoredCollection::upsert`] is the only way in, so no caller
/// can swap the map for rows that belong to another household, and the one-row-per-item keying
/// holds whatever a caller does. Instances are built per household; the device's set of them is
/// [`LocalCopies`](super::LocalCopies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCollection {
    /// The household this collection belongs to. It addresses exactly one logical collection; two
    /// instances with the same id would be two copies of one collection, which is why nothing in
    /// this module ever produces a second one for an id it already holds.
    household_id: HouseholdId,
    /// Exactly one row per item, keyed by identity. A `HashMap` rather than a `BTreeMap` because
    /// [`ItemId`] derives no `Ord` (it is frozen, and its inner string is opaque, so inventing an
    /// ordering on it would be a claim about the data the contract does not make). Callers that need
    /// a stable order get one from [`StoredCollection::wire_items`]; no behaviour here depends on
    /// the map's iteration order.
    items: HashMap<ItemId, StoredItem>,
}

impl StoredCollection {
    /// An empty collection for `household_id` — the honest starting point of a device that holds the
    /// household but has loaded nothing, which is SYNC-3's never-loaded state rather than an empty
    /// collection presented as current data.
    pub fn new(household_id: HouseholdId) -> Self {
        Self {
            household_id,
            items: HashMap::new(),
        }
    }

    /// The household this collection belongs to.
    pub fn household_id(&self) -> &HouseholdId {
        &self.household_id
    }

    /// Every stored row. The order is unspecified; it is not part of the layout's contract.
    pub fn items(&self) -> impl Iterator<Item = &StoredItem> {
        self.items.values()
    }

    /// The one row for `item_id`, if the collection holds it.
    pub fn item(&self, item_id: &ItemId) -> Option<&StoredItem> {
        self.items.get(item_id)
    }

    /// How many rows the collection holds: the number of distinct items, tombstones included.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the collection holds no row at all.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Store `item` under its identity, returning the row it replaced.
    ///
    /// This is the upsert that makes a second load idempotent: the key is the item's identity, so a
    /// write of an item the collection already holds replaces that item's single row instead of
    /// appending another. Which of two *differing* versions should have won is not decided here
    /// (task 2.6) — the row handed over is the row stored.
    pub fn upsert(&mut self, item: StoredItem) -> Option<StoredItem> {
        self.items.insert(item.item_id.clone(), item)
    }

    /// Upsert a loaded or merged batch, one row per item afterwards (SYNC-1).
    ///
    /// Running the same batch through twice — a refresh that returns items already present, or a
    /// re-issued write the service had already accepted — leaves the collection holding one row per
    /// item. Duplicates *within* one batch collapse the same way: the last row supplied for a key is
    /// the one kept.
    pub fn upsert_all(&mut self, items: impl IntoIterator<Item = StoredItem>) {
        for item in items {
            self.upsert(item);
        }
    }

    /// The stored rows as the frozen wire shape: one [`CollectionItem`] per item, ordered by item
    /// id.
    ///
    /// The order is imposed here rather than inherited from the map, because a `HashMap`'s order is
    /// not stable and a wire value that reorders itself between two identical reads is noise in
    /// every consumer's tests. The ordering carries no meaning: it is a function of the opaque id
    /// string alone, and nothing may read it as a version or a sequence.
    pub fn wire_items(&self) -> Vec<CollectionItem> {
        let mut wire: Vec<_> = self.items.values().map(CollectionItem::from).collect();
        wire.sort_by(|left, right| left.item_id.0.cmp(&right.item_id.0));
        wire
    }
}

/// The wire item handed to the store becomes the device's row unchanged.
///
/// No field is dropped or derived: the wire shape carries exactly what the layout owns. The
/// conversion exists so an incoming batch is translated once, in one place, instead of both sides
/// declaring the same three fields.
impl From<CollectionItem> for StoredItem {
    fn from(item: CollectionItem) -> Self {
        Self {
            item_id: item.item_id,
            updated_at: item.updated_at,
            deleted: item.deleted,
        }
    }
}

/// See [`From<CollectionItem> for StoredItem`].
impl From<&CollectionItem> for StoredItem {
    fn from(item: &CollectionItem) -> Self {
        Self::from(item.clone())
    }
}

/// The device's row becomes the wire item unchanged; see [`From<CollectionItem> for StoredItem`].
impl From<StoredItem> for CollectionItem {
    fn from(item: StoredItem) -> Self {
        Self {
            item_id: item.item_id,
            updated_at: item.updated_at,
            deleted: item.deleted,
        }
    }
}

/// See [`From<StoredItem> for CollectionItem`].
impl From<&StoredItem> for CollectionItem {
    fn from(item: &StoredItem) -> Self {
        Self::from(item.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The instants are opaque to this module — ordering two of them is task 2.6's and parsing them
    /// task 2.8's — so a test only ever compares a stored value with the value it supplied.
    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn id(value: &str) -> ItemId {
        ItemId(value.to_owned())
    }

    fn household() -> HouseholdId {
        HouseholdId("household-1".to_owned())
    }

    fn live(item_id: &str, updated_at: &str) -> StoredItem {
        StoredItem::live(id(item_id), at(updated_at))
    }

    fn tombstone(item_id: &str, updated_at: &str) -> StoredItem {
        StoredItem::tombstone(id(item_id), at(updated_at))
    }

    #[test]
    fn a_collection_carries_the_household_it_belongs_to() {
        let collection = StoredCollection::new(household());

        assert_eq!(collection.household_id(), &household());
    }

    #[test]
    fn a_new_collection_holds_nothing() {
        let collection = StoredCollection::new(household());

        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
        assert_eq!(collection.item(&id("item-1")), None);
    }

    #[test]
    fn the_named_constructors_write_the_state_they_name() {
        let row = live("item-1", "2026-10-19T10:00:00Z");
        let removed = tombstone("item-1", "2026-10-19T11:00:00Z");

        assert_eq!(row.item_id, id("item-1"));
        assert_eq!(row.updated_at, at("2026-10-19T10:00:00Z"));
        assert!(!row.deleted);
        assert_eq!(removed.item_id, id("item-1"));
        assert_eq!(removed.updated_at, at("2026-10-19T11:00:00Z"));
        assert!(removed.deleted);
    }

    #[test]
    fn a_second_upsert_of_the_same_item_keeps_exactly_one_copy() {
        let mut collection = StoredCollection::new(household());

        collection.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        collection.upsert(live("item-1", "2026-10-19T11:00:00Z"));

        let stored = collection.item(&id("item-1")).expect("item-1 is stored");
        assert_eq!(collection.len(), 1);
        assert_eq!(stored.updated_at, at("2026-10-19T11:00:00Z"));
    }

    #[test]
    fn upsert_reports_the_row_it_replaced() {
        let mut collection = StoredCollection::new(household());

        let first = collection.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        let second = collection.upsert(live("item-1", "2026-10-19T11:00:00Z"));

        assert_eq!(first, None);
        let replaced = second.expect("the second upsert replaced a stored row");
        assert_eq!(replaced.item_id, id("item-1"));
        assert_eq!(replaced.updated_at, at("2026-10-19T10:00:00Z"));
    }

    #[test]
    fn a_second_load_of_the_same_batch_does_not_duplicate_an_item() {
        let mut collection = StoredCollection::new(household());
        let batch = [
            live("item-1", "2026-10-19T10:00:00Z"),
            live("item-2", "2026-10-19T10:00:00Z"),
        ];

        collection.upsert_all(batch.clone());
        collection.upsert_all(batch.clone());

        assert_eq!(collection.len(), 2);
        let wire = collection.wire_items();
        let ids: Vec<&str> = wire.iter().map(|row| row.item_id.0.as_str()).collect();
        assert_eq!(ids, ["item-1", "item-2"]);
    }

    #[test]
    fn duplicates_within_one_batch_collapse_to_the_row_supplied_last() {
        let mut collection = StoredCollection::new(household());

        collection.upsert_all([
            live("item-1", "2026-10-19T10:00:00Z"),
            live("item-2", "2026-10-19T10:00:00Z"),
            tombstone("item-1", "2026-10-19T11:00:00Z"),
        ]);

        let stored = collection.item(&id("item-1")).expect("item-1 is stored");
        assert_eq!(collection.len(), 2);
        assert_eq!(stored.updated_at, at("2026-10-19T11:00:00Z"));
        assert!(stored.deleted);
    }

    #[test]
    fn a_removal_is_a_tombstone_rather_than_a_vanished_row() {
        let mut collection = StoredCollection::new(household());

        collection.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        collection.upsert(tombstone("item-1", "2026-10-19T11:00:00Z"));

        let stored = collection
            .item(&id("item-1"))
            .expect("the tombstone is a row");
        assert_eq!(collection.len(), 1);
        assert_eq!(stored.item_id, id("item-1"));
        assert_eq!(stored.updated_at, at("2026-10-19T11:00:00Z"));
        assert!(stored.deleted);
    }

    #[test]
    fn stored_rows_are_handed_to_the_wire_ordered_by_item_id() {
        let mut collection = StoredCollection::new(household());

        collection.upsert(live("item-3", "2026-10-19T10:00:00Z"));
        collection.upsert(live("item-1", "2026-10-19T11:00:00Z"));
        collection.upsert(tombstone("item-2", "2026-10-19T12:00:00Z"));

        let wire = collection.wire_items();
        assert_eq!(wire.len(), 3);
        let ids: Vec<&str> = wire.iter().map(|row| row.item_id.0.as_str()).collect();
        assert_eq!(ids, ["item-1", "item-2", "item-3"]);
        assert_eq!(wire[0].updated_at, at("2026-10-19T11:00:00Z"));
        assert_eq!(wire[1].updated_at, at("2026-10-19T12:00:00Z"));
        assert!(!wire[0].deleted);
        assert!(wire[1].deleted);
    }

    #[test]
    fn a_wire_item_round_trips_through_the_stored_row_unchanged() {
        let wire = CollectionItem {
            item_id: id("item-1"),
            updated_at: at("2026-10-19T10:00:00Z"),
            deleted: true,
        };

        let stored = StoredItem::from(&wire);
        assert_eq!(CollectionItem::from(&stored), wire);
        assert_eq!(StoredItem::from(wire.clone()), stored);
    }

    #[test]
    fn items_iterates_every_stored_row_once() {
        let mut collection = StoredCollection::new(household());

        collection.upsert(live("item-1", "2026-10-19T10:00:00Z"));
        collection.upsert(tombstone("item-1", "2026-10-19T11:00:00Z"));
        collection.upsert(live("item-2", "2026-10-19T10:00:00Z"));

        let rows: Vec<&StoredItem> = collection.items().collect();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.item_id == id("item-1")));
        assert!(rows.iter().any(|row| row.item_id == id("item-2")));
    }
}
