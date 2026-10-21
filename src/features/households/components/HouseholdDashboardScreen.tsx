/*
 * The dashboard shell (task 4.2; SET-1, HS-2).
 *
 * This is the P0 shell, not the P1 dashboard. It composes the adaptive app frame
 * — the same route tree, rendered as a bottom tab bar at 390×844 and as a
 * sidebar at 1280×800 — and it carries SET-1's gear control in the navigation
 * bar. P0 has no rabbit data, so the content area states what the household is
 * and nothing more; rabbit cards, empty states and quick-weigh belong to
 * DASH-1..DASH-9 in P1 (design.md R8) and are deliberately not invented here.
 *
 * **The gear.** It is an `IconButton` (44×44, required label) with the system's
 * gear glyph, its accessible name is `dashboard.settings` ("Open Settings &
 * Profile"), and it sits in the navigation bar's trailing slot so it stays in
 * the same place in both arrangements. SET-1 says activating it opens Settings &
 * Profile as its own screen: `src/routes/index.tsx` supplies
 * `onOpenSettings={() => navigate({ to: '/settings' })}` and the required prop
 * keeps the control from ever rendering without a destination. SET-1's
 * `[desktop]` scenario is satisfied by that route change — the sidebar's own
 * link brings the dashboard back.
 *
 * **Why the household name is the title.** It is the screen's identity and the
 * only household fact P0 has; it comes from the same active-household read the
 * entry guard has already resolved, so it is on screen without a spinner. The
 * person's name is SET-2's and belongs to Settings & Profile (task 4.3); it is
 * not duplicated here. When the book holds no active household the title falls
 * back to `dashboard.title` ("Your household") rather than an empty `h1`, and the
 * navigation destination always uses that key.
 *
 * **`useAppEvents` is mounted here, once**, above the content. Task 3.6 named
 * the shell as the caller: the subscription keeps the household and service
 * caches current for every screen that reads them. It is mounted once per
 * dashboard mount, which is the widest shell P0 has until a root `_app` layout
 * exists (routes are not this task's to add).
 */
import { AppShell } from '../../../components/ui/AppShell';
import { IconButton } from '../../../components/ui/IconButton';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import type { NavigationDestination } from '../../../components/ui/navigation';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import { useAppEvents } from '../../app-events';
import {
  GlyphDashboard,
  GlyphHousehold,
  GlyphSettings,
} from '../../design-preview/components/glyphs';
import { t } from '../../i18n';
import { useActiveHousehold } from '../queries';

export type HouseholdDashboardScreenProps = {
  /** Opens Settings & Profile as its own screen (SET-1). Required. */
  onOpenSettings: () => void;
};

export function HouseholdDashboardScreen({
  onOpenSettings,
}: HouseholdDashboardScreenProps) {
  useAppEvents();

  const activeHousehold = useActiveHousehold();
  const householdName = activeHousehold.data?.name ?? t('dashboard.title');

  /*
   * One destination, because P0 has one product route for the shell. Its label
   * is `dashboard.title` ("Your household"), not the household's own name: a
   * navigation item should name a place, and the same label must not change when
   * the active household is renamed or absent. It grows with the route tree.
   */
  const destinations: NavigationDestination[] = [
    {
      to: '/',
      label: t('dashboard.title'),
      icon: <GlyphDashboard />,
    },
  ];

  return (
    <AppShell
      destinations={destinations}
      mainData={{ 'data-household-entry': 'household' }}
    >
      <NavigationBar
        title={householdName}
        trailingActions={
          <IconButton label={t('dashboard.settings')} onClick={onOpenSettings}>
            <GlyphSettings />
          </IconButton>
        }
      />
      <Screen className="bs-dashboard">
        <div className="bs-dashboard__intro">
          <span aria-hidden="true" className="bs-dashboard__mark">
            <GlyphHousehold />
          </span>
          <Text tone="secondary">{t('dashboard.body')}</Text>
        </div>
      </Screen>
    </AppShell>
  );
}
