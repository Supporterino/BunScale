/*
 * The per-device display weight unit (task 3.5; SET-4, D0.4, R7).
 *
 * Two facts shape this module:
 *
 * 1. **The effective unit is the backend's answer.** `get_display_weight_unit`
 *    resolves the chain — the person's stored choice, otherwise the unit the
 *    device's region states, otherwise the canonical unit (grams). The regional
 *    rule is derived **in Rust** (design.md, D0.2; Q26 records that iOS cannot
 *    read it and therefore falls through to canonical), so nothing here
 *    computes a default, reads a locale or maps an argument. The hook surfaces
 *    exactly the unit the backend returned, which is what makes "the default
 *    follows the device region" testable as a rule at the Rust layer (task
 *    5.11) instead of being re-implemented against a guessed frontend locale.
 *
 * 2. **The write returns the unit it read back, not the one it was given.**
 *    `set_display_weight_unit` stores the choice and then answers with the
 *    effective unit on the device; a write that did not land is therefore never
 *    reported as stored (SET-4). The mutation writes *that* value into the unit
 *    cache — not `operation`'s argument, which is not part of the handle's
 *    visible shape anyway — so a refused or lost write cannot make the screen
 *    show the chosen unit.
 *
 * Retry identity follows the rest of the app (design.md D0.14): the mutation's
 * variable is the **operation handle** from `setDisplayWeightUnit`, so a retry
 * calls `run()` again and re-sends the id of the attempt that failed. A new
 * choice is a new operation and therefore a new handle; a correction is not.
 *
 * What a unit change does **not** touch: any household read. The unit is
 * per-device and never part of a household collection (D0.2, R7), and no stored
 * weight changes when it does (D0.4) — conversion happens only where a weight is
 * entered or displayed. P0 has no cached formatted weight, so the unit read is
 * the only entry a change affects; when P1 introduces one it must derive from
 * this same key rather than keep a second copy that could disagree.
 */
import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { getDisplayWeightUnit, type setDisplayWeightUnit } from './api';
import { settingsKeys } from './keys';

/** The unit read's outcome, used to derive this module's types. */
type UnitOutcome = Awaited<ReturnType<typeof getDisplayWeightUnit>>;

/** `set_display_weight_unit`'s handle, as `api.ts` returns it (D0.14). */
type SetUnitOperation = ReturnType<typeof setDisplayWeightUnit>;

/** The display unit — the contract's `WeightUnit`. */
export type WeightUnit = CommandData<UnitOutcome>;

/**
 * A unit command's error: a `CommandError` (the local-storage `unknown` is the
 * only cause this pair reports), or the `Error` a broken IPC call re-throws.
 */
export type DisplayWeightUnitFailure = CommandFailure<UnitOutcome> | Error;

/** The unit key's type, so the options cannot disagree with the factory. */
type DisplayWeightUnitKey = ReturnType<typeof settingsKeys.displayWeightUnit>;

/**
 * The effective display unit (SET-4).
 *
 * A local read: it resolves on this device, does not touch the shared service
 * and does not fail because a household read failed. No `staleTime` override —
 * mounting Settings re-reads it through the shared client's defaults, which is
 * how a value changed elsewhere (another window, a later launch) is picked up.
 */
export function displayWeightUnitQueryOptions() {
  return queryOptions<
    WeightUnit,
    DisplayWeightUnitFailure,
    WeightUnit,
    DisplayWeightUnitKey
  >({
    queryKey: settingsKeys.displayWeightUnit(),
    queryFn: () => getDisplayWeightUnit().then(unwrapCommandResult),
  });
}

/** The effective display unit, reading the cached copy first. */
export function useDisplayWeightUnit() {
  return useQuery(displayWeightUnitQueryOptions());
}

/**
 * Store this device's display unit (SET-4).
 *
 * Start a change with `mutate(setDisplayWeightUnit(unit))`; the handle is the
 * mutation's variable, so re-submitting the same handle is the retry and
 * carries the original `request_id`. On success the **returned** effective unit
 * is written to the unit cache, so every weight display reads the value the
 * backend confirmed. An in-flight read is cancelled first: a read that started
 * before the write could otherwise resolve afterwards with the pre-change unit
 * and overwrite the confirmed one.
 *
 * A failed change writes nothing: the cache keeps the last confirmed unit and
 * the surface shows the failure's cause (the `unknown` copy must read as "this
 * device could not store the choice", never as a service failure — see the
 * backend command's note for task 3.7).
 */
export function useSetDisplayWeightUnit() {
  const queryClient = useQueryClient();

  return useMutation<WeightUnit, DisplayWeightUnitFailure, SetUnitOperation>({
    mutationFn: (operation) => operation.run().then(unwrapCommandResult),
    onSuccess: async (unit) => {
      await queryClient.cancelQueries({
        queryKey: settingsKeys.displayWeightUnit(),
      });

      queryClient.setQueryData<WeightUnit>(
        settingsKeys.displayWeightUnit(),
        unit,
      );
    },
  });
}
