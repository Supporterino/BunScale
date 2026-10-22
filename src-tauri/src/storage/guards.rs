//! The storage boundary's invariants, enforced where data enters and leaves (NFR-10, D0.4).
//!
//! `design.md` gives the storage boundary one job and four properties. Task 2.3 built the
//! collection **layout** ([`StoredCollection`](super::StoredCollection), [`LocalCopies`]); this
//! module adds the **guards** that make three of those properties structural rather than a calling
//! convention:
//!
//! 1. **Household isolation.** Every read and write goes *through* a [`HouseholdId`], and the guard
//!    remembers which household claimed each [`ItemId`]. A read addressed to another household is
//!    [`BoundaryRefusal::ForeignHousehold`], never `None`: "not yours" and "absent" are different
//!    outcomes the caller can act on differently, so a cross-household attempt cannot be mistaken
//!    for a missing row. A write that would put an item already owned by one household into another
//!    is refused whole, so an item's household cannot change by asking again.
//! 2. **Canonical representation.** [`Canonical<T>`] is the only way to hold a value whose type has
//!    a single stored form ([`CanonicalForm`]). Its field is private and its one constructor refuses
//!    a value that is not already canonical, so a second representation cannot be constructed.
//!    P0 ships no value of that kind — the weight domain and its grams are P1's ([`crate::settings`]
//!    owns the *display* unit and converts only at entry/display) — so the rule is a small helper
//!    the first canonical type can use, exercised here by a test-only type rather than by inventing
//!    a weight model.
//! 3. **Immutability.** [`GuardedStore::mark_immutable`] records that an entity may no longer be
//!    written. The marker set is private, there is no unmark, and every write path in this module
//!    consults it before touching a row, so a marked entity cannot be rewritten through the public
//!    API — the same "private field plus only checked mutators" shape [`StoredCollection`] uses for
//!    its row set.
//!
//! **Item ids are treated as globally unique on one device, deliberately.** The frozen contract says
//! an [`ItemId`] is unique *within* its household, and the service keys rows on
//! `(household_id, item_id)`. [`GuardedStore::owners`] claims ownership by the item id alone — the
//! stronger rule — because that is what lets a read answer "another household's item" without
//! scanning every copy, and it is what makes "an item belongs to exactly one household" checkable at
//! the boundary. The consequence is accepted and pinned by a test: a second household's otherwise
//! legitimate item that reuses an id is refused as [`BoundaryRefusal::ForeignHousehold`] rather than
//! stored. A later phase that needs per-household id reuse moves the claim key to
//! `(HouseholdId, ItemId)`; until then, silently attaching one id to two households would be the
//! worse outcome.
//!
//! **What this module deliberately is not.** It holds no clock (2.8), no last-write-wins comparator
//! (2.6), no network path (2.4/2.5) and no command surface (2.1/2.2): every decision here is a pure
//! function of the values handed in. The guarded store owns the device's copies and exposes them
//! read-only, because a caller that could reach a [`LocalCopy`] mutably could write around the
//! marker it just set. The one value it takes from outside is the load instant, supplied by the read
//! path exactly as [`LocalCopy`] takes it — this module still never reads a clock.
//!
//! **Refusals are outcomes, not messages.** [`BoundaryRefusal`] names the invariant that was
//! violated and which households or item were involved, so the command/sync layer can map a refusal
//! to its own report instead of parsing an error string. It is the boundary's own classification;
//! nothing here crosses the IPC boundary, where the frozen [`CommandError`] surface applies.
//!
//! [`StoredCollection`]: super::StoredCollection
//! [`CommandError`]: crate::ipc::types::CommandError

use std::collections::{HashMap, HashSet};

use super::collection::StoredItem;
use super::local::{LocalCopies, LocalCopy};
use crate::ipc::types::{HouseholdCollection, HouseholdId, Instant, ItemId};

/// Why the storage boundary refused an operation (NFR-10).
///
/// One variant per invariant, with the identifiers needed to act on it: the caller can tell a
/// cross-household attempt from an absent value, a non-canonical value from a mutable one, and an
/// immutable entity from an ordinary write. The type carries no free-form detail: a refusal is a
/// classification, never a message to display (the IPC layer owns wording, NFR-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryRefusal {
    /// The value belongs to `owner` and was reached through `requested`.
    ///
    /// `requested` is the household the caller addressed; `owner` is the household that holds the
    /// value. They differ whenever a caller tries to read or write another household's item, or
    /// hands over a collection whose own household id does not match the one it was addressed
    /// through.
    ForeignHousehold {
        /// The household the operation was addressed through.
        requested: HouseholdId,
        /// The household that actually holds the value.
        owner: HouseholdId,
    },
    /// A value with one stored representation arrived in a different one.
    ///
    /// `canonical_form` names the representation the type accepts, so the caller can report the
    /// correct form without this boundary knowing the type's domain wording.
    NonCanonical {
        /// The one form the value's type stores, as the type names it.
        canonical_form: &'static str,
    },
    /// A write targeted an entity the boundary holds as immutable.
    Immutable {
        /// The entity whose row was not written.
        item_id: ItemId,
    },
}

impl std::fmt::Display for BoundaryRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignHousehold { requested, owner } => write!(
                formatter,
                "the value belongs to household {} and cannot be reached through household {}",
                owner.0, requested.0,
            ),
            Self::NonCanonical { canonical_form } => {
                write!(formatter, "the value is not stored as {canonical_form}")
            }
            Self::Immutable { item_id } => {
                write!(formatter, "the entity {} is immutable", item_id.0)
            }
        }
    }
}

impl std::error::Error for BoundaryRefusal {}

/// A type with exactly one canonical stored representation (D0.4, NFR-10).
///
/// Implementors describe the form and answer whether a value is already in it. [`Canonical<T>`]
/// admits only values that answer `true`; nothing in this boundary rewrites a value into the
/// canonical form. Refusing rather than normalising is deliberate: a boundary that converted input
/// would accept two representations and hide the caller's mistake, while refusing leaves exactly
/// one constructible representation and names it in [`BoundaryRefusal::NonCanonical`].
///
/// P1's weight type is the first expected implementor: grams are stored, and an amount expressed in
/// another unit is refused here rather than quietly converted. The conversion from a person's entry
/// happens at the display boundary ([`crate::settings::unit`]) before this type ever sees a value.
pub trait CanonicalForm: Sized {
    /// The one stored representation, named for the refusal this module returns.
    const FORM: &'static str;

    /// Whether `self` is already in [`CanonicalForm::FORM`].
    ///
    /// This is the boundary's only canonicalisation rule: it may compare, but it must not rewrite —
    /// a value this returns `true` for is stored unchanged, which is what "the stored value survives
    /// a presentation change" (NFR-10) needs.
    fn is_canonical(&self) -> bool;
}

/// A value admitted for storage, guaranteed to be in its canonical form (D0.4, NFR-10).
///
/// The inner value is private and [`Canonical::admit`] is the only constructor, so a
/// `Canonical<T>` cannot exist around a non-canonical `T`. A storage type uses this as the field
/// type of a canonical value rather than a bare `T`, which is what makes "only the canonical form
/// is stored" a property of the type instead of a check a caller is trusted to have made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canonical<T>(T);

impl<T: CanonicalForm> Canonical<T> {
    /// Admit `value` for storage if it is already in its canonical form.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal::NonCanonical`] naming [`CanonicalForm::FORM`] when it is not. The value
    /// is not normalised and is not stored: the refusal is the whole outcome.
    pub fn admit(value: T) -> Result<Self, BoundaryRefusal> {
        if value.is_canonical() {
            Ok(Self(value))
        } else {
            Err(BoundaryRefusal::NonCanonical {
                canonical_form: T::FORM,
            })
        }
    }
}

impl<T> Canonical<T> {
    /// The stored value, in its one canonical form.
    pub fn get(&self) -> &T {
        &self.0
    }

    /// The stored value by value. It is canonical because the only way in was [`Canonical::admit`].
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// The device's household copies, reachable only through the boundary's guards (NFR-10).
///
/// The copies, the item-to-household claims and the immutable markers are all private, and no
/// method hands out a mutable [`LocalCopy`]: every write goes through [`GuardedStore::upsert_item`],
/// [`GuardedStore::upsert_all`], [`GuardedStore::upsert_wire_collection`] (a merge) or
/// [`GuardedStore::load_wire_collection`] (a load that also moves the loaded-at marker), each of
/// which checks isolation and immutability first. Read access is [`GuardedStore::local_copies`] and
/// [`GuardedStore::item`]; the former is read-only, so the checks cannot be bypassed by reaching
/// around them. The running app keeps it behind its state mutex and exposes only checked
/// operations ([`crate::state::AppState`]), never the store itself.
///
/// The store starts empty and is filled only through its checked mutators, so the claims it holds
/// and the rows it stores are written together and cannot disagree through the public API.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GuardedStore {
    /// The device's copies; see [`LocalCopies`].
    copies: LocalCopies,
    /// Which household claimed each item identity, for the life of this store.
    ///
    /// A claim is never moved or removed: an item's identity belongs to exactly one household, and
    /// a tombstone keeps that identity just as a live row does. The map is private and only the
    /// checked write and load paths add to it.
    ///
    /// The key is the item id alone, which is stronger than the frozen contract's "unique within
    /// its household": see the module documentation for why that is deliberate and what it refuses.
    owners: HashMap<ItemId, HouseholdId>,
    /// The entities the boundary refuses to write again.
    immutable: HashSet<ItemId>,
}

impl GuardedStore {
    /// An empty boundary: no copies, no claims, no immutable entities.
    pub fn new() -> Self {
        Self::default()
    }

    /// The device's copies, read-only.
    ///
    /// Read-only on purpose: a mutable view would let a caller write a row without passing the
    /// isolation and immutability checks below (see the type documentation).
    pub fn local_copies(&self) -> &LocalCopies {
        &self.copies
    }

    /// The copy held for `household_id`, if this device holds one.
    ///
    /// The lookup is keyed on the household id, so it can only ever resolve that household's copy;
    /// an absent copy is `None` ("this device holds no copy") and is a different fact from a stored
    /// item that is not this household's (see [`GuardedStore::item`]).
    pub fn collection(&self, household_id: &HouseholdId) -> Option<&LocalCopy> {
        self.copies.get(household_id)
    }

    /// The household that claimed `item_id`, if any.
    ///
    /// This is the identity claim, not the row: it survives a tombstone and is never moved. It
    /// exists for the command/sync layer to report ownership without exposing the private map.
    pub fn household_of(&self, item_id: &ItemId) -> Option<&HouseholdId> {
        self.owners.get(item_id)
    }

    /// The row for `item_id`, reached through `household_id` (NFR-10).
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal::ForeignHousehold`] when another household claimed the item. A cross-
    /// household read is never `Ok(None)`: the caller must be able to tell "not yours" from
    /// "absent", and only an item no household holds answers `Ok(None)`.
    pub fn item(
        &self,
        household_id: &HouseholdId,
        item_id: &ItemId,
    ) -> Result<Option<&StoredItem>, BoundaryRefusal> {
        self.ensure_owner(household_id, item_id)?;

        Ok(self
            .copies
            .get(household_id)
            .and_then(|copy| copy.resolve().item(item_id)))
    }

    /// Upsert one row into `household_id`'s copy, returning the row it replaced (SYNC-1, NFR-10).
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal::ForeignHousehold`] when another household already claimed the item, and
    /// [`BoundaryRefusal::Immutable`] when the entity is marked immutable. Both refusals happen
    /// before the copy is touched, so a refused write changes nothing.
    pub fn upsert_item(
        &mut self,
        household_id: &HouseholdId,
        item: StoredItem,
    ) -> Result<Option<StoredItem>, BoundaryRefusal> {
        self.admit_write(household_id, &item)?;

        let item_id = item.item_id.clone();
        let replaced = self.copies.copy_or_insert(household_id).upsert(item);
        self.owners
            .entry(item_id)
            .or_insert_with(|| household_id.clone());

        Ok(replaced)
    }

    /// Upsert a batch into `household_id`'s copy, all or nothing (SYNC-1, NFR-10).
    ///
    /// Every row is checked before any is written, so a batch containing one foreign or immutable
    /// item is refused whole and leaves the copy exactly as it was. This is what keeps the
    /// `(household_id, item_id)` claim consistent: a partial batch could otherwise move some rows
    /// while refusing others.
    ///
    /// # Errors
    ///
    /// As [`GuardedStore::upsert_item`], naming the first row that violates an invariant.
    pub fn upsert_all(
        &mut self,
        household_id: &HouseholdId,
        items: impl IntoIterator<Item = StoredItem>,
    ) -> Result<(), BoundaryRefusal> {
        let items: Vec<StoredItem> = items.into_iter().collect();

        for item in &items {
            self.admit_write(household_id, item)?;
        }

        let mut claimed = Vec::with_capacity(items.len());
        {
            let copy = self.copies.copy_or_insert(household_id);

            for item in items {
                claimed.push(item.item_id.clone());
                copy.upsert(item);
            }
        }

        for item_id in claimed {
            self.owners
                .entry(item_id)
                .or_insert_with(|| household_id.clone());
        }

        Ok(())
    }

    /// Load one wire collection into the household it is addressed through, recording the load
    /// instant beside the rows (SYNC-3, NFR-10).
    ///
    /// This is the **load** path: it replaces the household's copy wholesale through
    /// [`LocalCopies::replace_from_wire`], so the copy carried afterwards has
    /// `loaded_at: Some(loaded_at)` even when the collection holds no rows. That is what keeps a
    /// loaded empty collection from reading as SYNC-3's never-loaded state; the merge path
    /// ([`GuardedStore::upsert_wire_collection`]) leaves the marker alone because a merge is not a
    /// load.
    ///
    /// Every row is checked before the copy is replaced, so a foreign or changed marked row refuses
    /// the load whole and leaves both the rows and the previous marker untouched. A marked entity is
    /// accepted **only when the incoming row is identical to the one stored in this household's
    /// copy** — identity, version, tombstone flag and content alike — a changed row is a write to
    /// it, and the marker means "no write". Content is part of the comparison on purpose: the same
    /// version delivered with a different payload would replace the stored profile behind the
    /// marker, which is exactly the loss the marker forbids (D1.16, RAB-9). An entity marked
    /// immutable is for the same reason treated as undeletable through a load: a collection that
    /// omits it would make the replacement remove it, so the load is refused rather than allowed to
    /// drop it silently. **The consequence is a constraint on the service, and it is not recoverable
    /// from the client side:** there is no unmark, so the only recovery is the service re-delivering
    /// the marked row unchanged. A service-side delete therefore leaves that household's load
    /// permanently refused, which is why the domain must not mark entities the service is allowed to
    /// delete.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal::ForeignHousehold`] when the value's household differs from
    /// `household_id` or when one of its rows is claimed by another household, and
    /// [`BoundaryRefusal::Immutable`] when a marked row differs from the stored one or when the load
    /// would omit a stored marked entity.
    pub fn load_wire_collection(
        &mut self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
        loaded_at: Instant,
    ) -> Result<(), BoundaryRefusal> {
        if &collection.household_id != household_id {
            return Err(BoundaryRefusal::ForeignHousehold {
                requested: household_id.clone(),
                owner: collection.household_id,
            });
        }

        let rows: Vec<StoredItem> = collection.items.iter().map(StoredItem::from).collect();

        for row in &rows {
            self.ensure_owner(household_id, &row.item_id)?;

            if self.immutable.contains(&row.item_id) {
                // A refresh normally re-delivers a marked row unchanged. That is not a write and is
                // allowed; anything that would alter the row is refused (NFR-10).
                let unchanged = self
                    .copies
                    .get(household_id)
                    .and_then(|copy| copy.resolve().item(&row.item_id))
                    .is_some_and(|stored| stored == row);

                if !unchanged {
                    return Err(BoundaryRefusal::Immutable {
                        item_id: row.item_id.clone(),
                    });
                }
            }
        }

        // A replacement also *removes* the rows the incoming collection omits. Removing a marked
        // entity is as much a write as changing it, so the load is refused rather than allowed to
        // drop it silently.
        let incoming: HashSet<ItemId> = rows.iter().map(|row| row.item_id.clone()).collect();

        if let Some(copy) = self.copies.get(household_id) {
            for stored in copy.resolve().items() {
                if self.immutable.contains(&stored.item_id) && !incoming.contains(&stored.item_id) {
                    return Err(BoundaryRefusal::Immutable {
                        item_id: stored.item_id.clone(),
                    });
                }
            }
        }

        let claimed = rows.into_iter().map(|row| row.item_id);
        // The load replaces this household's copy and records the marker; the rows were all checked
        // above, and the type holds one copy per household (SYNC-1, SYNC-3).
        self.copies.replace_from_wire(collection, loaded_at);

        for item_id in claimed {
            self.owners
                .entry(item_id)
                .or_insert_with(|| household_id.clone());
        }

        Ok(())
    }

    /// **Merge** a wire collection into the household it is addressed through (NFR-10).
    ///
    /// The collection's own [`HouseholdCollection::household_id`] must be the one the caller
    /// addressed; a value describing another household is refused rather than filed under the
    /// requested id. Its rows then go through [`GuardedStore::upsert_all`], so isolation and
    /// immutability apply to every row and the batch is all or nothing.
    ///
    /// This is the merge path, not a load: it upserts the rows it is given and leaves the copy's
    /// loaded-at marker exactly as it was. A read that actually loaded a collection uses
    /// [`GuardedStore::load_wire_collection`], because only a load may record when it happened
    /// (SYNC-3); a merge that moved the marker would present rows this device did not load.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal::ForeignHousehold`] when the value's household differs from
    /// `household_id`, and the refusals of [`GuardedStore::upsert_all`] for its rows.
    pub fn upsert_wire_collection(
        &mut self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
    ) -> Result<(), BoundaryRefusal> {
        if &collection.household_id != household_id {
            return Err(BoundaryRefusal::ForeignHousehold {
                requested: household_id.clone(),
                owner: collection.household_id,
            });
        }

        self.upsert_all(
            household_id,
            collection.items.into_iter().map(StoredItem::from),
        )
    }

    /// Mark `item_id` immutable: every later write to it is refused (NFR-10).
    ///
    /// There is deliberately no unmark. The marker is a private set, and it is checked by every
    /// write path in this type before a copy is touched, so a caller cannot mark an entity and then
    /// bypass the marker through another public method. Marking an identity is permanent for the
    /// life of this store; whether an entity *should* be marked is the owning domain's decision.
    pub fn mark_immutable(&mut self, item_id: &ItemId) {
        self.immutable.insert(item_id.clone());
    }

    /// Whether `item_id` is marked immutable.
    pub fn is_immutable(&self, item_id: &ItemId) -> bool {
        self.immutable.contains(item_id)
    }

    /// Refuse an item this household does not own (NFR-10).
    fn ensure_owner(
        &self,
        household_id: &HouseholdId,
        item_id: &ItemId,
    ) -> Result<(), BoundaryRefusal> {
        match self.owners.get(item_id) {
            Some(owner) if owner != household_id => Err(BoundaryRefusal::ForeignHousehold {
                requested: household_id.clone(),
                owner: owner.clone(),
            }),
            _ => Ok(()),
        }
    }

    /// Check both invariants a write must satisfy, before the write touches anything.
    fn admit_write(
        &self,
        household_id: &HouseholdId,
        item: &StoredItem,
    ) -> Result<(), BoundaryRefusal> {
        // Isolation first: a foreign attempt is answered as foreign whatever else is true of the
        // entity, so the refusal does not reveal whether another household's row is immutable.
        self.ensure_owner(household_id, &item.item_id)?;

        if self.immutable.contains(&item.item_id) {
            return Err(BoundaryRefusal::Immutable {
                item_id: item.item_id.clone(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::types::{CollectionItem, CollectionSource, Instant, ItemPayload};

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

    fn wire(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: id(item_id),
            updated_at: at(updated_at),
            deleted: false,
            payload: None,
        }
    }

    fn wire_tombstone(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: id(item_id),
            updated_at: at(updated_at),
            deleted: true,
            payload: None,
        }
    }

    /// One rabbit item's content, as `save_item` submits it (D1.16).
    fn rabbit_payload(name: &str) -> ItemPayload {
        ItemPayload::Rabbit {
            rabbit: crate::ipc::types::Rabbit {
                id: crate::ipc::types::RabbitId("item-1".to_owned()),
                household_id: household("household-1"),
                name: name.to_owned(),
                photo_ref: None,
                breed: None,
                dob: None,
                dob_unknown: true,
                sex: crate::ipc::types::Sex::Unknown,
                neuter_status: crate::ipc::types::NeuterStatus::Unknown,
                adoption_date: None,
                is_archived: false,
                weight_drop_threshold_percent: 5.0,
            },
        }
    }

    /// A test-only canonical type: a string is canonical exactly while it carries no uppercase
    /// character. It is not a domain model of anything the product stores — P0 has none with a
    /// canonical form — only a stand-in that lets the helper's contract be exercised.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Lowercase(String);

    impl CanonicalForm for Lowercase {
        const FORM: &'static str = "lowercase";

        fn is_canonical(&self) -> bool {
            !self.0.chars().any(char::is_uppercase)
        }
    }

    /// One wire collection for `household_id` holding `items`.
    fn wire_collection(
        household_id: &HouseholdId,
        items: Vec<CollectionItem>,
    ) -> HouseholdCollection {
        HouseholdCollection {
            household_id: household_id.clone(),
            items,
            source: CollectionSource::Service,
            loaded_at: None,
        }
    }

    /// The refusal of an operation that had to be refused.
    fn refused<T: std::fmt::Debug>(result: Result<T, BoundaryRefusal>) -> BoundaryRefusal {
        result.expect_err("the operation is refused")
    }

    #[test]
    fn a_value_already_in_its_canonical_form_is_admitted() {
        let admitted = Canonical::admit(Lowercase("home".to_owned())).expect("canonical");

        assert_eq!(admitted.get(), &Lowercase("home".to_owned()));
    }

    #[test]
    fn a_non_canonical_value_is_refused_and_names_the_form_it_would_have_to_be_in() {
        let refusal =
            Canonical::admit(Lowercase("Home".to_owned())).expect_err("the value is not lowercase");

        assert_eq!(
            refusal,
            BoundaryRefusal::NonCanonical {
                canonical_form: "lowercase",
            }
        );
    }

    #[test]
    fn an_admitted_value_round_trips_unchanged_and_a_refusal_returns_no_value() {
        // Refusal, not normalisation: the successful path preserves the value exactly, while the
        // refused path answers with the refusal alone. There is no second representation to store.
        let value = Lowercase("home".to_owned());
        let admitted = Canonical::admit(value.clone()).expect("canonical");

        assert_eq!(admitted.get(), &value);
        assert_eq!(admitted.into_inner(), value);

        match Canonical::admit(Lowercase("Mixed".to_owned())) {
            Err(BoundaryRefusal::NonCanonical { canonical_form }) => {
                assert_eq!(canonical_form, Lowercase::FORM);
            }
            other => panic!("a non-canonical value must be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_read_through_the_owning_household_resolves_the_item() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        store
            .upsert_item(&one, live("item-1", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let resolved = store.item(&one, &id("item-1")).expect("own item");

        assert_eq!(
            resolved.expect("the item is stored"),
            &live("item-1", "2026-10-19T10:00:00Z")
        );
    }

    #[test]
    fn a_read_through_another_household_is_a_typed_refusal_never_an_absence() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        store
            .upsert_item(&one, live("item-1", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let outcome = store.item(&two, &id("item-1"));

        assert_eq!(
            outcome,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: two,
                owner: one,
            })
        );
        // The refused outcome is a refusal, not `Ok(None)`: the caller cannot read "not yours" as
        // "absent" (NFR-10).
        assert_ne!(outcome, Ok(None));
    }

    #[test]
    fn an_item_no_household_holds_reads_as_absent() {
        let store = GuardedStore::new();

        assert_eq!(
            store.item(&household("household-1"), &id("item-1")),
            Ok(None)
        );
        assert_eq!(store.household_of(&id("item-1")), None);
    }

    #[test]
    fn an_item_cannot_be_written_into_a_second_household() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        let original = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, original.clone())
            .expect("a first write");

        let refused = store.upsert_item(&two, live("item-1", "2026-10-19T11:00:00Z"));

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: two.clone(),
                owner: one.clone(),
            })
        );
        assert!(store.local_copies().get(&two).is_none());
        assert_eq!(
            store.item(&one, &id("item-1")),
            Ok(Some(&original)),
            "the owning household's row is unchanged",
        );
    }

    #[test]
    fn a_batch_containing_a_foreign_item_is_refused_whole() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        store
            .upsert_item(&two, live("item-2", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let refused = store.upsert_all(
            &one,
            [
                live("item-1", "2026-10-19T10:00:00Z"),
                live("item-2", "2026-10-19T10:00:00Z"),
            ],
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: one.clone(),
                owner: two.clone(),
            })
        );
        // Nothing from the refused batch landed, not even the item that was not foreign.
        assert_eq!(store.item(&one, &id("item-1")), Ok(None));
        assert_eq!(store.household_of(&id("item-1")), None);
        assert_eq!(store.local_copies().get(&one), None);
        assert_eq!(
            store.item(&two, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T10:00:00Z")))
        );
    }

    #[test]
    fn a_wire_collection_describing_another_household_is_refused() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");

        let refused = store.upsert_wire_collection(
            &one,
            wire_collection(&two, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: one.clone(),
                owner: two,
            })
        );
        assert!(store.local_copies().is_empty());
        assert_eq!(store.household_of(&id("item-1")), None);
    }

    #[test]
    fn a_wire_collection_containing_another_households_item_is_refused() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        store
            .upsert_item(&two, live("item-2", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let refused = store.upsert_wire_collection(
            &one,
            wire_collection(&one, vec![wire("item-2", "2026-10-19T11:00:00Z")]),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: one.clone(),
                owner: two.clone(),
            })
        );
        assert_eq!(store.local_copies().get(&one), None);
        assert_eq!(
            store.item(&two, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T10:00:00Z")))
        );
    }

    #[test]
    fn a_wire_collection_for_its_own_household_is_stored() {
        let mut store = GuardedStore::new();
        let one = household("household-1");

        store
            .upsert_wire_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
            )
            .expect("the collection is addressed correctly");

        assert_eq!(
            store.item(&one, &id("item-1")),
            Ok(Some(&live("item-1", "2026-10-19T10:00:00Z")))
        );
        assert_eq!(store.household_of(&id("item-1")), Some(&one));
    }

    #[test]
    fn a_guarded_load_records_the_load_instant_beside_the_rows() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let loaded_at = at("2026-10-19T11:00:00Z");

        store
            .load_wire_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                loaded_at.clone(),
            )
            .expect("the collection is addressed correctly");

        let copy = store.collection(&one).expect("the copy is held");
        assert_eq!(copy.loaded_at(), Some(&loaded_at));
        assert_eq!(copy.resolve().len(), 1);
        assert_eq!(store.household_of(&id("item-1")), Some(&one));
    }

    #[test]
    fn a_guarded_load_of_an_empty_collection_is_a_loaded_empty_copy() {
        // The state the boundary must not produce is a non-empty copy with no marker; this is the
        // other side of it — a loaded copy that happens to be empty is still explicitly loaded.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let loaded_at = at("2026-10-19T11:00:00Z");

        store
            .load_wire_collection(&one, wire_collection(&one, Vec::new()), loaded_at.clone())
            .expect("an empty load is a load");

        let copy = store.collection(&one).expect("the copy is held");
        let reported = copy.to_wire(CollectionSource::Local);

        assert!(copy.resolve().is_empty());
        assert_eq!(copy.loaded_at(), Some(&loaded_at));
        assert_eq!(reported.loaded_at, Some(loaded_at));
    }

    #[test]
    fn a_guarded_load_describing_another_household_is_refused() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");

        let refused = store.load_wire_collection(
            &one,
            wire_collection(&two, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: one.clone(),
                owner: two,
            })
        );
        assert!(store.local_copies().is_empty());
    }

    #[test]
    fn a_guarded_load_that_would_rewrite_a_marked_entity_is_refused_and_changes_nothing() {
        // A load replaces the household's copy; a row that differs from the stored one is a write to
        // a marked entity and refuses the load before replacement, so neither the stored row nor the
        // marker moves.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let original = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, original.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        let refused = store.load_wire_collection(
            &one,
            wire_collection(&one, vec![wire("item-1", "2026-10-19T11:00:00Z")]),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-1"),
            })
        );
        let copy = store.collection(&one).expect("the copy is still held");
        assert_eq!(copy.loaded_at(), None, "no load was recorded");
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&original)));

        // The comparison covers the whole row, not just its identity or version: the same
        // `updated_at` with the tombstone flag flipped is still a write to the marked entity.
        let flipped = store.load_wire_collection(
            &one,
            wire_collection(&one, vec![wire_tombstone("item-1", "2026-10-19T10:00:00Z")]),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            flipped,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-1"),
            })
        );
        let copy = store.collection(&one).expect("the copy is still held");
        assert_eq!(copy.loaded_at(), None, "no load was recorded");
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&original)));
    }

    #[test]
    fn a_guarded_load_that_would_change_a_marked_rows_content_is_refused() {
        // "Unchanged" means the whole row, content included: the same version and tombstone flag
        // with a different payload would overwrite the stored profile behind the marker, so the load
        // is a write and is refused (D1.16, RAB-9).
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let original = StoredItem::from(CollectionItem {
            payload: Some(rabbit_payload("Pebble")),
            ..wire("item-1", "2026-10-19T10:00:00Z")
        });
        store
            .upsert_item(&one, original.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        let refused = store.load_wire_collection(
            &one,
            wire_collection(
                &one,
                vec![CollectionItem {
                    payload: Some(rabbit_payload("Clover")),
                    ..wire("item-1", "2026-10-19T10:00:00Z")
                }],
            ),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-1"),
            })
        );
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&original)));
    }

    #[test]
    fn a_guarded_load_that_would_omit_a_marked_entity_is_refused_and_changes_nothing() {
        // A replacement removes every row the incoming collection omits. Removing a marked entity
        // would change stored data behind the marker, so the load is refused before it can happen.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let kept = live("item-1", "2026-10-19T10:00:00Z");
        let marked = live("item-2", "2026-10-19T10:00:00Z");
        store
            .upsert_all(&one, [kept.clone(), marked.clone()])
            .expect("a first write");
        store.mark_immutable(&id("item-2"));

        let refused = store.load_wire_collection(
            &one,
            wire_collection(&one, vec![wire("item-1", "2026-10-19T11:00:00Z")]),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-2"),
            })
        );
        let copy = store.collection(&one).expect("the copy is still held");
        assert_eq!(copy.loaded_at(), None, "no load was recorded");
        assert_eq!(
            store.item(&one, &id("item-1")),
            Ok(Some(&kept)),
            "the load did not partially apply",
        );
        assert_eq!(store.item(&one, &id("item-2")), Ok(Some(&marked)));
    }

    #[test]
    fn a_guarded_load_carrying_a_marked_row_unchanged_succeeds_and_records_the_marker() {
        // A normal refresh re-delivers a marked row at the version already stored. That is not a
        // write, so it must not block the load: otherwise a household with any marked entity could
        // never advance `loaded_at` (SYNC-3).
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let marked = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, marked.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));
        let loaded_at = at("2026-10-19T11:00:00Z");

        store
            .load_wire_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        wire("item-1", "2026-10-19T10:00:00Z"),
                        wire("item-2", "2026-10-19T11:00:00Z"),
                    ],
                ),
                loaded_at.clone(),
            )
            .expect("an unchanged marked row does not make the load a write");

        let copy = store.collection(&one).expect("the copy is held");
        assert_eq!(copy.loaded_at(), Some(&loaded_at));
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&marked)));
        assert_eq!(
            store.item(&one, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T11:00:00Z")))
        );
    }

    #[test]
    fn a_refused_guarded_load_leaves_the_previous_marker_and_a_later_load_moves_it() {
        // "A refusal changes nothing" includes the marker: a refused load must not advance it, and a
        // later successful load must.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let marked = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, marked.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        let first = at("2026-10-19T10:00:00Z");
        store
            .load_wire_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                first.clone(),
            )
            .expect("the marked row is unchanged");

        let refused = store.load_wire_collection(
            &one,
            wire_collection(&one, vec![wire("item-1", "2026-10-19T11:00:00Z")]),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-1"),
            })
        );
        let copy = store.collection(&one).expect("the copy is still held");
        assert_eq!(copy.loaded_at(), Some(&first), "the refused load moved it");
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&marked)));

        let second = at("2026-10-19T11:00:00Z");
        store
            .load_wire_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        wire("item-1", "2026-10-19T10:00:00Z"),
                        wire("item-2", "2026-10-19T11:00:00Z"),
                    ],
                ),
                second.clone(),
            )
            .expect("the later load succeeds with the marked row unchanged");

        let copy = store.collection(&one).expect("the copy is held");
        assert_eq!(copy.loaded_at(), Some(&second));
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&marked)));
        assert_eq!(
            store.item(&one, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T11:00:00Z")))
        );
    }

    #[test]
    fn a_guarded_load_containing_another_households_item_is_refused_whole() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        store
            .upsert_item(&two, live("item-2", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let refused = store.load_wire_collection(
            &one,
            wire_collection(
                &one,
                vec![
                    wire("item-1", "2026-10-19T10:00:00Z"),
                    wire("item-2", "2026-10-19T10:00:00Z"),
                ],
            ),
            at("2026-10-19T11:00:00Z"),
        );

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: one.clone(),
                owner: two.clone(),
            })
        );
        assert_eq!(store.local_copies().get(&one), None);
        assert_eq!(
            store.item(&two, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T10:00:00Z")))
        );
    }

    #[test]
    fn a_write_to_a_mutable_entity_replaces_its_row() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        store
            .upsert_item(&one, live("item-1", "2026-10-19T10:00:00Z"))
            .expect("a first write");

        let replaced = store
            .upsert_item(&one, live("item-1", "2026-10-19T11:00:00Z"))
            .expect("the entity is mutable");

        assert_eq!(replaced, Some(live("item-1", "2026-10-19T10:00:00Z")));
        assert_eq!(
            store.item(&one, &id("item-1")),
            Ok(Some(&live("item-1", "2026-10-19T11:00:00Z")))
        );
    }

    #[test]
    fn a_write_to_an_immutable_entity_is_refused_and_leaves_the_row_unchanged() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let original = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, original.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        let refused = store.upsert_item(&one, live("item-1", "2026-10-19T11:00:00Z"));

        assert_eq!(
            refused,
            Err(BoundaryRefusal::Immutable {
                item_id: id("item-1"),
            })
        );
        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&original)));
        assert!(store.is_immutable(&id("item-1")));
    }

    #[test]
    fn the_immutable_marker_cannot_be_bypassed_through_another_write_path() {
        // Every public write path must consult the marker. If one of them wrote directly to the
        // copy — or if a mutable copy escaped — this test would observe a changed row.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let original = live("item-1", "2026-10-19T10:00:00Z");
        store
            .upsert_item(&one, original.clone())
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        let changed = live("item-1", "2026-10-19T11:00:00Z");
        let unmarked = live("item-2", "2026-10-19T11:00:00Z");
        let immutable = BoundaryRefusal::Immutable {
            item_id: id("item-1"),
        };

        assert_eq!(refused(store.upsert_item(&one, changed.clone())), immutable);
        // The unmarked row comes *first*: if `upsert_all` checked and wrote as it went, item-2 would
        // land before the marked row was reached. It must not.
        assert_eq!(
            refused(store.upsert_all(&one, [unmarked.clone(), changed.clone()])),
            immutable
        );
        assert_eq!(
            store.item(&one, &id("item-2")),
            Ok(None),
            "the unmarked row in the refused batch did not land",
        );
        assert_eq!(
            refused(store.upsert_wire_collection(
                &one,
                wire_collection(
                    &one,
                    vec![
                        wire("item-2", "2026-10-19T11:00:00Z"),
                        wire("item-1", "2026-10-19T11:00:00Z"),
                    ],
                ),
            )),
            immutable
        );
        assert_eq!(store.item(&one, &id("item-2")), Ok(None));

        assert_eq!(store.item(&one, &id("item-1")), Ok(Some(&original)));
    }

    #[test]
    fn marking_one_entity_immutable_leaves_its_households_other_entities_writable() {
        let mut store = GuardedStore::new();
        let one = household("household-1");
        store
            .upsert_all(
                &one,
                [
                    live("item-1", "2026-10-19T10:00:00Z"),
                    live("item-2", "2026-10-19T10:00:00Z"),
                ],
            )
            .expect("a first write");
        store.mark_immutable(&id("item-1"));

        store
            .upsert_item(&one, live("item-2", "2026-10-19T11:00:00Z"))
            .expect("item-2 is not marked");

        assert_eq!(
            store.item(&one, &id("item-2")),
            Ok(Some(&live("item-2", "2026-10-19T11:00:00Z")))
        );
    }

    #[test]
    fn a_tombstone_keeps_its_households_claim() {
        // An identity is the household's for the life of the item, so a removed row cannot be
        // re-claimed by a second household even though it is no longer live.
        let mut store = GuardedStore::new();
        let one = household("household-1");
        let two = household("household-2");
        store
            .upsert_item(
                &one,
                StoredItem::tombstone(id("item-1"), at("2026-10-19T11:00:00Z")),
            )
            .expect("a tombstone write");

        let refused = store.upsert_item(&two, live("item-1", "2026-10-19T12:00:00Z"));

        assert_eq!(
            refused,
            Err(BoundaryRefusal::ForeignHousehold {
                requested: two,
                owner: one.clone(),
            })
        );
        assert_eq!(store.household_of(&id("item-1")), Some(&one));
    }
}
