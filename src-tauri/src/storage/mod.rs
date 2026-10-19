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
//!   device's set of those copies, with the loaded-at marker the read path reports (SYNC-3).
//!
//! **What is deliberately absent, and who owns it.** This module holds no network call and no
//! service concern: the shared-service client, the reachability gate and the gated save path are
//! task 2.4's; the refresh orchestration that decides *when* a copy is replaced — startup, a
//! foreground return, a reachable service — is task 2.5's; the last-write-wins comparator that
//! decides *which* of two versions survives is 2.6's; the cross-household and immutability guards
//! are 2.7's; timestamp parsing and formatting are 2.8's; and the failure surface those tasks report
//! through is 2.11's. Keeping the layout here free of all of them is what lets the layout be tested
//! without an app, a clock or a service.
//!
//! **No frozen wire type is duplicated.** [`CollectionItem`](crate::ipc::types::CollectionItem),
//! [`HouseholdCollection`](crate::ipc::types::HouseholdCollection) and
//! [`CollectionSource`](crate::ipc::types::CollectionSource) stay as task 1.3 froze them; the
//! storage types convert to and from them instead of re-declaring their fields.

pub mod collection;
pub mod local;

pub use collection::{StoredCollection, StoredItem};
pub use local::{LocalCopies, LocalCopy};
