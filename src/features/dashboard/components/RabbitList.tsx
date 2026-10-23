/*
 * The dashboard rabbit list (task 4.1; DASH-1, DASH-2, DASH-3).
 *
 * One read, one list: `useDashboardRows` asks the active household's dashboard
 * read for its rows and this component presents them as P0's grouped `List` of
 * `RabbitRow`s. The rows arrive already ordered — most recent weigh-in first,
 * never-weighed last (DASH-3) — and are rendered in payload order on purpose.
 * Sorting here would be the second derivation the read model exists to avoid,
 * and it would have to reproduce the backend's own ordering rules.
 *
 * **What this list does not own.** DASH-8's empty state (task 4.2) and DASH-4's
 * per-row drop flag (task 4.12) are the next tasks on this surface; DASH-9's
 * quick-weight control (4.7) composes into the row's control slot. Until one of
 * them lands, zero rows and a read that has not answered yet both render the
 * `fallback` the screen supplies — P0's "what this household is" intro. That is
 * a nothing-loaded surface, not an invented empty state: "no rabbits" is a fact
 * only a loaded answer can state, and 4.2 is the task that states it.
 *
 * **The row control is supplied, not invented.** DASH-6's destination is the
 * rabbit dashboard, which P1's route tree does not hold until task 4.11, so the
 * dashboard mounts this list without `onOpenRabbit` and each row stays static.
 * 4.11 passes the handler and the same row becomes a real button; no route is
 * guessed here and no control is rendered without a destination.
 */
import type { ReactNode } from 'react';
import { List } from '../../../components/ui/List';
import { useDashboardRows } from '../queries';
import { RabbitRow } from './RabbitRow';

export type RabbitListProps = {
  /**
   * Shown while there is no loaded row to present — before the first answer,
   * on a failed read, and for a household with no rabbits (until task 4.2's
   * empty state replaces it). The screen supplies P0's intro.
   */
  fallback?: ReactNode;
  /** Opens a row's rabbit dashboard (DASH-6; task 4.11 supplies this). */
  onOpenRabbit?: (rabbitId: string) => void;
};

export function RabbitList({ fallback = null, onOpenRabbit }: RabbitListProps) {
  const rows = useDashboardRows();

  /*
   * `data` is the last successful answer: `undefined` before the first one,
   * still present during a refetch, and undefined again when the read failed
   * and never succeeded. A failed read has no cause copy that belongs to this
   * task's catalog, so it degrades to the same nothing-loaded surface rather
   * than presenting a stale claim or an invented message.
   */
  const data = rows.data;

  if (data === undefined || data.length === 0) {
    return <>{fallback}</>;
  }

  return (
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
  );
}
