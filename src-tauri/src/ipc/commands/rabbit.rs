//! Rabbit-profile commands: the P1 lifecycle over one household's rabbits (RAB-1 … RAB-10).
//!
//! Seven commands, frozen by task 1.1: [`create_rabbit`], [`list_rabbits`], [`get_rabbit`],
//! [`update_rabbit`], [`archive_rabbit`], [`restore_rabbit`] and [`delete_rabbit`]. Each body is a
//! typed stub returning [`CommandError::not_implemented`] — section 2 replaces them one task at a
//! time, and the signatures are what `src/bindings.ts` is generated from, so a stub is the contract
//! rather than a placeholder to be reshaped.
//!
//! Three things the signatures already encode, because section 2 must not have to renegotiate them:
//!
//! * **Writes carry a [`RequestId`], reads do not.** Every state-changing command is idempotent on it
//!   (D0.14) and re-issues the same value on a retry, so its failure can carry the identity the retry
//!   needs; the three reads are idempotent already and report `request_id: None`.
//! * **`update_rabbit` replaces the whole profile** through [`RabbitProfile`] rather than a partial
//!   patch: the edit form is pre-filled, and a patch would need a double-optional per field to tell
//!   "unchanged" from "cleared" (see [`RabbitProfile`]).
//! * **Archived means immutable at the boundary** (D1.5, RAB-8). Every write here refuses an archived
//!   target — `update_rabbit` and the two lifecycle commands included — and reports it through the one
//!   failure surface. P0's cause list is frozen and has no "immutable target" variant, so the refusal
//!   is `validation`: the request itself is refused and a retry cannot clear it (D1.5, NFR-10).
//!
//! Every body below is a stub, so each one consumes its arguments deliberately: the parameters are
//! part of the frozen signature and the bindings' argument names, and an unused-parameter warning
//! under `clippy -D warnings` would otherwise have to be silenced.

use super::super::types::*;

/// Create a rabbit profile in a household (RAB-1, RAB-2, RAB-3).
///
/// The profile is stored in the household's shared collection, so every member resolves it (HS-8).
/// `household_id` is explicit even though the person has an active household: the write belongs to
/// the collection it names, and the active household is a per-device selection rather than a scope
/// the backend may infer.
///
/// # Errors
///
/// `validation` for a blank or whitespace-only name, which is treated as blank (RAB-2), and for a
/// blank operation identity. The write path's service conditions — `connectivity`, `no_account`,
/// `allowance_exceeded`, `rate_limited`, `timeout` — are reachable because the profile is a shared
/// write (D1.1), and `unknown` is the honest remainder.
#[tauri::command]
#[specta::specta]
pub async fn create_rabbit(
    household_id: HouseholdId,
    profile: RabbitProfile,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let _ = (household_id, profile);

    Err(CommandError::not_implemented(Some(request_id)))
}

/// The household's rabbits, as either the active list or the archived list (DASH-1, RAB-6).
///
/// The two values select **one list each**, never a union: `include_archived: false` is DASH-1's
/// active list — every non-archived rabbit of the household, in no required order because the
/// dashboard sorts by most recent weigh-in (DASH-3) — and `include_archived: true` is the archived
/// list, RAB-6's "retrievable from an archived list", and contains no active rabbit. The name reads
/// like a union only because it began as the design's draft parameter; the contract's meaning is the
/// one stated here and in the design, and section 2 must not implement "both lists".
///
/// An empty result is **not** proof that the household holds no rabbit: as with
/// [`list_households`](super::household::list_households), a device that never loaded the collection
/// and cannot reach the service answers the same way (SYNC-3), so the empty state is a conclusion to
/// draw only from a fresh reachable [`ServiceStatus`](crate::ipc::types::ServiceStatus).
///
/// # Errors
///
/// None is expected: the read resolves against the locally kept copy, so unreachability is a status
/// fact rather than a read failure (SYNC-3). A local store that cannot answer is `unknown`.
#[tauri::command]
#[specta::specta]
pub async fn list_rabbits(
    household_id: HouseholdId,
    include_archived: bool,
) -> Result<Vec<RabbitSummary>, CommandError> {
    let _ = (household_id, include_archived);

    Err(CommandError::not_implemented(None))
}

/// One rabbit's profile and archived state (DASH-6, RAB-5).
///
/// # Errors
///
/// `validation` for an id this household's collection does not hold, or for a blank id. A rabbit of
/// another household is reported the same way and never resolved: RAB-3 forbids presenting a rabbit
/// of one household as a rabbit of another.
#[tauri::command]
#[specta::specta]
pub async fn get_rabbit(rabbit_id: RabbitId) -> Result<Rabbit, CommandError> {
    let _ = rabbit_id;

    Err(CommandError::not_implemented(None))
}

/// Replace the editable fields of an active rabbit's profile (RAB-5, WT-7).
///
/// Every field is editable through this command, including `weight_drop_threshold_percent`: the
/// threshold is a profile field (RAB-1/RAB-5) rather than a setting of its own, so editing it is an
/// ordinary shared write and every member receives it. Because the alert is derived on read (D1.3), a
/// threshold edit re-derives the entry flags and the dashboard flag without rewriting an entry.
///
/// The change applies only to an **active** rabbit; an archived rabbit's profile is not editable
/// (RAB-5, D1.5).
///
/// # Errors
///
/// `validation` for a blank name (RAB-2), for an archived or unknown rabbit — the request is refused
/// rather than accepted and ignored — and for a blank operation identity. The write path's service
/// conditions are reachable as for [`create_rabbit`], and `unknown` is the honest remainder.
#[tauri::command]
#[specta::specta]
pub async fn update_rabbit(
    rabbit_id: RabbitId,
    profile: RabbitProfile,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let _ = (rabbit_id, profile);

    Err(CommandError::not_implemented(Some(request_id)))
}

/// Archive a rabbit: a reversible soft delete that discards nothing (RAB-6, RAB-9).
///
/// The rabbit leaves the active list and appears in the archived one, and every weight entry it holds
/// stays stored — nothing purges archived data, and this command deletes none of it (RAB-9). The
/// operation is reversible through [`restore_rabbit`]. Archiving is **not** a destructive action under
/// NFR-9, so it carries no consequence confirmation (D1.8); the confirmation belongs to
/// [`delete_rabbit`].
///
/// # Errors
///
/// `validation` for an unknown rabbit, for one that is already archived, and for a blank operation
/// identity. The write path's service conditions are reachable as for [`create_rabbit`].
#[tauri::command]
#[specta::specta]
pub async fn archive_rabbit(
    rabbit_id: RabbitId,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let _ = rabbit_id;

    Err(CommandError::not_implemented(Some(request_id)))
}

/// Return an archived rabbit to the active list with its history intact (RAB-7).
///
/// Every weight entry recorded before archiving is still present and unchanged afterwards: restoring
/// moves the rabbit's archived state and nothing else.
///
/// # Errors
///
/// `validation` for an unknown rabbit, for one that is not archived, and for a blank operation
/// identity. The write path's service conditions are reachable as for [`create_rabbit`].
#[tauri::command]
#[specta::specta]
pub async fn restore_rabbit(
    rabbit_id: RabbitId,
    request_id: RequestId,
) -> Result<Rabbit, CommandError> {
    let _ = rabbit_id;

    Err(CommandError::not_implemented(Some(request_id)))
}

/// Permanently delete an **archived** rabbit and every weight entry it holds (RAB-10).
///
/// Irreversible, and shared with every member: once it completes, no member resolves the rabbit or its
/// entries. Deletion is available only for an archived rabbit — an active one is refused, which is what
/// keeps archiving the recoverable step before the point of no return (D1.8, R7). The confirmation
/// that states the consequence is the frontend's (NFR-9); this command is called only after it.
///
/// # Errors
///
/// `validation` for an **active** rabbit — the deliberate narrowing of RAB-10 (D1.8) — for an unknown
/// rabbit, and for a blank operation identity. The write path's service conditions are reachable as for
/// [`create_rabbit`]; a deletion the service did not accept leaves the rabbit in place and is never
/// reported as removed.
#[tauri::command]
#[specta::specta]
pub async fn delete_rabbit(rabbit_id: RabbitId, request_id: RequestId) -> Result<(), CommandError> {
    let _ = rabbit_id;

    Err(CommandError::not_implemented(Some(request_id)))
}
