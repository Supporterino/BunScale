/*
 * The household entry guard's one status dependency (tasks 3.3 and 3.4; SYNC-2,
 * SYNC-3, NFR-1).
 *
 * Task 3.3 kept this module as the guard's seam and said task 3.4 would replace
 * its body; this is that replacement. The read-and-probe logic now lives in the
 * status store (`./status`), so the guard consumes the same cached observation
 * and triggers the same single probe as every surface instead of reading the
 * tracker behind the store's back. What the seam still guarantees, and what the
 * guard relies on, is unchanged: the tracker is read first (a local, network-free
 * command), a probe happens only when nothing has been observed in this process,
 * and a failed read is thrown rather than turned into a status the backend never
 * produced.
 *
 * The probe's 5-second bound and the three-failure offline threshold are the
 * backend's (D0.13); nothing here counts or upgrades them.
 */
import type { QueryClient } from '@tanstack/react-query';
import type { ServiceStatus } from './status';
import { readCurrentServiceStatus } from './status';

export type { ServiceStatus } from './status';

/** The status the entry guard decides on (see `./status`). */
export async function readEntryServiceStatus(
  queryClient: QueryClient,
): Promise<ServiceStatus> {
  return readCurrentServiceStatus(queryClient);
}
