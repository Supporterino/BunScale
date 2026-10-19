//! The storage boundary: where the device keeps the data it holds (D0.2, D0.4, D0.5, NFR-10).
//!
//! `design.md`'s "State and boundaries" gives this directory one job — household isolation,
//! canonical representation, immutability and absolute instants — and the ADR fixes its position:
//! the device's copy of a household's collection lives here, and the refresh that replaces that copy
//! goes *through* `src-tauri/src/sync/` (ADR 0002 R1). The local copy is a cache of the shared
//! collection, never a competing record.
//!
//! Task 2.3 implements the first piece of that boundary — the collection **layout**:
//!
//! - [`StoredItem`] — one row's identity, its version marker and its tombstone flag;
//! - [`StoredCollection`] — one household's rows, keyed so that a second load of an item upserts
//!   rather than duplicates (SYNC-1);
//! - [`LocalCopy`] / [`LocalCopies`] — the device's copy of one household's collection and the
//!   device's set of those copies, with the loaded-at marker the read path reports (SYNC-3);
//! - [`CollectionStore`] — the file under the app's own data directory that keeps those copies
//!   across a relaunch (task 2.5), with [`CollectionStoreError`] as its failure classification.
//!
//! Task 2.7 adds the boundary's **guards** on top of that layout:
//!
//! - [`BoundaryRefusal`] — the typed outcome of a refused cross-household access, a non-canonical
//!   value or a write to an immutable entity, so a caller can act on the reason rather than parse a
//!   message;
//! - [`GuardedStore`] — the device's copies behind checked read/write/load operations, with the
//!   item-to-household claim and the immutable marker private; it is also what the running app keeps
//!   behind its state mutex, so a command cannot reach a mutable [`LocalCopies`];
//! - [`Canonical`] / [`CanonicalForm`] — the one admitted representation of a value whose type has a
//!   single stored form. P0 ships no such value — P1's weight is the first — so the helper is what
//!   that type will use rather than a weight model invented here.
//!
//! Task 2.8 adds the timestamp half of that boundary: [`AbsoluteInstant`] parses the frozen wire
//! instant into canonical UTC, defaults to the creation moment only when no value was supplied,
//! passes a supplied instant through unchanged, and refuses malformed text through [`InstantError`]
//! instead of defaulting it (D0.5, DM-6).
//!
//! **What is deliberately absent, and who owns it.** This module holds no network call and no
//! service concern: the shared-service client, the reachability gate and the gated save path are
//! task 2.4's; the refresh orchestration that decides *when* a copy is replaced — startup, a
//! foreground return, a reachable service — is task 2.5's; the last-write-wins comparator that
//! decides *which* of two versions survives is 2.6's; the failure surface those tasks report
//! through is 2.11's. Keeping the boundary free of them is what lets its layout, guards and
//! timestamps be tested without an app, a clock or a service.
//!
//! **No frozen wire type is duplicated.** [`CollectionItem`](crate::ipc::types::CollectionItem),
//! [`HouseholdCollection`](crate::ipc::types::HouseholdCollection) and
//! [`CollectionSource`](crate::ipc::types::CollectionSource) stay as task 1.3 froze them; the
//! storage types convert to and from them instead of re-declaring their fields.

pub mod collection;
pub mod guards;
pub mod instant;
pub mod local;
pub mod persist;

pub use collection::{StoredCollection, StoredItem};
pub use guards::{BoundaryRefusal, Canonical, CanonicalForm, GuardedStore};
pub use instant::{AbsoluteInstant, InstantError};
pub use local::{LocalCopies, LocalCopy};
pub use persist::{
    COLLECTIONS_FILE_NAME, CollectionStore, CollectionStoreError, DeviceSnapshot,
    PersistedHousehold,
};
