/*
 * The weight entry detail's route content (task 4.21; WT-8, NFR-9).
 *
 * One entry, as 4.19's route names it: the value in the display unit, the
 * timestamp, the whole note, the derived drop marker, the photos, and the two
 * actions WT-5 gives an entry — correct it, or remove it behind a confirmation
 * that states the consequence.
 *
 * **The rabbit comes from the entry; the mounted key is the search's.** The
 * route's typed search carries the rabbit that scopes the detail's cache key
 * (`../keys`' `detail`), but that value can be blank on a deep link (4.19's
 * documented limit). The key is not moved onto `entry.rabbit_id`: a second
 * observer under the entry's rabbit would need its mount read suppressed — the
 * shared client revalidates on mount on purpose (`src/lib/query-client.ts`) —
 * and a rule to prefer whichever observer read last. The limit is therefore
 * named rather than hidden: on the mismatched path a confirmed edit asks for its
 * own confirming read (below), and the derived drop flag keeps the value the last
 * read produced until the entry is read again, the side `../keys`' `detail`
 * warns about.
 *
 * Every write this screen starts is scoped by `entry.rabbit_id` once the entry
 * is loaded, so an edit or a delete cannot invalidate another rabbit's series,
 * and Back returns to the entry's own rabbit rather than to nothing. The profile
 * read is the one that answers the two facts the writes need and the payload
 * does not carry: the household the invalidation is scoped to (NFR-10) and
 * whether the rabbit is archived, in which case the entry is read-only (RAB-8,
 * D1.5). That read is normally already cached — this screen is reached from the
 * rabbit's dashboard — and while it is unavailable the actions are withheld
 * rather than offered and refused, with the failure named and its refresh
 * offered in their place.
 *
 * **Edit replaces the facts, it does not open a second surface.** The
 * established pattern is the inline form that takes over the surface that
 * opened it (`RabbitForm`, `WeightForm`); 4.9's form gains the update mode, and
 * its Cancel and a confirmed save both return here. What it shows after a save
 * is the confirming read rather than a local guess (D1): when the search's
 * rabbit agrees with the entry's, the update's `series(entry.rabbit_id)`
 * invalidation reaches the mounted key, and when it does not — the blank or
 * wrong `?rabbit=` deep link — the screen asks for that read itself, because an
 * invalidation cannot reach a key outside the entry's prefix.
 *
 * **Delete is irreversible, so it is confirmed** (NFR-9). The dialog states the
 * consequence — the entry is removed for every member of the household and
 * cannot be undone — and is the weight feature's own copy of the pattern
 * `DeleteRabbitDialog` established, reusing the frozen `.bs-delete-dialog`
 * rules: the design system defers a modal primitive, and a second consumer is
 * what would justify promoting one.
 *
 * **Every state keeps a way back** (Q11). Back is mounted above the pending and
 * failed reads too, so neither is a dead end, and it returns to the surface that
 * opened the detail: the rabbit dashboard at the windows it was left at, or the
 * dashboard list focused on the entry's rabbit.
 *
 * The layout is the same `AppShell` every surface uses, so iOS and macOS share
 * this node and it joins the existing adaptive arrangement instead of forking
 * the tree (DASH-1).
 */
import { useNavigate } from '@tanstack/react-router';
import {
  type CSSProperties,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { ErrorState } from '../../../components/ui/ErrorState';
import { Heading } from '../../../components/ui/Heading';
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
import { useRabbitProfile } from '../../rabbits/queries';
import { useDisplayWeightUnit } from '../../settings/display-unit';
import { describeSaveFailure } from '../../sync/failure';
import { describeReadFailure } from '../../sync/read-failure';
import { entryHasDropAlert } from '../alerts';
import { useDeleteWeightEntry } from '../mutations';
import { useWeightEntry } from '../queries';
import type { WeightFormOrigin, WeightRange } from '../search';
import { DeleteWeightEntryDialog } from './DeleteWeightEntryDialog';
import { WeightForm } from './WeightForm';
import { formatWeight } from './WeightSeries';

/** The `main` region's id, so the shell's landmark has a stable name. */
const WEIGHT_ENTRY_MAIN_ID = 'bs-weight-entry-main';

export type WeightEntryDetailScreenProps = {
  /** The entry the route's typed parameter names (WT-8). */
  entryId: string;
  /**
   * The rabbit the entry belongs to, from the route's typed search. It scopes
   * the read's cache entry, and it is not an argument of `get_weight_entry`.
   */
  rabbitId: string;
  /** The surface this detail was opened from (4.19's `WeightFormOrigin`). */
  from: WeightFormOrigin;
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

/*
 * The photos strip (WT-8, NFR-9). It is feature-local layout rather than a new
 * primitive or a new `screens.css` rule because this task owns the feature's
 * components: each photo takes the column's full width, the strip scrolls
 * horizontally one photo at a time, and every value is a token. Promote it into
 * `screens.css` unchanged if a second surface ever needs a strip.
 *
 * The images are decorative: they carry no description in the payload, and the
 * section's heading already names what they are.
 */
const PHOTO_SECTION_STYLE: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 'var(--spacing-stack-sm)',
};

const PHOTO_STRIP_STYLE: CSSProperties = {
  display: 'flex',
  gap: 'var(--space-2)',
  overflowX: 'auto',
  overscrollBehaviorX: 'contain',
  scrollSnapType: 'x mandatory',
};

const PHOTO_STYLE: CSSProperties = {
  flex: '0 0 100%',
  width: '100%',
  height: 'auto',
  borderRadius: 'var(--radius-card)',
  scrollSnapAlign: 'start',
};

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
  const rabbit = entry.data?.rabbit_id ?? rabbitId;
  const profile = useRabbitProfile(rabbit);
  const remove = useDeleteWeightEntry();
  const [editing, setEditing] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const actionsRef = useRef<HTMLDivElement>(null);
  const wasEditing = useRef(false);
  const removed = useRef(false);

  /*
   * One exit for every state: `replace` keeps the detail out of the back stack,
   * the way the weight form and the rabbit dashboard's close do, so a browser
   * Back after leaving does not reopen a screen the person already left.
   */
  const leave = useCallback(() => {
    if (from === 'rabbit' && rabbit !== '') {
      void navigate({
        to: '/rabbits/$rabbitId',
        params: { rabbitId: rabbit },
        replace: true,
        search: { trend, history },
      });
      return;
    }

    if (rabbit === '') {
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
      state: (previous) => ({ ...previous, focusRabbitId: rabbit }),
    });
  }, [from, history, navigate, rabbit, trend]);

  /*
   * A confirmed deletion has removed the entry this surface presents, so it
   * closes — the same way an archived rabbit's detail leaves the row behind
   * (RAB-10). The write hook invalidated the rabbit's series, so the surface
   * this returns to no longer offers the entry.
   */
  useEffect(() => {
    if (!remove.isSuccess || removed.current) {
      return;
    }

    removed.current = true;
    leave();
  }, [remove.isSuccess, leave]);

  /*
   * The form replaces the facts, so closing it hands focus back to the control
   * that opened it instead of dropping it on `<body>` — the swap pattern the
   * rabbit detail and the history sub-view use.
   */
  useEffect(() => {
    if (editing) {
      wasEditing.current = true;
      return;
    }

    if (!wasEditing.current) {
      return;
    }

    wasEditing.current = false;
    actionsRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
  }, [editing]);

  const destinations: NavigationDestination[] = [
    {
      to: '/',
      label: t('dashboard.title'),
      icon: <GlyphDashboard />,
    },
  ];

  const data = entry.data;

  /*
   * Both actions are writes, so they need the two facts only the profile answers:
   * the household that scopes the invalidation (NFR-10), and an unarchived
   * rabbit (RAB-8, D1.5). Without either, the entry is read-only; when the read
   * that answers them failed rather than answered "archived", the failure is
   * named below with its own refresh instead of leaving the actions silently
   * absent (NFR-8, D1.16).
   */
  const writable =
    data !== undefined &&
    profile.data !== undefined &&
    !profile.data.is_archived;

  /*
   * A failed profile read is presented only when it left no profile to act on: a
   * failed background refresh of a profile already in the cache keeps its data,
   * and those facts are still the ones the writes need.
   */
  const profileFailure =
    profile.data === undefined && profile.error !== null ? profile.error : null;

  const deleteFailure =
    remove.error === null ? null : describeSaveFailure(remove.error);
  const deleteValidation =
    deleteFailure !== null && deleteFailure.cause === 'validation'
      ? deleteFailure
      : null;

  /*
   * A non-`validation` deletion failure is already recorded by
   * `useDeleteWeightEntry` for the shell's retry surface; leaving the dialog
   * open would offer a second, differently identified attempt beside that
   * retry (D0.14). A `validation` refusal stays in the dialog, which is the
   * surface that owns the action's input.
   */
  useEffect(() => {
    if (remove.error === null) {
      return;
    }

    if (describeSaveFailure(remove.error).cause !== 'validation') {
      setConfirmingDelete(false);
    }
  }, [remove.error]);

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
        {data === undefined ? (
          entry.error === null ? (
            <LoadingIndicator label={t('weight.entry.loading')} />
          ) : (
            <ReadFailure
              error={entry.error}
              onRefresh={() => {
                void entry.refetch();
              }}
              title={t('weight.entry.title')}
            />
          )
        ) : unit.data === undefined ? (
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
        ) : editing && profile.data !== undefined ? (
          <WeightForm
            entry={data}
            householdId={profile.data.household_id}
            onCancel={() => {
              setEditing(false);
            }}
            onSaved={() => {
              setEditing(false);

              /*
               * The mounted key is the search's rabbit, so an update's
               * `series(entry.rabbit_id)` invalidation reaches it only when the
               * two agree. When they do not, ask for the confirming read rather
               * than leave the pre-edit value on screen (D1).
               */
              if (data.rabbit_id !== rabbitId) {
                void entry.refetch();
              }
            }}
            rabbitId={data.rabbit_id}
          />
        ) : (
          <>
            <Card className="bs-weight-entry">
              <Text variant="title-2">
                {formatWeight(data.value_grams, unit.data)}
              </Text>
              <Text tone="secondary">
                <time dateTime={data.timestamp}>
                  {TIMESTAMP_FORMAT.format(new Date(data.timestamp))}
                </time>
              </Text>
              {/*
               * The whole note, in the body role: it is another fact about the
               * entry rather than a caption, and it wraps rather than being
               * truncated, because a truncated note is not the note (NFR-9).
               */}
              {data.note === null ? null : <Text as="p">{data.note}</Text>}
              {entryHasDropAlert(data) ? (
                <Text as="p">
                  <Badge tone="warning">{t('weight.dropAlert')}</Badge>
                </Text>
              ) : null}
              {/*
               * The action row reuses the form's `__actions` rule (flex, gap,
               * safe wrapping) rather than adding a second identical one: the
               * detail is a sibling surface of the same feature.
               */}
              {writable ? (
                <div className="bs-weight-form__actions" ref={actionsRef}>
                  <Button
                    onClick={() => {
                      setEditing(true);
                    }}
                    variant="secondary"
                  >
                    {t('weight.entry.edit')}
                  </Button>
                  <Button
                    onClick={() => {
                      remove.reset();
                      setConfirmingDelete(true);
                    }}
                    variant="destructive"
                  >
                    {t('weight.entry.delete')}
                  </Button>
                </div>
              ) : profileFailure === null ? null : (
                /*
                 * The read the actions need failed, so it is named where they
                 * would be: the facts stay readable, and the refresh is the one
                 * action a read has (NFR-8, D1.16).
                 */
                <ReadFailure
                  error={profileFailure}
                  onRefresh={() => {
                    void profile.refetch();
                  }}
                  title={t('rabbit.loadFailed.title')}
                />
              )}
            </Card>
            {data.photo_refs.length === 0 ? null : (
              <PhotoStrip references={data.photo_refs} />
            )}
            {confirmingDelete && profile.data !== undefined ? (
              <DeleteWeightEntryDialog
                errorMessage={
                  deleteValidation === null
                    ? undefined
                    : t(deleteValidation.messageKey)
                }
                onCancel={() => {
                  setConfirmingDelete(false);
                  remove.reset();
                }}
                onConfirm={() => {
                  remove.mutate({
                    entryId: data.id,
                    householdId: profile.data.household_id,
                    rabbitId: data.rabbit_id,
                  });
                }}
                pending={remove.isPending}
              />
            ) : null}
          </>
        )}
      </Screen>
    </AppShell>
  );
}

/**
 * The photos, when the payload has any (WT-8, NFR-9).
 *
 * Nothing is drawn for an entry without photos: no empty frame, no caption
 * promising images that do not exist. A reference that cannot be loaded is
 * dropped from the strip rather than left as a broken image, because P1 carries
 * no photo picker and an opaque reference is not necessarily a URL this build
 * can resolve.
 */
function PhotoStrip({ references }: { references: readonly string[] }) {
  const headingId = useId();

  return (
    <section aria-labelledby={headingId} style={PHOTO_SECTION_STYLE}>
      <Heading as="h2" id={headingId} variant="section-header">
        {t('weight.detail.photos')}
      </Heading>
      {/*
       * Not a focus stop: the images are decorative and nothing is reachable
       * only through the scroll, so the strip stays out of the tab order rather
       * than adding a stop a keyboard user cannot use.
       */}
      <div style={PHOTO_STRIP_STYLE}>
        {references.map((reference, index) => (
          <EntryPhoto
            key={`${reference}-${String(index)}`}
            reference={reference}
          />
        ))}
      </div>
    </section>
  );
}

function EntryPhoto({ reference }: { reference: string }) {
  const [failed, setFailed] = useState(false);

  if (failed) {
    return null;
  }

  return (
    <img
      alt=""
      onError={() => {
        setFailed(true);
      }}
      src={reference}
      style={PHOTO_STYLE}
    />
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
