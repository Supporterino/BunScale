/*
 * The `household:changed` invalidation policy (task 3.6; SYNC-1, HS-4, DM-3).
 *
 * The event is a change report, not data: these specs pin which cached answers
 * each `kind` supersedes, and — as importantly — which ones it leaves alone.
 * The switch spec (task 3.3) already shows that invalidating
 * `householdKeys.all` would refetch collections a switch did not touch; the
 * collection assertions below are the event-layer counterpart of that rule.
 *
 * The listener itself is exercised through the real event bridge in
 * `src/features/app-events.test.tsx`; this spec calls the mapping directly so
 * each kind is checked without waiting on an asynchronous subscription.
 */
import type { QueryClient } from '@tanstack/react-query';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createTestQueryClient } from '../../test/setup';
import { applyHouseholdChange } from './events';
import { householdKeys } from './keys';
import type { Household, HouseholdSummary } from './queries';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

const HOUSEHOLD_A: Household = {
  id: 'household-a',
  name: 'Household A',
  created_at: AT,
  owner_member_id: 'member-a',
  is_active: true,
  is_owner: true,
};

const HOUSEHOLD_B: Household = {
  id: 'household-b',
  name: 'Household B',
  created_at: AT,
  owner_member_id: 'member-b',
  is_active: false,
  is_owner: false,
};

function summary(household: Household, isActive: boolean): HouseholdSummary {
  return {
    id: household.id,
    name: household.name,
    created_at: household.created_at,
    is_active: isActive,
    is_owner: household.is_owner,
    member_count: 1,
  };
}

/** Put a held set, an active household and two collections in the cache. */
function seed(client: QueryClient): void {
  client.setQueryData(householdKeys.list(), [
    summary(HOUSEHOLD_A, true),
    summary(HOUSEHOLD_B, false),
  ]);
  client.setQueryData(householdKeys.active(), HOUSEHOLD_A);
  client.setQueryData(householdKeys.collection(HOUSEHOLD_A.id), {
    household_id: HOUSEHOLD_A.id,
    items: [],
    source: 'local',
    loaded_at: AT,
  });
  client.setQueryData(householdKeys.collection(HOUSEHOLD_B.id), {
    household_id: HOUSEHOLD_B.id,
    items: [],
    source: 'local',
    loaded_at: AT,
  });
}

function isInvalidated(client: QueryClient, key: readonly unknown[]): boolean {
  return client.getQueryState(key)?.isInvalidated === true;
}

describe('a household:changed event', () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  for (const kind of ['created', 'switched'] as const) {
    it(`invalidates the held-set pair for ${kind} and no collection`, () => {
      const client = createTestQueryClient();

      seed(client);
      applyHouseholdChange(client, {
        household_id: HOUSEHOLD_B.id,
        kind,
      });

      expect(isInvalidated(client, householdKeys.list())).toBe(true);
      expect(isInvalidated(client, householdKeys.active())).toBe(true);
      expect(
        isInvalidated(client, householdKeys.collection(HOUSEHOLD_A.id)),
      ).toBe(false);
      expect(
        isInvalidated(client, householdKeys.collection(HOUSEHOLD_B.id)),
      ).toBe(false);
    });
  }

  it('drops the departed household’s collection on removed', () => {
    const client = createTestQueryClient();

    seed(client);
    applyHouseholdChange(client, {
      household_id: HOUSEHOLD_B.id,
      kind: 'removed',
    });

    expect(isInvalidated(client, householdKeys.list())).toBe(true);
    expect(isInvalidated(client, householdKeys.active())).toBe(true);

    // The rows are no longer held by this person, so they are removed rather
    // than left cached-but-stale for the rest of the process. The other
    // household's collection is not the departed one and stays cached.
    expect(
      client.getQueryState(householdKeys.collection(HOUSEHOLD_B.id)),
    ).toBeUndefined();
    expect(
      isInvalidated(client, householdKeys.collection(HOUSEHOLD_A.id)),
    ).toBe(false);
    expect(
      client.getQueryData(householdKeys.collection(HOUSEHOLD_A.id)),
    ).toBeDefined();
  });

  it('invalidates only the named household’s collection for refreshed', () => {
    const client = createTestQueryClient();

    seed(client);
    applyHouseholdChange(client, {
      household_id: HOUSEHOLD_B.id,
      kind: 'refreshed',
    });

    expect(
      isInvalidated(client, householdKeys.collection(HOUSEHOLD_B.id)),
    ).toBe(true);
    expect(
      isInvalidated(client, householdKeys.collection(HOUSEHOLD_A.id)),
    ).toBe(false);
    expect(isInvalidated(client, householdKeys.list())).toBe(false);
    expect(isInvalidated(client, householdKeys.active())).toBe(false);
  });

  it('never invalidates the whole household feature', () => {
    const client = createTestQueryClient();

    seed(client);

    for (const kind of [
      'created',
      'switched',
      'refreshed',
      'removed',
    ] as const) {
      applyHouseholdChange(client, {
        household_id: HOUSEHOLD_A.id,
        kind,
      });
    }

    // `householdKeys.all` would cover both collections; the collections are
    // each invalidated only by their own `refreshed` event.
    expect(
      isInvalidated(client, householdKeys.collection(HOUSEHOLD_B.id)),
    ).toBe(false);
  });
});
