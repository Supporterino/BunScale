/*
 * The `service:status-changed` cache policy (task 3.6; NFR-1, R3, SYNC-2).
 *
 * The event carries the whole `ServiceStatus` the backend just observed, not a
 * request to re-read it, so the right cache operation is a **write**, not an
 * invalidation: `syncKeys.status()` receives the payload verbatim and every
 * surface reading the status store shows the observation the backend recorded.
 * The backend publishes the event before the command that produced it returns
 * (see `check_service_reachability`'s contract), so the event and the command's
 * answer carry the same value; writing it here costs no second read.
 *
 * It must **not** touch household data: a service or account observation says
 * nothing about a household's rows, and invalidating them would make every
 * status change refetch collections (NFR-1's "only the affected queries").
 * Sources of the status — a probe, a foreground return, a window regaining
 * focus — all record on the backend's tracker and publish through this one
 * event, so this module is the only live path into the status cache.
 *
 * This module is **pure policy**: the subscription lives in `./api.ts`, the
 * feature's only generated-bindings seam (project hard rule 2). Nothing here
 * imports `src/bindings.ts` or knows Tauri's event envelope; `api.ts` unwraps
 * the payload and hands this function the contract's `ServiceStatus`.
 */
import type { QueryClient } from '@tanstack/react-query';
import { syncKeys } from './keys';
import type { ServiceStatus } from './status';

/**
 * Record one observed status into the shared cache.
 *
 * Separate from the listener so the write policy can be asserted without an
 * event bridge. The payload is the backend's own value; nothing is defaulted,
 * merged with an older observation or re-timestamped here — that would let the
 * frontend claim a freshness the tracker never reported (R3).
 */
export function applyServiceStatus(
  queryClient: QueryClient,
  status: ServiceStatus,
): void {
  queryClient.setQueryData(syncKeys.status(), status);
}
