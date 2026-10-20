/*
 * The household entry guard's one status dependency (task 3.3; SYNC-2, SYNC-3,
 * NFR-1).
 *
 * The guard may start first-run onboarding only when a **fresh** `ServiceStatus`
 * reports `state: reachable` and `account_valid: true` — "no household" alone
 * proves nothing, because it is also what an unreachable service with no local
 * copy looks like (design.md, "No household is a fresh-outcome conclusion,
 * never an absence of data"). This module is where that one fact is read, so
 * the dependency is explicit: task 3.4's status store replaces this body — its
 * observation, its three-failure threshold and its `service:status-changed`
 * subscription — without the guard or the routes changing.
 *
 * What "fresh" means here is R3's definition and nothing stronger: a status is
 * an observation only while `observed_at` is present, and a status is never
 * carried over from an earlier session (the backend starts each process
 * `unknown`). A status that has no observation yet is therefore probed once,
 * through `check_service_reachability`, whose 5-second bound is D0.13's.
 * Reusing a later observation without re-probing is deliberate: a probe is a
 * network round trip, and turning a route navigation into one would make the
 * guard as slow as the service. Refining whether an observation is still *recent
 * enough* is task 3.4's, and it can do so here.
 */
import type { CommandData } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { checkServiceReachability, getServiceStatus } from './api';

/** The last observed service/account status — `ServiceStatus` in the contract. */
export type ServiceStatus = CommandData<
  Awaited<ReturnType<typeof getServiceStatus>>
>;

/**
 * The status the entry guard decides on: the last observation, or one fresh
 * probe when nothing has been observed in this process yet.
 *
 * A failed read is not converted into an `unknown` status here. "We have not
 * looked" is a real state (`unknown` with every observation absent), and
 * synthesising one would put a value the backend never produced in the guard's
 * hands; the failure is thrown instead, so the route's error surface shows it
 * rather than the guard guessing (R3).
 */
export async function readEntryServiceStatus(): Promise<ServiceStatus> {
  const observed = unwrapCommandResult(await getServiceStatus());

  if (observed.observed_at !== null) {
    return observed;
  }

  return unwrapCommandResult(await checkServiceReachability());
}
