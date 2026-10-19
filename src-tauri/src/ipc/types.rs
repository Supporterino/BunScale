//! The frozen shared types of the P0 IPC contract (task 1.3).
//!
//! Every type here crosses the Rust/frontend boundary, so the single builder in [`super::builder`]
//! exports it into `src/bindings.ts`: the generated file, not this module, is what the frontend
//! compiles against. The names and shapes are frozen for this phase — later phases reuse them
//! unchanged (`CommandError`, `SaveCause`, `Instant`, `WeightUnit`, `HouseholdId`, `MemberId`) — so
//! changing one is a reviewed contract change (a deliberate bindings diff under D0.18), never an
//! implementation detail.
//!
//! Nothing here holds behaviour. The rules the types describe — canonical storage, absolute
//! instants, reachability gating, cause-specific failure — are implemented by section 2 behind the
//! stubs in [`super::commands`], and the frontend reaches them only through
//! `src/features/*/api.ts` (project hard rule 2).

use serde::{Deserialize, Serialize};
use specta::Type;

/// The identity of one client-originated operation (D0.14).
///
/// The frontend generates a fresh value for every state-changing command and re-uses the *same*
/// value when it retries that command. That reuse is what makes the retry the failure surface
/// offers safe: the shared service keys its request ledger on `(account_user_id, request_id)` and
/// returns the stored outcome for a re-issued call instead of performing the change twice (D0.7
/// criterion 6).
///
/// **This is a semantic value, not a diagnostic field.** It names the operation the service must not
/// duplicate; it is not a trace id and not a correlation id for logs, and it must never be
/// regenerated to retry an operation. `src/features/*/api.ts` owns that discipline (task 3.1).
///
/// The contract fixes no format: the value is opaque to the service and to the UI, and only its
/// uniqueness per operation and its stability across a retry matter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct RequestId(pub String);

/// A household's opaque, stable identifier (D0.7 criterion 1).
///
/// It addresses one logical collection, shared verbatim by every member of that household (SYNC-1).
/// The value is opaque: it is never derived from data the frontend can compute, and nothing may be
/// parsed out of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct HouseholdId(pub String);

/// One member's opaque, stable identifier within a household (D0.3, SET-2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct MemberId(pub String);

/// The account's stable identifier: the account service's subject claim (`sub`, D0.7 follow-up 2).
///
/// It is never the local member id: one account may be a member of several households, and member
/// ids are per household.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct AccountUserId(pub String);

/// The opaque identifier of one item inside a household's collection (SYNC-1).
///
/// It is stable across members and across devices, and unique within its household: the store keys
/// on `(household_id, item_id)`, which is how "exactly one copy of every item" is an invariant
/// rather than a client-side check (D0.7 criterion 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct ItemId(pub String);

/// An absolute instant on the wire: RFC 3339 with an offset (D0.5, DM-6).
///
/// Timestamps are never local-time strings and never bare offsets. The storage boundary normalises
/// the value to UTC on store, a supplied instant is stored unchanged — including one in the past —
/// and a value round-trips without being re-interpreted against the device's timezone. The newtype
/// is transparent, so the generated frontend type is a plain `string`; only the storage boundary
/// interprets its content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct Instant(pub String);

/// The per-device display unit for weights (SET-4).
///
/// It is a presentation choice only. Stored values stay in the canonical unit — grams (D0.4) — and
/// conversion happens at the entry/display boundary, so changing this value never changes a stored
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WeightUnit {
    /// Gram (the canonical stored unit).
    Grams,
    /// Kilogram (a display unit only).
    Kilograms,
}

/// The single cause taxonomy of the one failure surface (D0.6, NFR-8).
///
/// The list is frozen and exhaustive: every failure reports exactly one of these causes, no failure
/// may be reported without one, and P0 adds no variant. Each cause has its own copy (task 3.4 maps
/// each to its message key, task 4.5 renders it), so no failure can degrade into a generic
/// "something went wrong". The variants are the wire tokens the service's own reports map onto
/// (D0.7 criterion 4):
///
/// - [`SaveCause::Connectivity`] — no HTTP response at all: DNS, TLS, connect or read failure.
/// - [`SaveCause::NoAccount`] — the service answered `401`: the account is missing or invalid.
/// - [`SaveCause::AllowanceExceeded`] — the service reported the household owner's exhausted
///   storage allowance as its own condition, never as a generic write error (D0.12).
/// - [`SaveCause::RateLimited`] — the gateway answered `429` with `Retry-After`.
/// - [`SaveCause::Timeout`] — no outcome arrived within the operation's bound: the
///   `SAVE_ATTEMPT_TIMEOUT` of 15 s for a save attempt, the `REACHABILITY_PROBE_TIMEOUT` of 5 s for
///   a reachability probe (D0.13), which is why `ServiceStatus.cause` can be `timeout` too.
/// - [`SaveCause::Validation`] — the input was refused: a blank household name in P0 (HS-1),
///   RAB-2's and WT-4's refusals in P1.
/// - [`SaveCause::Unknown`] — the failure could not be classified. An honest last resort, never a
///   fallback that hides a cause the product already knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SaveCause {
    /// No response from the shared service.
    Connectivity,
    /// The account is missing or invalid.
    NoAccount,
    /// The household owner's storage allowance is exhausted.
    AllowanceExceeded,
    /// The service is temporarily rate limiting.
    RateLimited,
    /// No outcome arrived inside the save-attempt bound.
    Timeout,
    /// The input was refused as invalid.
    Validation,
    /// The failure could not be classified.
    Unknown,
}

/// The one typed failure surface every command reports through (D0.6, NFR-8).
///
/// There is deliberately no free-form `detail` field: a message the product did not author is the
/// bare "something went wrong" NFR-8 forbids, and paths, tokens and content must never cross this
/// boundary (NFR-3). The UI renders `message_key`'s copy, which is cause-specific by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CommandError {
    /// What went wrong (see [`SaveCause`]).
    pub cause: SaveCause,
    /// Whether re-issuing this same command — with the same
    /// [`request_id`](CommandError::request_id) — can succeed, which is the signal the frontend uses
    /// to decide whether to offer a retry at all. `true` for the conditions a later attempt can
    /// clear: the four the service reports — `connectivity`, `no_account`, `allowance_exceeded` and
    /// `rate_limited` (SYNC-2 offers the retry for all four) — and `timeout`, whose re-issue the
    /// shared service deduplicates (D0.14). `false` for `validation`, whose refusal is a property of
    /// the input rather than of the attempt, and for `unknown`, about which no outcome can be
    /// promised.
    pub retryable: bool,
    /// The i18n key of the cause-specific copy. It is always non-empty, and every `SaveCause` maps
    /// to a distinct key (task 5.10); the scaffold that resolves keys is task 3.7's.
    pub message_key: String,
    /// The failed operation's identity — **semantic, not diagnostic** (D0.14).
    ///
    /// It is the `RequestId` the command was given. A retry re-issues it so the shared service
    /// deduplicates the call rather than performing the change twice: that is what makes the retry
    /// the surface offers safe. It is never a trace id and never a value to parse.
    ///
    /// `Some` for every command that takes a `RequestId` — every state-changing command.
    /// `None` only for the failure of a read or of the reachability probe, which carry no operation
    /// identity to deduplicate and are idempotent, so their retry is a fresh call rather than a
    /// re-issued one.
    pub request_id: Option<RequestId>,
}

/// How reachable the shared service and the account were at the last observation (NFR-1).
///
/// [`ServiceState::Unknown`] is the state before any fresh outcome exists: a status is never carried
/// over from a previous session (R3). [`ServiceState::Unreachable`] is reported only after
/// `OFFLINE_FAILURE_THRESHOLD` = 3 consecutive failed probes without an intervening success
/// (D0.13). P3 extends the set with `syncing`; P0's set is exactly these three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    /// The service answered and the account check produced its outcome.
    Reachable,
    /// The failure threshold was reached without an intervening success.
    Unreachable,
    /// Nothing has been observed yet.
    Unknown,
}

/// The last observed service/account status (NFR-1, SYNC-2).
///
/// It is also the payload of the `service:status-changed` event; the event name is registered in
/// [`super::events`] so it cannot drift from this type (P3 extends this struct with the
/// `consecutive_failures` count and the `syncing` state).
///
/// The observation is allowed to be absent rather than defaulted: "we have not looked" and "we
/// looked and it was fine" are different facts, and only the second may drive the status indicator
/// (R3). That is why [`ServiceStatus::account_valid`] and [`ServiceStatus::observed_at`] are
/// optional — a default of `false`/"now" would be exactly the cached guess R3 forbids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ServiceStatus {
    /// The reachability outcome. `unknown` until a fresh probe has completed.
    pub state: ServiceState,
    /// Whether the account was valid in that fresh outcome (D0.7 criterion 4). `None` whenever no
    /// account outcome exists — in particular before anything has been observed, or when the probe
    /// could not reach the service to ask.
    pub account_valid: Option<bool>,
    /// The specific condition behind `state`/`account_valid`, when it was not the plain "reachable
    /// and identified" outcome: `connectivity`, `no_account`, `rate_limited` or `timeout`. `None`
    /// for a plain success and for `unknown`.
    pub cause: Option<SaveCause>,
    /// When the observation was made. `None` until a fresh outcome exists.
    pub observed_at: Option<Instant>,
}

/// One household as the contract presents it (HS-1, HS-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Household {
    /// The opaque, stable identifier that also addresses the household's collection.
    pub id: HouseholdId,
    /// The name the creator supplied. Non-blank by contract; the domain validates it (task 2.1),
    /// not the wire type.
    pub name: String,
    /// When the household was created, as an absolute instant (D0.5).
    pub created_at: Instant,
    /// The member that created the household, recorded as its owner (D0.3). Ownership gates P3's
    /// destructive deletion only; P0 defines the field so no retrofit is needed later.
    pub owner_member_id: MemberId,
    /// Whether this is the person's active household. At most one household is active per person
    /// (DM-3), or none when the person holds none.
    pub is_active: bool,
    /// Whether the *current person* is the household's owner, derived from `owner_member_id` and
    /// the current identity. P3 gates its delete control on it.
    pub is_owner: bool,
}

/// One row of the household list (HS-4): every household the person holds, with the active one
/// indicated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HouseholdSummary {
    /// The household's opaque identifier.
    pub id: HouseholdId,
    /// The household's name.
    pub name: String,
    /// When the household was created, as an absolute instant (D0.5).
    pub created_at: Instant,
    /// Whether this row is the active household.
    pub is_active: bool,
    /// Whether the current person is the household's owner.
    pub is_owner: bool,
    /// How many members the household has. At least 1, because the creator is recorded as its first
    /// member (HS-1); member management that grows it is P3's.
    pub member_count: u32,
}

/// One member of a household (D0.3, SET-2).
///
/// P0 records the creator as the first member and freezes the shape; P3's member list is the first
/// command to return it, which is why [`super::builder`] exports it explicitly. `display_name` is
/// optional because a name may not resolve — an absent name never blocks a screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Member {
    /// The member's opaque identifier within its household.
    pub id: MemberId,
    /// The account behind the member.
    pub account_user_id: AccountUserId,
    /// The member's display name, when one is known.
    pub display_name: Option<String>,
}

/// The current person's identity as Settings & Profile shows it (SET-2).
///
/// The fallback chain is SET-2's, and its last tier is not produced here: a localized "You"-style
/// placeholder is frontend copy (task 3.7), so this type reports "no name known" instead of
/// inventing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Identity {
    /// The account id (`sub`), or `None` while the account cannot be resolved.
    pub account_user_id: Option<AccountUserId>,
    /// The name to show: the account's display name when the identity resolves, otherwise the
    /// locally known member name, otherwise `None` — and then the frontend renders its placeholder.
    pub display_name: Option<String>,
    /// Whether the *account* identity resolved (the chain's first tier), i.e. whether
    /// `display_name` came from the account rather than from the local member record.
    ///
    /// `false` with `display_name: Some(..)` is the fallback tier; `false` with `None` is
    /// unresolved. An unresolved identity never fails the command: it must not disturb the screen.
    pub resolved: bool,
}

/// Where a household-collection read resolved its items (D0.2, SYNC-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CollectionSource {
    /// The refresh that accompanied the read returned them from the shared service.
    Service,
    /// They came from the local copy, because the service was unreachable or no refresh was
    /// attempted. The copy is a cache of the collection, never a competing record.
    Local,
}

/// One item's identity and version inside a household's collection (SYNC-1, SYNC-5).
///
/// P0 fixes the collection layer, not a domain schema: an item's *content* arrives with the first
/// domain type (P1's rabbits and weight entries), and adding a field here is an additive contract
/// change. What P0 exposes is the part every member must resolve identically — one opaque id, one
/// version marker, and whether the row is a tombstone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CollectionItem {
    /// The item's opaque, stable identifier within its household.
    pub item_id: ItemId,
    /// The item's version marker: the client-supplied `updated_at` that last-write-wins compares
    /// (D0.7 criterion 3). It is distinct from any user-supplied domain timestamp, which is stored
    /// unchanged beside it (D0.5).
    pub updated_at: Instant,
    /// Whether this row is a tombstone — the item was removed and the removal is reported to every
    /// member instead of the row silently vanishing (a removal must appear in the change feed).
    pub deleted: bool,
}

/// One household's collection as a read resolved it (SYNC-1, SYNC-3).
///
/// The three SYNC-3 states are distinguishable here rather than collapsed into an empty list:
/// refreshed data is `source: service`; previously loaded data served while unreachable is
/// `source: local` with `loaded_at` set; and nothing ever loaded while unreachable is
/// `source: local` with `loaded_at: None` — the explicit connectivity-required state, never an
/// empty list presented as the household's current data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HouseholdCollection {
    /// The household the collection belongs to.
    pub household_id: HouseholdId,
    /// The collection after the read, each item exactly once (SYNC-1).
    pub items: Vec<CollectionItem>,
    /// Where [`HouseholdCollection::items`] came from.
    pub source: CollectionSource,
    /// When this device last loaded the collection from the service; `None` when it never has.
    pub loaded_at: Option<Instant>,
}

/// A redacted summary of the diagnostics sink (NFR-3, task 2.12).
///
/// Identifiers, counts and outcomes are what the sink may record; no rabbit name, note text, photo
/// content or weight value may appear here or in the records it summarises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DiagnosticsSummary {
    /// How many records the sink holds.
    pub record_count: u32,
    /// When the oldest held record was recorded; `None` when the sink is empty.
    pub oldest_at: Option<Instant>,
    /// When the newest held record was recorded; `None` when the sink is empty.
    pub newest_at: Option<Instant>,
}
