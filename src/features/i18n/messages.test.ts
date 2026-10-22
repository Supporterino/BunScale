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
 *
 * Task 3.8 extends the same file with the P1 groups and pins them below: the
 * never-weighed indication (DASH-2), the lifecycle labels and the deletion
 * consequence (RAB-10, NFR-9), the archived read-only indication (RAB-8/DM-4),
 * the weight form's refusals and outlier confirmation (WT-4), the four ranges in
 * `WEIGHT_RANGES` order (WT-6), the contract's sex and neuter options (D1.14)
 * and D1.16's `error.household_content_unloaded`, which must stay distinct from
 * both `error.unknown` and the never-loaded connectivity copy.
 */
import { describe, expect, it } from 'vitest';
import { IDENTITY_PLACEHOLDER_MESSAGE_KEY } from '../settings/identity';
import type { SaveMessageKey } from '../sync/failure';
import { SAVE_CAUSE_MESSAGE_KEYS } from '../sync/failure';
import { WEIGHT_RANGES } from '../weight/search';
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

  it('gives the dashboard its own title and body, not onboarding copy', () => {
    expect(t('dashboard.title')).toBe('Your household');
    expect(t('dashboard.body')).not.toBe(t('onboarding.body'));
  });

  it('resolves the shared cancel action and the household empty state', () => {
    const expected = [
      ['common.cancel', 'Cancel'],
      ['settings.households.emptyTitle', 'No household on this device'],
      [
        'settings.households.emptyBody',
        'Create one here to start sharing with the people you live with.',
      ],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of expected) {
      expect(t(key)).toBe(copy);
      expect(t(key).trim().length).toBeGreaterThan(0);
      expect(t(key)).not.toBe(key);
    }
  });

  it('resolves the account-connectivity and entry-failure headings', () => {
    const expected = [
      [
        'connectivity.account.title',
        'This device cannot use the shared service right now.',
      ],
      ['entry.loadFailed.title', 'Your household could not be loaded.'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of expected) {
      expect(t(key)).toBe(copy);
      expect(t(key).trim().length).toBeGreaterThan(0);
      expect(t(key)).not.toBe(key);
    }

    // The account heading must not tell an online person to connect.
    expect(t('connectivity.account.title')).not.toMatch(/internet|connect/i);
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

describe('the P1 message catalog', () => {
  it('leaves the unclassified failure copy operation-neutral', () => {
    expect(t('error.unknown')).toBe('That did not work. Nothing was changed.');
  });

  it('gives unknown content its own refusal, not the generic or never-loaded copy', () => {
    const refusal = t('error.household_content_unloaded');

    // D1.16 is a distinct condition: rows were loaded, one row's content was
    // not. It is neither D0.6's unclassified failure nor SYNC-3's never-loaded
    // state, and its copy must not borrow either.
    expect(refusal).not.toBe(t('error.unknown'));
    expect(refusal).not.toBe(t('connectivity.required.body'));
    expect(refusal).toMatch(/this device could not read/i);
    expect(refusal).toMatch(/next sync/i);
    expect(refusal).not.toMatch(/connect to the internet/i);

    for (const [key, message] of Object.entries(P0_MESSAGES)) {
      if (key.startsWith('connectivity.')) {
        expect(refusal).not.toBe(message);
      }
    }
  });

  it('marks a never-weighed rabbit in place of a date', () => {
    expect(t('dashboard.neverWeighed')).toBe('Never weighed');
    expect(t('dashboard.neverWeighed')).not.toBe(t('dashboard.lastWeighed'));
  });

  it('pins the dashboard empty state and add-first-rabbit prompt', () => {
    const expected = [
      ['dashboard.emptyTitle', 'No rabbits yet'],
      [
        'dashboard.emptyBody',
        'Add your first rabbit to start recording weight.',
      ],
      ['rabbit.add', 'Add rabbit'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of expected) {
      expect(t(key)).toBe(copy);
    }
  });

  it('pins the lifecycle labels and the deletion consequence', () => {
    const expected = [
      ['rabbit.archive', 'Archive'],
      ['rabbit.archiving', 'Archiving…'],
      ['rabbit.restore', 'Restore'],
      ['rabbit.restoring', 'Restoring…'],
      ['rabbit.delete', 'Delete permanently'],
      ['rabbit.deleting', 'Deleting…'],
      ['rabbit.deleteConfirm.title', 'Delete this rabbit permanently?'],
      ['rabbit.deleteConfirm.confirm', 'Delete permanently'],
      ['rabbit.deleteConfirm.cancel', 'Keep rabbit'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of expected) {
      expect(t(key)).toBe(copy);
    }

    // RAB-10: the confirmation states what is lost, not merely that it is
    // permanent. It cascades to the rabbit's weight entries.
    expect(t('rabbit.deleteConfirm.body')).toMatch(/weight entries/i);
    expect(t('rabbit.deleteConfirm.body')).toMatch(/cannot be undone/i);

    // D1.8: archiving is the reversible step, so it is not destructive under
    // NFR-9 and carries no consequence confirmation.
    expect(
      Object.keys(P0_MESSAGES).filter((key) =>
        key.startsWith('rabbit.archiveConfirm.'),
      ),
    ).toEqual([]);
  });

  it('marks an archived rabbit as read-only', () => {
    expect(t('rabbit.archived.label')).toBe('Archived');
    expect(t('rabbit.archived.readOnly')).toMatch(/archived/i);
    expect(t('rabbit.archived.readOnly')).toMatch(/read-only/i);
  });

  it('pins the rabbit presentation fallbacks and blank-name prompt', () => {
    expect(t('rabbit.thumbnailPlaceholder')).toBe('No photo');
    expect(t('rabbit.dob.unknown')).toBe('Unknown');
    expect(t('rabbit.form.blankName')).toBe('Enter a name for your rabbit.');
  });

  it('labels exactly the contract’s sex and neuter options', () => {
    const sex = [
      ['rabbit.sex.male', 'Male'],
      ['rabbit.sex.female', 'Female'],
      ['rabbit.sex.unknown', 'Unknown'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;
    const neuter = [
      ['rabbit.neuterStatus.intact', 'Intact'],
      ['rabbit.neuterStatus.neutered', 'Neutered'],
      ['rabbit.neuterStatus.spayed', 'Spayed'],
      ['rabbit.neuterStatus.unknown', 'Unknown'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of [...sex, ...neuter]) {
      expect(t(key)).toBe(copy);
    }

    // The display strings are keys; the values stay the contract's. Nothing
    // outside D1.14's two value sets may exist under these prefixes.
    expect(
      Object.keys(P0_MESSAGES).filter(
        (key) => key.startsWith('rabbit.sex.') && key !== 'rabbit.sex.label',
      ),
    ).toEqual(sex.map(([key]) => key));
    expect(
      Object.keys(P0_MESSAGES).filter(
        (key) =>
          key.startsWith('rabbit.neuterStatus.') &&
          key !== 'rabbit.neuterStatus.label',
      ),
    ).toEqual(neuter.map(([key]) => key));
  });

  it('gives each weight refusal and the outlier confirmation its own copy', () => {
    const prompts = [
      ['weight.prompt.empty', 'Enter a weight.'],
      ['weight.prompt.unreadable', 'Enter the weight as a number.'],
      [
        'weight.prompt.zero',
        'A weight must be more than zero. Enter a valid weight.',
      ],
      [
        'weight.prompt.negative',
        'A weight cannot be negative. Enter a valid weight.',
      ],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    for (const [key, copy] of prompts) {
      expect(t(key)).toBe(copy);
    }

    // WT-4's two refusals name their cause rather than sharing a synonym.
    expect(new Set(prompts.map(([, copy]) => copy)).size).toBe(prompts.length);

    // WT-4's outlier is a confirmation that asks whether the value is intended,
    // with two distinct outcomes; declining is not an error.
    expect(t('weight.confirmOutlier.body')).toMatch(/intended/i);
    expect(t('weight.confirmOutlier.confirm')).not.toBe(
      t('weight.confirmOutlier.decline'),
    );
  });

  it('labels the four weight ranges in WEIGHT_RANGES order', () => {
    const expected = [
      ['weight.range.days_7', '7 days'],
      ['weight.range.days_30', '30 days'],
      ['weight.range.days_90', '90 days'],
      ['weight.range.all', 'Full history'],
    ] as const satisfies ReadonlyArray<readonly [MessageKey, string]>;

    // The range control renders the catalog's order, so the catalog must match
    // the order the contract's four windows are presented in (WT-6).
    expect(
      Object.keys(P0_MESSAGES).filter((key) => key.startsWith('weight.range.')),
    ).toEqual(WEIGHT_RANGES.map((range) => `weight.range.${range}`));

    for (const [key, copy] of expected) {
      expect(t(key)).toBe(copy);
    }
  });

  it('confirms a weight-entry deletion with its consequence', () => {
    expect(t('weight.entry.delete')).toBe('Delete entry');
    expect(t('weight.entry.deleteConfirm.title')).toBe(
      'Delete this weight entry?',
    );
    expect(t('weight.entry.deleteConfirm.body')).toMatch(/cannot be undone/i);
    expect(t('weight.entry.deleteConfirm.confirm')).not.toBe(
      t('weight.entry.deleteConfirm.cancel'),
    );
  });
});
