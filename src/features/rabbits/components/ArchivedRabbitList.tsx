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
 * archived rabbit (D1.8): it is here and on the archived rabbit's read-only
 * surface (`RabbitDetail`), and it is absent from an active rabbit's profile.
 * Task 4.6 owns the rest of the archived rabbit's read-only presentation; this
 * screen does not build it.
 *
 * **Failures.** A `validation` refusal belongs to the surface that owns the
 * input: the delete dialog shows it, and a restore shows it beside its control.
 * Every other cause is recorded by the write hooks for the route shell's
 * `SaveFailureSurface` (SYNC-2, NFR-8), exactly as the profile and form flows
 * do, and is not presented a second time here.
 *
 * **The empty state is announced when it replaces rows.** A `role="status"`
 * region is mounted with this screen and kept empty while rows are present, so
 * it exists before the content changes; when the rows go to zero after a restore
 * or a permanent deletion, the empty state's copy is written into it (the
 * pattern `ConnectivityRequiredScreen` uses). `EmptyState` deliberately sets no
 * live role of its own. Opening the screen on an already-empty list writes
 * nothing, because that is the initial state rather than a replacement.
 *
 * **Focus survives the row that leaves.** After a restore or a confirmed
 * deletion the action's own control is unmounted with its row, so the screen
 * moves focus to the row that took its place (its Restore control) or, when the
 * last row leaves, to the Back control. The dialog is told not to restore focus
 * to a trigger that is being removed.
 *
 * **The row's dashboard is reachable on every row (4.17; RAB-6, RAB-8,
 * DASH-7).** A `ListRow` inside the card opens the rabbit's dashboard, where the
 * read-only timeline lives, so reaching it never needs a deep link (NFR-9). It
 * is a separate control from the lifecycle buttons: the row is a real button,
 * the Restore and Delete controls stay siblings rather than nesting inside it,
 * and the card itself is not the control. `onOpen` is supplied by the screen
 * that owns the route, exactly as `RabbitList`'s `onOpenRabbit` is; without it
 * the row offers no destination and paints no open affordance.
 */
import { useEffect, useRef, useState } from 'react';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { EmptyState } from '../../../components/ui/EmptyState';
import { ErrorState } from '../../../components/ui/ErrorState';
import { Heading } from '../../../components/ui/Heading';
import { List } from '../../../components/ui/List';
import { ListRow } from '../../../components/ui/ListRow';
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
  /**
   * Opens one archived rabbit's dashboard (4.17; RAB-6, RAB-8, DASH-7). The
   * screen owns the route; absent, a row has no open affordance rather than a
   * control without a destination.
   */
  onOpen?: (rabbitId: string) => void;
};

export function ArchivedRabbitList({
  onClose,
  onOpen,
}: ArchivedRabbitListProps) {
  const archived = useArchivedRabbits();
  const householdId = useActiveHousehold().data?.id;
  const containerRef = useRef<HTMLDivElement>(null);
  const hadRows = useRef(false);
  const leavingIndex = useRef<number | null>(null);
  const [announcement, setAnnouncement] = useState('');

  // The entry point unmounts with the dashboard list, so the new surface takes
  // focus instead of dropping it on `<body>`.
  useEffect(() => {
    containerRef.current?.querySelector('button')?.focus();
  }, []);

  /*
   * The transition to zero rows is what the empty state announces: before it,
   * this screen held at least one archived rabbit, so the empty state replaced
   * content rather than being the initial state. A later non-empty read clears
   * the region, so the next transition is a change again.
   */
  useEffect(() => {
    if (archived.data === undefined) {
      return;
    }

    if (archived.data.length > 0) {
      hadRows.current = true;
      setAnnouncement('');
      return;
    }

    if (hadRows.current) {
      setAnnouncement(
        `${t('rabbit.archivedList.title')}. ${t('rabbit.archivedList.empty')}`,
      );
    }
  }, [archived.data]);

  /*
   * The row's own control is unmounted with it, so focus moves to the row that
   * took its place — or to Back when the last row left. This runs on the
   * confirming re-read, after the list has actually changed.
   */
  useEffect(() => {
    if (leavingIndex.current === null || archived.data === undefined) {
      return;
    }

    const index = leavingIndex.current;

    leavingIndex.current = null;

    const cards =
      containerRef.current?.querySelectorAll<HTMLElement>('.bs-archived__card');

    if (cards !== undefined && cards.length > 0) {
      cards[Math.min(index, cards.length - 1)]
        ?.querySelector<HTMLButtonElement>('.bs-archived__restore')
        ?.focus();
      return;
    }

    containerRef.current
      ?.querySelector<HTMLButtonElement>('.bs-archived__back')
      ?.focus();
  }, [archived.data]);

  return (
    <div className="bs-archived" ref={containerRef}>
      {/*
       * Mounted before any content can replace rows, and empty until then: a
       * region that appears with the replacement announces nothing. It is
       * visually hidden on the region itself (out of flow), so an empty region
       * does not add a gap step to the column.
       */}
      <div
        className="bs-visually-hidden"
        data-announcement="archived"
        role="status"
      >
        {announcement}
      </div>
      <div className="bs-archived__actions-row">
        <Button className="bs-archived__back" onClick={onClose} variant="plain">
          {t('settings.back')}
        </Button>
      </div>
      <ArchivedContent
        error={archived.error}
        householdId={householdId}
        onLeaving={(index) => {
          leavingIndex.current = index;
        }}
        onOpen={onOpen}
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
  error,
  householdId,
  onLeaving,
  onOpen,
  onRetry,
  rabbits,
}: {
  error: RabbitListFailure | null;
  householdId: string | undefined;
  onLeaving: (index: number) => void;
  onOpen: ((rabbitId: string) => void) | undefined;
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
      <EmptyState title={t('rabbit.archivedList.title')}>
        {t('rabbit.archivedList.empty')}
      </EmptyState>
    );
  }

  return (
    <>
      <Heading as="h2" variant="title-2">
        {t('rabbit.archivedList.title')}
      </Heading>
      <ul className="bs-archived__rows">
        {rabbits.map((rabbit, index) => (
          <li key={rabbit.id}>
            <ArchivedRabbit
              householdId={householdId}
              index={index}
              onLeaving={onLeaving}
              onOpen={onOpen}
              rabbit={rabbit}
            />
          </li>
        ))}
      </ul>
    </>
  );
}

/**
 * One archived rabbit: its name, its archived state, the way into its
 * dashboard, and the two lifecycle actions. Each row owns its own mutations and
 * its own dialog, so an attempt on one rabbit cannot present its pending or
 * refused state against another.
 */
function ArchivedRabbit({
  householdId,
  index,
  onLeaving,
  onOpen,
  rabbit,
}: {
  householdId: string | undefined;
  /** Position in the list, so focus can land where the row was. */
  index: number;
  /** The row is about to leave; the list hands focus on after the re-read. */
  onLeaving: (index: number) => void;
  /** Opens this rabbit's dashboard; absent, the row offers no destination. */
  onOpen: ((rabbitId: string) => void) | undefined;
  rabbit: RabbitSummary;
}) {
  const restore = useRestoreRabbit();
  const remove = useDeleteRabbit();
  const [confirming, setConfirming] = useState(false);
  const notifiedRestore = useRef(false);

  /*
   * A confirmed restore unmounts this row. The list needs to know where it was
   * so focus can follow the confirming re-read, and this component must not try
   * to keep focus on a control that is about to disappear.
   */
  useEffect(() => {
    if (!restore.isSuccess || notifiedRestore.current) {
      return;
    }

    notifiedRestore.current = true;
    onLeaving(index);
  }, [restore.isSuccess, index, onLeaving]);

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

    onLeaving(index);
    setConfirming(false);
    remove.reset();
  }, [remove.isSuccess, remove.reset, index, onLeaving]);

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
      {/*
       * 4.17, NFR-9: a direct, visible way into the rabbit's dashboard. It is
       * the list row's own affordance, not a nested control, so Restore and
       * Delete below stay separate targets.
       */}
      {onOpen === undefined ? null : (
        <List>
          <ListRow
            accessory="chevron"
            onClick={() => {
              onOpen(rabbit.id);
            }}
            title={t('rabbit.archived.open')}
          />
        </List>
      )}
      {restoreValidation === null ? null : (
        <Notice tone="danger">{t(restoreValidation)}</Notice>
      )}
      <div className="bs-archived__actions">
        <Button
          className="bs-archived__restore"
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
          restoreFocus={!remove.isSuccess}
        />
      ) : null}
    </Card>
  );
}
