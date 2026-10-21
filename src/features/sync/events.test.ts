/*
 * The `service:status-changed` cache write (task 3.6; NFR-1, R3).
 *
 * The event carries the observation itself, so the hook writes it into
 * `syncKeys.status()` rather than asking the backend again. The spec also pins
 * the isolation rule: a status event says nothing about household data and must
 * leave it alone.
 *
 * The subscription itself is exercised in `src/features/app-events.test.tsx`.
 */
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createTestQueryClient } from '../../test/setup';
import { householdKeys } from '../households/keys';
import { applyServiceStatus } from './events';
import { syncKeys } from './keys';
import type { ServiceStatus } from './status';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/** Nothing observed in this process yet (R3). */
const UNOBSERVED: ServiceStatus = {
  state: 'unknown',
  account_valid: null,
  cause: null,
  observed_at: null,
};

/** An answered probe that identified the account. */
const AVAILABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

describe('a service:status-changed event', () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it('writes the observed status into the status cache', () => {
    const client = createTestQueryClient();

    client.setQueryData(syncKeys.status(), UNOBSERVED);
    applyServiceStatus(client, AVAILABLE);

    expect(client.getQueryData(syncKeys.status())).toEqual(AVAILABLE);
  });

  it('leaves household data and its invalidation state untouched', () => {
    const client = createTestQueryClient();

    client.setQueryData(householdKeys.list(), []);
    client.setQueryData(householdKeys.active(), null);
    client.setQueryData(householdKeys.collection('household-a'), {
      household_id: 'household-a',
      items: [],
      source: 'local',
      loaded_at: AT,
    });

    applyServiceStatus(client, AVAILABLE);

    expect(client.getQueryData(householdKeys.list())).toEqual([]);
    expect(client.getQueryData(householdKeys.active())).toBeNull();
    expect(
      client.getQueryState(householdKeys.collection('household-a'))
        ?.isInvalidated,
    ).toBe(false);
  });
});
