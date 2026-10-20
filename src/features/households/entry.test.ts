/*
 * Task 3.3's evidence for the entry rule (HS-2, DM-3, SYNC-3).
 *
 * The rule the contract states is that "no household" is a *conclusion*, not an
 * absence of data, so the decision has two halves and each is asserted here:
 *
 * - a household in the device's book skips onboarding whatever the service says;
 * - onboarding starts only on a **fresh** status that is reachable *and* carries
 *   a usable account — an unobserved status, a failed probe and a `401` all
 *   resolve to the connectivity slot instead.
 *
 * `resolveHouseholdEntry` is exercised through mocked IPC rather than by mocking
 * `./api`: the route tree imports this module eagerly through
 * `src/test/setup.ts`, so a `vi.mock` of the wrapper would register after the
 * module graph was built. Mocking `invoke` is the seam `testing-strategy`
 * prescribes for this layer, and it lets the spec assert the load order that
 * matters — the service is never consulted when the book already has a
 * household.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createTestQueryClient } from '../../test/setup';
import type { ServiceStatus } from '../sync/entry-status';
import { householdEntryDecision, resolveHouseholdEntry } from './entry';
import type { Household, HouseholdSummary } from './queries';

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

/** Before any probe has completed in this process (R3). */
const UNOBSERVED: ServiceStatus = {
  state: 'unknown',
  account_valid: null,
  cause: null,
  observed_at: null,
};

/** A reachable service with an identified account — the only onboarding answer. */
const REACHABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: true,
  cause: null,
  observed_at: AT,
};

/** Three failed probes; the backend applies the threshold (D0.13). */
const UNREACHABLE: ServiceStatus = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

/** The service answered but refused the account (D0.7 criterion 4). */
const NO_ACCOUNT: ServiceStatus = {
  state: 'reachable',
  account_valid: false,
  cause: 'no_account',
  observed_at: AT,
};

const NO_PRESENCE = { households: [], activeHousehold: null } as const;

describe('the household entry decision', () => {
  it('starts onboarding only on a fresh reachable status with a usable account', () => {
    expect(householdEntryDecision(NO_PRESENCE, REACHABLE)).toEqual({
      kind: 'onboarding',
    });
  });

  it('skips onboarding whenever a household is held, whatever the service says', () => {
    const presence = {
      households: [SUMMARY],
      activeHousehold: HOUSEHOLD,
    };

    expect(householdEntryDecision(presence, UNREACHABLE)).toEqual({
      kind: 'household',
    });
    expect(householdEntryDecision(presence, UNOBSERVED)).toEqual({
      kind: 'household',
    });
  });

  it('treats a non-null active household as a held household', () => {
    expect(
      householdEntryDecision(
        { households: [], activeHousehold: HOUSEHOLD },
        REACHABLE,
      ),
    ).toEqual({ kind: 'household' });
  });

  it('never onboards without an observation', () => {
    expect(householdEntryDecision(NO_PRESENCE, UNOBSERVED)).toEqual({
      kind: 'service-required',
      status: UNOBSERVED,
    });

    /*
     * The first failed probe records an observation without reaching the
     * three-failure threshold, so `state` is still `unknown` with a cause: it
     * must not be read as "reachable" just because something was observed.
     */
    const failedProbe: ServiceStatus = {
      ...UNOBSERVED,
      cause: 'connectivity',
      observed_at: AT,
    };

    expect(householdEntryDecision(NO_PRESENCE, failedProbe)).toEqual({
      kind: 'service-required',
      status: failedProbe,
    });
  });

  it('never onboards when the account is not usable', () => {
    expect(householdEntryDecision(NO_PRESENCE, NO_ACCOUNT)).toEqual({
      kind: 'service-required',
      status: NO_ACCOUNT,
    });

    const unknownAccount: ServiceStatus = { ...REACHABLE, account_valid: null };

    expect(householdEntryDecision(NO_PRESENCE, unknownAccount)).toEqual({
      kind: 'service-required',
      status: unknownAccount,
    });
  });

  it('never onboards when no status was read at all', () => {
    expect(householdEntryDecision(NO_PRESENCE, null)).toEqual({
      kind: 'service-required',
      status: null,
    });
  });
});

describe('resolving the entry', () => {
  let invoked: string[];
  let answers: Map<string, () => unknown>;

  beforeEach(() => {
    invoked = [];
    answers = new Map();

    mockIPC((command) => {
      invoked.push(command);

      const answer = answers.get(command);

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('answers from the book without reading the service', async () => {
    answers.set('list_households', () => [SUMMARY]);
    answers.set('get_active_household', () => HOUSEHOLD);

    const entry = await resolveHouseholdEntry(createTestQueryClient());

    expect(entry).toEqual({ kind: 'household' });
    expect(invoked).toContain('list_households');
    expect(invoked).not.toContain('get_service_status');
    expect(invoked).not.toContain('check_service_reachability');
  });

  it('probes once when nothing was observed, and onboards on a reachable account', async () => {
    answers.set('list_households', () => []);
    answers.set('get_active_household', () => null);
    answers.set('get_service_status', () => UNOBSERVED);
    answers.set('check_service_reachability', () => REACHABLE);

    const entry = await resolveHouseholdEntry(createTestQueryClient());

    expect(entry).toEqual({ kind: 'onboarding' });
    expect(invoked).toEqual([
      'list_households',
      'get_active_household',
      'get_service_status',
      'check_service_reachability',
    ]);
  });

  it('does not probe a status that already carries an observation', async () => {
    answers.set('list_households', () => []);
    answers.set('get_active_household', () => null);
    answers.set('get_service_status', () => UNREACHABLE);

    const entry = await resolveHouseholdEntry(createTestQueryClient());

    expect(entry).toEqual({ kind: 'service-required', status: UNREACHABLE });
    expect(invoked).not.toContain('check_service_reachability');
  });

  it('re-reads the book for every decision, so a household created since is seen', async () => {
    const client = createTestQueryClient();

    answers.set('list_households', () => []);
    answers.set('get_active_household', () => null);
    answers.set('get_service_status', () => UNOBSERVED);
    answers.set('check_service_reachability', () => REACHABLE);

    await expect(resolveHouseholdEntry(client)).resolves.toEqual({
      kind: 'onboarding',
    });

    /*
     * The person creates a household: the book now holds one, and nothing else
     * about this process changes — no remount, no invalidation. The second
     * decision must read the book again rather than trust the cache that still
     * holds `[]`/`null`, or HS-2's onboarding would be presented although a
     * household exists.
     */
    answers.set('list_households', () => [SUMMARY]);
    answers.set('get_active_household', () => HOUSEHOLD);

    await expect(resolveHouseholdEntry(client)).resolves.toEqual({
      kind: 'household',
    });
  });

  it('never onboards when the book cannot be read', async () => {
    answers.set('list_households', () => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });
    answers.set('get_active_household', () => null);

    await expect(
      resolveHouseholdEntry(createTestQueryClient()),
    ).rejects.toMatchObject({ cause: 'unknown' });

    expect(invoked).not.toContain('get_service_status');
    expect(invoked).not.toContain('check_service_reachability');
  });
});
