/*
 * SET-2 at the frontend layer (task 3.5).
 *
 * The chain is the backend's, so the properties asserted here are the ones the
 * frontend could otherwise get wrong:
 *
 * - a `resolved: true` name is the account's, a `resolved: false` name is the
 *   local fallback, and `display_name: null` is **no name** — never replaced by
 *   an invented one;
 * - the last tier is the key `settings.identity.you`, not the copy. A hook that
 *   returned "You" would put English copy in a data module and make it
 *   untranslatable (task 3.7 owns the string); the assertion is exact, so a
 *   literal can only appear here by changing the key;
 * - an unresolved identity is a **ready** answer, not a failure (`get_identity`
 *   has no product failure path), and a broken read is reported as `failed`
 *   rather than silently presented as unresolved;
 * - an answer that already arrived survives a later refetch failure, so a broken
 *   refresh does not replace the name with an error.
 *
 * IPC is mocked at `invoke`, the seam `testing-strategy` prescribes for this
 * layer: the real `./api` wrapper and the generated `typedError` stay in the
 * path, so what arrives at the hook is what a mocked command answer actually
 * produces (a value, or the contract's failure object).
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  createTestQueryClient,
  TestQueryClientProvider,
} from '../../test/setup';
import type { Identity } from './identity';
import {
  IDENTITY_PLACEHOLDER_MESSAGE_KEY,
  identityDisplayName,
  identityQueryOptions,
  useIdentity,
  useIdentityName,
} from './identity';
import { settingsKeys } from './keys';

/** The account tier answered. */
const RESOLVED: Identity = {
  account_user_id: 'account-1',
  display_name: 'Ada',
  resolved: true,
};

/** The account has no display name; the locally known member name is shown. */
const LOCAL_FALLBACK: Identity = {
  account_user_id: 'account-1',
  display_name: 'Ada Lovelace',
  resolved: false,
};

/** P0's honest answer: no account and no attributable local member (Q25). */
const UNRESOLVED: Identity = {
  account_user_id: null,
  display_name: null,
  resolved: false,
};

describe('SET-2’s name chain', () => {
  it('reports the account tier as the account’s name', () => {
    expect(identityDisplayName(RESOLVED)).toEqual({
      kind: 'name',
      name: 'Ada',
      source: 'account',
    });
  });

  it('reports the locally known name as the fallback, not as resolved', () => {
    expect(identityDisplayName(LOCAL_FALLBACK)).toEqual({
      kind: 'name',
      name: 'Ada Lovelace',
      source: 'local',
    });
  });

  it('hands the last tier over as a key, with no invented name', () => {
    const displayName = identityDisplayName(UNRESOLVED);

    expect(displayName).toEqual({
      kind: 'placeholder',
      messageKey: IDENTITY_PLACEHOLDER_MESSAGE_KEY,
    });
    expect('name' in displayName).toBe(false);
    expect(IDENTITY_PLACEHOLDER_MESSAGE_KEY).toBe('settings.identity.you');
  });

  it('treats a resolved flag without a name as no name', () => {
    // The backend never pairs these, but the wire type admits it. The frontend
    // can see the absent name, not a reason to guess one.
    expect(identityDisplayName({ ...UNRESOLVED, resolved: true })).toEqual({
      kind: 'placeholder',
      messageKey: IDENTITY_PLACEHOLDER_MESSAGE_KEY,
    });
  });
});

describe('the identity read', () => {
  let answers: Array<() => unknown>;
  let invoked: string[];

  beforeEach(() => {
    answers = [];
    invoked = [];

    mockIPC((command) => {
      invoked.push(command);

      const answer = answers.shift();

      if (answer === undefined) {
        throw new Error(`unexpected mocked command "${command}"`);
      }

      return answer();
    });
  });

  afterEach(() => {
    clearMocks();
  });

  it('caches the identity the backend resolved', async () => {
    answers.push(() => UNRESOLVED);

    const client = createTestQueryClient();
    const identity = await client.fetchQuery(identityQueryOptions());

    expect(identity).toEqual(UNRESOLVED);
    expect(invoked).toEqual(['get_identity']);
  });

  it('presents an unresolved identity as ready with the placeholder', async () => {
    answers.push(() => UNRESOLVED);

    const client = createTestQueryClient();
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(() => useIdentityName(), { wrapper });

    expect(result.current).toEqual({ kind: 'pending' });

    await waitFor(() => {
      expect(result.current).toEqual({
        kind: 'ready',
        identity: UNRESOLVED,
        displayName: {
          kind: 'placeholder',
          messageKey: 'settings.identity.you',
        },
      });
    });

    // SET-2: an unresolved identity is a normal answer, not a failure — the
    // rest of Settings never sees an error from it, and the backend's
    // `resolved: false` reaches the screen untouched.
    expect(result.current.kind).not.toBe('failed');

    if (result.current.kind === 'ready') {
      expect(result.current.identity.resolved).toBe(false);
      expect(result.current.identity.display_name).toBeNull();
    }
  });

  it('keeps the last resolved name when a later refetch fails', async () => {
    answers.push(
      () => RESOLVED,
      () => {
        throw {
          cause: 'unknown',
          retryable: false,
          message_key: 'error.unknown',
          request_id: null,
        };
      },
    );

    const client = createTestQueryClient();
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, { client, children });
    const { result } = renderHook(
      () => ({ name: useIdentityName(), identity: useIdentity() }),
      { wrapper },
    );

    const resolved = {
      kind: 'ready',
      identity: RESOLVED,
      displayName: {
        kind: 'name',
        name: 'Ada',
        source: 'account',
      },
    };

    await waitFor(() => {
      expect(result.current.name).toEqual(resolved);
    });

    await act(async () => {
      await result.current.identity.refetch();
    });

    // The later read broke, but the answer that already arrived is still the
    // backend's last word: the name must not drop to `failed`.
    expect(client.getQueryState(settingsKeys.identity())?.error).not.toBeNull();

    await waitFor(() => {
      expect(result.current.identity.error).not.toBeNull();
    });

    expect(result.current.identity.isError).toBe(true);
    expect(result.current.name).toEqual(resolved);
  });

  it('reports a broken read instead of pretending the name is unresolved', async () => {
    answers.push(() => {
      throw {
        cause: 'unknown',
        retryable: false,
        message_key: 'error.unknown',
        request_id: null,
      };
    });

    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(TestQueryClientProvider, {
        client: createTestQueryClient(),
        children,
      });
    const { result } = renderHook(() => useIdentityName(), { wrapper });

    await waitFor(() => {
      expect(result.current.kind).toBe('failed');
    });

    if (result.current.kind === 'failed') {
      expect(result.current.error).toMatchObject({ cause: 'unknown' });
    }
  });
});
