/*
 * Sync and reachability commands (SYNC-1, SYNC-2, SYNC-3, NFR-1): the last
 * observed service/account status and a fresh reachability probe. The only
 * module that imports these generated bindings.
 *
 * Neither call is a state-changing operation, so neither takes a request id:
 * `check_service_reachability` is idempotent and a retry is simply a fresh call,
 * which is the contract's `request_id: null` case (design.md D0.14).
 *
 * Unreachability is not a command failure either — it is `state: unreachable`,
 * and `unknown` (with every observation absent) until a fresh outcome exists,
 * never a value carried over from an earlier session (R3). Task 3.4 holds the
 * status; task 3.6 subscribes to `service:status-changed` here and writes the
 * observation through `./events`.
 *
 * The subscription is in this module because the generated event descriptor is
 * part of the same bindings seam: project hard rule 2 has no event exception,
 * so this stays the feature's one importer of `src/bindings.ts` and `./events`
 * holds only the cache policy.
 */

import type { UnlistenFn } from '@tauri-apps/api/event';
import type { CommandError, ServiceStatus } from '../../bindings';
import { commands, events } from '../../bindings';
import type { CommandResult } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/**
 * The last observed service/account status, with when it was observed (NFR-1,
 * R3).
 *
 * Makes no network call, so reading it cannot change what it reports. It is a
 * current observation only while `observed_at` is present; while the state is
 * `unknown`, the household reads prove nothing on their own (SYNC-3).
 */
export function getServiceStatus(): Outcome<ServiceStatus> {
  return commands.getServiceStatus();
}

/**
 * Run a fresh reachability probe, record it and return it (SYNC-2, NFR-1).
 *
 * The probe checks the service and the account separately and is bounded by
 * `REACHABILITY_PROBE_TIMEOUT`; the offline state is reported only after three
 * consecutive failures without an intervening success. Both bounds live in Rust
 * (design.md D0.13), and a failed probe is a status rather than a command
 * failure (SYNC-3). The outcome is published as `service:status-changed` before
 * this returns.
 */
export function checkServiceReachability(): Outcome<ServiceStatus> {
  return commands.checkServiceReachability();
}

/**
 * Subscribe to `service:status-changed` (task 3.6; NFR-1, R3).
 *
 * The generated event descriptor is imported here because project hard rule 2
 * makes `api.ts` the only module per feature that may touch `src/bindings.ts`;
 * `./events` applies the payload to the cache and imports no generated code.
 * The handler receives the contract's `ServiceStatus`, not Tauri's event
 * envelope, so the policy cannot depend on the bridge's shape.
 *
 * The returned promise is the Tauri `listen` promise; the caller keeps it for
 * cleanup (convention 4).
 */
export function subscribeToServiceStatusChanges(
  handler: (status: ServiceStatus) => void,
): Promise<UnlistenFn> {
  return events.serviceStatusChanged.listen((event) => {
    handler(event.payload);
  });
}
