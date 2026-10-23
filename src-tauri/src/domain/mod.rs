//! The household domain: the model P0's household commands share, and the invariants they must
//! preserve (HS-1, HS-3; D0.3, D0.14).
//!
//! The contract in [`crate::ipc::types`] fixes the *shape* that crosses the boundary. This module
//! fixes what that shape may not violate:
//!
//! * A household's name is non-blank. Empty and whitespace-only input are the same refusal, and it
//!   happens before anything is sent to the shared service (HS-1).
//! * The member that created a household is recorded as its first member **and** as its owner. The
//!   owner field exists from P0 so P3's destructive deletion does not need a retrofit (D0.3).
//! * A newly created household becomes the active one, and the person has at most one active
//!   household (DM-3). [`HouseholdBook`] makes that a property of the store rather than of a caller
//!   remembering to clear the previous one.
//! * A replayed create is the same operation, not a second household. The command carries the same
//!   [`RequestId`](crate::ipc::types::RequestId) to the service, whose ledger deduplicates it
//!   (D0.14), and the book records the operation's outcome so the local mirror cannot grow a second
//!   household either.
//!
//! [`household`] holds the model and its invariants; [`create`] is the one operation that writes a
//! new household through [`crate::sync::SaveGate`]. Keeping the two separate is what lets the model
//! be tested without a service, a clock or an app, while the write path is exercised against the
//! scripted transport.
//!
//! [`identity`] fixes the other question the household model raises: whose name Settings & Profile
//! shows. It implements SET-2's chain — the account's display name, then the locally known member
//! name for the same account, then the unresolved marker — so one place decides the name, an
//! unattributable local name is never borrowed, and an unresolved identity is a normal value rather
//! than a failure that could block the screen.
//!
//! [`rabbit`] starts P1's domain: one rabbit's RAB-1 field set, the three rules creation fixes — an
//! unsupplied field stays absent, `dob_unknown: true` excludes a date of birth (I8), and the name
//! must not be blank, since empty and whitespace-only are the same refusal (RAB-2) — and the one
//! write path a rabbit is created or edited through, P0's reachability-gated save (D1.1). An edit
//! replaces the whole editable set of an existing rabbit, threshold included, while the identity and
//! household it keeps come from the target rather than from the submission (RAB-5). A household
//! holds any number of rabbits, each its own item with its own identity and each belonging to
//! exactly one household (RAB-3). The module holds no storage write: the created or edited rabbit
//! reaches the frontend from the command that wrote it, and this device's **copy** of the
//! household's collection is the read path's (SYNC-3).
//!
//! [`rabbit`] also owns the P1 lifecycle/read rules that pair with that content: archiving and
//! restoring change only the archived flag of a rabbit read from this device's copy and submit the
//! whole item, so nothing the profile holds is replaced by a partial payload and a restored rabbit
//! keeps its weight history untouched (RAB-6, RAB-7, RAB-9); [`rabbit::ensure_writable`] is the one
//! archived-target rule a content write applies at the boundary, so an archived rabbit's profile and
//! weight entries cannot be created, edited or deleted however the write is reached (RAB-8, DM-4;
//! D1.5); and a rabbit list is either the active set or the archived set — never a union — whose rows
//! must all be describable, so a row whose content this device does not hold refuses as `unknown`
//! instead of being omitted (D1.16).
//!
//! Permanent deletion is the one irreversible operation, and it too is archived-only: the plan is the
//! archived rabbit plus every weight-entry row that names it, and each is written as a tombstone —
//! `deleted` as the write's own argument, with the payload retained (D1.17) — into the household's one
//! collection, so no member resolves the rabbit or its entries afterwards. An active rabbit is refused,
//! and a row this device cannot describe refuses the whole deletion rather than removing a subset or a
//! partial payload (RAB-10; D1.8, D1.16).
//!
//! [`weight`] starts the weight loop: one entry's value, timestamp and optional note, and the one item
//! write a create goes through. The value is canonical grams and the entry carries no unit of its own —
//! the display unit is applied at the frontend boundary (DM-1, WT-3) — the timestamp is the supplied
//! moment or the operation's creation instant, and the note is optional and kept whole (WT-1). An
//! entry's identity is its create operation's identity, so a retried create is the same row (D0.14),
//! and the derived drop-alert flag is written `false` rather than persisted (D1.16, D1.3). The value
//! rules of WT-4 are applied where the draft becomes a record: a value of zero or less is refused, and
//! a value above the plausibility bound is refused until the draft confirms it — the bound is a
//! confirmation threshold, so a confirmed outlier is stored at its true value (D1.9). The same rule is
//! public for the edit path. The edit and delete paths, the reads and the derivation are the tasks that
//! follow.
//!
//! **Nothing here touches Tauri.** The commands in [`crate::ipc`] are adapters over this module, and
//! the shared state in [`crate::state`] owns the instance a command borrows.

pub mod create;
pub mod household;
pub mod identity;
pub mod rabbit;
pub mod weight;

pub use create::{CREATE_HOUSEHOLD_RPC, create_household};
pub use household::{HouseholdBook, HouseholdName, HouseholdRecord, validate_name};
pub use identity::{local_member_id, local_member_name, resolve_identity};
pub use rabbit::{RabbitRecord, SAVE_ITEM_RPC, save_rabbit};
