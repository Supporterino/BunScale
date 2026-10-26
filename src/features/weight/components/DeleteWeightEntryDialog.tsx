/*
 * The weight entry deletion confirmation (task 4.21; WT-5, WT-8, NFR-9).
 *
 * WT-5's delete removes the entry for every member of the household and cannot
 * be undone, so NFR-9 requires a confirmation that states the consequence before
 * anything is removed. The body is the catalog's
 * `weight.entry.deleteConfirm.body`; the confirm control is the destructive
 * variant and the safe one ("Keep entry") takes focus.
 *
 * **A dialog, because the design system defers `Dialog`/`Alert`.** The frozen
 * inventory has no modal primitive, and this is the weight feature's own copy of
 * the pattern `DeleteRabbitDialog` established rather than a promoted primitive:
 * the inventory is closed and a third consumer is what would justify one. It
 * reuses that pattern's `.bs-delete-dialog` rules unchanged, so the scrim,
 * elevation, safe-area padding and the scrolling panel are already the reviewed
 * ones, and it carries the parts a confirmation cannot go without:
 *
 *   - `role="alertdialog"` with `aria-modal`, labelled by the title and
 *     described by the consequence;
 *   - focus moved to the safe action on open, a real Tab trap inside the panel,
 *     Escape cancelling, and focus returned to the control that opened it.
 *     There is deliberately no press-outside-to-dismiss: an irreversible
 *     confirmation is answered by one of its own two controls;
 *   - both controls locked while the write is in flight, because the operation
 *     may land and closing the dialog would hide the outcome.
 */
import { useEffect, useId, useRef } from 'react';
import { Button } from '../../../components/ui/Button';
import { Heading } from '../../../components/ui/Heading';
import { Notice } from '../../../components/ui/Notice';
import { Text } from '../../../components/ui/Text';
import { t } from '../../i18n';

export type DeleteWeightEntryDialogProps = {
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

export function DeleteWeightEntryDialog({
  pending,
  errorMessage,
  onCancel,
  onConfirm,
}: DeleteWeightEntryDialogProps) {
  const titleId = useId();
  const descriptionId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const opener = useRef<Element | null>(null);

  /*
   * The safe action takes focus, and the control that opened the dialog gets it
   * back on close unless the confirmed deletion has already removed it with the
   * surface it belonged to.
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

  /*
   * The keyboard contract is bound to the document, not the panel: a click on
   * the scrim can hand focus to `<body>`, and a panel-scoped listener would then
   * never see Tab or Escape again.
   */
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
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
        panel.focus();
        return;
      }

      if (document.activeElement === panel) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
        return;
      }

      if (!panel.contains(document.activeElement)) {
        event.preventDefault();
        first.focus();
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

    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [pending, onCancel]);

  return (
    <div className="bs-delete-dialog">
      <div
        aria-busy={pending ? true : undefined}
        aria-describedby={descriptionId}
        aria-labelledby={titleId}
        aria-modal="true"
        className="bs-delete-dialog__panel"
        ref={panelRef}
        role="alertdialog"
        tabIndex={-1}
      >
        <Heading as="h2" id={titleId} variant="title-3">
          {t('weight.entry.deleteConfirm.title')}
        </Heading>
        <div className="bs-delete-dialog__body" id={descriptionId}>
          <Text as="p">{t('weight.entry.deleteConfirm.body')}</Text>
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
            {t('weight.entry.deleteConfirm.cancel')}
          </Button>
          <Button loading={pending} onClick={onConfirm} variant="destructive">
            {pending
              ? t('weight.entry.deleting')
              : t('weight.entry.deleteConfirm.confirm')}
          </Button>
        </div>
      </div>
    </div>
  );
}
