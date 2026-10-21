/*
 * The P0 catalog's contract with its callers (task 3.7; SET-2, NFR-8).
 *
 * Three properties are asserted, and each fails in the way the scaffold needs
 * it to:
 *
 * - **every `SaveCause` has copy** — the iteration is typed with the store's
 *   own `SaveMessageKey`, so a cause whose key is missing from the catalog also
 *   fails `bun run typecheck`, not only this test. The messages must be
 *   non-empty and distinct, so "the specific cause" of SYNC-2's scenario is
 *   actually named rather than seven synonyms for "failed".
 * - **the Settings placeholder resolves** — `t(IDENTITY_PLACEHOLDER_MESSAGE_KEY)`
 *   is passed the key the 3.5 hook emits; a rename on either side breaks
 *   compilation here before a screen can render a key.
 * - **an unknown key cannot render raw** — the only route to one is a cast, and
 *   the lookup answers the catalogued generic message instead of echoing it.
 */
import { describe, expect, it } from 'vitest';
import { IDENTITY_PLACEHOLDER_MESSAGE_KEY } from '../settings/identity';
import type { SaveMessageKey } from '../sync/failure';
import { SAVE_CAUSE_MESSAGE_KEYS } from '../sync/failure';
import type { MessageKey } from './index';
import { P0_MESSAGES, t } from './index';

/** Every key the store can emit, taken from the store's own map. */
const FAILURE_KEYS: readonly SaveMessageKey[] = Object.values(
  SAVE_CAUSE_MESSAGE_KEYS,
);

describe('the P0 message catalog', () => {
  it('answers every failure cause with distinct, non-empty copy', () => {
    expect(FAILURE_KEYS.length).toBeGreaterThan(0);

    const messages = FAILURE_KEYS.map((key) => {
      const message = t(key);

      expect(message.trim().length).toBeGreaterThan(0);
      expect(message).not.toBe(key);

      return message;
    });

    expect(new Set(messages).size).toBe(messages.length);
  });

  it('names the exhausted allowance as its own cause, not a generic failure', () => {
    const allowance = t(SAVE_CAUSE_MESSAGE_KEYS.allowance_exceeded);

    expect(allowance).toContain('storage');
    expect(allowance).not.toBe(t(SAVE_CAUSE_MESSAGE_KEYS.unknown));
  });

  it('resolves the placeholder key the Settings hook emits', () => {
    expect(t(IDENTITY_PLACEHOLDER_MESSAGE_KEY)).toBe('You');
  });

  it('never uses a key itself as copy', () => {
    for (const [key, message] of Object.entries(P0_MESSAGES)) {
      expect(message.trim().length).toBeGreaterThan(0);
      expect(message).not.toBe(key);
    }
  });

  it('cannot render an unknown key raw', () => {
    const unknown = 'settings.does.not.exist' as MessageKey;

    expect(t(unknown)).toBe('Something went wrong.');
    expect(t(unknown)).not.toBe(unknown);
    expect(t(unknown)).not.toContain('settings.');
  });

  it('rejects an uncatalogued literal at compile time', () => {
    // @ts-expect-error - `settings.not.a.key` is not a MessageKey.
    expect(t('settings.not.a.key')).toBe('Something went wrong.');
  });
});
