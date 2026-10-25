/*
 * One rabbit's profile detail (task 4.4; RAB-5, HS-8).
 *
 * The read is `get_rabbit`, served from this device's copy. Every RAB-1 field
 * is presented as a value — name, photo state, breed, date of birth (or the
 * explicit "unknown"), sex, neuter status, adoption date — plus the rabbit's
 * weight-drop threshold, which is a profile field like the rest (WT-7). The
 * threshold is displayed here and edited in the form, never inline: RAB-5 asks
 * for an edit control, not for seven always-live inputs around the data.
 *
 * **The edit control is directly visible (RAB-5, NFR-9).** It is a real
 * `Button` in the detail's own action row, above the values, on the surface
 * itself. There is no long press, no context menu and no overflow menu to reach
 * it; a member who did not create the rabbit (HS-8) sees the same control.
 *
 * **Only a loaded profile offers the edit.** `update_rabbit` resolves the rabbit
 * from this device's copy, so a read that has no data leaves nothing to edit:
 * D1.16's content-unloaded refusal (`cause: unknown` carrying
 * `error.household_content_unloaded`) and `get_rabbit`'s own `validation`
 * refusal (an unknown or blank id, `api.ts`) both withhold the control rather
 * than offering an action the backend must reject. The two are not relabelled:
 * `describeReadFailure` keeps the surface key when the backend named one and
 * otherwise renders the cause's own copy, and a refresh is offered only when a
 * fresh read can change the answer (`validation` is the one that cannot). While
 * the read is in flight the same rule holds — no data, no action — and the
 * pending surface says what is loading. An archived rabbit is likewise not
 * editable (RAB-5); its whole presentation is read-only (task 4.6 below).
 *
 * **A saved edit returns to this detail with the new values.** `useUpdateRabbit`
 * invalidates the profile key (and the rabbit's series, so the threshold's
 * derived flags are re-read — 3.7), and this component observes that key, so
 * the detail re-renders from the confirming read. Sharing the change with every
 * member is the shared service's job; the UI's obligation is to present what the
 * confirmed read answers rather than a local guess (D1).
 *
 * **The archive control is directly visible and asks nothing (task 4.5; RAB-6).**
 * It sits beside the edit control for an active rabbit, not behind a long press
 * or an overflow menu, and it needs no consequence confirmation: archiving is
 * the reversible step (RAB-9, design.md D1.8). The confirmed write invalidates
 * both rabbit lists, the profile and the dashboard rows, so this detail closes
 * back to the dashboard list the rabbit has just left — the archived list is
 * where it now lives. A `validation` refusal (another member archived it first)
 * is shown here; every other cause is recorded by the write hook for the route
 * shell's failure surface.
 *
 * **Permanent deletion is present only on the archived rabbit (RAB-10, D1.8).**
 * It is in this detail's action row for an archived rabbit — the read-only
 * surface task 4.5 names beside the archived list — and it opens the same
 * `DeleteRabbitDialog` the list uses, consequence copy included. An active
 * rabbit's profile never shows it: archiving is a real step before the point of
 * no return rather than a decorative flag.
 *
 * **An archived rabbit is presented read-only (task 4.6; RAB-8, DM-4, DASH-7).**
 * This detail is the top of the rabbit's dashboard — `RabbitDashboard` mounts it
 * above the trend and the History section — so the state is stated on the
 * surface itself, in the catalog's own words: the existing
 * `rabbit.archived.label` badge beside the name and `rabbit.archived.readOnly`
 * beneath the identity. The edit and archive controls are *withheld*, not
 * disabled: a greyed control still offers the action, and RAB-8 asks for no
 * edit control at all (RAB-5). Permanent deletion is the one lifecycle action an
 * archived rabbit's surface carries, because RAB-10 places it here as well as
 * on the archived list; restore stays on the list, because it operates on the
 * archive rather than on the profile this surface presents. The weight surfaces
 * below this card carry no write control on either rabbit, so the archived
 * rabbit's trend, its history and its entry rows are read-only by construction
 * and need no second guard here. If another member archives the rabbit while
 * the edit form is open, the refreshed profile closes that form rather than
 * leaving a Save behind on a profile that can no longer be written.
 *
 * **Focus.** The Back control takes focus when the detail replaces the list, and
 * the edit control takes it back when the form closes — the same swap pattern
 * task 4.3 established for the dashboard's Add/form pair.
 */
import { type ReactNode, useEffect, useRef, useState } from 'react';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { ErrorState } from '../../../components/ui/ErrorState';
import { Heading } from '../../../components/ui/Heading';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Notice } from '../../../components/ui/Notice';
import { Text } from '../../../components/ui/Text';
import { type MessageKey, t } from '../../i18n';
import { describeSaveFailure } from '../../sync/failure';
import { describeReadFailure } from '../../sync/read-failure';
import { archiveRabbit, deleteRabbit } from '../api';
import { useArchiveRabbit, useDeleteRabbit } from '../mutations';
import {
  type Rabbit,
  type RabbitProfileFailure,
  useRabbitProfile,
} from '../queries';
import { DeleteRabbitDialog } from './DeleteRabbitDialog';
import {
  DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT,
  RabbitForm,
} from './RabbitForm';

/**
 * Dates are the contract's UTC calendar instants (I8), so they are formatted in
 * UTC: the day entered is the day every member and device sees, not the previous
 * day in a western time zone.
 */
const calendarFormat = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeZone: 'UTC',
});

/** D1.14's recorded value sets, mapped to their catalogued display strings. */
const SEX_LABELS = {
  male: 'rabbit.sex.male',
  female: 'rabbit.sex.female',
  unknown: 'rabbit.sex.unknown',
} as const satisfies Record<Rabbit['sex'], MessageKey>;

const NEUTER_LABELS = {
  intact: 'rabbit.neuterStatus.intact',
  neutered: 'rabbit.neuterStatus.neutered',
  spayed: 'rabbit.neuterStatus.spayed',
  unknown: 'rabbit.neuterStatus.unknown',
} as const satisfies Record<Rabbit['neuter_status'], MessageKey>;

export type RabbitDetailProps = {
  /** The rabbit the dashboard row opened (DASH-6). */
  rabbitId: string;
  /** Returns to the dashboard without changing anything. */
  onClose: () => void;
};

type Field = {
  readonly term: string;
  readonly value: ReactNode;
};

export function RabbitDetail({ rabbitId, onClose }: RabbitDetailProps) {
  const profile = useRabbitProfile(rabbitId);
  const archive = useArchiveRabbit();
  const remove = useDeleteRabbit();
  const [editing, setEditing] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const wasEditing = useRef(false);
  const closedAfterArchive = useRef(false);
  const closedAfterDelete = useRef(false);

  // The list unmounts with this detail, so focus is handed to its own entry
  // point instead of being dropped on `<body>`.
  useEffect(() => {
    containerRef.current?.querySelector('button')?.focus();
  }, []);

  /*
   * The confirmed archive closes the detail. The write hook invalidates both
   * lists and the dashboard rows, so the list this returns to already presents
   * the rabbit's new home: the archived list is where it now is (RAB-6).
   */
  useEffect(() => {
    if (!archive.isSuccess || closedAfterArchive.current) {
      return;
    }

    closedAfterArchive.current = true;
    onClose();
  }, [archive.isSuccess, onClose]);

  /*
   * A confirmed deletion has removed the rabbit, so this detail closes like the
   * archived list leaves the row behind (RAB-10).
   */
  useEffect(() => {
    if (!remove.isSuccess || closedAfterDelete.current) {
      return;
    }

    closedAfterDelete.current = true;
    setConfirmingDelete(false);
    onClose();
  }, [remove.isSuccess, onClose]);

  /*
   * A non-validation failure is already recorded by `useDeleteRabbit` for the
   * shell's retry surface; leaving the dialog open would offer a second,
   * differently identified attempt beside that retry (D0.14). A `validation`
   * refusal stays in the dialog, which owns the input.
   */
  useEffect(() => {
    if (remove.error === null) {
      return;
    }

    if (describeSaveFailure(remove.error).cause !== 'validation') {
      setConfirmingDelete(false);
    }
  }, [remove.error]);

  /*
   * Returning from the form waits for the read the save invalidated: focusing
   * the edit control while the detail still shows the previous values would
   * announce the stale profile as the confirmed one.
   */
  useEffect(() => {
    if (editing) {
      wasEditing.current = true;
      return;
    }

    if (!wasEditing.current || profile.isFetching) {
      return;
    }

    wasEditing.current = false;

    /*
     * By the control's own marker, not its visible text: a label is copy and
     * must not be the identity a focus target is found by. When the form was
     * closed because the refreshed profile reports archived, that control no
     * longer exists and the read-only detail's Back takes focus instead of the
     * node being dropped on `<body>`.
     */
    const container = containerRef.current;

    (
      container?.querySelector<HTMLButtonElement>('.bs-rabbit-detail__edit') ??
      container?.querySelector<HTMLButtonElement>('button')
    )?.focus();
  }, [editing, profile.isFetching]);

  /*
   * Another member can archive the rabbit while this form is open. The read this
   * component already observes is the confirming one, so the form closes the
   * moment it reports archived: RAB-5 and RAB-8 ask for no edit control on an
   * archived profile, and leaving Save on screen would offer a write the
   * contract refuses (D1.5).
   */
  useEffect(() => {
    if (editing && profile.data?.is_archived === true) {
      setEditing(false);
    }
  }, [editing, profile.data?.is_archived]);

  /*
   * The Back control is rendered before the read settles, so opening the detail
   * hands focus to a control that can leave it instead of dropping it on
   * `<body>`. The edit control appears only beside data the backend can resolve
   * (D1.16) and only for an active rabbit (RAB-5).
   */
  const editable = profile.data !== undefined && !profile.data.is_archived;

  /*
   * RAB-10 places the irreversible control on the archived list *and* on the
   * archived rabbit's read-only surface. Only an archived profile carries it;
   * an active profile keeps none (D1.8).
   */
  const deletable = profile.data?.is_archived === true;

  /*
   * A `validation` refusal belongs to this surface, because there is no form to
   * give it to: the only reachable one is "the rabbit is already archived" from
   * a member who moved it first. Every other cause is recorded by
   * `useArchiveRabbit` for the route shell's failure surface.
   */
  const archiveFailure =
    archive.error === null ? null : describeSaveFailure(archive.error);
  const archiveValidation =
    archiveFailure !== null && archiveFailure.cause === 'validation'
      ? archiveFailure.messageKey
      : null;

  const deleteFailure =
    remove.error === null ? null : describeSaveFailure(remove.error);
  const deleteValidation =
    deleteFailure !== null && deleteFailure.cause === 'validation'
      ? deleteFailure
      : null;

  return (
    <div className="bs-rabbit-detail" ref={containerRef}>
      {editing ? (
        profile.data === undefined ? null : (
          <RabbitForm
            householdId={profile.data.household_id}
            initial={{ rabbitId, profile: profile.data }}
            onCancel={() => {
              setEditing(false);
            }}
            onSaved={() => {
              setEditing(false);
            }}
          />
        )
      ) : (
        <>
          <div className="bs-rabbit-detail__actions">
            <Button onClick={onClose} variant="plain">
              {t('settings.back')}
            </Button>
            <div className="bs-rabbit-detail__action-group">
              {editable ? (
                <Button
                  className="bs-rabbit-detail__edit"
                  onClick={() => {
                    setEditing(true);
                  }}
                  variant="secondary"
                >
                  {t('rabbit.edit')}
                </Button>
              ) : null}
              {/*
               * RAB-6, NFR-9: directly visible, no confirmation (D1.8). The
               * failed write's non-validation causes go to the shell; the
               * validation one is shown below this row.
               */}
              {editable ? (
                <Button
                  className="bs-rabbit-detail__archive"
                  loading={archive.isPending}
                  onClick={() => {
                    archive.mutate(archiveRabbit(rabbitId));
                  }}
                  variant="secondary"
                >
                  {archive.isPending
                    ? t('rabbit.archiving')
                    : t('rabbit.archive')}
                </Button>
              ) : null}
              {/*
               * RAB-10, NFR-9: the archived read-only surface carries the
               * irreversible control, and it only opens the shared confirmation.
               */}
              {deletable ? (
                <Button
                  onClick={() => {
                    remove.reset();
                    setConfirmingDelete(true);
                  }}
                  variant="destructive"
                >
                  {t('rabbit.delete')}
                </Button>
              ) : null}
            </div>
          </div>
          {archiveValidation === null ? null : (
            <Notice tone="danger">{t(archiveValidation)}</Notice>
          )}
          {confirmingDelete && profile.data !== undefined ? (
            <DeleteRabbitDialog
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
                  householdId: profile.data.household_id,
                  operation: deleteRabbit(rabbitId),
                  rabbitId,
                });
              }}
              pending={remove.isPending}
              rabbitName={profile.data.name}
              restoreFocus={!remove.isSuccess}
            />
          ) : null}
          {profile.data === undefined ? (
            <PendingProfile
              error={profile.error}
              onRetry={() => {
                void profile.refetch();
              }}
            />
          ) : (
            <ReadProfile rabbit={profile.data} />
          )}
        </>
      )}
    </div>
  );
}

/**
 * No loaded profile: the pending read, or its failure with its cause.
 *
 * A read is described by `describeReadFailure`, not the write mapper: D1.16's
 * content-unloaded refusal is `cause: unknown` carrying the
 * `error.household_content_unloaded` surface key, and `validation` is
 * `get_rabbit`'s unknown/blank-id refusal. The surface key wins when the
 * backend named one, a cause's own copy is the fallback, and the refresh action
 * appears only when a fresh read can change the answer (`validation` cannot).
 */
function PendingProfile({
  error,
  onRetry,
}: {
  error: RabbitProfileFailure | null;
  onRetry: () => void;
}) {
  if (error === null) {
    return <LoadingIndicator label={t('entry.loading')} variant="screen" />;
  }

  const failure = describeReadFailure(error);

  return (
    <ErrorState
      {...(failure.refreshable ? { onRetry } : {})}
      retryLabel={t('error.retry')}
      title={t('rabbit.loadFailed.title')}
    >
      {t(failure.messageKey)}
    </ErrorState>
  );
}

/**
 * A loaded profile: every RAB-1 value, threshold included (RAB-5), and the
 * archived state when it applies (RAB-8, task 4.6).
 */
function ReadProfile({ rabbit }: { rabbit: Rabbit }) {
  const fields: Field[] = [
    { term: t('rabbit.form.breedLabel'), value: rabbit.breed ?? '—' },
    {
      term: t('rabbit.form.dobLabel'),
      value:
        rabbit.dob_unknown || rabbit.dob === null ? (
          t('rabbit.dob.unknown')
        ) : (
          <time dateTime={rabbit.dob}>
            {calendarFormat.format(new Date(rabbit.dob))}
          </time>
        ),
    },
    { term: t('rabbit.sex.label'), value: t(SEX_LABELS[rabbit.sex]) },
    {
      term: t('rabbit.neuterStatus.label'),
      value: t(NEUTER_LABELS[rabbit.neuter_status]),
    },
    {
      term: t('rabbit.form.adoptionDateLabel'),
      value:
        rabbit.adoption_date === null ? (
          '—'
        ) : (
          <time dateTime={rabbit.adoption_date}>
            {calendarFormat.format(new Date(rabbit.adoption_date))}
          </time>
        ),
    },
    {
      term: t('rabbit.form.thresholdLabel'),
      value: `${rabbit.weight_drop_threshold_percent ?? DEFAULT_WEIGHT_DROP_THRESHOLD_PERCENT}%`,
    },
  ];

  return (
    <Card className="bs-rabbit-detail__card">
      <div className="bs-rabbit-detail__identity">
        <RabbitPhoto reference={rabbit.photo_ref} />
        <Heading as="h2" className="bs-rabbit-detail__name" variant="title-2">
          {rabbit.name}
        </Heading>
        {/*
         * RAB-8, DM-4: the archived marker is the one the archived list already
         * uses (`rabbit.archived.label`), so the state reads the same wherever
         * the rabbit appears. It is text, not a colour or a lock glyph.
         */}
        {rabbit.is_archived ? (
          <Badge tone="neutral">{t('rabbit.archived.label')}</Badge>
        ) : null}
      </div>
      {/*
       * The read-only sentence sits on the surface itself rather than in a
       * transient notice: it is the profile's state, and it covers the dashboard
       * sections below, which offer no write control for either rabbit (DASH-7).
       */}
      {rabbit.is_archived ? (
        <Text as="p" tone="secondary" variant="subheadline">
          {t('rabbit.archived.readOnly')}
        </Text>
      ) : null}
      <dl className="bs-rabbit-detail__fields">
        {fields.map((field) => (
          <div className="bs-rabbit-detail__field" key={field.term}>
            <dt className="bs-rabbit-detail__term">{field.term}</dt>
            <dd className="bs-rabbit-detail__value">{field.value}</dd>
          </div>
        ))}
      </dl>
    </Card>
  );
}

/**
 * RAB-1's photo, presented wherever the rabbit is (P1 carries the reference
 * but no picker, D1.11). A supplied reference is the image; the placeholder is
 * the fallback both when the reference is absent and when it cannot be loaded,
 * because an opaque reference is not yet a URL this build can resolve — the
 * same rule `RabbitRow` applies. The placeholder carries its own accessible
 * name; the image is decorative, because the heading already names the rabbit.
 */
function RabbitPhoto({ reference }: { reference: string | null }) {
  const [failed, setFailed] = useState(false);

  if (reference !== null && !failed) {
    return (
      <span className="bs-rabbit-detail__photo">
        <img
          alt=""
          className="bs-rabbit-detail__image"
          onError={() => {
            setFailed(true);
          }}
          src={reference}
        />
      </span>
    );
  }

  return (
    <span
      aria-label={t('rabbit.thumbnailPlaceholder')}
      className="bs-rabbit-detail__photo"
      role="img"
    >
      <svg
        aria-hidden="true"
        fill="none"
        height="24"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
        viewBox="0 0 24 24"
        width="24"
      >
        <rect height="15" rx="3" width="18" x="3" y="4.5" />
        <circle cx="8.5" cy="9.5" r="1.5" />
        <path d="M4 16.5 8.5 12.5l3.5 3 3-2.5 5 4.5" />
      </svg>
    </span>
  );
}
