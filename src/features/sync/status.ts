/*
 * The service-status store (task 3.4; NFR-1, R3, SYNC-2, SYNC-3).
 *
 * `ServiceStatus` is the backend's last observation: `state`, `account_valid`,
 * `cause` and — the field that matters most here — `observed_at`. This module
 * holds that value in the shared query cache, exposes it as a view a surface can
 * render, and owns the one probe action. Three rules shape it:
 *
 * 1. **`unknown` until a fresh outcome exists, and never a carried-over guess
 *    (R3).** `observed_at === null` is the whole definition of "we have not
 *    looked": the view then says `not-observed`, whatever the other fields
 *    happen to contain. The store never seeds a status, never defaults one, and
 *    never remembers one across a process — the tracker lives in Rust and starts
 *    `unknown` every launch.
 * 2. **The offline threshold belongs to the backend (D0.13).** `unreachable` is
 *    reported there only after three consecutive failed checks without an
 *    intervening success, and every probe the frontend makes is a check on the
 *    same counter. So the store has **no counter of its own** — and no
 *    in-between view: below the threshold the tracker leaves the status
 *    untouched, so a failed check reads as `not-observed` on a device that never
 *    reached the service, or as the last answered probe. A failed *command* (an
 *    IPC-level `Error`) changes no status at all: it is a defect, not a probe
 *    result, and it is re-thrown so the caller's error surface shows it.
 * 3. **The read is the tracker, not the cache.** `readCurrentServiceStatus`
 *    asks the backend every time (`staleTime: 0` on a local, network-free
 *    command) and only probes when nothing has been observed. That is what the
 *    entry guard consumes, so the guard cannot decide on a cached status from
 *    an earlier moment.
 *
 * What this module deliberately does **not** do: subscribe to
 * `service:status-changed` (task 3.6 owns the subscription) and map a cause to
 * copy (task 3.4's `./failure` does that for the failure surface, not for the
 * status).
 */
import type { QueryClient } from '@tanstack/react-query';
import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { checkServiceReachability, getServiceStatus } from './api';
import type { SaveCause } from './failure';
import { syncKeys } from './keys';

/** A wrapper outcome, used to derive this store's types (hard rule 2). */
type StatusOutcome = Awaited<ReturnType<typeof getServiceStatus>>;

/** The last observed service/account status — `ServiceStatus` in the contract. */
export type ServiceStatus = CommandData<StatusOutcome>;

/** The status read's error: a `CommandError`, or the IPC `Error` it re-throws. */
export type ServiceStatusFailure = CommandFailure<StatusOutcome> | Error;

/** The status key's type, so the options cannot disagree with the factory. */
type StatusKey = ReturnType<typeof syncKeys.status>;

/**
 * The last observed status, read from the backend's tracker (NFR-1, R3).
 *
 * The command makes no network call and has no product failure path in P0, so
 * this is a local read; an IPC-level rejection is the `failed` case that
 * `useServiceStatusView` reports. `staleTime` is left to the shared client,
 * which means mounting a surface refetches it. An observation is current only
 * while `observed_at` is present — see `serviceStatusView`.
 */
export function serviceStatusQueryOptions() {
  return queryOptions<
    ServiceStatus,
    ServiceStatusFailure,
    ServiceStatus,
    StatusKey
  >({
    queryKey: syncKeys.status(),
    queryFn: () => getServiceStatus().then(unwrapCommandResult),
  });
}

/** The last observed status, reading the cached copy first. */
export function useServiceStatus() {
  return useQuery(serviceStatusQueryOptions());
}

/**
 * Run a fresh reachability probe, record it and return it (SYNC-2, NFR-1).
 *
 * The probe is `check_service_reachability`, bounded by
 * `REACHABILITY_PROBE_TIMEOUT` and counted on the backend's three-failure
 * threshold; its answer is written into the status cache so every surface reads
 * the observation this call produced. The backend also publishes it as
 * `service:status-changed` before returning; task 3.6 owns that subscription.
 *
 * A rejected call changes no cached status. An unreachable service is a *status*
 * (`state`/`cause`), not a rejection (SYNC-3), so a rejection here is an
 * IPC-level defect and is passed to the caller.
 */
export async function probeServiceReachability(
  queryClient: QueryClient,
): Promise<ServiceStatus> {
  const status = unwrapCommandResult(await checkServiceReachability());

  queryClient.setQueryData(syncKeys.status(), status);

  return status;
}

/** The probe as an action, for a surface that offers "check again". */
export function useProbeServiceReachability() {
  const queryClient = useQueryClient();

  return useMutation<ServiceStatus, ServiceStatusFailure, void>({
    mutationFn: () => probeServiceReachability(queryClient),
  });
}

/**
 * The status a decision should use: the backend's last observation, or one
 * fresh probe when nothing has been observed in this process yet.
 *
 * The household entry guard (task 3.3) is the caller, through
 * `src/features/sync/entry-status.ts`. Reading the tracker first keeps a
 * navigation from becoming a network round trip when an observation exists;
 * probing only when `observed_at` is null is what lets a first run ever reach a
 * conclusion (SYNC-3). The probe's outcome is cached, so the guard and the
 * surfaces share one observation.
 */
export async function readCurrentServiceStatus(
  queryClient: QueryClient,
): Promise<ServiceStatus> {
  const status = await queryClient.fetchQuery({
    ...serviceStatusQueryOptions(),
    staleTime: 0,
  });

  if (status.observed_at !== null) {
    return status;
  }

  return probeServiceReachability(queryClient);
}

/**
 * The status as a surface should present it.
 *
 * The backend's tracker produces exactly three shapes, and this mapping follows
 * them rather than an imagined middle ground (`src-tauri/src/sync/status.rs`):
 *
 * - **nothing observed** — `state: unknown` with every field absent. That is
 *   R3's "we have not looked", and `not-observed` is the only view for it.
 * - **an answered probe** — `state: reachable`, an observation, and the account
 *   half either `true` (`available`), `false` (the service refused the account,
 *   `account-unusable`) or `null` (the probe did not determine the account — a
 *   rate-limited or unreadable answer — which is `account-undetermined`, not
 *   "unusable": the contract's `null` means "no account outcome exists").
 * - **three consecutive unanswered probes** — `state: unreachable`, the
 *   threshold D0.13 places in Rust. `offline` is exactly that. There is no
 *   in-between state: below the threshold the tracker leaves the status
 *   **untouched**, so a prior success keeps reporting `reachable` and a device
 *   that never reached one stays `unknown` with no observation. One failed check
 *   therefore reads as the last answered probe's answer, never as offline — and
 *   the store adds no counter that could say otherwise.
 *
 * `observed_at` is checked first because it is R3's marker of "we looked": a
 * `reachable`-looking value without one is not an observation. The wire type
 * also admits `state: unknown` *with* an observation, which this backend never
 * produces; that value is presented as `not-observed` rather than guessed into a
 * cause, and is stated here rather than described as normal input.
 */
export type ServiceStatusView =
  | { kind: 'not-observed' }
  | { kind: 'available' }
  | { kind: 'account-unusable'; cause: SaveCause | null }
  | { kind: 'account-undetermined'; cause: SaveCause | null }
  | { kind: 'offline'; cause: SaveCause | null };

/** Derive the view from one observation (pure: no counting, no defaults). */
export function serviceStatusView(status: ServiceStatus): ServiceStatusView {
  if (status.observed_at === null) {
    return { kind: 'not-observed' };
  }

  if (status.state === 'unreachable') {
    return { kind: 'offline', cause: status.cause };
  }

  if (status.state === 'reachable') {
    if (status.account_valid === true) {
      return { kind: 'available' };
    }

    if (status.account_valid === false) {
      return { kind: 'account-unusable', cause: status.cause };
    }

    return { kind: 'account-undetermined', cause: status.cause };
  }

  // `unknown` with an observation: not produced by this backend (see above).
  return { kind: 'not-observed' };
}

/**
 * What a surface reads: pending, failed, or the view.
 *
 * The three are separate because a surface must not show "loading" forever on a
 * failed read. `get_service_status` has no product failure path, so a rejection
 * here is an IPC-level defect; it is `failed` with that error rather than a
 * permanent pending state.
 *
 * Data wins over a later refetch error on purpose: the last observation is still
 * the last thing the backend said, and `observed_at` says when, so the surface
 * keeps presenting it instead of dropping to an error (the same loaded-data
 * reasoning as SYNC-3, applied to the status itself).
 */
export type ServiceStatusViewResult =
  | { kind: 'pending' }
  | { kind: 'failed'; error: ServiceStatusFailure }
  | { kind: 'ready'; view: ServiceStatusView };

/** The view a component renders, with pending and failure kept distinct. */
export function useServiceStatusView(): ServiceStatusViewResult {
  const status = useServiceStatus();

  if (status.data !== undefined) {
    return { kind: 'ready', view: serviceStatusView(status.data) };
  }

  if (status.error !== null) {
    return { kind: 'failed', error: status.error };
  }

  return { kind: 'pending' };
}
