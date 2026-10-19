//! Household commands: creating a household, and holding and switching the active one (HS-1…HS-4,
//! DM-3).
//!
//! `create_household` (task 2.1) is P0's only state-changing command that writes to the shared
//! service, so the reachability-gated write path — and with it every reachable `SaveCause` — sits
//! behind it (2.4, 5.4). The other three (task 2.2) read and write the process's household book and
//! never reach the service, so their failure surface is narrower by construction.
//!
//! **Switching the active household is local state, not a shared-collection write.** The active
//! household is the person's selection over the households they hold, not an item of any household's
//! collection, and P0's frozen service surface has no operation for it; D0.1's gate therefore does
//! not apply. A switch assigns the book's one active id, adds no second write path, and its only
//! failure is `validation` for a household the person does not hold. The contract still carries a
//! `request_id` (frozen in 1.3), and it is carried on any failure like every state-changing
//! command's; a re-issue is naturally idempotent because the operation assigns one field one value.
//! [`crate::domain::household`] records the same decision beside the invariant it protects (DM-3).
//!
//! The two reads carry D0.2's rule into the contract: an empty list and a `None` are conclusions the
//! frontend may draw only from a fresh, reachable [`ServiceStatus`], never from the read itself
//! (SYNC-3).

// The shared contract types, including the ones only the doc comments link (see the parent module's
// docs for why this is a glob).
use super::super::types::*;

/// Create a household with a non-blank name; the creator becomes its first member and it becomes
/// the active household (HS-1, HS-2).
///
/// The write is attempted only through the reachability-gated save path, so it is refused before
/// anything leaves the device when the shared service cannot be reached or when no account is
/// available (SYNC-2, D0.1). No local-only household is created in either case. P0 ships no sign-in
/// flow, so the running app has no account yet and answers `no_account` until one is provisioned
/// (Q25).
///
/// The operation is **idempotent on `request_id`** (D0.14): re-issuing it with the same value
/// returns the household that already exists instead of creating a second one. The value is carried
/// to the shared service, whose request ledger deduplicates the call, and the domain records the
/// operation's outcome so a replay cannot add a second household locally either. That is what makes
/// the retry SYNC-2 offers safe and closes R6's duplicated-household risk.
///
/// # Errors
///
/// `validation` for an empty or whitespace-only name, which is treated as blank (HS-1), for a blank
/// operation identity, and when the service refuses the request body with `400` or `422`.
/// `no_account` when no account is available (Q25) or the service answers `401`. The write path's
/// other causes are reachable here because this is P0's only state-changing command that writes to
/// the shared service: `connectivity` when no HTTP answer arrives, `allowance_exceeded` when the
/// household owner's allowance is exhausted (D0.12), `rate_limited` for the gateway's `429`, and
/// `timeout` when no outcome arrives inside D0.13's bounds. `unknown` is the honest remainder: a
/// probe answer this version cannot classify, and an accepted answer whose household body cannot be
/// read. No failure is ever presented as a created household (SYNC-2).
#[tauri::command]
#[specta::specta]
pub async fn create_household(
    name: String,
    request_id: RequestId,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Household, CommandError> {
    state.create_household(name, request_id).await
}

/// Every household the person holds, with the active one indicated (HS-3, HS-4, DM-3).
///
/// Served from the process's household book, which is this device's local view (D0.2), in the order
/// the person acquired the households. Siblings are separate records: no member or item of one
/// household appears in another's row (HS-3), and exactly one row carries `is_active`, or none when
/// the person holds no household (DM-3).
///
/// An empty list is **not** proof that the person holds no household: it cannot distinguish "no
/// household" from "nothing was ever loaded and the service is unreachable" (SYNC-3). The list is
/// what HS-2's route guard reads at launch, so the guard may start first-run onboarding only when a
/// fresh [`ServiceStatus`] reports `state: reachable` and `account_valid: true`; while that status
/// is `unknown` or `unreachable`, the guard shows the connectivity-required state and does not start
/// onboarding (task 3.3 consumes this rule; task 3.4 holds the status).
///
/// # Errors
///
/// None is reachable: the read is local, so an empty book is `Ok(vec![])` rather than a failure.
/// Unreachability is a status fact (`get_service_status`), not a read failure.
#[tauri::command]
#[specta::specta]
pub async fn list_households(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<HouseholdSummary>, CommandError> {
    Ok(state.list_households())
}

/// The person's active household, or `None` when they hold none (HS-2, HS-3, DM-3).
///
/// `None` is the onboarding trigger — HS-2 presents household creation in that state and never once
/// a household exists — but it is a *conclusion*, not an absence of data: `None` also covers a
/// device with no local copy and an unreachable service. It may be read as "no household" only when
/// a fresh [`ServiceStatus`] reports `state: reachable` and `account_valid: true`; while that status
/// is `unknown` or `unreachable`, the guard shows the connectivity-required state instead of
/// onboarding (SYNC-3, D0.2). At most one household is active (DM-3), and the read is served from
/// the same book as [`list_households`].
///
/// # Errors
///
/// None is reachable: "no household" is `Ok(None)` and the read is local, so it makes no service
/// request that could fail.
#[tauri::command]
#[specta::specta]
pub async fn get_active_household(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Option<Household>, CommandError> {
    Ok(state.active_household())
}

/// Switch the active household and return the household that is active afterwards (HS-3, HS-4,
/// DM-3).
///
/// The switch is **local state**: it changes this process's household book and never reaches the
/// shared service, because the active household is a selection over the held households rather than
/// an item of any household's collection (see [`crate::domain::household`]). The contract's
/// `request_id` is therefore not a service idempotency key here; it is still carried on a failure,
/// and re-issuing a call is naturally safe because the operation assigns one field one value.
///
/// DM-3 is enforced **per device/process** here: this book is the process's state, so two devices can
/// each show a different active household for the same person while each keeps exactly one.
/// Cross-device agreement would need a per-account selection stored by the shared service, which
/// P0's frozen contract does not have.
///
/// The result is the post-state rather than an echo of the argument: it is read back from the book
/// after the switch. This command never removes the active household — it refuses an id the person
/// does not hold — so it cannot trigger DM-3's re-election, and the post-state is the requested
/// household. Re-election happens when a household leaves the held set, through
/// [`crate::domain::household::HouseholdBook::release`], which a held-set refresh uses. A refused
/// switch changes nothing and the previously active household stays active.
///
/// # Errors
///
/// `validation` when the person does not hold the household. That is the only failure this command
/// can observe: because nothing leaves the device, the service's conditions (`connectivity`,
/// `no_account`, `allowance_exceeded`, `rate_limited`, `timeout`) are unreachable here, and a failed
/// switch is never presented as a switched one.
#[tauri::command]
#[specta::specta]
pub async fn set_active_household(
    household_id: HouseholdId,
    request_id: RequestId,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Household, CommandError> {
    state.set_active_household(household_id, request_id)
}
