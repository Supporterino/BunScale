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
//! instead of adding a second one. That is what makes a replay safe even if the answer is lost
//! between the service and this process, and it is why no code path can put two households under
//! one operation identity.
//!
//! **The wire type is produced here, not re-declared.** [`HouseholdRecord::to_wire`] fills the
//! frozen [`Household`] shape, including the two derived flags: `is_active` comes from the book's
//! single active id, and `is_owner` comes from the owner member's account compared with the current
//! account. No domain type duplicates the contract's fields.

use std::collections::HashMap;

use crate::ipc::types::{
    AccountUserId, CommandError, Household, HouseholdId, Instant, Member, MemberId, RequestId,
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
        Household {
            id: self.id.clone(),
            name: self.name.as_str().to_owned(),
            created_at: self.created_at.clone(),
            owner_member_id: self.owner_member_id.clone(),
            is_active,
            is_owner: self.is_owned_by(current_account),
        }
    }
}

/// The person's households and their one active household, keyed by identity (HS-3, HS-4, DM-3).
///
/// The type exists so the two set-level invariants have a home: a household id addresses exactly one
/// record, and at most one record is active. It is the domain counterpart of the service's
/// household set; the service remains the source of truth, and this is the process's view of it.
///
/// **One active id, not a flag per record.** A record cannot be "active" in isolation, so there is
/// no state in which the type holds two active households, and switching the active household later
/// (task 2.2) is a single assignment rather than a loop that must clear every sibling. `None` is the
/// honest state of a person holding no household, never an empty list pretending to be one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdBook {
    /// One record per household id; re-recording an id replaces its one record.
    households: HashMap<HouseholdId, HouseholdRecord>,
    /// The id of the active household, or `None` when none is active.
    active: Option<HouseholdId>,
    /// The operation ledger: which household a `RequestId` already produced (D0.14).
    operations: HashMap<RequestId, HouseholdId>,
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
    /// A **first** outcome stores the record, remembers which household `request_id` produced, and
    /// makes that household the active one (HS-1). A **replay** — the same `request_id` recorded
    /// before — returns the stored household and changes nothing, in particular not the active
    /// household: a retried create is the same creation, not a new activation (D0.14).
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
        self.households.insert(household_id.clone(), record.clone());
        self.operations.insert(request_id, household_id.clone());
        self.active = Some(household_id);

        record
    }

    /// The household an operation identity already produced, cloned for the caller to return.
    fn recorded_for(&self, request_id: &RequestId) -> Option<HouseholdRecord> {
        let household_id = self.operations.get(request_id)?;

        self.households.get(household_id).cloned()
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
