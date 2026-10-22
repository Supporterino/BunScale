//! The P0 event contract: `household:changed` and `service:status-changed` (task 1.3).
//!
//! Both events are registered in the single builder's `collect_events![]` and mounted by
//! `Builder::mount_events` inside the app's one `setup` closure, so the running app and the
//! generated bindings cannot disagree about which events exist.
//!
//! The frontend subscribes through the generated `events` object — `events.householdChanged` and
//! `events.serviceStatusChanged` (task 3.6) — and never builds an event name by hand. The names
//! carry a colon because they are frozen in `domain:event` form (`tauri-ipc-contract`, design
//! "IPC contract").
//!
//! **Why the trait is implemented by hand.** `tauri-specta` gates its `Event` derive macro behind
//! its optional `derive` feature, and section 0 pins that crate with `features = ["typescript"]`
//! alone (D0.8). `tauri_specta::Event` has exactly one required item — `const NAME` — so an
//! explicit implementation registers the same event the derive would, with the colon-bearing name
//! the contract fixes, and this task needs no manifest change. If a later phase enables `derive`
//! (P1 adds `rabbit:changed` and `weight:changed`), these two blocks are what it replaces with
//! `#[tauri_specta(event_name = "...")]` on the payload types.
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

use super::types::{HouseholdId, ServiceStatus};

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
