//! The event contract: P0's `household:changed` and `service:status-changed` (task 1.3), and P1's
//! `rabbit:changed` and `weight:changed` (task 1.1).
//!
//! All four events are registered in the single builder's `collect_events![]` and mounted by
//! `Builder::mount_events` inside the app's one `setup` closure, so the running app and the
//! generated bindings cannot disagree about which events exist.
//!
//! The frontend subscribes through the generated `events` object — `events.householdChanged`,
//! `events.serviceStatusChanged`, `events.rabbitChanged` and `events.weightChanged` (task 3.6) — and
//! never builds an event name by hand. The names carry a colon because they are frozen in
//! `domain:event` form (`tauri-ipc-contract`, design "IPC contract").
//!
//! **Why the trait is implemented by hand.** `tauri-specta` gates its `Event` derive macro behind
//! its optional `derive` feature, and section 0 pins that crate with `features = ["typescript"]`
//! alone (D0.8). `tauri_specta::Event` has exactly one required item — `const NAME` — so an
//! explicit implementation registers the same event the derive would, with the colon-bearing name
//! the contract fixes, and this module needs no manifest change. The four payloads below each get one
//! such block; if a later phase enables `derive`, those blocks are what it replaces with
//! `#[tauri_specta(event_name = "...")]` on the payload types.
//!
//! # The P1 pair
//!
//! [`RabbitChanged`] and [`WeightChanged`] mirror [`HouseholdChanged`] deliberately: household-scoped
//! payloads that name the affected entity and the kind of change, so a member's edit reaches the other
//! members without a cold reload (HS-8) and the frontend invalidates only the affected queries. They
//! carry no data the frontend would have to apply — the invalidation refetches through the read
//! commands, so an event can never become a second write path or a stale copy of the truth.
//!
//! # Who publishes `household:changed`
//!
//! The event name and payload live here because they are the contract; the producer is the
//! [`HouseholdChangePublisher`] seam below, mirroring `sync::lifecycle`'s
//! [`StatusPublisher`](crate::sync::StatusPublisher). The running app's implementation is
//! [`tauri::AppHandle`], which emits the frozen event name; the tests use a recorder. The command
//! adapters publish `created`/`switched` after a successful call, and the platform lifecycle trigger
//! publishes `refreshed` for the active household after a reachable, account-valid probe. Nothing
//! publishes from `load_household` or from `re_evaluate`: the frontend's `refreshed` policy
//! invalidates the collection, whose query calls `load_household`, so an emit from the load would
//! refetch in a loop.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::types::{HouseholdId, RabbitId, ServiceStatus, WeightEntryId};

/// How a household changed, as `household:changed` reports it (SYNC-1, HS-3, DM-3).
///
/// P3 reuses the same four kinds unchanged, where `removed` carries deletion propagation.
/// `switched` is the active household changing — per person, not per collection — and `refreshed`
/// is a reload of a loaded collection from the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdChangeKind {
    /// A household was created and became active (HS-1).
    Created,
    /// The person's active household changed (HS-3, HS-4).
    Switched,
    /// A loaded collection was reloaded from the service.
    Refreshed,
    /// A household left the person's held set.
    Removed,
}

/// The `household:changed` payload: which household changed and how.
///
/// The household id is part of the payload so the frontend can invalidate only the affected
/// household's queries (task 3.6) instead of refetching everything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HouseholdChanged {
    /// The household the change belongs to.
    pub household_id: HouseholdId,
    /// What happened to it.
    pub kind: HouseholdChangeKind,
}

/// Registers the frozen event name for [`HouseholdChanged`].
impl tauri_specta::Event for HouseholdChanged {
    const NAME: &'static str = "household:changed";
}

/// Where one household change is published.
///
/// It exists for the same reason as
/// [`StatusPublisher`](crate::sync::StatusPublisher): the production half is the app handle's
/// event bus, and a unit test can observe the exact sequence without building a Tauri app. The
/// seam covers *publication only* — the command or lifecycle path that decides a change happened is
/// identical in both cases.
pub trait HouseholdChangePublisher {
    /// Publish `change` to whoever is listening.
    fn publish(&self, change: &HouseholdChanged);
}

/// Publish through Tauri's own event bus (the frozen `household:changed` name).
///
/// The event was mounted by `ipc::builder().mount_events(app)` in the app's single `setup`, so
/// [`tauri_specta::Event::emit`] resolves the frozen name from the registry instead of building one
/// by hand. A publication failure is deliberately **not** a command failure: the state change has
/// already happened and is what the caller returns, so a listener that could not be reached does not
/// make the change less true (the same rule `StatusPublisher` documents).
impl HouseholdChangePublisher for tauri::AppHandle {
    fn publish(&self, change: &HouseholdChanged) {
        let _published = tauri_specta::Event::emit(change, self);
    }
}

/// Registers the frozen event name for [`ServiceStatus`].
///
/// The payload is a shared type — it is also `get_service_status`'s result — so the type lives in
/// [`super::types`] and only its event name lives here.
impl tauri_specta::Event for ServiceStatus {
    const NAME: &'static str = "service:status-changed";
}

/// How a rabbit changed, as `rabbit:changed` reports it (RAB-1, RAB-5 … RAB-7, RAB-10).
///
/// The five kinds are the five lifecycle operations the contract exposes, so a listener can act on
/// the difference between a reversible archive and an irreversible deletion without a second read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RabbitChangeKind {
    /// A rabbit profile was created (RAB-1).
    Created,
    /// A field of an active rabbit's profile was edited (RAB-5).
    Updated,
    /// A rabbit was archived: a reversible soft delete (RAB-6).
    Archived,
    /// An archived rabbit was returned to the active list (RAB-7).
    Restored,
    /// An archived rabbit and its entries were permanently removed (RAB-10).
    Deleted,
}

/// The `rabbit:changed` payload: which rabbit in which household changed, and how.
///
/// It mirrors [`HouseholdChanged`]: the household id scopes the invalidation and the rabbit id narrows
/// it to one row and one detail. The household id is carried even though the rabbit id addresses the
/// rabbit, because the frontend's cache is keyed by household and a change must invalidate the
/// dashboard of the household it belongs to without a lookup (task 3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RabbitChanged {
    /// The household the rabbit belongs to.
    pub household_id: HouseholdId,
    /// The rabbit that changed. For [`RabbitChangeKind::Deleted`] it is the rabbit that no longer
    /// exists — the id a listener needs to drop its cached rows.
    pub rabbit_id: RabbitId,
    /// What happened to it.
    pub kind: RabbitChangeKind,
}

/// Registers the frozen event name for [`RabbitChanged`].
impl tauri_specta::Event for RabbitChanged {
    const NAME: &'static str = "rabbit:changed";
}

/// How a weight entry changed, as `weight:changed` reports it (WT-1, WT-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WeightChangeKind {
    /// An entry was recorded (WT-1).
    Created,
    /// A past entry was corrected (WT-5).
    Updated,
    /// A past entry was removed (WT-5).
    Deleted,
}

/// The `weight:changed` payload: which entry of which rabbit changed, and how.
///
/// The rabbit id is carried beside the entry id because every weight surface is scoped by rabbit: a
/// listener invalidates that rabbit's trend, history, its dashboard row and — because an edit or a
/// deletion re-derives the alerts around the change (D1.13) — the entries next to it. The entry id
/// narrows that further to the detail view, and names the entry that no longer exists for a deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WeightChanged {
    /// The household the rabbit and its entry belong to.
    pub household_id: HouseholdId,
    /// The rabbit whose entry changed.
    pub rabbit_id: RabbitId,
    /// The entry that changed, or the entry that was removed.
    pub entry_id: WeightEntryId,
    /// What happened to it.
    pub kind: WeightChangeKind,
}

/// Registers the frozen event name for [`WeightChanged`].
impl tauri_specta::Event for WeightChanged {
    const NAME: &'static str = "weight:changed";
}

/// The publication seam's test side: it records what the app would have emitted.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    use super::{HouseholdChangePublisher, HouseholdChanged};

    /// A publisher that keeps every change it was handed, in order.
    #[derive(Debug, Default)]
    pub(crate) struct RecordingHouseholdPublisher {
        published: Mutex<Vec<HouseholdChanged>>,
    }

    impl RecordingHouseholdPublisher {
        /// Every recorded change, in publication order.
        pub(crate) fn published(&self) -> Vec<HouseholdChanged> {
            self.published
                .lock()
                .expect("the records are readable")
                .clone()
        }
    }

    impl HouseholdChangePublisher for RecordingHouseholdPublisher {
        fn publish(&self, change: &HouseholdChanged) {
            self.published
                .lock()
                .expect("the records are writable")
                .push(change.clone());
        }
    }
}
