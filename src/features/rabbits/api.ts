/*
 * Rabbit commands (RAB-1 … RAB-10): the only module that imports their
 * generated bindings. Project hard rule 2 keeps `src/bindings.ts` out of
 * routes, hooks and components, so every rabbit call in the app goes through
 * these functions.
 *
 * The five writes take a `RequestId` (design.md D0.14) and are returned as a
 * `RequestOperation` rather than a promise, exactly as P0's household writes
 * are: the handle mints the id once and every `run()` re-sends it, so the retry
 * the failure surface offers is the same operation instead of a second create.
 * A corrected input is a new intent and therefore a new handle, never a re-run
 * of one that carries the refused input.
 *
 * The two reads take no id and are returned as the contract's `Result` as it
 * stands. None of them proves an absence: an empty active list is what DASH-8
 * keys its empty state on, and `list_rabbits` answers either the active list or
 * the archived list, never a union of the two.
 *
 * `list_rabbits`'s `include_archived` is the contract's own name for a switch
 * that is **not** inclusive — `false` selects the active list, `true` selects
 * the archived list. It is passed through unchanged rather than hidden behind
 * two wrappers, because the frozen surface has one command and this module
 * mirrors it one-to-one; the trap is stated here so a call site cannot mistake
 * `true` for "both".
 *
 * Every write is refused on an archived rabbit and reports that through P0's
 * single failure surface as `cause: validation` (D1.5, D1.8) — the boundary
 * holds it, and the UI withholds the controls as a second line rather than the
 * only one. No wrapper pre-checks it.
 *
 * The `rabbit:changed` subscription lives beside these commands (task 3.9;
 * RAB-5, RAB-6, RAB-7, RAB-10): the generated event descriptor is part of the
 * same bindings seam, so this module remains the feature's one importer of
 * `src/bindings.ts`, the way `household:changed` sits in the household module.
 */
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  CommandError,
  HouseholdId,
  Rabbit,
  RabbitChanged,
  RabbitId,
  RabbitProfile,
  RabbitSummary,
} from '../../bindings';
import { commands, events } from '../../bindings';
import type { CommandResult, RequestOperation } from '../../lib/request-id';
import { requestOperation } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/** A state-changing command's handle; `run()` again re-uses the request id. */
type Operation<T> = RequestOperation<T, CommandError>;

/** The `rabbit:changed` payload, as the contract defines it (task 3.9). */
export type RabbitChange = RabbitChanged;

/**
 * Add a rabbit to a household (RAB-1, RAB-2, RAB-3).
 *
 * The profile is the whole editable field set: the optional fields a person did
 * not supply are `null`, and "unknown" is a recorded value rather than an
 * absence where the contract says so. A blank or whitespace-only name is
 * refused with `cause: validation` by the backend (RAB-2), and so is
 * `dob_unknown: true` together with a `dob` (I8) — neither is re-decided here,
 * because a form that guessed would disagree with the boundary that enforces it.
 *
 * The write only leaves the device while the service is reachable and the
 * account is valid (D1.1), so the write-path causes are all reachable behind it.
 */
export function createRabbit(
  householdId: HouseholdId,
  profile: RabbitProfile,
): Operation<Rabbit> {
  return requestOperation<Rabbit, CommandError>((requestId) =>
    commands.createRabbit(householdId, profile, requestId),
  );
}

/**
 * One list of a household's rabbits: the active ones, or the archived ones
 * (DASH-1, RAB-6).
 *
 * `includeArchived: false` answers the active list and `true` answers the
 * archived list — never both, so a screen that wants a union has to make two
 * reads and say so. The empty state DASH-8 defines is keyed on the *active*
 * list: a household whose rabbits are all archived has zero rows here while its
 * archived list stays reachable as its own screen.
 *
 * A local read: it never touches the shared service, so an empty list is a
 * value and not a failure, and no service cause is reachable.
 */
export function listRabbits(
  householdId: HouseholdId,
  includeArchived: boolean,
): Outcome<RabbitSummary[]> {
  return commands.listRabbits(householdId, includeArchived);
}

/**
 * One rabbit's full profile (DASH-6, RAB-5).
 *
 * The derived values are absent by design: a rabbit carries no baseline or
 * healthy range (D5, D1.7). An unknown or blank id is refused with
 * `cause: validation`; the other failures are the local set, because the read
 * is served from this device's copy.
 */
export function getRabbit(rabbitId: RabbitId): Outcome<Rabbit> {
  return commands.getRabbit(rabbitId);
}

/**
 * Replace a rabbit's editable field set (RAB-5, WT-7).
 *
 * A whole-profile replacement, not a patch (the contract carries no
 * double-optional to tell "unchanged" from "cleared"): the form is pre-filled
 * with the current profile and submits all of it. The threshold is one of these
 * fields, so a threshold edit reaches every member through the ordinary change
 * event and the drop alerts are re-derived from it on read (D1.3).
 *
 * A blank name or a write to an archived rabbit is refused with
 * `cause: validation` (RAB-2, D1.5); the write-path causes are all reachable.
 */
export function updateRabbit(
  rabbitId: RabbitId,
  profile: RabbitProfile,
): Operation<Rabbit> {
  return requestOperation<Rabbit, CommandError>((requestId) =>
    commands.updateRabbit(rabbitId, profile, requestId),
  );
}

/**
 * Archive a rabbit: the reversible soft delete (RAB-6, RAB-9).
 *
 * Nothing is removed and nothing is purged, so the archived list and the
 * archived rabbit's own screens stay reachable. An unknown id or one that is
 * already archived is refused with `cause: validation` — archiving twice is not
 * silently absorbed here.
 */
export function archiveRabbit(rabbitId: RabbitId): Operation<Rabbit> {
  return requestOperation<Rabbit, CommandError>((requestId) =>
    commands.archiveRabbit(rabbitId, requestId),
  );
}

/**
 * Return an archived rabbit to the active list (RAB-7).
 *
 * An unknown id or one that is not archived is refused with
 * `cause: validation`; restoring an active rabbit is not a no-op this wrapper
 * hides.
 */
export function restoreRabbit(rabbitId: RabbitId): Operation<Rabbit> {
  return requestOperation<Rabbit, CommandError>((requestId) =>
    commands.restoreRabbit(rabbitId, requestId),
  );
}

/**
 * Remove an archived rabbit and its entries for good (RAB-10).
 *
 * The only removal path in P1, and the contract restricts it to an **archived**
 * rabbit: an active or unknown rabbit is refused with `cause: validation`. The
 * confirmation that states the consequence is the caller's (RAB-10, task 4.5) —
 * this wrapper must never be called without it.
 *
 * The contract's `()` arrives as `null` on the wire: the removal is the answer
 * and a deleted rabbit has no value to return.
 */
export function deleteRabbit(rabbitId: RabbitId): Operation<null> {
  return requestOperation<null, CommandError>((requestId) =>
    commands.deleteRabbit(rabbitId, requestId),
  );
}

/**
 * Subscribe to `rabbit:changed` (task 3.9; RAB-5, RAB-6, RAB-7, RAB-10).
 *
 * The generated event descriptor is imported here, beside the commands, because
 * project hard rule 2 names this module — and only this module per feature — as
 * the one that may touch `src/bindings.ts`. The handler receives the contract's
 * payload, not Tauri's event envelope, so the cache policy cannot depend on the
 * bridge's shape.
 *
 * The returned promise is the Tauri `listen` promise: the caller keeps it and
 * calls the resolved unlisten on cleanup (convention 4). No payload is buffered
 * here — an event is a change report, and the queries own the data.
 */
export function subscribeToRabbitChanges(
  handler: (change: RabbitChange) => void,
): Promise<UnlistenFn> {
  return events.rabbitChanged.listen((event) => {
    handler(event.payload);
  });
}
