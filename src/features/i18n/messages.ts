/*
 * The P0 message catalog (task 3.7; SET-2, NFR-8).
 *
 * One flat table of keys to English strings. It is flat on purpose: the keys
 * the rest of the app already speaks — `SAVE_CAUSE_MESSAGE_KEYS`'s
 * `error.connectivity`-style values and `IDENTITY_PLACEHOLDER_MESSAGE_KEY`'s
 * `settings.identity.you` — are literal strings, and a nested tree would need a
 * second, parallel key syntax to address them. Flat keys mean the store's
 * `messageKey` can be passed to `t` unchanged, with no lookup table translating
 * between two naming schemes.
 *
 * **What is guaranteed, and how.**
 *
 * - The failure map is declared with `satisfies Record<SaveMessageKey, string>`.
 *   `SaveMessageKey` is derived from the store's own map
 *   (`src/features/sync/failure.ts`), so a cause added, removed or renamed
 *   there is a **compile error here** until this table matches it. The store
 *   cannot start naming a key the catalog lacks.
 * - `settings.identity.you` is duplicated as a literal here rather than
 *   imported from the Settings feature, so the data catalog carries no React
 *   Query or command-wrapper module graph. The spec asserts
 *   `t(IDENTITY_PLACEHOLDER_MESSAGE_KEY)`; if the Settings hook renames its key,
 *   that call stops typechecking and the test stops resolving.
 * - `MessageKey` is `keyof typeof P0_MESSAGES`, so `t` cannot be called with a
 *   key the catalog does not contain. The one way to reach it with an unknown
 *   value is an explicit cast; `t` checks the table at runtime and answers a
 *   catalogued generic message instead of echoing the key, so a missing entry
 *   can never render `settings.some.typo` to the person.
 *
 * **What is deliberately not here.** German is not shipped: NFR-5 puts full
 * German/English coverage in P4, and half a translation — or a locale switch
 * with one locale populated — would be worse than one complete language. This
 * task adds the scaffold and the English P0 set; a second locale becomes a
 * second catalog and a locale selector, without changing any caller. No
 * interpolation is implemented either: the P0 screens build copy with data
 * (a household's name beside a translated label), and the one dynamic string
 * that would need pluralisation — member counts — is not part of HS-4's
 * Settings list. Both are recorded in the task report rather than guessed.
 *
 * Nothing here touches a route or a component: tasks 4.1–4.5 import `t` from
 * `./index` and pass keys, and this file stays the one place English product
 * copy lives.
 */
import type { SaveMessageKey } from '../sync/failure';

/**
 * The copy for every failure cause the store can name.
 *
 * `satisfies Record<SaveMessageKey, string>` is the link to
 * `src/features/sync/failure.ts`: that map decides the key names, this map must
 * answer all of them, and a mismatch is a type error rather than a screen that
 * renders a key.
 *
 * `error.unknown` is worded to serve both roles the cause has (task 2.10's
 * note): the display-unit store reports a local failure as `unknown`, and a
 * genuinely unclassified failure also lands here. It therefore names neither
 * the service nor connectivity — only that the change did not complete.
 */
const FAILURE_MESSAGES = {
  'error.connectivity':
    'No connection. Check your internet connection and try again.',
  'error.no_account':
    'Your account is not available right now. Try again later.',
  'error.allowance_exceeded': 'This household has reached its storage limit.',
  'error.rate_limited':
    'Too many requests right now. Wait a moment and try again.',
  'error.timeout': 'The service took too long to answer. Try again.',
  'error.validation': 'That change is not valid. Check it and try again.',
  'error.unknown': 'Something went wrong. Try again.',
} as const satisfies Record<SaveMessageKey, string>;

/**
 * Every P0 string, keyed by the value the app passes to `t`.
 *
 * Grouped by the surface that uses it; the order inside the object is the
 * reading order a screen has, not an ordering the lookup relies on.
 */
export const P0_MESSAGES = {
  // The failure surface (SYNC-2, NFR-8, task 4.5).
  ...FAILURE_MESSAGES,
  'error.retry': 'Try again',
  'error.dismiss': 'Dismiss',
  'error.notSaved': 'Not saved',

  // SET-2's last tier. The Settings hook returns this key, never the string.
  'settings.identity.you': 'You',

  // First-run household creation (HS-1, HS-2, task 4.1).
  'onboarding.title': 'Create your household',
  'onboarding.body':
    'A household is where you and the people you live with share your data.',

  // Creating a household from either entry point (HS-1, HS-4).
  'household.nameLabel': 'Household name',
  'household.namePlaceholder': 'Our household',
  'household.create': 'Create household',
  'household.creating': 'Creating…',
  'household.blankName': 'Enter a name for your household.',

  // The dashboard's Settings & Profile entry point (SET-1, task 4.2).
  'dashboard.title': 'Your household',
  'dashboard.body':
    'This is your household’s shared space. Rabbit and care records will appear here.',
  'dashboard.settings': 'Open Settings & Profile',

  // Settings & Profile (SET-1, SET-2, SET-4, HS-4, task 4.3).
  'settings.title': 'Settings & Profile',
  'settings.back': 'Back',
  'settings.identity.heading': 'Profile',
  'settings.identity.label': 'Name',
  'settings.households.heading': 'Households',
  'settings.households.active': 'Active',
  'settings.households.makeActive': 'Make active',
  'settings.households.owner': 'Owner',
  'settings.households.createAnother': 'Create another household',
  'settings.households.emptyTitle': 'No household on this device',
  'settings.households.emptyBody':
    'Create one here to start sharing with the people you live with.',
  'settings.households.switchFailed':
    'That household is no longer available. Refresh and try again.',
  'settings.displayUnit.heading': 'Weight unit',
  'settings.displayUnit.description':
    'Used only when a weight is entered or shown. Saved values do not change.',
  'settings.displayUnit.grams': 'Grams',
  'settings.displayUnit.kilograms': 'Kilograms',
  'settings.displayUnit.saving': 'Saving…',

  // Connectivity and previously loaded data (SYNC-3, task 4.4).
  'connectivity.required.title': 'Connect to continue',
  'connectivity.required.body':
    'Your household data has not been loaded on this device yet. Connect to the internet and try again.',
  'connectivity.localCopy':
    'Showing saved data. Recent changes from other members may not be here yet.',
  'connectivity.offline': 'Offline',
  'connectivity.checkAgain': 'Check again',
  'connectivity.checking': 'Checking…',

  // Shared actions, and the last resort for a key cast around the type system.
  'common.unexpectedError': 'Something went wrong.',
  'common.cancel': 'Cancel',
} as const;

/** Every key the catalog can answer for. */
export type MessageKey = keyof typeof P0_MESSAGES;

/**
 * The English string for `key`.
 *
 * The parameter type is the catalog's own key union, so a missing or misspelt
 * key is a compile error in application code. The runtime check exists for the
 * one path types cannot cover — a key that arrived through a cast or from data
 * — and answers a catalogued generic message rather than the raw key, so a bug
 * cannot show `settings.some.typo` to a person.
 */
export function t(key: MessageKey): string {
  if (Object.hasOwn(P0_MESSAGES, key)) {
    return P0_MESSAGES[key];
  }

  return P0_MESSAGES['common.unexpectedError'];
}
