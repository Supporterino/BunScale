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
 *
 * **P1 extends this same catalog (task 3.8).** The rabbit, weight and dashboard
 * surfaces speak the same flat table through the same `t`, so there is still one
 * place English copy lives. The P1 keys are grouped by the surface that reads
 * them, exactly as the P0 keys are; the failure map stays as P0 froze it, and
 * P1's one refusal key sits beside it rather than inside it (see below).
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
 * `error.unknown` is worded for the unclassified failure only (D0.6, NFR-8,
 * Q58, Q60). The wording is operation-neutral on purpose: this key is not only
 * a refused write, it is also the body of read and status surfaces — the
 * account-unclassified state in `ConnectivityRequiredScreen` and the load
 * failure in `HouseholdEntryScreens` — where no change was ever attempted. So
 * it says "that did not work" and leaves "nothing was changed" as a promise
 * about the shared data, rather than describing a change that did not complete.
 * It names neither the service nor connectivity, because the frontend cannot
 * know more. It deliberately does not promise a retry: the backend marks this
 * cause `retryable: false` (`src-tauri/src/ipc/failure.rs`), so no surface
 * offers one.
 *
 * The display-unit store's local, device-specific failure gets its own key
 * (`settings.unit.store_failed`) instead of borrowing this one; task 8.3 shows
 * it at the Settings unit section.
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
  'error.unknown': 'That did not work. Nothing was changed.',
} as const satisfies Record<SaveMessageKey, string>;

/**
 * Every P0 and P1 string, keyed by the value the app passes to `t`.
 *
 * Grouped by the surface that uses it; the order inside the object is the
 * reading order a screen has, not an ordering the lookup relies on.
 */
export const P0_MESSAGES = {
  // The failure surface (SYNC-2, NFR-8, task 4.5).
  ...FAILURE_MESSAGES,
  // D1.16's refusal of a row whose content this device cannot read. It is a
  // surface key on the frozen `unknown` cause, not a cause of its own, so it
  // cannot live inside `FAILURE_MESSAGES` (that map is exhaustively checked
  // against `SaveMessageKey`). The copy is deliberately neither `error.unknown`
  // nor `connectivity.required.body`: rows were loaded, one row's content was
  // not, and the next sync fills it.
  'error.household_content_unloaded':
    'This device could not read details of the household data it holds. They will arrive with the next sync.',
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

  // The entry route's pending surface: the household book/status is being read
  // at launch (`beforeLoad` may probe for up to 5 s). It names the wait instead
  // of a bare "Loading", so VoiceOver says what is pending (Q48, task 7.6).
  'entry.loading': 'Loading your household…',

  // The entry guard's load-failure heading; the cause copy is the body (SYNC-3).
  'entry.loadFailed.title': 'Your household could not be loaded.',

  // The dashboard's Settings & Profile entry point (SET-1, task 4.2).
  'dashboard.title': 'Your household',
  'dashboard.body':
    'This is your household’s shared space. Rabbit and care records will appear here.',
  'dashboard.settings': 'Open Settings & Profile',
  // The P1 dashboard list (DASH-1, DASH-2, DASH-4, DASH-8, DASH-9).
  'dashboard.lastWeighed': 'Last weighed',
  // DASH-2's indication in place of a last-weighed date.
  'dashboard.neverWeighed': 'Never weighed',
  'dashboard.emptyTitle': 'No rabbits yet',
  'dashboard.emptyBody': 'Add your first rabbit to start recording weight.',

  // Settings & Profile (SET-1, SET-2, SET-4, HS-4, task 4.3).
  'settings.title': 'Settings & Profile',
  'settings.back': 'Back',
  'settings.identity.heading': 'Profile',
  'settings.identity.label': 'Name',
  'settings.households.heading': 'Households',
  // The list read: the first thing the section shows while the households are
  // being loaded, so it names the list rather than the generic "Loading".
  'settings.households.loading': 'Loading households…',
  'settings.households.active': 'Active',
  'settings.households.makeActive': 'Make active',
  'settings.households.owner': 'Owner',
  'settings.households.createAnother': 'Create another household',
  'settings.households.emptyTitle': 'No household on this device',
  'settings.households.emptyBody':
    'Create one here to start sharing with the people you live with.',
  'settings.households.switchFailed':
    'That household is no longer available. Refresh and try again.',
  // A switch in flight, beside the failure copy above: the same action, before
  // the shared service confirms it (`D1` — no optimistic apply).
  'settings.households.switching': 'Switching household…',
  'settings.displayUnit.heading': 'Weight unit',
  // The section's initial read (before the control exists), distinct from the
  // save in flight below: `Saving…` must not stand in for "not loaded yet".
  'settings.displayUnit.loading': 'Loading your weight unit…',
  'settings.displayUnit.description':
    'Used only when a weight is entered or shown. Saved values do not change.',
  'settings.displayUnit.grams': 'Grams',
  'settings.displayUnit.kilograms': 'Kilograms',
  'settings.displayUnit.saving': 'Saving…',
  // The local unit store failing on this device, not the shared service: the
  // choice never left the device, so the copy names that and promises no retry
  // (Q58; shown by task 8.3).
  'settings.unit.store_failed':
    'Your choice was not saved on this device. Nothing was changed.',

  // Connectivity and previously loaded data (SYNC-3, task 4.4).
  'connectivity.required.title': 'Connect to continue',
  'connectivity.required.body':
    'Your household data has not been loaded on this device yet. Connect to the internet and try again.',
  'connectivity.account.title':
    'This device cannot use the shared service right now.',
  'connectivity.localCopy':
    'Showing saved data. Recent changes from other members may not be here yet.',
  'connectivity.offline': 'Offline',
  'connectivity.checkAgain': 'Check again',
  'connectivity.checking': 'Checking…',

  // Rabbit lifecycle controls (RAB-6, RAB-7, RAB-10, NFR-9). Archiving is the
  // reversible step and needs no consequence confirmation (D1.8), so there is
  // deliberately no `rabbit.archiveConfirm.*` group beside the delete one.
  'rabbit.archive': 'Archive',
  'rabbit.archiving': 'Archiving…',
  'rabbit.restore': 'Restore',
  'rabbit.restoring': 'Restoring…',
  'rabbit.delete': 'Delete permanently',
  'rabbit.deleting': 'Deleting…',
  // Permanent deletion states its consequence before anything is removed
  // (RAB-10, NFR-9); it cascades to the rabbit's weight entries.
  'rabbit.deleteConfirm.title': 'Delete this rabbit permanently?',
  'rabbit.deleteConfirm.body':
    'The rabbit and all of its weight entries will be deleted for every member of the household. This cannot be undone.',
  'rabbit.deleteConfirm.confirm': 'Delete permanently',
  'rabbit.deleteConfirm.cancel': 'Keep rabbit',
  // The archived surface and the list that reaches it (RAB-6, RAB-8, DM-4).
  'rabbit.archived.label': 'Archived',
  'rabbit.archived.readOnly':
    'This rabbit is archived. Its history is read-only.',
  'rabbit.archivedList.title': 'Archived rabbits',
  'rabbit.archivedList.empty': 'No archived rabbits.',
  // The thumbnail fallback wherever a rabbit is presented, used as its
  // accessible name (RAB-1).
  'rabbit.thumbnailPlaceholder': 'No photo',

  // Rabbit profile creation and editing (RAB-1, RAB-2, RAB-4, RAB-5). The add
  // control and the create form's submit share one label; editing has its own.
  'rabbit.add': 'Add rabbit',
  'rabbit.edit': 'Edit profile',
  'rabbit.form.createTitle': 'Add a rabbit',
  'rabbit.form.editTitle': 'Edit rabbit',
  'rabbit.form.nameLabel': 'Name',
  'rabbit.form.namePlaceholder': 'Rabbit’s name',
  'rabbit.form.breedLabel': 'Breed',
  'rabbit.form.breedPlaceholder': 'Optional',
  'rabbit.form.dobLabel': 'Date of birth',
  'rabbit.form.dobUnknown': 'Date of birth unknown',
  // The profile's value when a date of birth is explicitly unknown (RAB-1).
  'rabbit.dob.unknown': 'Unknown',
  'rabbit.form.adoptionDateLabel': 'Adoption date',
  'rabbit.form.thresholdLabel': 'Weight-drop threshold',
  'rabbit.form.thresholdHelp':
    'A drop larger than this percentage raises an alert. The default is 5%.',
  // RAB-2's blank-name refusal, shown on the name field.
  'rabbit.form.blankName': 'Enter a name for your rabbit.',
  'rabbit.form.save': 'Save changes',
  'rabbit.form.saving': 'Saving…',

  // The sex and neuter-status options (D1.14, RAB-1). The values stay the
  // contract's `male | female | unknown` and `intact | neutered | spayed |
  // unknown`; these keys are only their display strings.
  'rabbit.sex.label': 'Sex',
  'rabbit.sex.male': 'Male',
  'rabbit.sex.female': 'Female',
  'rabbit.sex.unknown': 'Unknown',
  'rabbit.neuterStatus.label': 'Neuter status',
  'rabbit.neuterStatus.intact': 'Intact',
  'rabbit.neuterStatus.neutered': 'Neutered',
  'rabbit.neuterStatus.spayed': 'Spayed',
  'rabbit.neuterStatus.unknown': 'Unknown',

  // The weight entry form (WT-1, WT-3, WT-4, NFR-9). No unit picker exists
  // (D1.2), so the catalog carries no unit choice for this surface.
  'weight.log': 'Log weight',
  'weight.form.editTitle': 'Edit weight',
  'weight.form.valueLabel': 'Weight',
  'weight.form.timestampLabel': 'Date and time',
  'weight.form.noteLabel': 'Note',
  'weight.form.notePlaceholder': 'Optional note',
  'weight.form.save': 'Save weight',
  'weight.form.saving': 'Saving…',
  // One prompt per `WeightPromptState` in `../weight/form.ts` (WT-4). Zero and
  // negative are refusals with their own cause, not the generic failure.
  'weight.prompt.empty': 'Enter a weight.',
  'weight.prompt.unreadable': 'Enter the weight as a number.',
  'weight.prompt.zero':
    'A weight must be more than zero. Enter a valid weight.',
  'weight.prompt.negative':
    'A weight cannot be negative. Enter a valid weight.',
  // An above-bound value is a confirmation, not a rejection (WT-4, D1.9). The
  // copy stays unit-neutral because the value is shown in the person's unit.
  'weight.confirmOutlier.title': 'Is this weight correct?',
  'weight.confirmOutlier.body':
    'This is higher than the usual range for a rabbit. Confirm that the value is intended before it is saved.',
  'weight.confirmOutlier.confirm': 'Save it anyway',
  'weight.confirmOutlier.decline': 'Change the value',

  // The weight entry detail (WT-8, NFR-9). Value, timestamp and note reuse the
  // form's labels above; only the entry's own actions need new copy.
  'weight.entry.title': 'Weight entry',
  'weight.entry.edit': 'Edit entry',
  'weight.entry.delete': 'Delete entry',
  'weight.entry.deleting': 'Deleting…',
  'weight.entry.deleteConfirm.title': 'Delete this weight entry?',
  'weight.entry.deleteConfirm.body':
    'This weight entry will be deleted for every member of the household. This cannot be undone.',
  'weight.entry.deleteConfirm.confirm': 'Delete entry',
  'weight.entry.deleteConfirm.cancel': 'Keep entry',

  // The rabbit dashboard's trend, its History section and the shared drop
  // marker (DASH-4, DASH-7, WT-6, WT-7).
  'weight.trend.title': 'Weight trend',
  'weight.trend.chartLabel': 'Weight trend chart',
  'weight.history.title': 'History',
  'weight.history.open': 'Weight history',
  'weight.dropAlert': 'Weight drop warning',
  'weight.empty': 'No weight entries yet.',
  // The four windows, in `WEIGHT_RANGES`' order (search.ts), because the range
  // control renders the catalog in this order rather than sorting it.
  'weight.range.days_7': '7 days',
  'weight.range.days_30': '30 days',
  'weight.range.days_90': '90 days',
  'weight.range.all': 'Full history',

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
