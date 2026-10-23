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
 * **The profile detail is inline too (task 4.4; RAB-5).** `RabbitList` already
 * accepts `onOpenRabbit`; this screen supplies it, so each row becomes a real
 * control and a row replaces the list with `RabbitDetail` in the same scroll
 * region. That is the same swap the form uses, and the detail owns its own edit
 * state. Task 4.11 replaces this destination with the rabbit dashboard route;
 * until then the detail is the destination the row's control has.
 *
 * **The archived list is reached from here, in every state (task 4.5; RAB-6).**
 * A grouped list row above the content opens `ArchivedRabbitList` in the same
 * scroll region. It is rendered outside the form/detail/list swap, so the
 * pending read, zero rows, rows, the create form and the profile all carry the
 * way in: a household whose rabbits are all archived is not trapped behind "No
 * rabbits yet", because that empty state belongs to the active list (DASH-8,
 * design.md D1.6). Opening the archived list hides the dashboard content rather
 * than unmounting it, so an unsaved create or edit draft survives the visit;
 * closing the list returns focus to the entry point.
 *
 * **An archive that empties the list is announced through a region that already
 * exists.** A `role="status"` region is mounted with the dashboard and kept
 * empty while rows are present; on the rows→zero transition the empty state's
 * copy is written into it, because a live region created at the moment of the
 * replacement announces nothing.
 *
 * **Focus follows the swap in both directions.** The Add control unmounts when
 * the form takes its place, so the form focuses its first field on mount; on
 * cancel or a confirmed create the list returns and this screen puts focus back
 * on the Add control it was opened from. A row likewise unmounts when the detail
 * takes its place, and the detail's Back control returns focus to that row by
 * the rabbit's id, so a duplicate name or a rename before closing still lands on
 * the row that was opened. Without that, a keyboard or screen-reader user would
 * restart at the skip link after every swap.
 */
import { useEffect, useRef, useState } from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { IconButton } from '../../../components/ui/IconButton';
import { List } from '../../../components/ui/List';
import { ListRow } from '../../../components/ui/ListRow';
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
import { ArchivedRabbitList } from '../../rabbits/components/ArchivedRabbitList';
import { RabbitDetail } from '../../rabbits/components/RabbitDetail';
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
  const [openRabbitId, setOpenRabbitId] = useState<string | null>(null);
  const [archivedOpen, setArchivedOpen] = useState(false);
  const [emptyAnnouncement, setEmptyAnnouncement] = useState('');
  const wasCreating = useRef(false);
  const wasOpen = useRef(false);
  const wasArchived = useRef(false);
  const lastRabbitId = useRef<string | null>(null);
  const hadActiveRows = useRef(false);

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
   * The same swap for the detail: the row that opened it has been unmounted, so
   * Back returns focus to that row by the rabbit's id, not by its visible text —
   * two rabbits may share a name prefix ("Hazel" and "Hazel 2"). The read is
   * waited on because the detail's own save invalidates the dashboard rows, and
   * focusing a row the new read has not painted yet would land nowhere.
   */
  useEffect(() => {
    if (openRabbitId !== null) {
      wasOpen.current = true;
      lastRabbitId.current = openRabbitId;
      return;
    }

    if (!wasOpen.current || dashboardRows.isFetching) {
      return;
    }

    wasOpen.current = false;

    const id = lastRabbitId.current;
    const scope = document.querySelector('[data-household-entry="household"]');
    const row =
      id === null
        ? undefined
        : Array.from(
            scope?.querySelectorAll<HTMLElement>('[data-rabbit-id]') ?? [],
          ).find((element) => element.dataset.rabbitId === id);

    /*
     * An archived rabbit is no longer an active row, so there is nothing to
     * return to. The archived entry point is where that rabbit went, and it is
     * the next useful control rather than `<body>` (task 4.5, RAB-6).
     */
    (
      row?.querySelector<HTMLButtonElement>('button.bs-list-row__target') ??
      document.querySelector<HTMLButtonElement>(
        '.bs-dashboard__archived button',
      )
    )?.focus();
  }, [openRabbitId, dashboardRows.isFetching]);

  /*
   * The archived list's entry point is unmounted while that screen is open, so
   * closing it returns focus to the control it was opened from (task 4.5).
   */
  useEffect(() => {
    if (archivedOpen) {
      wasArchived.current = true;
      return;
    }

    if (!wasArchived.current) {
      return;
    }

    wasArchived.current = false;
    document
      .querySelector<HTMLButtonElement>('.bs-dashboard__archived button')
      ?.focus();
  }, [archivedOpen]);

  /*
   * An archive can leave the active list at zero rows, and the empty state then
   * replaces content rather than being the screen's initial state (DASH-8's
   * contract in `EmptyState`). A `role="status"` region is mounted with the
   * dashboard and stays empty while rows are present, so the region exists
   * before the replacement; the empty state's copy is written into it on the
   * rows→zero transition. First arrival on an all-archived household writes
   * nothing. A later non-empty read clears it, so the next transition is a
   * change again.
   *
   * While the archived list is presented the active list is off screen: a
   * rows→zero change arriving then must not announce the dashboard's empty
   * state over that screen, so the region is cleared and left silent. It is
   * re-evaluated when the dashboard is shown again.
   */
  useEffect(() => {
    if (archivedOpen) {
      setEmptyAnnouncement('');
      return;
    }

    if (dashboardRows.data === undefined) {
      return;
    }

    if (dashboardRows.data.length > 0) {
      hadActiveRows.current = true;
      setEmptyAnnouncement('');
      return;
    }

    if (hadActiveRows.current) {
      setEmptyAnnouncement(
        `${t('dashboard.emptyTitle')}. ${t('dashboard.emptyBody')}`,
      );
    }
  }, [dashboardRows.data, archivedOpen]);

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
        {/*
         * Mounted before anything can replace rows, and empty until then: a
         * region that appears with the replacement announces nothing. It is
         * visually hidden on the region itself (out of flow), so an empty
         * region does not add a gap step to the content column.
         */}
        <div
          className="bs-visually-hidden"
          data-announcement="dashboard"
          role="status"
        >
          {emptyAnnouncement}
        </div>
        {activeHousehold.data === undefined ||
        activeHousehold.data === null ? null : (
          <LocalCopyNotice householdId={activeHousehold.data.id} />
        )}
        {/*
         * The dashboard stays mounted while the archived list is open: opening
         * it from the create form or the detail's edit form must not unmount
         * them and discard a draft. `hidden` is on a wrapper with no `display`
         * of its own, so the attribute is not overridden; the inner wrapper
         * keeps the screen's stack rhythm.
         */}
        <div className="bs-dashboard__surface" hidden={archivedOpen}>
          <div className="bs-dashboard__content">
            {/*
             * RAB-6's entry point, present in every dashboard state — the
             * pending read, zero rows, rows, the form and the detail. Without
             * it an all-archived household is trapped behind "No rabbits yet",
             * because the empty state is keyed on the active list (DASH-8).
             */}
            <List className="bs-dashboard__archived">
              <ListRow
                accessory="chevron"
                onClick={() => {
                  setArchivedOpen(true);
                }}
                title={t('rabbit.archivedList.title')}
              />
            </List>
            {creatingRabbit &&
            activeHousehold.data !== undefined &&
            activeHousehold.data !== null ? (
              <RabbitForm
                householdId={activeHousehold.data.id}
                onCancel={() => setCreatingRabbit(false)}
                onSaved={() => setCreatingRabbit(false)}
              />
            ) : openRabbitId !== null ? (
              <RabbitDetail
                onClose={() => setOpenRabbitId(null)}
                rabbitId={openRabbitId}
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
                onOpenRabbit={setOpenRabbitId}
              />
            )}
          </div>
        </div>
        {archivedOpen ? (
          <ArchivedRabbitList
            onClose={() => {
              setArchivedOpen(false);
            }}
          />
        ) : null}
      </Screen>
    </AppShell>
  );
}
