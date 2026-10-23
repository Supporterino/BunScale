/*
 * The dashboard rabbit list (task 4.1; DASH-1, DASH-2, DASH-3), with the
 * zero-row empty state added by task 4.2 (DASH-8, RAB-4).
 *
 * One read, one list: `useDashboardRows` asks the active household's dashboard
 * read for its rows and this component presents them as P0's grouped `List` of
 * `RabbitRow`s. The rows arrive already ordered — most recent weigh-in first,
 * never-weighed last (DASH-3) — and are rendered in payload order on purpose.
 * Sorting here would be the second derivation the read model exists to avoid,
 * and it would have to reproduce the backend's own ordering rules.
 *
 * **No answer is not "no rabbits".** `data === undefined` means no read has
 * resolved: the first one is still in flight, or it failed before it ever
 * succeeded. That is the caller's `fallback` (P0's "what this household is"
 * intro), because "no rabbits" is a fact only a resolved answer can state.
 * DASH-8's empty state is therefore decided on `data.length === 0`, never on
 * the absence of data.
 *
 * **The add control is outside the zero-row branch (task 4.3; RAB-4).** The
 * empty state keeps its own prompt for the zero-row case, while a toolbar
 * control is rendered for the pending and non-empty cases. Every state the
 * dashboard can be in therefore carries a visible way into the creation form;
 * without the toolbar the list only offered it once it had resolved to zero.
 *
 * **The empty state is keyed on the active list.** `get_dashboard` carries the
 * household's non-archived rabbits only, so a household whose rabbits are all
 * archived resolves to zero rows here and presents the empty state, while the
 * archived list stays reachable as its own screen (design.md D1.6). This list
 * never consults the archived read to decide: the empty state belongs to the
 * active list, not to a household rabbit count.
 *
 * **A failure is 4.15's surface, not this one.** A read that never resolved
 * falls back exactly as before; nothing here invents failure copy.
 *
 * **What this list still does not own.** DASH-4's per-row drop flag (task 4.12)
 * and DASH-9's quick-weight control (4.7) compose into the row's control slot.
 *
 * **The row control is supplied, not invented.** DASH-6's destination is the
 * rabbit dashboard, which P1's route tree does not hold until task 4.11, so the
 * dashboard mounts this list without `onOpenRabbit` and each row stays static.
 * 4.11 passes the handler and the same row becomes a real button; no route is
 * guessed here and no control is rendered without a destination.
 */
import type { ReactNode } from 'react';
import { Button } from '../../../components/ui/Button';
import { List } from '../../../components/ui/List';
import { t } from '../../i18n';
import { useDashboardRows } from '../queries';
import { EmptyRabbitList } from './EmptyRabbitList';
import { RabbitRow } from './RabbitRow';

export type RabbitListProps = {
  /**
   * Shown while no answer has resolved — before the first one, and when the read
   * failed and never succeeded. Zero resolved rows is DASH-8's empty state, not
   * this fallback.
   */
  fallback?: ReactNode;
  /** Opens a row's rabbit dashboard (DASH-6; task 4.11 supplies this). */
  onOpenRabbit?: (rabbitId: string) => void;
  /**
   * Opens the rabbit-creation form (RAB-4). Task 4.3's dashboard supplies it in
   * both the empty state and the toolbar; absent, the control renders disabled
   * rather than pretending to have a destination.
   */
  onAddRabbit?: () => void;
};

export function RabbitList({
  fallback = null,
  onOpenRabbit,
  onAddRabbit,
}: RabbitListProps) {
  const rows = useDashboardRows();

  /*
   * `data` is the last successful answer: `undefined` before the first one,
   * still present during a refetch, and undefined again when the read failed
   * and never succeeded. A failed read has no cause copy that belongs to this
   * task's catalog, so it keeps the nothing-loaded fallback rather than
   * presenting an invented message (4.15 owns the failure surface).
   */
  const data = rows.data;

  if (data !== undefined && data.length === 0) {
    return <EmptyRabbitList onAddRabbit={onAddRabbit} />;
  }

  return (
    <>
      {/*
       * RAB-4's non-zero-row half: present while the read is pending and while
       * it holds rows, so the form is reachable in every dashboard state. It
       * stays labelled but disabled when the caller supplies no destination,
       * exactly as the empty state's action does.
       */}
      <div className="bs-dashboard__toolbar">
        <Button
          disabled={onAddRabbit === undefined}
          onClick={onAddRabbit}
          variant="secondary"
        >
          {t('rabbit.add')}
        </Button>
      </div>

      {data === undefined ? (
        fallback
      ) : (
        <List>
          {data.map((row) => (
            <RabbitRow
              key={row.rabbit.id}
              onOpen={
                onOpenRabbit === undefined
                  ? undefined
                  : () => {
                      onOpenRabbit(row.rabbit.id);
                    }
              }
              row={row}
            />
          ))}
        </List>
      )}
    </>
  );
}
