/*
 * The permanent-deletion confirmation (task 4.5; RAB-10, NFR-9).
 *
 * RAB-10 makes permanent deletion irreversible and shared with every member, so
 * NFR-9 requires a confirmation that states the consequence before anything is
 * removed. The body is the catalog's `rabbit.deleteConfirm.body` — the rabbit and
 * all of its weight entries, for every member, and it cannot be undone — and the
 * confirm control is the destructive variant. Archiving is not confirmed: it is
 * the reversible step (design.md D1.8, `rabbit.archiveConfirm.*` deliberately
 * does not exist).
 *
 * **A dialog, because the design system defers `Dialog`/`Alert`.** The frozen
 * inventory has no modal primitive (`design-system`, "Deferred primitives"), and
 * this is the first P1 surface that genuinely needs one. It stays a feature
 * component rather than a new primitive: the inventory is closed, and a second
 * consumer is what would justify promoting it. It carries the parts a
 * confirmation cannot go without:
 *
 *   - `role="alertdialog"` with `aria-modal`, labelled by the title and
 *     described by the consequence;
 *   - a scrim on the semantic `--color-scrim`, above the shell's failure surface;
 *   - focus moved to the safe action (cancel) on open, a real Tab trap inside the
 *     panel, Escape cancelling, and focus returned to the control that opened
 *     it. There is deliberately no press-outside-to-dismiss: an irreversible
 *     confirmation is answered by one of its own two controls, and a stray tap
 *     on the scrim should not speak for the person either way;
 *   - 44 pt controls (the system `Button`), safe-area padding at every edge, and
 *     a panel that scrolls rather than growing past the viewport with Dynamic
 *     Type.
 *
 * Cancelling is refused while the write is in flight: the operation may land,
 * and closing the dialog would hide the outcome of an irreversible action. The
 * confirm control is disabled and busy for the same reason.
 */
import { type KeyboardEvent, useEffect, useId, useRef } from 'react';
import { Button } from '../../../components/ui/Button';
import { Heading } from '../../../components/ui/Heading';
import { Notice } from '../../../components/ui/Notice';
import { Text } from '../../../components/ui/Text';
import { t } from '../../i18n';

export type DeleteRabbitDialogProps = {
  /** Identifies the rabbit the consequence applies to. */
  rabbitName: string;
  /** The delete write is in flight. */
  pending: boolean;
  /**
   * The `validation` refusal the dialog owns, already resolved to copy. Every
   * other cause is the route shell's failure surface, not this one.
   */
  errorMessage?: string;
  onCancel: () => void;
  onConfirm: () => void;
};

export function DeleteRabbitDialog({
  rabbitName,
  pending,
  errorMessage,
  onCancel,
  onConfirm,
}: DeleteRabbitDialogProps) {
  const titleId = useId();
  const descriptionId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const opener = useRef<Element | null>(null);

  /*
   * The safe action takes focus, and the control that opened the dialog gets it
   * back on close. The opener is captured before the focus move, and only
   * restored while it is still connected: the archived row may have been
   * removed by the confirmed deletion, in which case the browser's next focus
   * target is the honest one.
   */
  useEffect(() => {
    opener.current = document.activeElement;
    panelRef.current
      ?.querySelector<HTMLButtonElement>('.bs-delete-dialog__cancel')
      ?.focus();

    return () => {
      const previous = opener.current;

      if (previous instanceof HTMLElement && previous.isConnected) {
        previous.focus();
      }
    };
  }, []);

  /*
   * A busy write disables both controls, so the button that had focus would lose
   * it to `<body>`. The panel takes it instead and the trap below keeps it
   * there: the dialog stays the thing being operated while it is in flight.
   */
  useEffect(() => {
    if (pending) {
      panelRef.current?.focus();
    }
  }, [pending]);

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'Escape') {
      if (!pending) {
        event.preventDefault();
        onCancel();
      }

      return;
    }

    if (event.key !== 'Tab') {
      return;
    }

    const panel = panelRef.current;

    if (panel === null) {
      return;
    }

    const focusable = Array.from(
      panel.querySelectorAll<HTMLElement>('button:not([disabled])'),
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (first === undefined || last === undefined) {
      event.preventDefault();
      return;
    }

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
      return;
    }

    if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  return (
    <div className="bs-delete-dialog">
      <div
        aria-busy={pending ? true : undefined}
        aria-describedby={descriptionId}
        aria-labelledby={titleId}
        aria-modal="true"
        className="bs-delete-dialog__panel"
        onKeyDown={handleKeyDown}
        ref={panelRef}
        role="alertdialog"
        tabIndex={-1}
      >
        <Heading as="h2" id={titleId} variant="title-3">
          {t('rabbit.deleteConfirm.title')}
        </Heading>
        <div className="bs-delete-dialog__body" id={descriptionId}>
          <Text as="p" tone="secondary" variant="subheadline">
            {rabbitName}
          </Text>
          <Text as="p">{t('rabbit.deleteConfirm.body')}</Text>
        </div>
        {errorMessage === undefined ? null : (
          <Notice tone="danger">{errorMessage}</Notice>
        )}
        <div className="bs-delete-dialog__actions">
          <Button
            className="bs-delete-dialog__cancel"
            disabled={pending}
            onClick={onCancel}
            variant="plain"
          >
            {t('rabbit.deleteConfirm.cancel')}
          </Button>
          <Button loading={pending} onClick={onConfirm} variant="destructive">
            {pending ? t('rabbit.deleting') : t('rabbit.deleteConfirm.confirm')}
          </Button>
        </div>
      </div>
    </div>
  );
}
