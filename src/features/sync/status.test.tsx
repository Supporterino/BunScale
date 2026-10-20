/*
 * Task 3.4's evidence for the status store (NFR-1, R3, SYNC-2, SYNC-3).
 *
 * The view is asserted against the shapes the backend's tracker actually
 * produces (`src-tauri/src/sync/status.rs`), not against a middle ground it
 * cannot produce:
 *
 * - `not-observed` is decided by `observed_at` alone (R3);
 * - with an observation, `reachable` splits into available / account-unusable
 *   (`false`) / account-undetermined (`null`: rate limited or unreadable, which
 *   is not "refused");
 * - `offline` is only the backend's `unreachable`, after three unanswered
 *   probes. A single failed check leaves the status untouched, so it reads as
 *   the last answered probe — never as offline.
 *
 * IPC is mocked at `invoke` because the route tree loads these modules eagerly
 * through `src/test/setup.ts` (`testing-strategy` prescribes this seam).
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import { syncKeys } from './keys';
import type { ServiceStatus } from './status';
import {
  probeServiceReachability,
  readCurrentServiceStatus,
  serviceStatusView,
  useServiceStatus,
  useServiceStatusView,
} from './status';

/** Stands in for the contract's `Instant`. */
const AT = '2026-10-20T09:00:00Z';

/**
 * Before any probe has completed in this process (R3) — and also what a first
 * *failed* check leaves behind, because the tracker does not touch the status
 * below its threshold.
 */
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

/** The service answered and refused the account (401). */
const NO_ACCOUNT: ServiceStatus = {
  state: 'reachable',
  account_valid: false,
  cause: 'no_account',
  observed_at: AT,
};

/** The account check was rate limited: no account outcome exists. */
const RATE_LIMITED: ServiceStatus = {
  state: 'reachable',
  account_valid: null,
  cause: 'rate_limited',
  observed_at: AT,
};

/** The account answer was unreadable, which puts no cause on the status. */
const UNREADABLE: ServiceStatus = {
  state: 'reachable',
  account_valid: null,
  cause: null,
  observed_at: AT,
};

/** Three consecutive unanswered probes without an intervening success (D0.13). */
const OFFLINE: ServiceStatus = {
  state: 'unreachable',
  account_valid: null,
  cause: 'connectivity',
  observed_at: AT,
};

describe('the status view', () => {
  it('reports not-observed until an observation exists', () => {
    expect(serviceStatusView(UNOBSERVED)).toEqual({ kind: 'not-observed' });

    // R3: a reachable-looking value without an observation is not an
    // observation — the marker is `observed_at`, nothing else.
    expect(serviceStatusView({ ...AVAILABLE, observed_at: null })).toEqual({
      kind: 'not-observed',
    });
  });

  it('reports availability only for a positively identified account', () => {
    expect(serviceStatusView(AVAILABLE)).toEqual({ kind: 'available' });
  });

  it('separates a refused account from an undetermined one', () => {
    expect(serviceStatusView(NO_ACCOUNT)).toEqual({
      kind: 'account-unusable',
      cause: 'no_account',
    });
    expect(serviceStatusView(RATE_LIMITED)).toEqual({
      kind: 'account-undetermined',
      cause: 'rate_limited',
    });
    expect(serviceStatusView(UNREADABLE)).toEqual({
      kind: 'account-undetermined',
      cause: null,
    });
  });

  it('presents offline only for the backend’s unreachable', () => {
    expect(serviceStatusView(OFFLINE)).toEqual({
      kind: 'offline',
      cause: 'connectivity',
    });
  });

  it('does not guess a cause for an observed unknown the backend never produces', () => {
    // The wire type admits this; the tracker does not produce it (see the view's
    // doc). It is presented as nothing-observed rather than as a cause.
    expect(serviceStatusView({ ...UNOBSERVED, observed_at: AT })).toEqual({
      kind: 'not-observed',
    });
  });
});

describe('the status store', () => {
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

  it('probes when nothing has been observed, and caches the probe', async () => {
    const client = createTestQueryClient();

    answers.set('get_service_status', () => UNOBSERVED);
    answers.set('check_service_reachability', () => UNOBSERVED);

    const status = await readCurrentServiceStatus(client);

    expect(status).toEqual(UNOBSERVED);
    expect(invoked).toContain('check_service_reachability');
    expect(client.getQueryData(syncKeys.status())).toEqual(UNOBSERVED);
  });

  it('does not probe a status that already carries an observation', async () => {
    const client = createTestQueryClient();

    answers.set('get_service_status', () => AVAILABLE);

    const status = await readCurrentServiceStatus(client);

    expect(status).toEqual(AVAILABLE);
    expect(invoked).not.toContain('check_service_reachability');
  });

  it('keeps the backend status as it is, so one failure is not upgraded', async () => {
    const client = createTestQueryClient();

    answers.set('get_service_status', () => UNOBSERVED);
    answers.set('check_service_reachability', () => UNOBSERVED);

    const status = await readCurrentServiceStatus(client);

    expect(status.state).toBe('unknown');
    expect(serviceStatusView(status)).toEqual({ kind: 'not-observed' });
    expect(client.getQueryData(syncKeys.status())).toEqual(UNOBSERVED);
  });

  it('writes a probe result into the cache for every surface', async () => {
    const client = createTestQueryClient();

    answers.set('check_service_reachability', () => OFFLINE);

    await probeServiceReachability(client);

    expect(client.getQueryData(syncKeys.status())).toEqual(OFFLINE);
  });

  it('treats a below-threshold probe answer as the last observation, not offline', async () => {
    const client = createTestQueryClient();

    /*
     * A probe whose checks got no answer leaves the tracker's status untouched
     * (`src-tauri/src/sync/status.rs:92-103`), so what comes back here is the
     * retained prior success: only the third consecutive failure reports
     * `unreachable`.
     */
    answers.set('check_service_reachability', () => AVAILABLE);

    const status = await probeServiceReachability(client);
    const view = serviceStatusView(status);

    expect(view.kind).not.toBe('offline');
    expect(view).toEqual({ kind: 'available' });
    expect(client.getQueryData(syncKeys.status())).toEqual(AVAILABLE);
  });

  it('reports a pending read as pending, then the view', async () => {
    const client = createTestQueryClient();

    answers.set('get_service_status', () => AVAILABLE);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useServiceStatusView(), { wrapper });

    expect(result.current).toEqual({ kind: 'pending' });

    await waitFor(() => {
      expect(result.current).toEqual({
        kind: 'ready',
        view: { kind: 'available' },
      });
    });
  });

  it('reports a failed read instead of loading forever', async () => {
    const client = createTestQueryClient();

    answers.set('get_service_status', () => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useServiceStatusView(), { wrapper });

    await waitFor(() => {
      expect(result.current.kind).toBe('failed');
    });

    if (result.current.kind === 'failed') {
      expect(result.current.error).toMatchObject({ cause: 'unknown' });
    }
  });

  it('keeps presenting the last observation when a later read fails', async () => {
    const client = createTestQueryClient();
    const failingRead = vi.fn(() => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });

    answers.set('get_service_status', () => AVAILABLE);

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(
      () => ({ view: useServiceStatusView(), status: useServiceStatus() }),
      { wrapper },
    );

    await waitFor(() => {
      expect(result.current.view).toEqual({
        kind: 'ready',
        view: { kind: 'available' },
      });
    });

    answers.set('get_service_status', failingRead);

    await act(async () => {
      await result.current.status.refetch();
    });

    // The observation is still the last thing the backend said; `observed_at`
    // says when, so a failed read does not replace it with an error screen.
    expect(failingRead).toHaveBeenCalled();
    expect(result.current.view).toEqual({
      kind: 'ready',
      view: { kind: 'available' },
    });
  });
});
