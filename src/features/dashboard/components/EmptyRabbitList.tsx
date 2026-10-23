/*
 * The dashboard's zero-rabbit empty state (task 4.2; DASH-8, RAB-4).
 *
 * DASH-8 asks for an empty state only when the active household holds no
 * rabbits; RAB-4 asks that the surface carry a prompt to add the first one and
 * that the creation form stay reachable whenever the dashboard is shown. On the
 * zero-row dashboard this component is where that control lives; the same
 * requirement's non-empty case is the list's toolbar control (task 4.3), which
 * this component does not draw. It is P0's `EmptyState` with the P1 copy and one
 * directly visible `Button`. It is not an error — nothing failed, there is
 * simply nothing yet — and it sets no live role, because it mounts with the
 * dashboard rather than replacing content after an action.
 *
 * **The add control is real and directly visible.** It is the empty state's one
 * action, not a hidden menu or a hover affordance: `rabbit.add` is its visible
 * label, `Button` gives it a 50 px control and a real `button` element, and it
 * is reachable by touch and by keyboard. `onAddRabbit` opens the inline creation
 * form; the dashboard always supplies it, so in the product the control is live.
 * The optional prop and its `disabled` rendering are only the defensive state
 * for a caller that has no destination to give — it stays present and labelled
 * rather than silently doing nothing. No destination is guessed here.
 */
import { Button } from '../../../components/ui/Button';
import { EmptyState } from '../../../components/ui/EmptyState';
import { t } from '../../i18n';

export type EmptyRabbitListProps = {
  /**
   * Opens the rabbit-creation form (RAB-4, task 4.3). Absent until that route
   * exists; the add control then renders disabled rather than doing nothing.
   */
  onAddRabbit?: () => void;
};

export function EmptyRabbitList({ onAddRabbit }: EmptyRabbitListProps) {
  return (
    <EmptyState
      action={
        <Button disabled={onAddRabbit === undefined} onClick={onAddRabbit}>
          {t('rabbit.add')}
        </Button>
      }
      title={t('dashboard.emptyTitle')}
    >
      {t('dashboard.emptyBody')}
    </EmptyState>
  );
}
