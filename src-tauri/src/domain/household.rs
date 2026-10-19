//! The household, member and ownership model, and the invariants a caller cannot skip (HS-1,
//! HS-3, HS-4, DM-3; D0.3).
//!
//! Three types carry the whole model:
//!
//! * [`HouseholdName`] — a name that is known not to be blank. Its only constructor is
//!   [`HouseholdName::parse`], so validation cannot be forgotten at a call site: empty and
//!   whitespace-only input both answer `None`, and [`validate_name`] turns that into the contract's
//!   `validation` cause (HS-1).
//! * [`HouseholdRecord`] — one household as the domain holds it, with its members and owner. It is
//!   built by [`HouseholdRecord::created`], which **puts the creator in the member list first and
//!   records that same member as the owner**, so "the creator is the first member and owner" is a
//!   property of construction rather than a check every later task repeats (D0.3, HS-1).
//! * [`HouseholdBook`] — the person's households, with at most one active one. The active id is a
//!   single field, so there is no representation in which two households are active; creating one
//!   sets that field to the new household, replacing the previous active one (DM-3).
//!
//! **The book also holds the operation ledger, and it is deliberately not the service's ledger.**
//! D0.14's durable deduplication is the shared service's `request_log`; the book's map is the
//! local mirror of what this process has already applied. A replay still goes through the gate with
//! the same `RequestId` — the service is asked and answers with its stored outcome — but
//! [`HouseholdBook::record_created`] returns the household already recorded for that identity
//! instead of adding a second one. When a household leaves the held set its operation is
//! **tombstoned** rather than forgotten, so a replay still returns that operation's stored household
//! and cannot put it back. That is what makes a replay safe even if the answer is lost between the
//! service and this process, and it is why no code path can put two households under one operation
//! identity.
//!
//! **The wire type is produced here, not re-declared.** [`HouseholdRecord::to_wire`] fills the
//! frozen [`Household`] shape, including the two derived flags: `is_active` comes from the book's
//! single active id, and `is_owner` comes from the owner member's account compared with the current
//! account. No domain type duplicates the contract's fields.
//!
//! # The active household is local state (HS-3, DM-3)
//!
//! Switching the active household is **not a shared-collection write**. The active household is the
//! person's selection over the households they hold, not an item inside any household's collection,
//! and D0.1's gate governs writes to those collections. P0's frozen service surface has no
//! operation for the selection either, so recording it here is not a second write path: a switch
//! assigns [`HouseholdBook::active`] and nothing leaves the device. Two consequences follow, and
//! both are stated where they are observable rather than left to be discovered:
//!
//! * The command that switches can refuse only a household the person does not hold — there is no
//!   probe to fail, no account to check and no transport to time out, so reporting any of the
//!   service's causes for it would name a condition that cannot occur.
//! * Re-issuing its [`RequestId`] is naturally idempotent, because the operation assigns one field
//!   one value; it needs no service ledger and no local operation ledger to be safe (D0.14's
//!   deduplication applies to operations the service performs).
//!
//! The selection lives in this process's book, like every other fact here. The transitions DM-3
//! defines are the two set operations: [`HouseholdBook::set_active`] for a person choosing among
//! held households, and [`HouseholdBook::release`] for a household leaving the held set — which
//! re-elects the first still-held household, or leaves none active when none remains.
//!
//! **DM-3 is therefore enforced per device/process, not across devices.** The book is this process's
//! state and P0 has no service-side per-account selection, so two devices can each hold a different
//! active household for the same person while each keeps exactly one active. Cross-device agreement
//! would need a per-account selection stored by the shared service, which P0's frozen contract does
//! not have; a later change that adds one would move this decision, not silently narrow it here.

use std::collections::HashMap;
use std::fmt;

use crate::ipc::types::{
    AccountUserId, CommandError, Household, HouseholdId, HouseholdSummary, Instant, Member,
    MemberId, RequestId,
};

/// A household's name, once it is known not to be blank (HS-1).
///
/// The stored value is the input as supplied: leading and trailing whitespace are preserved, just
/// as a person typed them. What the constructor refuses is a name that is **entirely** whitespace,
/// because that is the state HS-1 calls blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseholdName(String);

impl HouseholdName {
    /// Parse a name, or answer `None` when it is blank (HS-1).
    ///
    /// "Blank" means empty after trimming: `""`, `"   "` and `"\t\n"` are all refused, while a name
    /// with any non-whitespace character is accepted.
    pub fn parse(input: impl Into<String>) -> Option<Self> {
        let input = input.into();

        if input.trim().is_empty() {
            None
        } else {
            Some(Self(input))
        }
    }

    /// The name as it will be stored and shown.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate a household name and report a blank one through the contract's failure surface (HS-1).
///
/// It is the command path's first step, before the account check and before the gate: a person with
/// no account who submits a blank name is asked for a name, not told that their account is missing.
/// A refused name is not retryable — the same input would be refused again — which is the policy
/// [`crate::ipc::failure`] owns rather than a flag chosen here.
pub fn validate_name(
    input: impl Into<String>,
    request_id: &RequestId,
) -> Result<HouseholdName, CommandError> {
    HouseholdName::parse(input).ok_or_else(|| CommandError::validation(Some(request_id.clone())))
}

/// Why a household cannot be made active (HS-3, DM-3).
///
/// Exactly one refusal exists: the household is not among those the person holds. An active household
/// is not a free choice of id — a household that is not held has no members or data this process may
/// present, so electing it would make the book describe a household it cannot support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveHouseholdRefusal {
    /// The person does not hold the household, so it cannot be made active (DM-3).
    NotHeld(HouseholdId),
}

impl fmt::Display for ActiveHouseholdRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHeld(_) => formatter.write_str("the person does not hold the household"),
        }
    }
}

impl std::error::Error for ActiveHouseholdRefusal {}

/// One household as the domain holds it: identity, name, creation instant, members and owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseholdRecord {
    /// The opaque id the service assigned, which also addresses the household's collection.
    id: HouseholdId,
    /// The non-blank name, validated at construction.
    name: HouseholdName,
    /// When the household was created, as the service supplied it (D0.5).
    created_at: Instant,
    /// The member that created the household; also an entry in [`Self::members`] (D0.3).
    owner_member_id: MemberId,
    /// The member list. P0 holds exactly the creator; P3 grows it, and the creator stays first.
    members: Vec<Member>,
}

impl HouseholdRecord {
    /// Record a new household, with `owner_account_user_id` as its first member and owner.
    ///
    /// The two ownership facts are set together and cannot be set separately: the owner id is the
    /// member id of the [`Member`] placed at index 0. Constructing a record therefore cannot produce
    /// a household whose owner is absent from its members, or whose creator is not first (HS-1,
    /// D0.3). The display name is left absent: resolving a name is SET-2's fallback chain (task 2.9),
    /// and an unresolved name must never block creation.
    pub fn created(
        id: HouseholdId,
        name: HouseholdName,
        created_at: Instant,
        owner_member_id: MemberId,
        owner_account_user_id: AccountUserId,
    ) -> Self {
        let owner = Member {
            id: owner_member_id.clone(),
            account_user_id: owner_account_user_id,
            display_name: None,
        };

        Self {
            id,
            name,
            created_at,
            owner_member_id,
            members: vec![owner],
        }
    }

    /// The household's opaque, stable identifier.
    pub fn id(&self) -> &HouseholdId {
        &self.id
    }

    /// The validated name.
    pub fn name(&self) -> &HouseholdName {
        &self.name
    }

    /// When the household was created.
    pub fn created_at(&self) -> &Instant {
        &self.created_at
    }

    /// The member recorded as the owner (D0.3).
    pub fn owner_member_id(&self) -> &MemberId {
        &self.owner_member_id
    }

    /// The household's members, the creator first (HS-1).
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// The owner's member record, when it is present.
    ///
    /// [`HouseholdRecord::created`] cannot produce a record without it, so `None` means a value was
    /// built by some other path — which is a defect, and one [`Self::to_wire`] reports honestly as
    /// "not the owner" rather than panicking in a command.
    pub fn owner(&self) -> Option<&Member> {
        self.members
            .iter()
            .find(|member| member.id == self.owner_member_id)
    }

    /// Whether the owner member is the person identified by `account`.
    pub fn is_owned_by(&self, account: &AccountUserId) -> bool {
        self.owner()
            .is_some_and(|owner| &owner.account_user_id == account)
    }

    /// This record as the frozen wire type, with the two flags derived from the current state.
    ///
    /// `is_active` is the caller's answer from [`HouseholdBook::is_active`] rather than a flag on
    /// the record, because only the book knows whether this is the person's active household
    /// (DM-3). `is_owner` is derived from the member list, so it cannot disagree with
    /// `owner_member_id` (D0.3).
    pub fn to_wire(&self, current_account: &AccountUserId, is_active: bool) -> Household {
        self.to_wire_for(Some(current_account), is_active)
    }

    /// [`Self::to_wire`] for a read that may run without a resolved account.
    ///
    /// The household list and the active-household read are local reads and do not require an
    /// account, so `None` is a legitimate input. It reports `is_owner: false` rather than claiming
    /// ownership on behalf of an unknown person: P0 has no sign-in flow (Q25), and P3's delete
    /// control must not appear for an identity the product cannot name.
    pub fn to_wire_for(
        &self,
        current_account: Option<&AccountUserId>,
        is_active: bool,
    ) -> Household {
        Household {
            id: self.id.clone(),
            name: self.name.as_str().to_owned(),
            created_at: self.created_at.clone(),
            owner_member_id: self.owner_member_id.clone(),
            is_active,
            is_owner: self.is_owned_by_account(current_account),
        }
    }

    /// This record as one row of the household list, with the two flags derived from current state
    /// (HS-4).
    ///
    /// `member_count` is the recorded member list's length, which is at least 1 because the creator
    /// is always its first entry (HS-1). It counts the household's own members and nothing from a
    /// sibling household, which is what keeps two held households from describing each other's
    /// people (HS-3).
    pub fn to_summary(
        &self,
        current_account: Option<&AccountUserId>,
        is_active: bool,
    ) -> HouseholdSummary {
        HouseholdSummary {
            id: self.id.clone(),
            name: self.name.as_str().to_owned(),
            created_at: self.created_at.clone(),
            is_active,
            is_owner: self.is_owned_by_account(current_account),
            member_count: u32::try_from(self.members.len()).unwrap_or(u32::MAX),
        }
    }

    /// Whether the owner member is the person identified by `account`, when an account is known.
    ///
    /// `None` is `false`: an unresolved identity owns nothing as far as the product can tell.
    pub fn is_owned_by_account(&self, account: Option<&AccountUserId>) -> bool {
        account.is_some_and(|account| self.is_owned_by(account))
    }
}

/// What one create operation has produced, as this book remembers it (D0.14).
///
/// The outcome outlives the household. When a household leaves the held set its operation is
/// **tombstoned** rather than forgotten, so a replay of that create still answers with the same
/// household — the outcome the service stored — and changes nothing. Forgetting it would let a
/// replay re-add the released household and make it active, undoing DM-3's re-election.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordedOutcome {
    /// The operation created a household the person still holds; the id resolves against the book.
    Held(HouseholdId),
    /// The operation created a household that has since left the held set. The record is the
    /// operation's stored outcome, kept so a replay can answer without restoring it.
    Released(HouseholdRecord),
}

/// The person's households and their one active household, keyed by identity (HS-3, HS-4, DM-3).
///
/// The type exists so the two set-level invariants have a home: a household id addresses exactly one
/// record, and at most one record is active. It is the domain counterpart of the service's
/// household set; the service remains the source of truth, and this is the process's view of it.
///
/// **One active id, not a flag per record.** A record cannot be "active" in isolation, so there is
/// no state in which the type holds two active households, and switching the active household is a
/// single assignment rather than a loop that must clear every sibling. `None` is the honest state of
/// a person holding no household, never an empty list pretending to be one. The id is always
/// re-derived against the held set by [`Self::release`], which is where DM-3's re-election lives.
///
/// **The held order is recorded, not inferred.** `HashMap` iteration order varies between runs, so a
/// list built from it would reshuffle on every read and a re-election would pick a different
/// household each time. `held_order` records the order the person acquired the households in and is
/// the order the list is served in and the re-election falls back through. It is kept in step with
/// the map by the only three mutators: [`Self::record_created`], [`Self::release`] and
/// [`Self::set_active`] (the last changes no membership).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdBook {
    /// One record per household id; re-recording an id replaces its one record.
    households: HashMap<HouseholdId, HouseholdRecord>,
    /// The held ids in acquisition order, each appearing exactly once and always a key of
    /// `households`. It is what makes the list and the re-election deterministic.
    held_order: Vec<HouseholdId>,
    /// The id of the active household, or `None` when none is active.
    active: Option<HouseholdId>,
    /// The operation ledger: what each `RequestId` already produced, held or released (D0.14).
    operations: HashMap<RequestId, RecordedOutcome>,
}

impl HouseholdBook {
    /// A book holding no household and no active household.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many households the person holds.
    pub fn len(&self) -> usize {
        self.households.len()
    }

    /// Whether the person holds no household.
    pub fn is_empty(&self) -> bool {
        self.households.is_empty()
    }

    /// The record for `household_id`, when the person holds it.
    pub fn get(&self, household_id: &HouseholdId) -> Option<&HouseholdRecord> {
        self.households.get(household_id)
    }

    /// The active household, or `None` when the person holds none (DM-3).
    pub fn active(&self) -> Option<&HouseholdRecord> {
        self.active
            .as_ref()
            .and_then(|household_id| self.households.get(household_id))
    }

    /// Whether `household_id` is the active household.
    pub fn is_active(&self, household_id: &HouseholdId) -> bool {
        self.active.as_ref() == Some(household_id)
    }

    /// Record the outcome of one create operation, returning the household that operation denotes.
    ///
    /// A **first** outcome stores the record, appends it to the held order, remembers which household
    /// `request_id` produced, and makes that household the active one (HS-1). A **replay** — the same
    /// `request_id` recorded before — returns the stored household and changes nothing, in particular
    /// not the active household: a retried create is the same creation, not a new activation (D0.14).
    /// That holds even when the household has since left the held set: [`Self::release`] tombstones
    /// the operation, so its replay returns the released record and does not re-add it.
    ///
    /// The local ledger is the mirror, not the authority: the service's ledger deduplicates the
    /// request, and this map is what keeps a second local record impossible while the answer is
    /// being replayed.
    pub fn record_created(
        &mut self,
        request_id: RequestId,
        record: HouseholdRecord,
    ) -> HouseholdRecord {
        if let Some(existing) = self.recorded_for(&request_id) {
            return existing;
        }

        let household_id = record.id().clone();
        if !self.households.contains_key(&household_id) {
            self.held_order.push(household_id.clone());
        }
        self.households.insert(household_id.clone(), record.clone());
        self.operations
            .insert(request_id, RecordedOutcome::Held(household_id.clone()));
        self.active = Some(household_id);

        record
    }

    /// The household an operation identity already produced, cloned for the caller to return.
    ///
    /// A held outcome resolves against the current book; a released one answers from the tombstone.
    /// Neither path mutates the book, which is what keeps a replay from changing the held set or the
    /// active household.
    fn recorded_for(&self, request_id: &RequestId) -> Option<HouseholdRecord> {
        match self.operations.get(request_id)? {
            RecordedOutcome::Held(household_id) => self.households.get(household_id).cloned(),
            RecordedOutcome::Released(record) => Some(record.clone()),
        }
    }

    /// Every household the person holds, in acquisition order, as list rows (HS-3, HS-4).
    ///
    /// The order is deterministic — the order [`Self::record_created`] recorded the households, not
    /// `HashMap` iteration — so a screen does not reshuffle between reads and the re-election rule
    /// is reproducible. `current_account` decides `is_owner`; `None` reports every row as not owned
    /// rather than guessing (see [`HouseholdRecord::to_wire_for`]).
    pub fn summaries(&self, current_account: Option<&AccountUserId>) -> Vec<HouseholdSummary> {
        self.held_order
            .iter()
            .filter_map(|household_id| self.households.get(household_id))
            .map(|record| record.to_summary(current_account, self.is_active(record.id())))
            .collect()
    }

    /// Make a held household the active one and return its record (HS-3, HS-4).
    ///
    /// The returned record is the post-state the caller presents: reaching `Ok` is what makes it the
    /// active household, and it is read from the book rather than echoed from the argument, so the
    /// two cannot disagree.
    ///
    /// # Errors
    ///
    /// [`ActiveHouseholdRefusal::NotHeld`] when the person does not hold the household. Nothing
    /// changes in that case: the previous active household stays active, because a refused switch
    /// must not leave the person with none (DM-3).
    pub fn set_active(
        &mut self,
        household_id: &HouseholdId,
    ) -> Result<&HouseholdRecord, ActiveHouseholdRefusal> {
        let Some(record) = self.households.get(household_id) else {
            return Err(ActiveHouseholdRefusal::NotHeld(household_id.clone()));
        };

        self.active = Some(household_id.clone());

        Ok(record)
    }

    /// Drop a household from the held set and re-elect the active one when needed (DM-3).
    ///
    /// Returns the released record, or `None` when the person did not hold the household — in which
    /// case nothing changes. If the released household was the active one, the first household still
    /// held, in acquisition order, becomes active; when none remains, no household is active. DM-3
    /// fixes only that one of the remaining households becomes active, and acquisition order is the
    /// reproducible choice for it.
    ///
    /// P0 removes no household through a command — deletion is P3's — so this is the set-level
    /// operation a later refresh uses to reconcile the held set, and the one DM-3's re-election is
    /// defined on. The create operation that produced the household is **tombstoned**: its ledger
    /// entry becomes [`RecordedOutcome::Released`], so replaying that create returns the released
    /// record without re-adding it or undoing the re-election (D0.14). A replay is still not a new
    /// operation, and a genuinely new create is a new operation with its own `RequestId`.
    pub fn release(&mut self, household_id: &HouseholdId) -> Option<HouseholdRecord> {
        let released = self.households.remove(household_id)?;

        self.held_order.retain(|held| held != household_id);
        let tombstoned = RecordedOutcome::Released(released.clone());
        for outcome in self.operations.values_mut() {
            if matches!(outcome, RecordedOutcome::Held(held) if *held == *household_id) {
                *outcome = tombstoned.clone();
            }
        }
        self.reelect_active();

        Some(released)
    }

    /// Elect a held household when the active one is absent or no longer held (DM-3).
    ///
    /// This is the single place an active household is chosen without a person asking for one, so
    /// "at most one active" and "a held household becomes active when the previous one leaves" are
    /// properties of one assignment rather than of every caller.
    fn reelect_active(&mut self) {
        let active_is_held = self
            .active
            .as_ref()
            .is_some_and(|household_id| self.households.contains_key(household_id));

        if active_is_held {
            return;
        }

        self.active = self
            .held_order
            .iter()
            .find(|household_id| self.households.contains_key(*household_id))
            .cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_id(value: &str) -> RequestId {
        RequestId(value.to_owned())
    }

    fn account(value: &str) -> AccountUserId {
        AccountUserId(value.to_owned())
    }

    fn household(value: &str) -> HouseholdId {
        HouseholdId(value.to_owned())
    }

    fn member(value: &str) -> MemberId {
        MemberId(value.to_owned())
    }

    fn at(value: &str) -> Instant {
        Instant(value.to_owned())
    }

    fn record(id: &str, owner_member: &str, owner_account: &str) -> HouseholdRecord {
        HouseholdRecord::created(
            household(id),
            HouseholdName::parse("Home").expect("a non-blank name"),
            at("2026-10-19T10:00:00Z"),
            member(owner_member),
            account(owner_account),
        )
    }

    #[test]
    fn a_blank_name_is_refused_and_a_non_blank_one_is_kept_as_supplied() {
        for blank in ["", " ", "   ", "\t", "\n", " \t\n "] {
            assert_eq!(
                HouseholdName::parse(blank),
                None,
                "{blank:?} is whitespace-only and therefore blank"
            );
        }

        let padded = HouseholdName::parse("  Home  ").expect("a non-blank name");
        assert_eq!(padded.as_str(), "  Home  ");
    }

    #[test]
    fn a_blank_name_refusal_is_the_contracts_validation_cause_and_is_not_retryable() {
        let request_id = request_id("operation-1");

        for blank in ["", " \t\n"] {
            let error = validate_name(blank, &request_id).expect_err("a blank name is refused");

            assert_eq!(error.cause, crate::ipc::types::SaveCause::Validation);
            assert_eq!(error.message_key, "error.validation");
            assert!(!error.retryable);
            assert_eq!(error.request_id, Some(request_id.clone()));
        }
    }

    #[test]
    fn the_creator_is_the_first_member_and_the_owner() {
        let record = record("household-1", "member-1", "account-1");

        assert_eq!(record.members().len(), 1);
        let owner = record.owner().expect("the creator is recorded");
        assert_eq!(record.members()[0], *owner);
        assert_eq!(owner.id, member("member-1"));
        assert_eq!(owner.account_user_id, account("account-1"));
        assert_eq!(record.owner_member_id(), &member("member-1"));
        assert!(record.is_owned_by(&account("account-1")));
        assert!(!record.is_owned_by(&account("account-2")));

        let wire = record.to_wire(&account("account-1"), true);
        assert_eq!(wire.id, household("household-1"));
        assert_eq!(wire.name, "Home");
        assert_eq!(wire.owner_member_id, member("member-1"));
        assert!(wire.is_active);
        assert!(wire.is_owner);
    }

    #[test]
    fn creating_a_second_household_leaves_exactly_one_active() {
        let mut book = HouseholdBook::new();

        assert!(book.active().is_none());
        assert!(book.is_empty());

        book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );
        assert_eq!(
            book.active().expect("the first create activates").id(),
            &household("household-1")
        );

        book.record_created(
            request_id("operation-2"),
            record("household-2", "member-2", "account-1"),
        );

        assert_eq!(book.len(), 2);
        assert_eq!(
            book.active().expect("the second create activates").id(),
            &household("household-2")
        );
        assert!(!book.is_active(&household("household-1")));
        assert!(book.is_active(&household("household-2")));

        let first = book.get(&household("household-1")).expect("held");
        let first_wire = first.to_wire(&account("account-1"), false);
        assert!(!first_wire.is_active);
    }

    #[test]
    fn several_households_keep_their_own_members_and_data() {
        let mut book = HouseholdBook::new();

        book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );
        book.record_created(
            request_id("operation-2"),
            record("household-2", "member-2", "account-2"),
        );

        let first = book.get(&household("household-1")).expect("held");
        let second = book.get(&household("household-2")).expect("held");

        // Each record describes only its own household: one member each, with its own owner, and no
        // member of one appears as a member of the other (HS-3).
        assert_eq!(first.members().len(), 1);
        assert_eq!(second.members().len(), 1);
        assert_eq!(first.members()[0].id, member("member-1"));
        assert_eq!(second.members()[0].id, member("member-2"));
        assert_eq!(first.owner_member_id(), &member("member-1"));
        assert_eq!(second.owner_member_id(), &member("member-2"));
        assert!(first.is_owned_by(&account("account-1")));
        assert!(!first.is_owned_by(&account("account-2")));
        assert!(second.is_owned_by(&account("account-2")));
        assert!(!second.is_owned_by(&account("account-1")));
        assert_ne!(first.members()[0], second.members()[0]);

        // The list reports one row per held household and never merges them.
        let rows = book.summaries(Some(&account("account-1")));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, household("household-1"));
        assert_eq!(rows[1].id, household("household-2"));
        assert_eq!(rows[0].member_count, 1);
        assert_eq!(rows[1].member_count, 1);
        assert!(rows[0].is_owner);
        assert!(!rows[1].is_owner);
    }

    #[test]
    fn the_household_list_marks_exactly_one_active_row_in_acquisition_order() {
        let mut book = HouseholdBook::new();

        for (operation, id, owner) in [
            ("operation-1", "household-1", "member-1"),
            ("operation-2", "household-2", "member-2"),
        ] {
            book.record_created(request_id(operation), record(id, owner, "account-1"));
        }

        let rows = book.summaries(Some(&account("account-1")));

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter().filter(|row| row.is_active).count(),
            1,
            "HS-3 and DM-3 allow exactly one active household at a time"
        );
        assert!(!rows[0].is_active);
        assert!(rows[1].is_active);

        // Without a resolved account no row claims to be owned by the current person.
        let unresolved = book.summaries(None);
        assert!(unresolved.iter().all(|row| !row.is_owner));
    }

    #[test]
    fn switching_the_active_household_leaves_exactly_one_active() {
        let mut book = HouseholdBook::new();

        for (operation, id, owner) in [
            ("operation-1", "household-1", "member-1"),
            ("operation-2", "household-2", "member-2"),
        ] {
            book.record_created(request_id(operation), record(id, owner, "account-1"));
        }

        let switched = book
            .set_active(&household("household-1"))
            .expect("the person holds the household");

        assert_eq!(switched.id(), &household("household-1"));
        assert!(book.is_active(&household("household-1")));
        assert!(!book.is_active(&household("household-2")));
        assert_eq!(
            book.summaries(None)
                .iter()
                .filter(|row| row.is_active)
                .count(),
            1
        );
    }

    #[test]
    fn a_household_that_is_not_held_cannot_be_made_active() {
        let mut book = HouseholdBook::new();
        book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );

        let refusal = book
            .set_active(&household("household-2"))
            .expect_err("the person does not hold household-2");

        assert_eq!(
            refusal,
            ActiveHouseholdRefusal::NotHeld(household("household-2"))
        );
        // A refused switch changes nothing: the previous active household is still active and the
        // refused id was not added to the held set.
        assert!(book.is_active(&household("household-1")));
        assert!(book.get(&household("household-2")).is_none());
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn releasing_the_active_household_elects_the_first_still_held_one() {
        let mut book = HouseholdBook::new();

        for (operation, id, owner) in [
            ("operation-1", "household-1", "member-1"),
            ("operation-2", "household-2", "member-2"),
            ("operation-3", "household-3", "member-3"),
        ] {
            book.record_created(request_id(operation), record(id, owner, "account-1"));
        }
        assert!(book.is_active(&household("household-3")));

        let released = book
            .release(&household("household-3"))
            .expect("the person holds household-3");

        assert_eq!(released.id(), &household("household-3"));
        assert_eq!(book.len(), 2);
        assert!(book.get(&household("household-3")).is_none());
        let elected = book
            .active()
            .expect("one of the remaining households is elected");
        assert_eq!(
            elected.id(),
            &household("household-1"),
            "the household held longest is chosen, reproducibly"
        );
        assert!(!book.is_active(&household("household-2")));

        // Releasing a household that was not active leaves the active one alone.
        book.release(&household("household-2"));
        assert!(book.is_active(&household("household-1")));

        // Releasing an id the person does not hold is a no-op.
        assert!(book.release(&household("household-2")).is_none());
        assert!(book.is_active(&household("household-1")));
    }

    #[test]
    fn a_create_replayed_after_a_release_neither_re_adds_nor_reactivates_the_household() {
        let mut book = HouseholdBook::new();
        let second_operation = request_id("operation-2");

        book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );
        let second = book.record_created(
            second_operation.clone(),
            record("household-2", "member-2", "account-1"),
        );
        assert!(book.is_active(&household("household-2")));

        book.release(&household("household-2"))
            .expect("the person holds household-2");
        let elected = book.active().expect("household-1 is re-elected");
        assert_eq!(elected.id(), &household("household-1"));

        // The operation is tombstoned, not forgotten: the replay answers with the record the service
        // stored and changes neither the held set nor the active household.
        let replay = book.record_created(
            second_operation,
            record("household-2", "member-2", "account-1"),
        );

        assert_eq!(replay, second);
        assert_eq!(book.len(), 1, "the released household is not re-added");
        assert!(book.get(&household("household-2")).is_none());
        assert_eq!(book.summaries(None).len(), 1);
        assert_eq!(
            book.active().expect("the re-election is not undone").id(),
            &household("household-1")
        );
    }

    #[test]
    fn releasing_the_last_household_leaves_no_active_one() {
        let mut book = HouseholdBook::new();
        book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );

        book.release(&household("household-1"))
            .expect("the person holds household-1");

        assert!(book.is_empty());
        assert!(book.active().is_none());
        assert!(book.summaries(None).is_empty());
    }

    #[test]
    fn a_replayed_operation_returns_the_household_already_recorded_and_does_not_reactivate_it() {
        let mut book = HouseholdBook::new();

        let first = book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );
        let second = book.record_created(
            request_id("operation-2"),
            record("household-2", "member-2", "account-1"),
        );

        // The first household is no longer active; replaying its create must not steal activation
        // back from the second one, because a replay is not a new creation.
        let replay = book.record_created(
            request_id("operation-1"),
            record("household-1", "member-1", "account-1"),
        );

        assert_eq!(replay, first);
        assert_eq!(book.len(), 2);
        assert_eq!(
            book.active()
                .expect("the active household is unchanged")
                .id(),
            second.id()
        );
        assert!(book.is_active(&household("household-2")));
    }

    #[test]
    fn a_replay_with_a_different_service_answer_keeps_the_first_record() {
        let mut book = HouseholdBook::new();
        let operation = request_id("operation-1");

        let first = book.record_created(
            operation.clone(),
            record("household-1", "member-1", "account-1"),
        );

        // A service answer that named a different household for the same operation would be a
        // contract violation; the book still refuses to grow a second record for one operation.
        let replay = book.record_created(operation, record("household-2", "member-2", "account-2"));

        assert_eq!(replay, first);
        assert_eq!(book.len(), 1);
        assert!(book.get(&household("household-2")).is_none());
    }
}
