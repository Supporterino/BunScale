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
 * `src/bindings.ts` (project hard rule 2) — and the cache policy lives in
 * `households/events.ts` and `sync/events.ts`. This hook is the one place the
 * two are joined, once per session.
 *
 * Both subscriptions are torn down together on unmount (project convention 4).
 * A rejection is handled where the subscription is created, so a bridge that
 * fails mid-session does not surface as an unhandled rejection. It is still not
 * turned into a product state: P0 has no "live updates are off" surface, the
 * screens' own reads keep working, and the failure belongs to the event bridge
 * rather than to a screen's state. That limit is stated rather than hidden
 * behind an invented status.
 */
import { useQueryClient } from '@tanstack/react-query';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { subscribeToHouseholdChanges } from './households/api';
import { applyHouseholdChange } from './households/events';
import { subscribeToServiceStatusChanges } from './sync/api';
import { applyServiceStatus } from './sync/events';

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
 * Install both backend event subscriptions for the current query cache.
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

    return () => {
      release(householdChanges);
      release(statusChanges);
    };
  }, [queryClient]);
}
