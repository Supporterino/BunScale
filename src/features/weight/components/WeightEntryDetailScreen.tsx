/*
 * The weight entry detail's route content (task 4.19; WT-8, NFR-9).
 *
 * **Placeholder — task 4.21 owns the presentation.** This file exists so the
 * detail route has a screen to mount and the entry read has a surface: it draws
 * the read's states and the one control every state needs, Back, and it
 * deliberately stops there. The display-unit value's layout, the photo strip
 * (`weight.detail.photos`), Edit opening the pre-filled form and a confirmed
 * Delete are 4.21's; restyling this file is how that task starts.
 *
 * What it does own, because it is the route's contract rather than a design:
 *
 * - **The read.** `useWeightEntry` takes the entry id and the rabbit its key is
 *   scoped by (`../queries`); the rabbit arrives in the route's typed search
 *   because `get_weight_entry` answers the entry only.
 * - **Back in every state.** The detail is a pushed screen, so it never relies
 *   on a browser or native back gesture; the navigation bar's leading control
 *   returns to the surface that opened it — the rabbit dashboard at the windows
 *   it was left at, or the dashboard list focused on the rabbit — and it is
 *   mounted above the read's pending and failed states too, so neither is a
 *   dead end (Q11's rule, applied to this route).
 *
 * The layout is the same `AppShell` every surface uses, so iOS and macOS share
 * this node and it joins the existing adaptive arrangement instead of forking
 * the tree (DASH-1).
 */
import { useNavigate } from '@tanstack/react-router';
import { AppShell } from '../../../components/ui/AppShell';
import { Badge } from '../../../components/ui/Badge';
import { ErrorState } from '../../../components/ui/ErrorState';
import { IconButton } from '../../../components/ui/IconButton';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import type { NavigationDestination } from '../../../components/ui/navigation';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import {
  GlyphBack,
  GlyphDashboard,
} from '../../design-preview/components/glyphs';
import { t } from '../../i18n';
import { useDisplayWeightUnit } from '../../settings/display-unit';
import { describeReadFailure } from '../../sync/read-failure';
import { entryHasDropAlert } from '../alerts';
import { useWeightEntry } from '../queries';
import type { WeightRange } from '../search';
import { formatWeight } from './WeightSeries';

/** The `main` region's id, so the shell's landmark has a stable name. */
const WEIGHT_ENTRY_MAIN_ID = 'bs-weight-entry-main';

/**
 * Where Back, and later Edit and Delete, return (task 4.19, mirroring 4.18's
 * `from`): `rabbit` is that rabbit's own dashboard at the windows it was left
 * at; `dashboard` is the household list, focused on the entry's rabbit.
 *
 * The route's validator now names the same union as `WeightFormOrigin`
 * (`../search`), so this alias is a duplicate held only because this task's
 * scope was the placeholder's behaviour. Collapse it onto `WeightFormOrigin`
 * when 4.21 restyles this screen.
 */
export type WeightEntryOrigin = 'dashboard' | 'rabbit';

export type WeightEntryDetailScreenProps = {
  /** The entry the route's typed parameter names (WT-8). */
  entryId: string;
  /**
   * The rabbit the entry belongs to, from the route's typed search. It scopes
   * the read's cache entry and the return, and it is not an argument of
   * `get_weight_entry`.
   */
  rabbitId: string;
  /** The surface this detail was opened from. */
  from: WeightEntryOrigin;
  /** The rabbit dashboard's windows, returned unchanged (WT-6, D1.4). */
  trend: WeightRange;
  history: WeightRange;
};

/*
 * The timestamp is an instant, so it is shown in the device's zone; the value's
 * digits come from the product's one conversion (`./WeightSeries`'
 * `formatWeight`) and the number format carries the unit's symbol. No format
 * string is written by hand here.
 */
const TIMESTAMP_FORMAT = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
});

export function WeightEntryDetailScreen({
  entryId,
  rabbitId,
  from,
  trend,
  history,
}: WeightEntryDetailScreenProps) {
  const navigate = useNavigate();
  const entry = useWeightEntry(rabbitId, entryId);
  const unit = useDisplayWeightUnit();

  /*
   * One exit for every state: `replace` keeps the detail out of the back stack,
   * the way the weight form and the rabbit dashboard's close do, so a browser
   * Back after leaving does not reopen a screen the person already left.
   */
  function leave() {
    if (from === 'rabbit' && rabbitId !== '') {
      void navigate({
        to: '/rabbits/$rabbitId',
        params: { rabbitId },
        replace: true,
        search: { trend, history },
      });
      return;
    }

    if (rabbitId === '') {
      void navigate({ to: '/', replace: true });
      return;
    }

    /*
     * The list restores focus from the history entry's typed state (4.16), so
     * the row that named this rabbit is where the person lands.
     */
    void navigate({
      to: '/',
      replace: true,
      state: (previous) => ({ ...previous, focusRabbitId: rabbitId }),
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
    <AppShell destinations={destinations} mainId={WEIGHT_ENTRY_MAIN_ID}>
      <NavigationBar
        leadingAction={
          <IconButton label={t('settings.back')} onClick={leave}>
            <GlyphBack />
          </IconButton>
        }
        title={t('weight.entry.title')}
      />
      <Screen>
        <EntryContent entry={entry} unit={unit} />
      </Screen>
    </AppShell>
  );
}

/**
 * The read's states (D1.16, NFR-8).
 *
 * A pending read is a labelled spinner; a failed read is the cause's copy with a
 * refresh exactly when another read can change the answer (`describeReadFailure`
 * owns that decision) — never an empty detail; and the answer is the entry's
 * facts. The value waits for the display unit rather than guessing one, the same
 * boundary every weight surface keeps (WT-3, DM-1).
 */
function EntryContent({
  entry,
  unit,
}: {
  entry: ReturnType<typeof useWeightEntry>;
  unit: ReturnType<typeof useDisplayWeightUnit>;
}) {
  if (entry.data === undefined) {
    if (entry.error === null) {
      /*
       * Task 4.21 adds `weight.entry.loading`: `weight.loading` is the series
       * copy ("Loading weight entries…") and is wrong for one entry. It does not
       * exist yet, so this borrows it until that task adds the singular key.
       */
      return <LoadingIndicator label={t('weight.loading')} />;
    }

    return (
      <ReadFailure
        error={entry.error}
        onRefresh={() => {
          void entry.refetch();
        }}
        title={t('weight.entry.title')}
      />
    );
  }

  const { data } = entry;

  return (
    <>
      {unit.data === undefined ? (
        unit.error === null ? (
          <LoadingIndicator label={t('settings.displayUnit.loading')} />
        ) : (
          <ReadFailure
            error={unit.error}
            onRefresh={() => {
              void unit.refetch();
            }}
            title={t('weight.entry.title')}
          />
        )
      ) : (
        <Text variant="title-2">
          {formatWeight(data.value_grams, unit.data)}
        </Text>
      )}
      <Text tone="secondary">
        <time dateTime={data.timestamp}>
          {TIMESTAMP_FORMAT.format(new Date(data.timestamp))}
        </time>
      </Text>
      {data.note === null ? null : <Text>{data.note}</Text>}
      {entryHasDropAlert(data) ? (
        <Text>
          <Badge tone="warning">{t('weight.dropAlert')}</Badge>
        </Text>
      ) : null}
      {/*
       * The photos section is 4.21's (`weight.detail.photos`); the payload's
       * `photo_refs` is read there, so this placeholder draws no empty pager a
       * real entry would then replace.
       */}
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
