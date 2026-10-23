/*
 * The dashboard shell (task 4.2; SET-1, HS-2).
 *
 * The adaptive dashboard shell, now carrying P1's rabbit content. It composes
 * the app frame — the same route tree, rendered as a bottom tab bar at 390×844
 * and as a sidebar at 1280×800 — and it carries SET-1's gear control in the
 * navigation bar. Task 4.1 mounted the rabbit list in the content area
 * (DASH-1 … DASH-3); P0's "what this household is" intro is that list's
 * fallback while it has no rows to show. Task 4.2 added DASH-8's empty state,
 * and task 4.3 wires both it and the list's toolbar control to the inline
 * creation form below. The row control (DASH-9, 4.7) and the drop flag (DASH-4,
 * 4.12) remain later tasks.
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
 *
 * **The creation form is inline (task 4.3; RAB-4).** The design system defers
 * `Sheet`/`Dialog`, and P0's form pattern is inline (Settings' "create another
 * household"), so opening the add control replaces the list with `RabbitForm`
 * inside the same dashboard. Creating or cancelling returns to the list; the
 * confirmed write invalidates the dashboard read, so the new rabbit is what the
 * list presents. The add control is passed into `RabbitList`, which renders it
 * in the pending and non-empty states, while the zero-row state uses its own
 * prompt (DASH-8); every dashboard state therefore leads here.
 *
 * **Focus follows the swap in both directions.** The Add control unmounts when
 * the form takes its place, so the form focuses its first field on mount; on
 * cancel or a confirmed create the list returns and this screen puts focus back
 * on the Add control it was opened from. Without that, a keyboard or
 * screen-reader user would restart at the skip link after every swap.
 */
import { useEffect, useRef, useState } from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { IconButton } from '../../../components/ui/IconButton';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import type { NavigationDestination } from '../../../components/ui/navigation';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import { useAppEvents } from '../../app-events';
import { RabbitList } from '../../dashboard/components/RabbitList';
import { useDashboardRows } from '../../dashboard/queries';
import {
  GlyphDashboard,
  GlyphHousehold,
  GlyphSettings,
} from '../../design-preview/components/glyphs';
import { t } from '../../i18n';
import { RabbitForm } from '../../rabbits/components/RabbitForm';
import { LocalCopyNotice } from '../../sync/components/LocalCopyNotice';
import { useActiveHousehold } from '../queries';

export type HouseholdDashboardScreenProps = {
  /** Opens Settings & Profile as its own screen (SET-1). Required. */
  onOpenSettings: () => void;
};

export function HouseholdDashboardScreen({
  onOpenSettings,
}: HouseholdDashboardScreenProps) {
  useAppEvents();

  const [creatingRabbit, setCreatingRabbit] = useState(false);
  const wasCreating = useRef(false);

  const activeHousehold = useActiveHousehold();
  const householdName = activeHousehold.data?.name ?? t('dashboard.title');

  /*
   * The screen is also an observer of the list read so the form swap has a
   * settled answer to restore focus into. With no observer the invalidated read
   * only starts when the list remounts, which would briefly paint the empty
   * state's Add control and then replace it under the focus.
   */
  const dashboardRows = useDashboardRows();

  /*
   * The Add control is unmounted while the form is open, so the node it was in
   * no longer exists when the list comes back. Focus is returned by accessible
   * name: both the empty state and the toolbar use `rabbit.add`, and a new node
   * is the only one there is to focus. Waiting for the read to settle keeps the
   * focus on the control the final list actually presents.
   */
  useEffect(() => {
    if (creatingRabbit) {
      wasCreating.current = true;
      return;
    }

    if (!wasCreating.current || dashboardRows.isFetching) {
      return;
    }

    wasCreating.current = false;

    const scope = document.querySelector('[data-household-entry="household"]');
    const add = Array.from(scope?.querySelectorAll('button') ?? []).find(
      (button) => button.textContent?.trim() === t('rabbit.add'),
    );

    add?.focus();
  }, [creatingRabbit, dashboardRows.isFetching]);

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
        {activeHousehold.data === undefined ||
        activeHousehold.data === null ? null : (
          <LocalCopyNotice householdId={activeHousehold.data.id} />
        )}
        {creatingRabbit &&
        activeHousehold.data !== undefined &&
        activeHousehold.data !== null ? (
          <RabbitForm
            householdId={activeHousehold.data.id}
            onCancel={() => setCreatingRabbit(false)}
            onCreated={() => setCreatingRabbit(false)}
          />
        ) : (
          <RabbitList
            fallback={
              <div className="bs-dashboard__intro">
                <span aria-hidden="true" className="bs-dashboard__mark">
                  <GlyphHousehold />
                </span>
                <Text tone="secondary">{t('dashboard.body')}</Text>
              </div>
            }
            onAddRabbit={
              activeHousehold.data === undefined ||
              activeHousehold.data === null
                ? undefined
                : () => setCreatingRabbit(true)
            }
          />
        )}
      </Screen>
    </AppShell>
  );
}
