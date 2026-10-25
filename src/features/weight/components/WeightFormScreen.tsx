/*
 * The weight form as the route D1.10 requires (task 4.18; WT-2, DASH-9, D1.10).
 *
 * **One route, two quick paths.** D1.10's per-rabbit weigh control and its
 * quick-entry picker both open *this* screen with the rabbit bound through the
 * route's typed search parameters, so range, unit and binding behaviour stay in
 * one place instead of being re-mounted by each control that offers the form.
 * `rabbit` is the binding; `mode` names the entry kind both quick paths share
 * (only quick entry exists); `from` names the surface that opened the form, which
 * is where cancel and a confirmed save return. None of the three is history
 * state: a reload or a deep link reaches the same bound form, and no surface has
 * to guess where the person came from.
 *
 * **The range state travels with the route.** A form opened from a rabbit
 * dashboard has to return to that rabbit's own two windows (WT-6, D1.4), so the
 * route carries the same validated pair the rabbit route does
 * (`validateWeightRangeSearch`) and hands it back on the way out. The dashboard
 * origin ignores it; the rabbit origin cannot otherwise exist.
 *
 * **The form is gated on the reads it cannot invent.** The household comes from
 * the active-household read (NFR-10) and the rabbit is resolved through the
 * profile read, so a stale or unknown id in the search is that read's failure
 * surface rather than a form whose only possible outcome is a refused save. An
 * archived rabbit is not writable at all (RAB-8, D1.5): its profile is read-only,
 * so the form is withheld here as the UI half of the boundary's refusal. The
 * profile read is not household-scoped, so the screen also refuses a rabbit
 * whose household is not the active one: that binding's save would be refused
 * and its confirmed write would invalidate another household's dashboard
 * (NFR-10). A read failure gets `describeReadFailure`'s cause copy and a
 * refresh when a fresh read can change the answer (NFR-8, D1.16) — never an
 * empty or a loading state.
 *
 * **Back exists in every state of the route.** That is the reason this wrapper
 * exists at all (Q11): `WeightForm` gates its fields on the display-unit read,
 * and that gate has no cancel of its own, so while the unit is loading or after
 * the unit read failed the bare form would be a dead end. The navigation bar's
 * leading Back control is mounted above the content in every branch, and the
 * form's own Cancel is wired to the same destination, so the person can always
 * leave — with the entry unsaved, since nothing is written until Save.
 *
 * **Back claims focus only while no form is mounted.** The form focuses its
 * value field on mount, and passive effects flush child-first, so an
 * unconditional mount effect here would run afterwards and land every second
 * open on Back. The focus effect therefore checks that the form is not the
 * rendered branch and otherwise leaves 4.9's field focus standing.
 *
 * The layout is the same `AppShell` every surface uses, so this route joins the
 * existing adaptive arrangement instead of forking the tree (DASH-1).
 */
import { useNavigate } from '@tanstack/react-router';
import { type RefObject, useEffect, useRef } from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { ErrorState } from '../../../components/ui/ErrorState';
import { IconButton } from '../../../components/ui/IconButton';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import { Notice } from '../../../components/ui/Notice';
import type { NavigationDestination } from '../../../components/ui/navigation';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import {
  GlyphBack,
  GlyphDashboard,
} from '../../design-preview/components/glyphs';
import { useActiveHousehold } from '../../households/queries';
import { t } from '../../i18n';
import { useRabbitProfile } from '../../rabbits/queries';
import { describeReadFailure } from '../../sync/read-failure';
import { validateWeightRangeSearch, type WeightFormSearch } from '../search';
import { WeightForm } from './WeightForm';

/** The `main` region's id, so the shell's landmark has a stable name. */
const WEIGHT_FORM_MAIN_ID = 'bs-weight-form-main';

export type WeightFormScreenProps = {
  /** The route's validated binding, mode and origin (D1.10). */
  search: WeightFormSearch;
};

/**
 * The route's content: the shell, the always-present Back, and the form once the
 * household and rabbit it needs have resolved.
 */
export function WeightFormScreen({ search }: WeightFormScreenProps) {
  const navigate = useNavigate();
  const household = useActiveHousehold();
  const profile = useRabbitProfile(search.rabbit);
  const backRef = useRef<HTMLDivElement>(null);

  /*
   * Cancel and a confirmed save share one exit: back to the surface that opened
   * the form, focused on that rabbit. `replace` keeps the form out of the back
   * stack, as the rabbit dashboard's close does, so a browser Back does not
   * reopen a form the person already left.
   */
  function leave() {
    if (search.from === 'rabbit') {
      void navigate({
        to: '/rabbits/$rabbitId',
        params: { rabbitId: search.rabbit },
        replace: true,
        search: validateWeightRangeSearch(search),
      });
      return;
    }

    /*
     * The dashboard restores focus from the history entry's typed state (4.16),
     * so the row that offered the control is where the person lands — not the
     * top of the list.
     */
    void navigate({
      to: '/',
      replace: true,
      state: (previous) => ({
        ...previous,
        focusRabbitId: search.rabbit,
      }),
    });
  }

  const destinations: NavigationDestination[] = [
    {
      to: '/',
      label: t('dashboard.title'),
      icon: <GlyphDashboard />,
    },
  ];

  return (
    <AppShell destinations={destinations} mainId={WEIGHT_FORM_MAIN_ID}>
      <NavigationBar
        leadingAction={
          <div ref={backRef}>
            <IconButton label={t('settings.back')} onClick={leave}>
              <GlyphBack />
            </IconButton>
          </div>
        }
        title={t('weight.log')}
      />
      <Screen>
        <WeightFormContent
          backRef={backRef}
          household={household}
          onLeave={leave}
          profile={profile}
        />
      </Screen>
    </AppShell>
  );
}

/**
 * The read gate between the route and `WeightForm` (D1.16, D1.5).
 *
 * The household's answer comes first: without it there is no scope to write
 * into, and `null` is the entry route's own state (it sends that person to
 * onboarding), so this route draws no form rather than inventing one. The
 * profile then resolves the rabbit the search names; an archived rabbit shows its
 * read-only state instead of a write control.
 *
 * Back takes focus in each branch that draws no form. The effect runs after the
 * form's own child effects, so it must not claim focus when the form is the
 * branch that rendered — that would undo 4.9's value-field focus on every open
 * whose display unit is already cached.
 */
function WeightFormContent({
  backRef,
  household,
  profile,
  onLeave,
}: {
  backRef: RefObject<HTMLDivElement | null>;
  household: ReturnType<typeof useActiveHousehold>;
  profile: ReturnType<typeof useRabbitProfile>;
  onLeave: () => void;
}) {
  /*
   * The form is mounted, including while its display unit is loading or failed
   * (its own card owns those surfaces), exactly when the household is resolved,
   * the profile is not archived, and the rabbit belongs to that household.
   */
  const formShowing =
    household.data !== undefined &&
    household.data !== null &&
    profile.data !== undefined &&
    !profile.data.is_archived &&
    profile.data.household_id === household.data.id;

  useEffect(() => {
    if (formShowing) {
      return;
    }

    backRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
  }, [backRef, formShowing]);

  if (household.data === undefined) {
    return household.error === null ? (
      <LoadingIndicator label={t('entry.loading')} />
    ) : (
      <ReadFailure
        error={household.error}
        onRefresh={() => {
          void household.refetch();
        }}
        title={t('entry.loadFailed.title')}
      />
    );
  }

  /*
   * No active household is not a failure this screen can name: `/` owns that
   * state (onboarding, or the connectivity-required surface). The Back control
   * above is the way to it.
   */
  if (household.data === null) {
    return null;
  }

  if (profile.data === undefined) {
    return profile.error === null ? (
      <LoadingIndicator label={t('rabbit.loading')} />
    ) : (
      <ReadFailure
        error={profile.error}
        onRefresh={() => {
          void profile.refetch();
        }}
        title={t('rabbit.loadFailed.title')}
      />
    );
  }

  /*
   * `get_rabbit` is not household-scoped: it answers any rabbit this device
   * holds, including one of another household. A form bound to it would draw
   * fields whose save the boundary refuses, and a confirmed write would
   * invalidate the wrong household's dashboard (NFR-10). The binding is the
   * screen's own scope check, and a mismatch is invalid input: the search names
   * a rabbit this household does not own, which another read cannot fix.
   */
  if (profile.data.household_id !== household.data.id) {
    return (
      <ErrorState title={t('rabbit.loadFailed.title')}>
        {t('error.validation')}
      </ErrorState>
    );
  }

  if (profile.data.is_archived) {
    return <Notice tone="info">{t('rabbit.archived.readOnly')}</Notice>;
  }

  return (
    <>
      {/*
       * The binding made visible: the picker closes over the chosen rabbit and
       * the row path already names it, but the form itself only says "Log
       * weight". The name is data, not a new catalog string.
       */}
      <Text>{profile.data.name}</Text>
      <WeightForm
        householdId={household.data.id}
        onCancel={onLeave}
        onSaved={onLeave}
        rabbitId={profile.data.id}
      />
    </>
  );
}

/**
 * A failed read, with the cause's copy and its one action (NFR-8, D1.16).
 *
 * A fresh read is the only action a read has, so the refresh is offered exactly
 * when `describeReadFailure` says another read can change the answer; a
 * `validation` refusal gets no control that would repeat it.
 */
function ReadFailure({
  error,
  title,
  onRefresh,
}: {
  error: unknown;
  title: string;
  onRefresh: () => void;
}) {
  const failure = describeReadFailure(error);

  return (
    <ErrorState
      {...(failure.refreshable ? { onRetry: onRefresh } : {})}
      retryLabel={t('error.retry')}
      title={title}
    >
      {t(failure.messageKey)}
    </ErrorState>
  );
}
