/*
 * The entry route's screen compositions (HS-2, SYNC-3, DM-3).
 *
 * Tasks 3.3 placed the routes; tasks 4.1, 4.2 and 4.4 own the screens. The real
 * screens live in their own modules and are composed here, because the route
 * already imports this file; only the pending/error slots remain placeholders.
 * The one thing this file carries beyond the decision is `data-household-entry`,
 * the stable name of the entry state the guard reached, which the specs assert
 * instead of copy that will change. The dashboard's is on its `main`, through
 * `AppShell`'s `mainData`; the connectivity screen's is on its standalone
 * `Screen`'s `main`.
 *
 * Which surface is which:
 *
 * - `HouseholdOnboardingScreen` — HS-2's first-run household creation (task
 *   4.1), imported directly by the `/onboarding` route.
 * - `HouseholdDashboardScreen` — the P0 dashboard shell (task 4.2): adaptive
 *   navigation, the gear control's SET-1 destination, and the app-event
 *   subscription.
 * - `ConnectivityRequiredScreen` — the explicit connectivity-required state
 *   (task 4.4, `src/features/sync/components/`). The guard reaches it whenever
 *   the service or the account cannot confirm "no household"; it carries the
 *   observed status as data attributes and owns the probe + re-decision action.
 * - `EntryPendingPlaceholder` / `EntryErrorPlaceholder` — the route's pending and
 *   error surfaces, which `tanstack-router-conventions` requires on a route whose
 *   `beforeLoad` reads data. A failed read (an IPC defect, or a local book that
 *   cannot be read) lands here; it deliberately never lands in onboarding.
 */

import { useRouter } from '@tanstack/react-router';
import { ErrorState } from '../../../components/ui/ErrorState';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Screen } from '../../../components/ui/Screen';
import { t } from '../../i18n';
import { ConnectivityRequiredScreen } from '../../sync/components/ConnectivityRequiredScreen';
import { describeSaveFailure } from '../../sync/failure';
import type { HouseholdEntry } from '../entry';
import { HouseholdDashboardScreen } from './HouseholdDashboardScreen';
import { HouseholdOnboardingScreen } from './HouseholdOnboardingScreen';

/**
 * The entry route's screen: the dashboard, or the connectivity slot.
 *
 * `onOpenSettings` is the dashboard shell's SET-1 destination, supplied by
 * `src/routes/index.tsx` as `navigate({ to: '/settings' })`. It is required so
 * the gear can never render as an enabled control with no destination.
 */
export function HouseholdEntryScreen({
  entry,
  onOpenSettings,
}: {
  entry: HouseholdEntry;
  onOpenSettings: () => void;
}) {
  switch (entry.kind) {
    case 'household':
      return <HouseholdDashboardScreen onOpenSettings={onOpenSettings} />;
    case 'service-required':
      return <ConnectivityRequiredScreen status={entry.status} />;
    case 'onboarding':
      // The route redirects on this entry; rendering it is a defensive
      // fallback rather than a state this screen is designed for.
      return <HouseholdOnboardingScreen />;
  }
}

/*
 * HS-2's first-run surface (task 4.1). Built in its own module so the screen's
 * form, keyboard measurement and copy stay together; the `/onboarding` route
 * imports that module directly (Q43).
 */

/** The entry route's pending surface (`beforeLoad` may probe for up to 5 s). */
export function EntryPendingPlaceholder() {
  return (
    <Screen
      aria-busy="true"
      as="main"
      className="bs-entry-status"
      data-household-entry="pending"
    >
      <LoadingIndicator variant="screen" />
    </Screen>
  );
}

/**
 * The entry route's error surface.
 *
 * The route's `beforeLoad` reads the household book; a failed read is a failure,
 * so it is shown as an `ErrorState` and not as "no household" (the point of the
 * guard). The body is the cause map's copy (`describeSaveFailure`), and the retry
 * is the router's own `invalidate()`, which re-runs the failed `beforeLoad`
 * (a re-read, safe to repeat); `reset()` alone would only clear the boundary and
 * leave the stale decision in place. The `h1` is `entry.loadFailed.title`, with
 * the cause's copy directly beneath it.
 */
export function EntryErrorPlaceholder({ error }: { error: unknown }) {
  const failure = describeSaveFailure(error);
  const router = useRouter();

  return (
    <Screen
      as="main"
      className="bs-entry-status"
      data-failure={failure.cause}
      data-household-entry="error"
    >
      <ErrorState
        headingLevel="h1"
        onRetry={() => {
          void router.invalidate();
        }}
        retryLabel={t('error.retry')}
        title={t('entry.loadFailed.title')}
      >
        {t(failure.messageKey)}
      </ErrorState>
    </Screen>
  );
}
