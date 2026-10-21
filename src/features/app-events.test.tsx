/*
 * The single event mount (task 3.6; SYNC-1, NFR-1, R3).
 *
 * `useAppEvents()` is the one place the app subscribes, so this spec proves the
 * three things the shell will rely on:
 *
 * - both backend events reach the cache through the real generated event
 *   descriptors and the real Tauri event bridge (mocked at the plugin seam, not
 *   by replacing the events module);
 * - a `household:changed` invalidates only the held-set pair, while a
 *   `service:status-changed` writes the status;
 * - unmounting releases both listeners, so a later event changes nothing.
 *
 * `mockIPC(..., { shouldMockEvents: true })` is the library's event mock: it
 * intercepts `plugin:event|listen`, `|emit` and `|unlisten` itself, so `emit`
 * from `@tauri-apps/api/event` drives the same callbacks the backend would.
 * Any other command reaching the mock is a defect and fails the test.
 */
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createTestQueryClient, TestQueryClientProvider } from '../test/setup';
import { useAppEvents } from './app-events';
import { householdKeys } from './households/keys';
import type { Household, HouseholdSummary } from './households/queries';
import { syncKeys } from './sync/keys';
import type { ServiceStatus } from './sync/status';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const SUMMARY: HouseholdSummary = {
  id: HOUSEHOLD.id,
  name: HOUSEHOLD.name,
  created_at: HOUSEHOLD.created_at,
  is_active: true,
  is_owner: true,
  member_count: 1,
};

const UNOBSERVED: ServiceStatus = {
  state: 'unknown',
  account_valid: null,
  cause: null,
  observed_at: null,
};

const AVAILABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** Let the resolved `listen` promises reach their cleanup handlers. */
async function flush(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe('useAppEvents', () => {
  beforeEach(() => {
    mockIPC(
      () => {
        throw new Error('only event plugin commands are expected here');
      },
      { shouldMockEvents: true },
    );
  });

  afterEach(() => {
    clearMocks();
  });

  it('routes both events to their caches and releases both listeners on unmount', async () => {
    const client = createTestQueryClient();

    client.setQueryData(householdKeys.list(), [SUMMARY]);
    client.setQueryData(householdKeys.active(), HOUSEHOLD);
    client.setQueryData(householdKeys.collection(HOUSEHOLD.id), {
      household_id: HOUSEHOLD.id,
      items: [],
      source: 'local',
      loaded_at: AT,
    });
    client.setQueryData(syncKeys.status(), UNOBSERVED);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { unmount } = renderHook(() => useAppEvents(), { wrapper });

    await flush();

    await act(async () => {
      await emit('household:changed', {
        household_id: HOUSEHOLD.id,
        kind: 'switched',
      });
    });

    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      true,
    );
    expect(client.getQueryState(householdKeys.active())?.isInvalidated).toBe(
      true,
    );
    // A switch does not reload any collection (the rule `householdKeys.all`
    // would break).
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD.id))
        ?.isInvalidated,
    ).toBe(false);

    await act(async () => {
      await emit('service:status-changed', AVAILABLE);
    });

    expect(client.getQueryData(syncKeys.status())).toEqual(AVAILABLE);

    unmount();
    await flush();

    // A new event after unmount must reach nobody: both subscriptions were
    // released, so neither cache entry moves.
    client.setQueryData(syncKeys.status(), UNOBSERVED);
    client.setQueryData(householdKeys.collection('household-c'), {
      household_id: 'household-c',
      items: [],
      source: 'local',
      loaded_at: AT,
    });

    await act(async () => {
      await emit('service:status-changed', AVAILABLE);
      await emit('household:changed', {
        household_id: 'household-c',
        kind: 'refreshed',
      });
    });

    expect(client.getQueryData(syncKeys.status())).toEqual(UNOBSERVED);
    expect(
      client.getQueryState(householdKeys.collection('household-c'))
        ?.isInvalidated,
    ).toBe(false);
  });

  it('survives a failing event bridge without touching either cache', async () => {
    const client = createTestQueryClient();

    client.setQueryData(householdKeys.list(), [SUMMARY]);
    client.setQueryData(householdKeys.active(), HOUSEHOLD);
    client.setQueryData(householdKeys.collection(HOUSEHOLD.id), {
      household_id: HOUSEHOLD.id,
      items: [],
      source: 'local',
      loaded_at: AT,
    });
    client.setQueryData(syncKeys.status(), UNOBSERVED);

    // `shouldMockEvents` is off here, so `plugin:event|listen` reaches this
    // handler — and rejects. That is the bridge failing, not a product state.
    mockIPC((command) => {
      throw new Error(`event bridge failure: ${command}`);
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { unmount } = renderHook(() => useAppEvents(), { wrapper });

    // A rejected subscription must not become an unhandled rejection: the
    // handler is attached where the subscription is created, not only at
    // unmount.
    await flush();

    expect(client.getQueryData(syncKeys.status())).toEqual(UNOBSERVED);
    expect(client.getQueryData(householdKeys.list())).toEqual([SUMMARY]);
    expect(client.getQueryState(householdKeys.list())?.isInvalidated).toBe(
      false,
    );
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD.id))
        ?.isInvalidated,
    ).toBe(false);

    unmount();
    await flush();
  });
});
