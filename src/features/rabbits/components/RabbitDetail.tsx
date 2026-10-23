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
 * editable (RAB-5); task 4.6 owns the rest of its read-only presentation.
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
 * **Permanent deletion is deliberately absent (RAB-10, D1.8).** The irreversible
 * action lives only on the archived list and task 4.6's archived surface; an
 * active profile offers no such control, so archiving is a real step before the
 * point of no return rather than a decorative flag.
 *
 * **Focus.** The Back control takes focus when the detail replaces the list, and
 * the edit control takes it back when the form closes — the same swap pattern
 * task 4.3 established for the dashboard's Add/form pair.
 */
import { type ReactNode, useEffect, useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { ErrorState } from '../../../components/ui/ErrorState';
import { Heading } from '../../../components/ui/Heading';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Notice } from '../../../components/ui/Notice';
import { type MessageKey, t } from '../../i18n';
import { describeSaveFailure } from '../../sync/failure';
import { describeReadFailure } from '../../sync/read-failure';
import { archiveRabbit } from '../api';
import { useArchiveRabbit } from '../mutations';
import {
  type Rabbit,
  type RabbitProfileFailure,
  useRabbitProfile,
} from '../queries';
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
  const [editing, setEditing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const wasEditing = useRef(false);
  const closedAfterArchive = useRef(false);

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

    // By the control's own marker, not its visible text: a label is copy and
    // must not be the identity a focus target is found by.
    containerRef.current
      ?.querySelector<HTMLButtonElement>('.bs-rabbit-detail__edit')
      ?.focus();
  }, [editing, profile.isFetching]);

  /*
   * The Back control is rendered before the read settles, so opening the detail
   * hands focus to a control that can leave it instead of dropping it on
   * `<body>`. The edit control appears only beside data the backend can resolve
   * (D1.16) and only for an active rabbit (RAB-5).
   */
  const editable = profile.data !== undefined && !profile.data.is_archived;

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
            </div>
          </div>
          {archiveValidation === null ? null : (
            <Notice tone="danger">{t(archiveValidation)}</Notice>
          )}
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

/** A loaded profile: every RAB-1 value, threshold included (RAB-5). */
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
      </div>
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
