/*
 * The identity read and SET-2's name chain, from the frontend's side (task 3.5;
 * SET-2).
 *
 * The chain itself is the backend's (`src-tauri/src/domain/identity.rs`):
 * the account's display name, then the locally known member name, then no name.
 * `get_identity` answers that as `Identity { account_user_id?, display_name?,
 * resolved }`, where `resolved` says whether the *account* tier supplied the
 * name. The frontend's job is therefore not to resolve anything — it is to
 * report which tier answered and to leave the last tier alone:
 *
 * - `display_name` present and `resolved: true` is the account's own name;
 * - `display_name` present and `resolved: false` is the local member fallback;
 * - `display_name: null` means **no name is known**, and the localized
 *   "You"-style placeholder is copy the UI supplies. This module returns that
 *   tier as the i18n **key** `settings.identity.you` (exported below), never as
 *   a hard-coded string: task 3.7 owns the scaffold and the actual copy, and
 *   task 4.3 renders it. A hook that invented "You" here would put English
 *   product copy in a data module and make the string untranslatable.
 *
 * An unresolved identity is a normal answer, not a failure (SET-2's scenario:
 * it must not disrupt the screen). `useIdentityName` therefore separates three
 * cases a surface must not confuse: **pending** (the read has not answered),
 * **failed** (the IPC read itself broke — no name is known, and the failure is
 * reported rather than swallowed), and **ready** (the chain's answer, which may
 * be the placeholder). A surface can keep the rest of Settings usable in all
 * three; only the name is missing.
 *
 * The wire type admits `resolved: true` with `display_name: null`, which this
 * backend never produces. It is presented as the placeholder — the absence of a
 * name is what the frontend can see, and inventing one from a flag would be
 * exactly the guess SET-2 forbids.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { CommandData, CommandFailure } from '../../lib/command-result';
import { unwrapCommandResult } from '../../lib/command-result';
import { getIdentity } from './api';
import { settingsKeys } from './keys';

/** A wrapper outcome, used to derive this read's types (project hard rule 2). */
type IdentityOutcome = Awaited<ReturnType<typeof getIdentity>>;

/** The current person's identity — the contract's `Identity`. */
export type Identity = CommandData<IdentityOutcome>;

/**
 * The identity read's error: a `CommandError`, or the `Error` an IPC-level
 * rejection re-throws. `get_identity` has no product failure path, so in
 * practice this is a defect rather than a cause the backend reported.
 */
export type IdentityFailure = CommandFailure<IdentityOutcome> | Error;

/** The identity key's type, so the options cannot disagree with the factory. */
type IdentityKey = ReturnType<typeof settingsKeys.identity>;

/**
 * The identity read (SET-2).
 *
 * Local and network-free: the account tier is resolved from the process's
 * state, and the local fallback from this device's household book. Mounting
 * Settings therefore re-reads it without a service round trip; an unresolved
 * identity resolves to a value, never to an error.
 */
export function identityQueryOptions() {
  return queryOptions<Identity, IdentityFailure, Identity, IdentityKey>({
    queryKey: settingsKeys.identity(),
    queryFn: () => getIdentity().then(unwrapCommandResult),
  });
}

/**
 * The raw identity, including `account_user_id` and `resolved`.
 *
 * A surface that only shows the name uses `useIdentityName`; this is for one
 * that also needs the account id (nothing in P0 does).
 */
export function useIdentity() {
  return useQuery(identityQueryOptions());
}

/**
 * The i18n key of SET-2's last tier: the localized "You"-style placeholder.
 *
 * The hook never returns the copy itself. Task 3.7 adds this key to the P0
 * string set and provides the German/English strings; task 4.3 resolves it when
 * it renders the name. Renaming it is a copy-scaffold change, not a contract
 * change.
 */
export const IDENTITY_PLACEHOLDER_MESSAGE_KEY =
  'settings.identity.you' as const;

/** The one key this chain can hand to the UI. */
export type IdentityPlaceholderMessageKey =
  typeof IDENTITY_PLACEHOLDER_MESSAGE_KEY;

/**
 * What the name area shows: a name from one of the resolved tiers, or the
 * placeholder key with **no** name at all.
 *
 * `source` is the backend's `resolved` flag, not a second guess: `account` is
 * the account tier, `local` the locally known member name.
 */
export type IdentityDisplayName =
  | {
      readonly kind: 'name';
      readonly name: string;
      readonly source: 'account' | 'local';
    }
  | {
      readonly kind: 'placeholder';
      readonly messageKey: IdentityPlaceholderMessageKey;
    };

/**
 * Apply SET-2's chain to one identity answer (pure: no defaults, no copy).
 *
 * `display_name: null` is checked first because the absence of a name is the
 * only fact the frontend has; `resolved` describes where a name came from, not
 * whether one exists. The backend never pairs `resolved: true` with `null`, but
 * the wire type admits it, and such a value still has no name to show.
 */
export function identityDisplayName(identity: Identity): IdentityDisplayName {
  if (identity.display_name === null) {
    return {
      kind: 'placeholder',
      messageKey: IDENTITY_PLACEHOLDER_MESSAGE_KEY,
    };
  }

  return {
    kind: 'name',
    name: identity.display_name,
    source: identity.resolved ? 'account' : 'local',
  };
}

/**
 * The name area's state, with the three cases kept apart.
 *
 * Data wins over a later refetch error: an answer is an answer, and a failed
 * refresh does not erase the name that was already resolved (the same
 * loaded-data reasoning as SYNC-3). `failed` means no answer has ever arrived
 * and the read broke; the surface may render the placeholder — no name is known
 * either way — but it can also name the failure instead of hiding it.
 *
 * `identity` on the ready case is the backend's answer untouched, including
 * `resolved` and `account_user_id`. The name chain is already applied in
 * `displayName`; the raw flag stays available so a surface never has to infer
 * "was this the account or the local fallback?" from anything else.
 */
export type IdentityNameView =
  | { kind: 'pending' }
  | { kind: 'failed'; error: IdentityFailure }
  | {
      kind: 'ready';
      identity: Identity;
      displayName: IdentityDisplayName;
    };

/** The name to show, or the reason there is none yet (SET-2). */
export function useIdentityName(): IdentityNameView {
  const identity = useIdentity();

  if (identity.data !== undefined) {
    return {
      kind: 'ready',
      identity: identity.data,
      displayName: identityDisplayName(identity.data),
    };
  }

  if (identity.error !== null) {
    return { kind: 'failed', error: identity.error };
  }

  return { kind: 'pending' };
}
