//! The app-wide state the command adapters share, installed once in [`crate::run`]'s `setup`.
//!
//! The state is deliberately one value rather than a collection of managed singletons: the
//! household commands of this section and the read/status commands of the next all need the same
//! few things, and a single installation is what makes "the running app has exactly one transport,
//! one local copy set and one status" a property of construction.
//!
//! # What it carries, and why each part is here
//!
//! * **The shared service**, behind [`Arc<dyn SharedService>`]. Task 2.4's seam exists so the
//!   transport can be substituted; the app holds the real HTTPS client
//!   ([`HttpService`]), and a test holds a [`ScriptedService`](crate::sync::ScriptedService) in the
//!   same slot. The command path never names the concrete client, so adding the double to a test
//!   needs no test-only branch in the command.
//! * **The device's [`GuardedStore`]**, held privately behind a [`Mutex`]. Task 2.3 owns the layout
//!   and task 2.7 the guards; the read path (task 2.5) reaches it only through
//!   [`AppState::load_collection`] (a load that records `loaded_at`), [`AppState::merge_collection`]
//!   (a merge that leaves the marker alone) and [`AppState::read_collection`], each of which locks
//!   internally. No caller receives the store itself, so no command can replace it and discard the
//!   ownership claims and immutable markers; nor can it reach a mutable
//!   [`LocalCopies`](crate::storage::LocalCopies) and write around household isolation.
//! * **The [`StatusTracker`]**, behind a [`Mutex`]. Task 2.13's status commands will record fresh
//!   probe outcomes here; task 2.1 neither reads nor writes it, which is deliberate — the write
//!   path takes its own fresh probe and never consults a remembered status (D0.2, R3).
//! * **The [`HouseholdBook`]**, behind a [`Mutex`]. It holds the process's view of the person's
//!   households, including the operation ledger that makes a replayed create return the existing
//!   household (task 2.1, D0.14); task 2.2 will read it for the household list and the active one.
//! * **The current account**, when one exists. P0 has no sign-in or account-acquisition flow (Q25),
//!   so the running app is constructed with `None`; [`AppState::create_household`] then reports
//!   `no_account` before anything is sent, rather than pretending a write could succeed. The slot is
//!   an `Option` rather than a fabricated identity because "no account" is the honest state, and
//!   because no dependency is added for identity: the live-deployment task that provisions the first
//!   account can pass it here.
//!
//! # What it is not
//!
//! It is not a second write path, a queue or a cache of saved data. A create goes through
//! [`crate::domain::create_household`] and therefore through the gate; the book is updated only
//! after the gate returned a receipt, so nothing here can turn a refusal into a local success
//! (SYNC-2, D0.1). The mutexes are `std::sync::Mutex` and every guard is dropped before an
//! `.await`: the command locks the book only after the network call returns, and the status and
//! copy guards are not held across one at all.
//!
//! [`Arc<dyn SharedService>`]: std::sync::Arc
//! [`HttpService`]: crate::sync::HttpService
//! [`Mutex`]: std::sync::Mutex

use std::sync::{Arc, Mutex, MutexGuard};

use crate::domain::{self, HouseholdBook, validate_name};
use crate::ipc::types::{
    AccountUserId, CollectionSource, CommandError, Household, HouseholdCollection, HouseholdId,
    Instant, RequestId,
};
use crate::storage::{BoundaryRefusal, GuardedStore};
use crate::sync::http::AccessTokenSource;
use crate::sync::{HttpService, SharedService, StatusTracker};

/// The service origin the app is built with in P0.
///
/// There is **no live deployment** in this change (D0.7: the deployment is outside this repository
/// and outside its gate), so there is no origin to name. It is empty rather than invented, and it is
/// unobservable in P0 for the reason above the struct: no account exists, so
/// [`AppState::create_household`] refuses with `no_account` before the transport is asked anything.
/// The deployment task that first needs a real write supplies the real origin here.
const SERVICE_ORIGIN: &str = "";

/// The one managed state the command adapters borrow.
pub struct AppState {
    /// The shared service, or the scripted double in a test (task 2.4's seam).
    service: Arc<dyn SharedService>,
    /// The current account, or `None` while P0 has no sign-in flow (Q25).
    account: Option<AccountUserId>,
    /// The process's household set and operation ledger (task 2.1; read by task 2.2).
    households: Mutex<HouseholdBook>,
    /// The device's guarded copies of the collections it has loaded (tasks 2.3, 2.5 and 2.7).
    copies: Mutex<GuardedStore>,
    /// The last fresh service/account status (task 2.13).
    status: Mutex<StatusTracker>,
}

impl AppState {
    /// State over one transport and one account.
    ///
    /// This is the seam a test uses to substitute the scripted double: it is the same constructor
    /// the running app uses, only with a different service and an account instead of `None`. There
    /// is no test-only state type and no command branch that knows which transport it received.
    pub fn new(service: Arc<dyn SharedService>, account: Option<AccountUserId>) -> Self {
        Self {
            service,
            account,
            households: Mutex::new(HouseholdBook::new()),
            copies: Mutex::new(GuardedStore::new()),
            status: Mutex::new(StatusTracker::new()),
        }
    }

    /// The state the running P0 app installs: the real HTTPS client, and no account.
    ///
    /// The token source answers `None` because the platform Keychain belongs to a sign-in flow this
    /// change does not ship (Q25). That is the honest half of the construction: the transport is
    /// real, but it has no identity to send, and [`AppState::create_household`] reports `no_account`
    /// rather than attempting an unauthenticated write or creating local-only data.
    ///
    /// # Errors
    ///
    /// The pinned `reqwest` client cannot be built (a TLS backend that fails to initialise). There
    /// is no later call that could recover from it, and P0 has nothing to fall back to, so the app's
    /// `setup` reports it instead of hiding it behind a state that could never work.
    pub fn production() -> Result<Self, reqwest::Error> {
        let service = HttpService::new(SERVICE_ORIGIN, no_account_token())?;

        Ok(Self::new(Arc::new(service), None))
    }

    /// The current account, when one is available.
    pub fn account(&self) -> Option<&AccountUserId> {
        self.account.as_ref()
    }

    /// The shared service this app was built with.
    ///
    /// It is the seam task 2.13's probes and task 2.5's refresh will use, and the reason the state
    /// is constructed from `Arc<dyn SharedService>` rather than the concrete client: a test can hand
    /// it the scripted double through [`AppState::new`] without a different code path.
    pub fn service(&self) -> &dyn SharedService {
        self.service.as_ref()
    }

    /// The process's household set and operation ledger.
    pub fn households(&self) -> MutexGuard<'_, HouseholdBook> {
        lock(&self.households)
    }

    /// Load one household's collection through the guarded store, recording the load instant
    /// (SYNC-3, NFR-10).
    ///
    /// The collection mutex is locked and released inside this method, so the caller never receives
    /// the [`GuardedStore`] — let alone a replaceable handle to it. Task 2.5's refresh uses this for
    /// a load; [`AppState::merge_collection`] is the merge that leaves the marker alone.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal`] as [`GuardedStore::load_wire_collection`]: a collection describing another
    /// household, a row claimed by another household, or a changed/omitted marked entity.
    pub fn load_collection(
        &self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
        loaded_at: Instant,
    ) -> Result<(), BoundaryRefusal> {
        lock(&self.copies).load_wire_collection(household_id, collection, loaded_at)
    }

    /// Merge one household's rows through the guarded store, leaving `loaded_at` untouched
    /// (NFR-10).
    ///
    /// The collection mutex is locked and released inside this method; see
    /// [`AppState::load_collection`] for the load that records the marker.
    ///
    /// # Errors
    ///
    /// [`BoundaryRefusal`] as [`GuardedStore::upsert_wire_collection`].
    pub fn merge_collection(
        &self,
        household_id: &HouseholdId,
        collection: HouseholdCollection,
    ) -> Result<(), BoundaryRefusal> {
        lock(&self.copies).upsert_wire_collection(household_id, collection)
    }

    /// The collection this device holds for `household_id`, labelled with the source of the read
    /// (SYNC-3).
    ///
    /// `None` means this device holds no copy for the household — a different fact from a held copy
    /// whose `loaded_at` is `None`. The collection mutex is locked and released inside this method,
    /// so the read returns the wire value rather than a guard or a store reference.
    pub fn read_collection(
        &self,
        household_id: &HouseholdId,
        source: CollectionSource,
    ) -> Option<HouseholdCollection> {
        lock(&self.copies)
            .collection(household_id)
            .map(|copy| copy.to_wire(source))
    }

    /// The last fresh service/account status.
    pub fn status(&self) -> MutexGuard<'_, StatusTracker> {
        lock(&self.status)
    }

    /// Create a household with a non-blank name (HS-1, SYNC-2; D0.14).
    ///
    /// The steps are ordered for the person, not for the transport:
    ///
    /// 1. A blank name is `validation` even when no account exists, because the form asked for a
    ///    name and that is what has to change (HS-1).
    /// 2. No account is `no_account` before any probe. P0 always takes this path: it has no
    ///    sign-in flow (Q25), and pretending otherwise would report a write the service never saw.
    /// 3. With both, the domain operation goes through [`crate::sync::SaveGate`] — a fresh probe,
    ///    then the write — and its receipt is the only path that reaches the book.
    ///
    /// The book is recorded after the await, so no mutex guard is held across it. A replay returns
    /// the household the operation already produced and does not change which household is active
    /// (D0.14).
    ///
    /// # Errors
    ///
    /// `validation` for a blank name, `no_account` when no account is available, and otherwise the
    /// gate's mapped causes. A receipt whose household body cannot be read is `unknown`; nothing is
    /// recorded in that case, so a failed read can never become a local household.
    pub async fn create_household(
        &self,
        name: String,
        request_id: RequestId,
    ) -> Result<Household, CommandError> {
        let name = validate_name(name, &request_id)?;
        let Some(account) = self.account.as_ref() else {
            return Err(CommandError::no_account(Some(request_id)));
        };

        let record =
            domain::create_household(self.service.as_ref(), account, name, request_id.clone())
                .await?;

        let mut households = lock(&self.households);
        let stored = households.record_created(request_id, record);
        let is_active = households.is_active(stored.id());

        Ok(stored.to_wire(account, is_active))
    }
}

/// The token source P0 can offer: none.
///
/// The refresh token's storage is the platform Keychain, owned by a sign-in flow this change does
/// not implement (D0.7 criterion 4, Q25). Returning `None` is what makes the real client report the
/// absence of an account as itself instead of sending an unauthenticated request.
fn no_account_token() -> AccessTokenSource {
    Arc::new(|| None)
}

/// Lock a mutex, recovering a poisoned one rather than cascading the panic.
///
/// The critical sections here only move small values between fields and read them back; none can
/// leave the protected value inconsistent, so a panic elsewhere should not make this state
/// permanently unusable. The alternative — `expect` on every lock — would turn an unrelated panic
/// into a second failure in whichever command happens to run next.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ipc::types::{CollectionItem, ItemId, SaveCause};
    use crate::sync::double::ScriptedService;
    use crate::sync::double::test_support::block_on_ready;
    use crate::sync::{Answer, ServiceReply};

    fn account() -> AccountUserId {
        AccountUserId("account-1".to_owned())
    }

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn wire(item_id: &str, updated_at: &str) -> CollectionItem {
        CollectionItem {
            item_id: ItemId(item_id.to_owned()),
            updated_at: at(updated_at),
            deleted: false,
        }
    }

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

    /// One accepted create answer the domain can read.
    fn accepted_answer(household_id: &str) -> ServiceReply {
        ServiceReply::Answered(Answer::with_body(
            200,
            json!({
                "outcome": "applied",
                "household": {
                    "id": household_id,
                    "owner_member_id": "member-1",
                    "created_at": "2026-10-19T10:00:00Z",
                },
            }),
        ))
    }

    #[test]
    fn without_an_account_a_create_is_refused_with_no_account_and_nothing_is_sent() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        let create = state.create_household("Home".to_owned(), request_id.clone());
        let error = block_on_ready(create).expect_err("P0 has no account");

        assert_eq!(error.cause, SaveCause::NoAccount);
        assert_eq!(error.message_key, "error.no_account");
        assert!(error.retryable);
        assert_eq!(error.request_id, Some(request_id));

        // The honest outcome is a refusal, not an unauthenticated attempt and not a local-only
        // household (Q25, SYNC-2).
        assert!(service.calls().is_empty());
        assert_eq!(service.write_count(), 0);
        assert!(state.households().is_empty());
    }

    #[test]
    fn a_blank_name_is_refused_before_the_account_is_checked() {
        let service = Arc::new(ScriptedService::new());
        let state = AppState::new(service.clone(), None);
        let request_id = request_id("operation-1");

        for blank in ["", " \t\n"] {
            let create = state.create_household(blank.to_owned(), request_id.clone());
            let error = block_on_ready(create).expect_err("a blank name is refused");

            assert_eq!(error.cause, SaveCause::Validation);
            assert_eq!(error.request_id, Some(request_id.clone()));
        }

        assert!(service.calls().is_empty());
        assert!(state.households().is_empty());
    }

    /// The state offers the read path checked operations and never a replaceable store handle.
    ///
    /// There is deliberately no `copies()` accessor any more: a guard over `GuardedStore` could be
    /// overwritten wholesale (`*state.copies() = GuardedStore::new()`), discarding every ownership
    /// claim and immutable marker. These methods lock internally and return values or refusals, so
    /// that operation does not exist in the public API.
    #[test]
    fn the_state_offers_checked_collection_operations_and_no_replaceable_store() {
        let state = AppState::new(Arc::new(ScriptedService::new()), None);
        let one = household("household-1");
        let two = household("household-2");
        let loaded_at = at("2026-10-19T11:00:00Z");

        state
            .load_collection(
                &one,
                wire_collection(&one, vec![wire("item-1", "2026-10-19T10:00:00Z")]),
                loaded_at.clone(),
            )
            .expect("the collection is addressed correctly");

        let read = state
            .read_collection(&one, CollectionSource::Local)
            .expect("this device holds the copy");
        assert_eq!(read.loaded_at, Some(loaded_at));
        assert_eq!(read.items.len(), 1);

        // The state's handle is the guard: a collection addressed through the wrong household is
        // refused rather than written, so the narrow API does not weaken the invariant.
        assert!(matches!(
            state.merge_collection(&two, wire_collection(&one, vec![])),
            Err(BoundaryRefusal::ForeignHousehold { .. })
        ));
    }

    #[test]
    fn a_substituted_double_drives_the_real_create_path() {
        let service = Arc::new(ScriptedService::new().on_write([accepted_answer("household-1")]));
        let state = AppState::new(service.clone(), Some(account()));

        let create = state.create_household("Home".to_owned(), request_id("operation-1"));
        let household = block_on_ready(create).expect("the double accepted the write");

        assert_eq!(household.name, "Home");
        assert!(household.is_active);
        assert!(household.is_owner);
        assert_eq!(service.write_count(), 1);
        assert_eq!(
            state.households().active().expect("one is active").id(),
            &household.id
        );
    }
}
