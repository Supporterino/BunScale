//! The frozen shared types of the IPC contract: P0's set (task 1.3) and P1's rabbit and weight types
//! (task 1.1).
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

/// One item's content, in the envelope the write that saves it submits (RAB-6, RAB-9, WT-5, DM-4;
/// D1.16).
///
/// It is **one** shape shared by the item write and the collection row, so the version this device
/// holds and the version it writes cannot drift into two representations of one value. The variant
/// names and the member names (`kind`, `rabbit`, `weight_entry`) are frozen: they are exactly what
/// `domain::rabbit::save_body` already sends for a rabbit item, and every item kind added later
/// adds a variant rather than re-shaping the existing two.
///
/// It derives `PartialEq` but not `Eq`, because [`Rabbit::weight_drop_threshold_percent`] is a
/// floating-point percentage (D1.16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemPayload {
    /// A rabbit profile (RAB-1).
    Rabbit {
        /// The rabbit as its write body carries it.
        rabbit: Rabbit,
    },
    /// One weight entry (WT-1).
    WeightEntry {
        /// The entry as its write body carries it.
        weight_entry: WeightEntry,
    },
}

impl ItemPayload {
    /// The `kind` tokens this version models, one per variant.
    ///
    /// The feed uses this to tell a **foreign** payload — a kind this version does not model, whose
    /// row is dropped — from a **known** kind whose content did not decode, which is stored as
    /// unknown content (D1.16). A new variant adds its token here beside the variant itself, so the
    /// two can never disagree about what this version models.
    pub const KINDS: &'static [&'static str] = &["rabbit", "weight_entry"];
}

/// One item's identity, version, tombstone flag and content inside a household's collection
/// (SYNC-1, SYNC-5; D1.16).
///
/// The first three fields are the part every member must resolve identically — one opaque id, one
/// version marker, and whether the row is a tombstone. [`CollectionItem::payload`] is the item's
/// content, carried with the row so a read serves it without a second call (D1.16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
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
    /// The item's content, or `None` when this device has not read it (D1.16).
    ///
    /// `None` is **"content unknown"**, never "no content" and never "archived". A live row with no
    /// payload is a row this device learnt of without ever reading the item; an archived row is the
    /// opposite — a **present** payload carrying `is_archived: true`. The write path depends on the
    /// distinction: a whole-item write replaces the stored version, so a row whose content is
    /// unknown must be read before it is written, while an archived row holds everything restore
    /// needs.
    ///
    /// The field is `#[serde(default)]` so a leaner feed or a file written before this field
    /// existed reads as unknown content rather than as an unreadable answer (SYNC-3). It is always
    /// present on the wire, as `null`, so "unknown" is a visible value rather than an omission.
    #[serde(default)]
    pub payload: Option<ItemPayload>,
}

/// One household's collection as a read resolved it (SYNC-1, SYNC-3).
///
/// The three SYNC-3 states are distinguishable here rather than collapsed into an empty list:
/// refreshed data is `source: service`; previously loaded data served while unreachable is
/// `source: local` with `loaded_at` set; and nothing ever loaded while unreachable is
/// `source: local` with `loaded_at: None` — the explicit connectivity-required state, never an
/// empty list presented as the household's current data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
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

// ---------------------------------------------------------------------------------------------
// P1 — rabbits and weight entries (task 1.1, RAB-1 … RAB-10, WT-1 … WT-8, DASH-1 … DASH-9, DM-1)
// ---------------------------------------------------------------------------------------------
//
// Frozen by task 1.1 for `add-rabbit-weight-core-loop`. Everything here follows the P0 conventions
// above: opaque id newtypes are transparent, enums are `snake_case` on the wire, struct fields carry
// no `rename_all` (exactly like [`Household`] and [`HouseholdSummary`]), and no type carries a
// per-entry unit — grams are canonical (D1.2, DM-1). The three shapes that are **inputs** rather
// than read models ([`RabbitProfile`], [`WeightEntryDraft`]) exist because a command with three or
// more parameters takes one typed argument struct (`tauri-ipc-contract`); they are the wire form of
// what a form submits, and neither has a unit field either.

/// A rabbit's opaque, stable identifier within its household collection (RAB-1, D0.7 criterion 1).
///
/// Same style as [`HouseholdId`]: the value is opaque, stable across members and devices, and unique
/// within the household's collection. Nothing may be parsed out of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct RabbitId(pub String);

/// A weight entry's opaque, stable identifier within its household collection (WT-1, SYNC-1).
///
/// It is its own newtype rather than a shared "item id": a weight entry is addressed by the weight
/// commands and a rabbit by the rabbit commands, and the two must not be interchangeable at a call
/// site.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct WeightEntryId(pub String);

/// A canonical weight in grams (D1.2, DM-1).
///
/// It is the only weight representation the contract carries: a stored value, a submitted value and a
/// value on a chart point are all this type, and none of them has a unit field — the display unit
/// exists only in `src/features/weight/api.ts` at the boundary (see the design's IPC section, I1).
///
/// **Why a newtype over `i32`.** `tauri-specta` refuses to export 64-bit integers (`i64`, `u64`,
/// `usize`, …) because JavaScript numbers cannot hold them without precision loss, and
/// `bun run gen:bindings` fails on one. `i32` is JS-safe: every value it holds is exact as a `number`,
/// and grams over its range (over two million kilograms) are far beyond any weight this product will
/// ever record — the plausibility bound is 10 kg (WT-4, D1.9). The alternative `Instant` uses —
/// a `String` — would make arithmetic on a weight impossible without parsing it, and it is only right
/// for an instant because a timestamp is never computed on at the boundary. The newtype also keeps a
/// weight from being passed where a percentage or a count is expected, which a bare `i32` would not.
///
/// It is transparent, so the generated binding is `type Grams = number` and every `value_grams` field
/// is typed `Grams`: the JSON payload is a bare number, no name in the contract changes, and the unit
/// stays visible in the generated types.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(transparent)]
pub struct Grams(pub i32);

/// A rabbit's recorded sex (RAB-1, D1.14).
///
/// The set is closed and frozen with the contract: the creation and edit forms offer exactly these
/// three options, and [`Sex::Unknown`] is a recorded value, not an absent field — a person is not
/// forced to invent a value they do not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    /// Male.
    Male,
    /// Female.
    Female,
    /// Not stated, or not known.
    Unknown,
}

/// A rabbit's neuter/spay status (RAB-1, D1.14).
///
/// `neutered` and `spayed` stay distinct because the distinction is clinically meaningful; collapsing
/// them into one "fixed" value would lose it, and a free-text field would accept an invented status.
/// [`NeuterStatus::Unknown`] is recorded for the same reason [`Sex::Unknown`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NeuterStatus {
    /// Not neutered and not spayed.
    Intact,
    /// Neutered (a male rabbit).
    Neutered,
    /// Spayed (a female rabbit).
    Spayed,
    /// Not stated, or not known.
    Unknown,
}

/// One rabbit profile as the contract presents it (RAB-1 … RAB-5).
///
/// Every optional field is genuinely absent rather than defaulted: a breed, a photo reference and an
/// adoption date the person did not supply are `None`, and the frontend renders nothing for them.
/// [`Rabbit::dob`] with [`Rabbit::dob_unknown`] is the exception the requirement names — "unknown" is
/// a stated fact distinct from "absent", so it is recorded as a flag rather than inferred from `None`
/// (RAB-1's unknown-date-of-birth scenario).
///
/// There is deliberately **no baseline, target or healthy-range field** (D1.7, D5): re-introducing one
/// is a new change, not a deferred task.
///
/// It derives `PartialEq` but not `Eq`, because [`Rabbit::weight_drop_threshold_percent`] is a
/// floating-point percentage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Rabbit {
    /// The rabbit's opaque identifier.
    pub id: RabbitId,
    /// The household whose collection holds this rabbit (RAB-3).
    pub household_id: HouseholdId,
    /// The name the person supplied. Non-blank by contract; the domain validates it (RAB-2), not the
    /// wire type.
    pub name: String,
    /// A reference to the rabbit's photo, when one is recorded. Choosing one from the device's photo
    /// library is PHOTO-2 in P2 (D1.11); P1 only carries and renders the reference.
    pub photo_ref: Option<String>,
    /// The breed, when the person recorded one.
    pub breed: Option<String>,
    /// The date of birth, when one is known and not marked unknown.
    ///
    /// It is a **calendar date**, not a moment: it is pinned to a fixed offset (UTC) and never
    /// timezone-shifted, so the day a person entered is the day every member and every device shows
    /// (RAB-1; DM-6's absolute-instant rule is about timestamps, not dates).
    pub dob: Option<Instant>,
    /// Whether the person marked the date of birth as unknown (RAB-1). Distinct from `dob: None`
    /// meaning "not supplied".
    ///
    /// `true` requires `dob: None`; a command refuses the pair `true` with a date as `validation`
    /// rather than picking a winner.
    pub dob_unknown: bool,
    /// The recorded sex (D1.14).
    pub sex: Sex,
    /// The recorded neuter/spay status (D1.14).
    pub neuter_status: NeuterStatus,
    /// The adoption date, when the person recorded one.
    ///
    /// A **calendar date** like [`Rabbit::dob`]: pinned to a fixed offset (UTC), never timezone-shifted.
    pub adoption_date: Option<Instant>,
    /// Whether the rabbit is archived (RAB-6). Archived rabbits are hidden from the active list and
    /// their data is immutable at the write boundary (D1.5).
    pub is_archived: bool,
    /// The weight-drop threshold as a percentage of the previous entry (default 5, RAB-1/RAB-5,
    /// WT-7). The alert is derived from it on read and never stored (D1.3), so editing it re-derives
    /// every alert without rewriting an entry.
    pub weight_drop_threshold_percent: f64,
}

/// The editable field set of a rabbit profile, as the create and edit forms submit it (RAB-1, RAB-5).
///
/// It is the input counterpart of [`Rabbit`] — no id, no household, no archived state — and it is
/// used by **both** [`create_rabbit`](super::commands::rabbit::create_rabbit) and
/// [`update_rabbit`](super::commands::rabbit::update_rabbit).
///
/// **Update is a whole-profile replacement, not a partial patch.** A patch would have to distinguish
/// "field unchanged" from "field cleared" for every optional field, which the frozen wire style
/// carries no double-optional for; the edit form is pre-filled with the current profile anyway, so it
/// submits the full set. What the design's contract table calls `patch` is therefore this type: the
/// *command* is still `update_rabbit`, and every field of an active rabbit's profile — including the
/// threshold — is editable through it (RAB-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RabbitProfile {
    /// The name; a blank or whitespace-only value is refused (RAB-2).
    pub name: String,
    /// The photo reference, or `None` for none (D1.11).
    pub photo_ref: Option<String>,
    /// The breed, or `None`.
    pub breed: Option<String>,
    /// The date of birth, or `None` when not supplied or marked unknown.
    ///
    /// A **calendar date** pinned to a fixed offset (UTC), never timezone-shifted (RAB-1). `None` with
    /// `dob_unknown: true` means "unknown"; `Some` with `dob_unknown: true` is refused as
    /// `validation` rather than resolved by precedence.
    pub dob: Option<Instant>,
    /// Whether the date of birth is explicitly unknown (RAB-1).
    pub dob_unknown: bool,
    /// The recorded sex (D1.14). Always present: `unknown` is how "not stated" is recorded.
    pub sex: Sex,
    /// The recorded neuter/spay status (D1.14). Always present, `unknown` included.
    pub neuter_status: NeuterStatus,
    /// The adoption date, or `None`. A calendar date pinned to a fixed offset, like `dob`.
    pub adoption_date: Option<Instant>,
    /// The weight-drop threshold percentage (WT-7).
    pub weight_drop_threshold_percent: f64,
}

/// One rabbit as a list, row or picker presents it (DASH-1, WT-2).
///
/// It is the small shape the dashboard rows and the quick-entry picker need, so neither has to load
/// every profile field to render a name and a thumbnail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RabbitSummary {
    /// The rabbit's opaque identifier.
    pub id: RabbitId,
    /// The name to present.
    pub name: String,
    /// The photo to present as the thumbnail, or `None` — and then the frontend presents a
    /// placeholder (RAB-1).
    pub thumbnail_ref: Option<String>,
    /// Whether the rabbit is archived.
    pub is_archived: bool,
}

/// One row of the dashboard list (DASH-1, DASH-2, DASH-3, DASH-4).
///
/// Both derived values are computed on read and never stored (D1.3): [`DashboardRow::never_weighed`]
/// and [`DashboardRow::has_weight_drop_alert`]. `never_weighed` is redundant with
/// `last_weighed_at: None` on purpose — DASH-2 asks for an explicit indication, and the flag keeps the
/// frontend from re-deriving a requirement out of an absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DashboardRow {
    /// The rabbit this row presents.
    pub rabbit: RabbitSummary,
    /// When the rabbit was last weighed, or `None` when it never was.
    pub last_weighed_at: Option<Instant>,
    /// Whether the rabbit has no weight entry at all (DASH-2).
    pub never_weighed: bool,
    /// Whether any of the rabbit's entries carries an active weight-drop alert (DASH-4, WT-7). It is
    /// the same derivation the entry flag is, so the two can never disagree.
    pub has_weight_drop_alert: bool,
}

/// One weight entry as the detail view and the edit form present it (WT-1, WT-8, DM-1).
///
/// The value is grams and there is **no unit field** (D1.2, DM-1): a display unit is applied by
/// `src/features/weight/api.ts` at the boundary, so changing the unit cannot rewrite history.
///
/// [`WeightEntry::has_weight_drop_alert`] is the derived flag WT-8's detail shows (D1.13). It is not a
/// stored property of the entry — it is re-derived from the current entry set on every read, which is
/// why it sits on the response type rather than in the collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WeightEntry {
    /// The entry's opaque identifier.
    pub id: WeightEntryId,
    /// The rabbit the entry belongs to.
    pub rabbit_id: RabbitId,
    /// The canonical value in grams (D1.2). Never negative; zero and negative are refused (WT-4).
    pub value_grams: Grams,
    /// The entry's timestamp, absolute (DM-6). Defaults to the creation moment when the draft omits
    /// it, and is otherwise stored unchanged, including in the past (WT-1).
    pub timestamp: Instant,
    /// The entry's recorded creation instant (WT-7, D1.12).
    ///
    /// It is fixed once per create operation and never edited: [`WeightEntryDraft`] has no creation
    /// field, so no form can move it, and a correction preserves the stored value exactly as it
    /// preserves the identity. It is the tie key for WT-7's "the earlier-created entry is the previous
    /// entry", and it is an ordering key only — never displayed. It is **required, with no serde
    /// default**: an entry payload without it is content this version cannot describe (D1.16), because
    /// a default would invent a creation order.
    pub created_at: Instant,
    /// The full note, or `None`. Never truncated by the contract (WT-8).
    pub note: Option<String>,
    /// References to the entry's photos, in presentation order; empty when it has none (WT-8).
    pub photo_refs: Vec<String>,
    /// Whether the drop against the previous entry exceeds the rabbit's threshold (WT-7, D1.13).
    pub has_weight_drop_alert: bool,
}

/// What a weight entry form submits, for creation and for a correction (WT-1, WT-4, WT-5, DM-1).
///
/// The value is grams, converted from the person's display unit before the call: no command accepts a
/// per-entry unit (D1.2). [`WeightEntryDraft::timestamp`] is optional because WT-1 defaults it to the
/// moment of creation; the storage boundary stamps "now" rather than the command, so the default is
/// one instant, not one per retry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WeightEntryDraft {
    /// The canonical value in grams (DM-1). Zero and negative values are refused (WT-4).
    pub value_grams: Grams,
    /// The timestamp, or `None`, and the two commands read `None` differently.
    ///
    /// On `create_weight_entry` it records the moment of creation (WT-1). On `update_weight_entry` it
    /// **keeps the entry's existing timestamp** — an edit never restamps a back-dated entry to the
    /// present, and WT-5's "correct a past entry" is a correction of what the person changed, not a
    /// side effect on when it was weighed. A timestamp the person deliberately moves is an explicit
    /// `Some(..)`; there is no "reset to now" on the update path.
    pub timestamp: Option<Instant>,
    /// The optional note (WT-1).
    pub note: Option<String>,
    /// Whether the person confirmed a value above [`PLAUSIBILITY_BOUND_GRAMS`] (WT-4).
    ///
    /// A value above the bound is a *confirmation* threshold, not a rejection (D1.9): the same value
    /// is saved once the person confirms it. Until then the command refuses it through the failure
    /// surface, so the boundary — not only the form — holds WT-4.
    pub outlier_confirmed: bool,
}

/// The plausibility bound WT-4 states, in canonical grams: above 10 kg a value needs confirmation.
///
/// It is a **confirmation** threshold, never a rejection (D1.9): a genuinely heavy rabbit is still
/// recorded once the person confirms the value. The command is the enforcement point
/// ([`WeightEntryDraft::outlier_confirmed`]); `src/features/weight/api.ts` mirrors this number only to
/// decide *when* to ask, never what is stored, so the two cannot disagree about the saved value even
/// if they were to drift.
///
/// It is a [`Grams`] rather than a bare integer so it is compared and passed as the same type the
/// entries carry; a bare number here would be one more place a display unit could be assumed. It is a
/// constant, not a field of an exported type, so it appears in no binding.
pub const PLAUSIBILITY_BOUND_GRAMS: Grams = Grams(10_000);

/// One point of the weight trend and one row of the weight-only history (WT-6, WT-7).
///
/// It is the small shape both views need, so neither loads full notes or photo references to render a
/// series or a list. The flag is derived, exactly like [`WeightEntry::has_weight_drop_alert`] (D1.13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WeightEntrySummary {
    /// The entry's opaque identifier — also the detail view's route parameter (WT-8).
    pub id: WeightEntryId,
    /// The canonical value in grams (DM-1).
    pub value_grams: Grams,
    /// The entry's timestamp, absolute (DM-6).
    pub timestamp: Instant,
    /// Whether the drop against the previous entry exceeds the rabbit's threshold (WT-7).
    pub has_weight_drop_alert: bool,
}

/// A weight window: 7 days, 30 days, 90 days or the whole history (WT-6).
///
/// It is a **query window**, not a stored value (D1.4). The same enum serves the trend's range and the
/// weight-only history's range, and the two views hold theirs independently in the route's typed
/// search parameters, so changing one cannot change the other.
///
/// The wire values are spelled out per variant rather than derived with `rename_all = "snake_case"`:
/// serde's snake_case of `Days7` is `days7`, and the frozen set this change states is `days_7`,
/// `days_30`, `days_90`, `all`. The explicit renames are what make the generated binding say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum WeightRange {
    /// The last 7 days.
    #[serde(rename = "days_7")]
    Days7,
    /// The last 30 days.
    #[serde(rename = "days_30")]
    Days30,
    /// The last 90 days.
    #[serde(rename = "days_90")]
    Days90,
    /// The whole history: the absence of a lower bound, not a computed period.
    #[serde(rename = "all")]
    All,
}

/// One rabbit's weight series for one window, with the threshold it is judged against (WT-6, WT-7).
///
/// The threshold travels with the series so a surface that draws the chart can state the rule it is
/// applying without a second read, and so the entry flags and the dashboard flag are visibly one
/// derivation (R2). [`WeightTrend::points`] is ordered **oldest first** — the order the previous-entry
/// derivation uses (D1.12) — and a view that presents newest first reverses it.
///
/// The window limits which points are returned, **not** how the flags are derived: once task 2.13
/// derives the flag, it is computed over the rabbit's full entry set, so the earliest point of a
/// window can be flagged on the basis of an entry outside it (D1.13). Until then each point's
/// `has_weight_drop_alert` is the constant `false` — the payload's stored value is not content and is
/// ignored on every read (D1.16). A derivation that re-based at the window edge would make the same
/// entry disagree with the dashboard flag as soon as a range was selected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct WeightTrend {
    /// The window these points were read for.
    pub range: WeightRange,
    /// The entries inside the window, oldest first.
    pub points: Vec<WeightEntrySummary>,
    /// The rabbit's weight-drop threshold percentage at read time (WT-7).
    pub drop_threshold_percent: f64,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The one rabbit content the envelope tests carry.
    fn rabbit() -> Rabbit {
        Rabbit {
            id: RabbitId("item-1".to_owned()),
            household_id: HouseholdId("household-1".to_owned()),
            name: "Pebble".to_owned(),
            photo_ref: None,
            breed: Some("Mini Lop".to_owned()),
            dob: None,
            dob_unknown: true,
            sex: Sex::Female,
            neuter_status: NeuterStatus::Spayed,
            adoption_date: None,
            is_archived: true,
            weight_drop_threshold_percent: 5.0,
        }
    }

    /// The one weight entry the envelope tests carry.
    fn weight_entry() -> WeightEntry {
        WeightEntry {
            id: WeightEntryId("entry-1".to_owned()),
            rabbit_id: RabbitId("item-1".to_owned()),
            value_grams: Grams(1_250),
            timestamp: Instant("2026-10-20T09:00:00Z".to_owned()),
            created_at: Instant("2026-10-20T09:00:00Z".to_owned()),
            note: None,
            photo_refs: Vec::new(),
            has_weight_drop_alert: false,
        }
    }

    #[test]
    fn a_rabbit_payload_is_the_envelope_save_item_submits() {
        // Exactly the shape `domain::rabbit::save_body` sends: one internally tagged object whose
        // member names are frozen (D1.16). A second shape would let a write and a row drift apart.
        let payload = ItemPayload::Rabbit { rabbit: rabbit() };

        assert_eq!(
            serde_json::to_value(&payload).expect("the payload is encodable"),
            json!({ "kind": "rabbit", "rabbit": rabbit() })
        );
    }

    #[test]
    fn every_payload_variant_names_a_kind_this_version_models() {
        // Exhaustive on purpose: a new variant does not compile until it names its token here, and
        // that token is then checked against both its own wire tag and the list the feed classifies
        // by. A variant whose token is missing from `KINDS` would have its rows treated as foreign
        // and dropped (D1.16).
        fn token(payload: &ItemPayload) -> &'static str {
            match payload {
                ItemPayload::Rabbit { .. } => "rabbit",
                ItemPayload::WeightEntry { .. } => "weight_entry",
            }
        }

        for payload in [
            ItemPayload::Rabbit { rabbit: rabbit() },
            ItemPayload::WeightEntry {
                weight_entry: weight_entry(),
            },
        ] {
            let wire = serde_json::to_value(&payload).expect("the payload is encodable");
            let kind = token(&payload);

            assert_eq!(
                wire["kind"].as_str(),
                Some(kind),
                "the variant's token is the tag it is written with"
            );
            assert!(
                ItemPayload::KINDS.contains(&kind),
                "{kind} must be in ItemPayload::KINDS or the feed would drop its rows as foreign"
            );
        }
    }

    #[test]
    fn a_row_serialises_unknown_content_as_null_and_reads_a_missing_field_as_unknown() {
        let row = CollectionItem {
            item_id: ItemId("item-1".to_owned()),
            updated_at: Instant("2026-10-19T10:00:00Z".to_owned()),
            deleted: false,
            payload: None,
        };

        let encoded = serde_json::to_value(&row).expect("the row is encodable");
        assert_eq!(encoded["payload"], json!(null), "unknown is a visible null");

        // `#[serde(default)]`: a leaner feed (SYNC-3) reads as content unknown, not as unreadable.
        let leaned = json!({
            "item_id": "item-1",
            "updated_at": "2026-10-19T10:00:00Z",
            "deleted": false,
        });
        let decoded: CollectionItem =
            serde_json::from_value(leaned).expect("a row without the field is readable");
        assert_eq!(decoded.payload, None);
    }

    #[test]
    fn unknown_content_is_not_the_same_value_as_an_archived_payload() {
        let unknown = CollectionItem {
            item_id: ItemId("item-1".to_owned()),
            updated_at: Instant("2026-10-19T10:00:00Z".to_owned()),
            deleted: false,
            payload: None,
        };
        let archived = CollectionItem {
            payload: Some(ItemPayload::Rabbit { rabbit: rabbit() }),
            ..unknown.clone()
        };

        assert_ne!(unknown, archived);
        assert_eq!(unknown.payload, None);
        assert!(matches!(archived.payload, Some(ItemPayload::Rabbit { .. })));
    }
}
