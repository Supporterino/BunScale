/*
 * The household's archived rabbits (task 4.5; RAB-6, RAB-7, RAB-10, NFR-9).
 *
 * Archiving is a soft delete, so an archived rabbit has to stay reachable: this
 * is the screen the dashboard's archived entry point opens, and it lists the
 * household's archived rabbits with the two lifecycle actions that operate on
 * them. It reads `useArchivedRabbits`, which is `list_rabbits(household, true)`
 * — the archived list only, never a union with the active one (design.md D1.6).
 *
 * **Restore is one press.** `restore_rabbit` returns the rabbit to the active
 * list with its history unchanged (RAB-7); the write hook invalidates both lists
 * and the dashboard rows, so the row leaves this screen and the dashboard
 * presents it again. Nothing is discarded on the way.
 *
 * **Permanent deletion is confirmed with its consequence (RAB-10, NFR-9).**
 * The control is directly visible on every row and opens `DeleteRabbitDialog`,
 * whose body states the consequence: the rabbit and all of its weight entries,
 * irreversibly, for every member. Nothing is deleted until that confirmation is
 * given. Archiving, by contrast, asks nothing (design.md D1.8).
 *
 * **Where the lifecycle controls live.** Permanent deletion exists only for an
 * archived rabbit (D1.8): it is here and on task 4.6's archived read-only
 * surface, and it is absent from an active rabbit's profile. Task 4.6 owns the
 * rest of the archived rabbit's read-only presentation; this screen does not
 * build it.
 *
 * **Failures.** A `validation` refusal belongs to the surface that owns the
 * input: the delete dialog shows it, and a restore shows it beside its control.
 * Every other cause is recorded by the write hooks for the route shell's
 * `SaveFailureSurface` (SYNC-2, NFR-8), exactly as the profile and form flows
 * do, and is not presented a second time here.
 *
 * **The empty state is announced when it replaces rows.** The zero-row state
 * appears after a restore or a permanent deletion, so the caller wraps it in
 * `role="status"` — `EmptyState` deliberately sets no live role itself, because
 * a region that mounts with the screen would talk over its title. When the
 * screen is first opened on an already-empty list there is nothing to announce.
 */
import { useEffect, useRef, useState } from 'react';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { EmptyState } from '../../../components/ui/EmptyState';
import { ErrorState } from '../../../components/ui/ErrorState';
import { Heading } from '../../../components/ui/Heading';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Notice } from '../../../components/ui/Notice';
import { useActiveHousehold } from '../../households/queries';
import { t } from '../../i18n';
import { describeSaveFailure } from '../../sync/failure';
import { describeReadFailure } from '../../sync/read-failure';
import { deleteRabbit, restoreRabbit } from '../api';
import { useDeleteRabbit, useRestoreRabbit } from '../mutations';
import type { RabbitListFailure, RabbitSummary } from '../queries';
import { useArchivedRabbits } from '../queries';
import { DeleteRabbitDialog } from './DeleteRabbitDialog';

export type ArchivedRabbitListProps = {
  /** Returns to the dashboard without changing anything. */
  onClose: () => void;
};

export function ArchivedRabbitList({ onClose }: ArchivedRabbitListProps) {
  const archived = useArchivedRabbits();
  const householdId = useActiveHousehold().data?.id;
  const containerRef = useRef<HTMLDivElement>(null);
  const hadRows = useRef(false);

  // The entry point unmounts with the dashboard list, so the new surface takes
  // focus instead of dropping it on `<body>`.
  useEffect(() => {
    containerRef.current?.querySelector('button')?.focus();
  }, []);

  /*
   * The transition to zero rows is what the empty state announces: before it,
   * this screen held at least one archived rabbit, so the empty state replaced
   * content rather than being the initial state.
   */
  useEffect(() => {
    if (archived.data !== undefined && archived.data.length > 0) {
      hadRows.current = true;
    }
  }, [archived.data]);

  const announceEmpty =
    hadRows.current &&
    archived.data !== undefined &&
    archived.data.length === 0;

  return (
    <div className="bs-archived" ref={containerRef}>
      <div className="bs-archived__actions-row">
        <Button onClick={onClose} variant="plain">
          {t('settings.back')}
        </Button>
      </div>
      <ArchivedContent
        announceEmpty={announceEmpty}
        error={archived.error}
        householdId={householdId}
        onRetry={() => {
          void archived.refetch();
        }}
        rabbits={archived.data}
      />
    </div>
  );
}

/**
 * One of four states: the pending read, its failure, the zero-row empty state,
 * or the archived rabbits. The heading is part of the state that shows it, so
 * the empty state's own heading does not duplicate a screen heading above it.
 */
function ArchivedContent({
  announceEmpty,
  error,
  householdId,
  onRetry,
  rabbits,
}: {
  announceEmpty: boolean;
  error: RabbitListFailure | null;
  householdId: string | undefined;
  onRetry: () => void;
  rabbits: RabbitSummary[] | undefined;
}) {
  if (rabbits === undefined) {
    if (error === null) {
      return <LoadingIndicator label={t('entry.loading')} variant="screen" />;
    }

    const failure = describeReadFailure(error);

    return (
      <ErrorState
        {...(failure.refreshable ? { onRetry } : {})}
        retryLabel={t('error.retry')}
        title={t('rabbit.archivedList.title')}
      >
        {t(failure.messageKey)}
      </ErrorState>
    );
  }

  if (rabbits.length === 0) {
    return (
      <div role={announceEmpty ? 'status' : undefined}>
        <EmptyState title={t('rabbit.archivedList.title')}>
          {t('rabbit.archivedList.empty')}
        </EmptyState>
      </div>
    );
  }

  return (
    <>
      <Heading as="h2" variant="title-2">
        {t('rabbit.archivedList.title')}
      </Heading>
      <ul className="bs-archived__rows">
        {rabbits.map((rabbit) => (
          <li key={rabbit.id}>
            <ArchivedRabbit householdId={householdId} rabbit={rabbit} />
          </li>
        ))}
      </ul>
    </>
  );
}

/**
 * One archived rabbit: its name, its archived state, and the two lifecycle
 * actions. Each row owns its own mutations and its own dialog, so an attempt on
 * one rabbit cannot present its pending or refused state against another.
 */
function ArchivedRabbit({
  householdId,
  rabbit,
}: {
  householdId: string | undefined;
  rabbit: RabbitSummary;
}) {
  const restore = useRestoreRabbit();
  const remove = useDeleteRabbit();
  const [confirming, setConfirming] = useState(false);

  /*
   * A confirmed deletion closes the dialog. A non-validation failure closes it
   * too: the write hook has recorded the failed operation for the shell's
   * retry surface, and leaving the dialog open would offer a second, differently
   * identified attempt beside that retry (D0.14). A `validation` refusal stays
   * in the dialog, because that is the surface that owns the input.
   */
  useEffect(() => {
    if (!remove.isSuccess) {
      return;
    }

    setConfirming(false);
    remove.reset();
  }, [remove.isSuccess, remove.reset]);

  useEffect(() => {
    if (remove.error === null) {
      return;
    }

    if (describeSaveFailure(remove.error).cause !== 'validation') {
      setConfirming(false);
    }
  }, [remove.error]);

  const restoreFailure =
    restore.error === null ? null : describeSaveFailure(restore.error);
  const restoreValidation =
    restoreFailure !== null && restoreFailure.cause === 'validation'
      ? restoreFailure.messageKey
      : null;
  const deleteFailure =
    remove.error === null ? null : describeSaveFailure(remove.error);
  const deleteValidation =
    deleteFailure !== null && deleteFailure.cause === 'validation'
      ? deleteFailure
      : null;

  function openDialog() {
    remove.reset();
    setConfirming(true);
  }

  function confirmDelete() {
    if (householdId === undefined) {
      return;
    }

    remove.mutate({
      householdId,
      rabbitId: rabbit.id,
      operation: deleteRabbit(rabbit.id),
    });
  }

  return (
    <Card className="bs-archived__card" heading={rabbit.name} headingLevel="h3">
      <div className="bs-archived__meta">
        <Badge tone="neutral">{t('rabbit.archived.label')}</Badge>
      </div>
      {restoreValidation === null ? null : (
        <Notice tone="danger">{t(restoreValidation)}</Notice>
      )}
      <div className="bs-archived__actions">
        <Button
          loading={restore.isPending}
          onClick={() => {
            restore.mutate(restoreRabbit(rabbit.id));
          }}
          variant="secondary"
        >
          {restore.isPending ? t('rabbit.restoring') : t('rabbit.restore')}
        </Button>
        {/*
         * RAB-10, NFR-9: directly visible, and it only opens the confirmation.
         * Nothing is removed until that dialog's confirm control is pressed.
         */}
        <Button
          disabled={restore.isPending}
          onClick={openDialog}
          variant="destructive"
        >
          {t('rabbit.delete')}
        </Button>
      </div>
      {confirming ? (
        <DeleteRabbitDialog
          errorMessage={
            deleteValidation === null
              ? undefined
              : t(deleteValidation.messageKey)
          }
          onCancel={() => {
            setConfirming(false);
            remove.reset();
          }}
          onConfirm={confirmDelete}
          pending={remove.isPending}
          rabbitName={rabbit.name}
        />
      ) : null}
    </Card>
  );
}
