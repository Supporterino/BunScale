/*
 * Household commands (HS-1, HS-3, HS-4): the only module that imports their
 * generated bindings. Project hard rule 2 keeps `src/bindings.ts` out of
 * routes, hooks and components, so every household call in the app goes
 * through these functions.
 *
 * The two writes — `create_household` and `set_active_household` — are returned
 * as a `RequestOperation` rather than a promise, because `request_id` is the
 * operation's identity (design.md D0.14). `create_household` is idempotent on
 * it, so the retry SYNC-2 offers must re-send the id of the attempt that
 * failed; `set_active_household` is a local switch and follows the same
 * discipline even though no service ledger sees its id. Creating the handle
 * mints the id and `run()` is the first attempt as well as the retry — see
 * `src/lib/request-id.ts`.
 *
 * The reads take no id and return the contract's `Result` as it is.
 * `list_households` and `get_active_household` are served from this device's
 * household book (D0.2) and do not touch the shared service, so an empty list
 * and `null` are values rather than failures. `load_household` resolves the
 * local copy first and then refreshes it through the service when the service
 * and the account are reachable, reporting which happened through `source` and
 * `loaded_at`. Neither an empty list nor `null` proves the person holds no
 * household: only a fresh `ServiceStatus` with `state: reachable` and
 * `account_valid: true` does that (SYNC-3; the guard is task 3.3).
 *
 * The subscription to `household:changed` also lives here (task 3.6). The
 * generated event descriptor is part of the same bindings seam, so this module
 * remains the feature's one place that imports `src/bindings.ts`; `./events`
 * holds only the invalidation policy and imports no generated code.
 */

import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  CommandError,
  Household,
  HouseholdChanged,
  HouseholdCollection,
  HouseholdId,
  HouseholdSummary,
} from '../../bindings';
import { commands, events } from '../../bindings';
import type { CommandResult, RequestOperation } from '../../lib/request-id';
import { requestOperation } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/** A state-changing command's handle; `run()` again re-uses the request id. */
type Operation<T> = RequestOperation<T, CommandError>;

/** The `household:changed` payload, as the contract defines it (task 3.6). */
export type HouseholdChange = HouseholdChanged;

/**
 * Create a household and make it the active one (HS-1, HS-2).
 *
 * The creator becomes its first member. The write only leaves the device while
 * the service is reachable and the account is valid (SYNC-2), and a blank or
 * whitespace-only name is refused with `cause: validation`; both are the
 * backend's decision, not this wrapper's. A fresh handle per person-initiated
 * creation is the rule — two households are two operations — so call this once
 * per intent and keep the returned handle only to retry that intent. A corrected
 * name is a new intent: create the handle again rather than re-running one that
 * carries the refused input.
 */
export function createHousehold(name: string): Operation<Household> {
  return requestOperation<Household, CommandError>((requestId) =>
    commands.createHousehold(name, requestId),
  );
}

/**
 * Every household the person holds, with the active one indicated (HS-3, HS-4,
 * DM-3).
 *
 * An empty list is not proof of "no household" (SYNC-3): it cannot distinguish
 * that from "nothing was ever loaded and the service is unreachable".
 */
export function listHouseholds(): Outcome<HouseholdSummary[]> {
  return commands.listHouseholds();
}

/**
 * The active household, or `null` when there is none (HS-2, HS-3, DM-3).
 *
 * At most one household is active. `null` starts onboarding only when a fresh
 * reachable status says so; it also covers a device with no local copy and an
 * unreachable service (SYNC-3).
 */
export function getActiveHousehold(): Outcome<Household | null> {
  return commands.getActiveHousehold();
}

/**
 * Switch the active household and return the one active afterwards (HS-3,
 * HS-4, DM-3).
 *
 * The switch is local to this device: it never reaches the shared service, so
 * the request id names the operation without a service-side deduplication step.
 * A household the person does not hold is refused with `cause: validation` and
 * leaves the previous selection active.
 */
export function setActiveHousehold(
  householdId: HouseholdId,
): Operation<Household> {
  return requestOperation<Household, CommandError>((requestId) =>
    commands.setActiveHousehold(householdId, requestId),
  );
}

/**
 * One household's collection, resolved against the local copy first (SYNC-1,
 * SYNC-3).
 *
 * Unreachability is not an error here: `source` and `loaded_at` report the
 * three SYNC-3 states, so a collection that was never loaded is never shown as
 * an empty one.
 */
export function loadHousehold(
  householdId: HouseholdId,
): Outcome<HouseholdCollection> {
  return commands.loadHousehold(householdId);
}

/**
 * Subscribe to `household:changed` (task 3.6; SYNC-1, HS-4, DM-3).
 *
 * The generated event descriptor is imported here, beside the commands, because
 * project hard rule 2 names this module — and only this module per feature — as
 * the one that may touch `src/bindings.ts`. `./events` holds the invalidation
 * policy and imports no generated code; it receives the payload this function
 * unwraps from Tauri's event envelope.
 *
 * The returned promise is the Tauri `listen` promise: the caller keeps it and
 * calls the resolved unlisten on cleanup (convention 4). No payload is buffered
 * here — an event is a change report, and the queries own the data.
 */
export function subscribeToHouseholdChanges(
  handler: (change: HouseholdChange) => void,
): Promise<UnlistenFn> {
  return events.householdChanged.listen((event) => {
    handler(event.payload);
  });
}
