/*
 * The held household list as Settings & Profile presents it (task 3.5; HS-4,
 * HS-3, DM-3).
 *
 * The data belongs to the household feature: `useHouseholdList` reads the
 * device's book and `useActiveHousehold` reads the selection, and
 * `useSwitchActiveHousehold` writes both. This module adds only the
 * **presentation rule** HS-4 needs — "indicate which of them is active" — and it
 * is deliberately built on one of those two facts rather than on a third:
 *
 * - **The active read is the fact while it has an answer.** It is the state the
 *   switch writes (`householdKeys.active`) and the one the collection keying and
 *   the entry guard use, so the row marked active is the same household the rest
 *   of the app treats as active. The switch additionally rewrites the list's
 *   `is_active` flags in the same `onSuccess`, so the two agree at the instant
 *   the backend confirms; while the list's invalidation is still refetching, the
 *   last answer from the active read still names the switched household, so the
 *   marker does not flicker or move.
 * - **The list's own `is_active` is the fallback only while that read has no
 *   answer.** That window is a first mount before the active read resolves, or
 *   an active read that failed before producing any data — not a refetch error
 *   after an answer, which keeps the cached active-read value rather than
 *   switching sources. The book's own at-most-one flag is used as given rather
 *   than defaulting every row to inactive (which would briefly claim no
 *   household is active) or guessing a row.
 *
 * The rows expose **one** active fact: `isActive` replaces the contract's
 * `is_active` field instead of accompanying it. A row carrying both could show
 * one of them while the switch had updated the other, which is precisely the
 * disagreement this module exists to prevent. `activeMarker` records which
 * source produced the flag — the last answer from the active read, or the list
 * while the active read has none — not whether that answer is fresh; it is not a
 * second active flag.
 *
 * A row whose id is not in the list is not invented: when the active read names
 * a household the list has not (yet) returned, no row is marked active and
 * `activeHouseholdId` still names it. The list is the set; the active read is
 * the selection over it.
 */
import type {
  HouseholdListFailure,
  HouseholdSummary,
} from '../households/queries';
import { useActiveHousehold, useHouseholdList } from '../households/queries';

/**
 * One held household with the single active fact.
 *
 * `is_active` is omitted rather than shadowed (see the module note): a consumer
 * can only read `isActive`, so it cannot accidentally read the list's flag
 * while the active read says otherwise.
 */
export type SettingsHousehold = Omit<HouseholdSummary, 'is_active'> & {
  readonly isActive: boolean;
};

/** Where the active marker came from in a ready list. */
export type ActiveMarkerSource = 'active-read' | 'list';

/** The list as Settings & Profile renders it. */
export type SettingsHouseholdListState =
  | { kind: 'pending' }
  | { kind: 'failed'; error: HouseholdListFailure }
  | {
      kind: 'ready';
      /** Every held household, in the book's order, each with one flag. */
      readonly households: readonly SettingsHousehold[];
      /**
       * The active household's id, or `null` when none is active. Taken from
       * the active read when it has answered, otherwise from the list's flag.
       */
      readonly activeHouseholdId: string | null;
      /** Which source produced `isActive` on the rows. */
      readonly activeMarker: ActiveMarkerSource;
    };

/**
 * Mark the row the active fact names (pure).
 *
 * `activeHouseholdId` is `undefined` when the active read has not answered — a
 * different thing from `null`, which is the backend's "no household is active"
 * — and only then is the list's own flag used. The active-read branch marks at
 * most one row, because it compares ids; the fallback branch uses the book's
 * at-most-one `is_active` flags as given, so it presents exactly what the list
 * answered rather than silently dropping a second true flag.
 */
export function settingsHouseholds(
  households: readonly HouseholdSummary[],
  activeHouseholdId: string | null | undefined,
): SettingsHousehold[] {
  return households.map((row) => ({
    id: row.id,
    name: row.name,
    created_at: row.created_at,
    is_owner: row.is_owner,
    member_count: row.member_count,
    isActive:
      activeHouseholdId === undefined
        ? row.is_active
        : activeHouseholdId !== null && row.id === activeHouseholdId,
  }));
}

/**
 * The held households with the active one indicated (HS-4).
 *
 * Pending and failure follow the **list** read, because the list is the set
 * being rendered: without it there are no rows to indicate. A failed active
 * read does not fail the screen. If it failed before producing an answer, the
 * list's own flag is used and `activeMarker: 'list'` says so; if an answer was
 * already cached, a later refetch error leaves that answer in place and the
 * marker keeps reading `active-read` (data wins over a later error, as in the
 * identity read). Nothing here invalidates anything: this is a read over the
 * household feature's keys, and a household change is task 3.6's to route to the
 * list/active pair. The display unit lives under a different namespace and is
 * not touched by a household read.
 */
export function useSettingsHouseholdList(): SettingsHouseholdListState {
  const list = useHouseholdList();
  const active = useActiveHousehold();

  if (list.data === undefined) {
    if (list.error !== null) {
      return { kind: 'failed', error: list.error };
    }

    return { kind: 'pending' };
  }

  const activeHouseholdId =
    active.data === undefined ? undefined : (active.data?.id ?? null);
  const households = settingsHouseholds(list.data, activeHouseholdId);

  return {
    kind: 'ready',
    households,
    activeHouseholdId:
      activeHouseholdId === undefined
        ? (households.find((row) => row.isActive)?.id ?? null)
        : activeHouseholdId,
    activeMarker: activeHouseholdId === undefined ? 'list' : 'active-read',
  };
}
