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
//! **Nothing here touches Tauri.** The commands in [`crate::ipc`] are adapters over this module, and
//! the shared state in [`crate::state`] owns the instance a command borrows.

pub mod create;
pub mod household;

pub use create::{CREATE_HOUSEHOLD_RPC, create_household};
pub use household::{HouseholdBook, HouseholdName, HouseholdRecord, validate_name};
