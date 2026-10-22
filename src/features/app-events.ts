/*
 * The app's single event-subscription mount (task 3.6; SYNC-1, NFR-1, R3).
 *
 * The backend publishes `household:changed` and `service:status-changed`; both
 * are app-wide facts, not per-screen ones. This hook installs one subscription
 * for each and is mounted once by the screen that hosts the current session,
 * rather than by every screen that happens to read the affected data. That is
 * what keeps a second mount from duplicating the listeners, and it is why the
 * hook returns nothing and takes no arguments: the cache arrives through
 * context, exactly as the queries do.
 *
 * **Where it goes:** it is mounted once by each screen that hosts a session —
 * `src/features/households/components/HouseholdDashboardScreen.tsx` and
 * `src/features/settings/components/SettingsScreen.tsx` — above their content,
 * so a screen's reads are current while it is shown. The shell has no layout
 * route of its own in P0, so these two mounts are the app's subscription
 * points, and the export stays the documented entry.
 *
 * **Ownership of the two layers.** The `listen(...)` calls live in each
 * feature's `api.ts` — the only module per feature allowed to import
 * `src/bindings.ts` (project hard rule 2) — and P0's cache policy lives in
 * `households/events.ts` and `sync/events.ts`. This hook is the one place the
 * two are joined, once per session. P1's policy (`applyRabbitChange`,
 * `applyWeightChange` and `invalidateRabbitListsAndRows`) sits in this file
 * instead: it spans `rabbits`, `weight`, `dashboard` and the collection read, so
 * a feature-local `events.ts` would need three cross-feature imports for one
 * mapping.
 *
 * Every subscription is torn down together on unmount (project convention 4).
 * A rejection is handled where the subscription is created, so a bridge that
 * fails mid-session does not surface as an unhandled rejection. It is still not
 * turned into a product state: P0 has no "live updates are off" surface, the
 * screens' own reads keep working, and the failure belongs to the event bridge
 * rather than to a screen's state. That limit is stated rather than hidden
 * behind an invented status.
 *
 * **P1's two events (task 3.9; RAB-5, RAB-6, RAB-7, RAB-10, WT-5, DASH-4).**
 * `rabbit:changed` and `weight:changed` join the same mount because they are
 * app-wide reports too: each names a household, a rabbit and the kind of change
 * and carries no items, so it cannot be merged into the cache.
 *
 * They need one step P0's events do not: a **refresh of the device's copy before
 * any invalidation**. The copy is written only by the read path
 * (`load_household`), never by a write, so invalidating first would refetch the
 * projections from the stale copy the event exists to replace — a confirmed
 * archive would leave the rabbit in the active list, and a threshold edit would
 * leave the old flags, until the next foreground refresh (Q5). `applyRabbitChange`
 * and `applyWeightChange` therefore await the collection read, then invalidate
 * the projections of that event. The collection key itself is not invalidated:
 * the read has just written it, and naming it would refresh the copy twice for
 * one event.
 */
import type { QueryClient } from '@tanstack/react-query';
import { useQueryClient } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { dashboardKeys } from './dashboard/keys';
import { subscribeToHouseholdChanges } from './households/api';
import { applyHouseholdChange } from './households/events';
import { householdKeys } from './households/keys';
import { householdCollectionQueryOptions } from './households/queries';
import type { RabbitChange } from './rabbits/api';
import { subscribeToRabbitChanges } from './rabbits/api';
import { rabbitKeys } from './rabbits/keys';
import { subscribeToServiceStatusChanges } from './sync/api';
import { applyServiceStatus } from './sync/events';
import type { WeightChange } from './weight/api';
import { subscribeToWeightChanges } from './weight/api';
import { weightKeys } from './weight/keys';

/**
 * Observe a subscription's rejection at creation time.
 *
 * `listen` resolves asynchronously, and the mounting screen has no place to
 * await it.
 * Attaching the failure handler here keeps a failed bridge from becoming an
 * unhandled rejection while the effect is alive; the original promise is
 * returned unchanged so cleanup can still release a listener that did register.
 * The failure remains a defect with no product surface, not a state the screens
 * can act on.
 */
function observe(subscription: Promise<UnlistenFn>): Promise<UnlistenFn> {
  void subscription.catch(() => {
    // The event bridge failed; no listener exists to release or to act on.
  });

  return subscription;
}

/**
 * Release one subscription, tolerating a `listen` that never registered.
 *
 * Both branches are supplied so cleanup cannot itself produce an unhandled
 * rejection: a failed subscription has nothing to release.
 */
function release(subscription: Promise<UnlistenFn>): void {
  void subscription.then(
    (unlisten) => {
      unlisten();
    },
    () => {
      // No listener was registered, so there is nothing to release.
    },
  );
}

/**
 * The household the app is showing, or `null` when it is showing none.
 *
 * `householdKeys.active()` is P0's single answer to "which household's data may
 * this screen present", so an event naming any other household is not this
 * device's business (NFR-10). An active read that has not resolved yet means
 * nothing is shown, and there is nothing to keep current.
 */
function shownHouseholdId(client: QueryClient): string | null {
  const active = client.getQueryData<{ id: string } | null>(
    householdKeys.active(),
  );

  return active?.id ?? null;
}

/**
 * Refresh the device's copy of one household through P0's collection read.
 *
 * `staleTime: 0` states the requirement — revalidate now, do not serve whatever
 * the cache still holds — rather than inheriting it from the client's defaults,
 * the same way `resolveHouseholdEntry` does.
 *
 * A failure is not terminal. The content-unloaded refusal (D1.16) is exactly
 * what the service refresh fills, and an unreachable service leaves the
 * previously loaded collection in place. If the refusal survives the refresh, it
 * is the projections invalidated below (`list_rabbits`, `get_rabbit`,
 * `get_dashboard`, `get_weight_entries`) that answer it with their own `unknown`
 * cause, and those are the reads a surface presents; the collection query's own
 * error is not shown by any screen.
 */
async function refreshHouseholdCopy(
  client: QueryClient,
  householdId: string,
): Promise<void> {
  try {
    await client.fetchQuery({
      ...householdCollectionQueryOptions(householdId),
      staleTime: 0,
    });
  } catch {
    // Recorded on the collection query's error state by `fetchQuery`.
  }
}

/**
 * Invalidate the household's two lists and its dashboard rows.
 *
 * Shared by every `rabbit:changed` kind: a rabbit enters or leaves the active
 * and archived lists (archive, restore, delete), and a row carries the rabbit's
 * derived flags (DASH-1). The two list modes are distinct keys by design, so
 * both are named.
 */
function invalidateRabbitListsAndRows(
  client: QueryClient,
  householdId: string,
): void {
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(householdId, 'active'),
  });
  void client.invalidateQueries({
    queryKey: rabbitKeys.list(householdId, 'archived'),
  });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(householdId),
  });
}

/**
 * Apply one `rabbit:changed` report (RAB-5, RAB-6, RAB-7, RAB-10, DASH-4).
 *
 * The copy is refreshed first (see `refreshHouseholdCopy`), then the kind
 * decides what happens to the cached projections:
 *
 * - `deleted` **removes** the rabbit's profile and whole weight series instead
 *   of invalidating them. An invalidated query with an observer would refetch
 *   and refuse (the rabbit is gone), and TanStack keeps the last data on a
 *   failed refetch, so the deleted profile and history would stay on screen
 *   (RAB-10). `removeQueries` drops both entries, the same operation P0 uses for
 *   a departed household's collection.
 * - `created`, `updated`, `archived` and `restored` invalidate the named
 *   rabbit's profile and its whole weight series (the threshold travels with the
 *   series, so a profile edit re-derives the alerts).
 *
 * Both branches invalidate the household's two lists and its dashboard rows.
 * Another household's or another rabbit's key is never named (NFR-10).
 */
export async function applyRabbitChange(
  client: QueryClient,
  change: RabbitChange,
): Promise<void> {
  if (shownHouseholdId(client) !== change.household_id) {
    return;
  }

  await refreshHouseholdCopy(client, change.household_id);

  switch (change.kind) {
    case 'deleted':
      invalidateRabbitListsAndRows(client, change.household_id);
      client.removeQueries({ queryKey: rabbitKeys.detail(change.rabbit_id) });
      client.removeQueries({ queryKey: weightKeys.series(change.rabbit_id) });
      return;

    case 'created':
    case 'updated':
    case 'archived':
    case 'restored':
      invalidateRabbitListsAndRows(client, change.household_id);
      void client.invalidateQueries({
        queryKey: rabbitKeys.detail(change.rabbit_id),
      });
      void client.invalidateQueries({
        queryKey: weightKeys.series(change.rabbit_id),
      });
      return;
  }
}

/**
 * Apply one `weight:changed` report (WT-5, DASH-4).
 *
 * The refresh comes first for the same reason. The named rabbit's series, its
 * profile (named by `design.md`'s set for this event) and the household's
 * dashboard rows are invalidated: a weight write changes no profile field, but
 * the event is the cross-device path and the profile is part of the set the
 * design names. Another rabbit's and another household's keys are left alone
 * (NFR-10).
 */
export async function applyWeightChange(
  client: QueryClient,
  change: WeightChange,
): Promise<void> {
  if (shownHouseholdId(client) !== change.household_id) {
    return;
  }

  await refreshHouseholdCopy(client, change.household_id);

  void client.invalidateQueries({
    queryKey: weightKeys.series(change.rabbit_id),
  });
  void client.invalidateQueries({
    queryKey: rabbitKeys.detail(change.rabbit_id),
  });
  void client.invalidateQueries({
    queryKey: dashboardKeys.rows(change.household_id),
  });
}

/**
 * Install every backend event subscription for the current query cache.
 *
 * Call once in the screen that hosts the session — the dashboard and Settings.
 * One mount per screen is the contract: every read on that screen uses the
 * caches these listeners keep current, so a second subscriber in the same tree
 * would multiply listeners and their refetches without changing what is shown.
 */
export function useAppEvents(): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    const householdChanges = observe(
      subscribeToHouseholdChanges((change) => {
        applyHouseholdChange(queryClient, change);
      }),
    );
    const statusChanges = observe(
      subscribeToServiceStatusChanges((status) => {
        applyServiceStatus(queryClient, status);
      }),
    );
    const rabbitChanges = observe(
      subscribeToRabbitChanges((change) => {
        void applyRabbitChange(queryClient, change);
      }),
    );
    const weightChanges = observe(
      subscribeToWeightChanges((change) => {
        void applyWeightChange(queryClient, change);
      }),
    );

    return () => {
      release(householdChanges);
      release(statusChanges);
      release(rabbitChanges);
      release(weightChanges);
    };
  }, [queryClient]);
}
