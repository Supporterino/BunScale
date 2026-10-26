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
 * **DASH-9's quick-weight control opens the weight-form route** (task 4.7
 * rework; DASH-9, WT-2, NFR-9, D1.10). Every row carries the control
 * (`RabbitRow.onLogWeight`), and using it navigates to `/weight` with the rabbit
 * bound through typed search parameters — `rabbit`, `mode: 'quick'` and
 * `from: 'dashboard'` — instead of mounting 4.9's form on this surface. The
 * route owns the form, the household read and the return: cancel and a
 * confirmed save navigate back to `/` with `focusRabbitId` in the history entry
 * (4.18), and `HouseholdDashboardScreen` restores focus to that rabbit's row
 * once the list has settled (4.16). This list therefore holds no form state and
 * no focus machinery of its own.
 *
 * **DASH-4's per-row drop flag is the payload's flag, in the catalog's words**
 * (task 4.12; DASH-4, WT-7). A flagged row composes the same marker 4.11's entry
 * lists use — a `warning` badge carrying `weight.dropAlert` — so one state reads
 * the same on the dashboard and in the weight history, and it is text rather
 * than a colour-only dot. The selector is `dashboardRowHasDropAlert`, which
 * reads the row's derived flag and never folds the points this device holds;
 * an unflagged row passes no `flag` at all, so raising the threshold clears the
 * marker on the next read with no local state to reset.
 *
 * **WT-2's second quick path is the toolbar's rabbit picker** (task 4.20;
 * WT-2, NFR-9, Q12). It appears beside the add control once the read has rows —
 * before that there is no rabbit to offer — and choosing one navigates through
 * the same `logWeight` destination as a row's control, so both paths bind the
 * form identically. The control itself is `RabbitPicker`, feature-local per Q12.
 *
 * **The row's open control is supplied, not invented.** DASH-6's destination is
 * the rabbit dashboard route, which the screen owns: it passes `onOpenRabbit`
 * (task 4.16) and the row becomes a real button. Without a handler the row stays
 * static; no route is guessed here and no control is rendered without a
 * destination.
 */
import { useNavigate } from '@tanstack/react-router';
import type { ReactNode } from 'react';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { List } from '../../../components/ui/List';
import { t } from '../../i18n';
import { dashboardRowHasDropAlert } from '../../weight/alerts';
import { useDashboardRows } from '../queries';
import { EmptyRabbitList } from './EmptyRabbitList';
import { RabbitPicker } from './RabbitPicker';
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
  const navigate = useNavigate();

  /*
   * `data` is the last successful answer: `undefined` before the first one,
   * still present during a refetch, and undefined again when the read failed
   * and never succeeded. A failed read has no cause copy that belongs to this
   * task's catalog, so it keeps the nothing-loaded fallback rather than
   * presenting an invented message (4.15 owns the failure surface).
   */
  const data = rows.data;

  /*
   * DASH-9's control opens the one weight-form route, bound to the row's rabbit
   * through typed search parameters (D1.10). The route reads the household and
   * the profile itself, so this list passes no data across and needs no form
   * state.
   */
  function logWeight(rabbitId: string): void {
    void navigate({
      to: '/weight',
      search: { rabbit: rabbitId, mode: 'quick', from: 'dashboard' },
    });
  }

  if (data !== undefined && data.length === 0) {
    return <EmptyRabbitList onAddRabbit={onAddRabbit} />;
  }

  return (
    <>
      {/*
       * RAB-4's non-zero-row half: present while the read is pending and while
       * it holds rows, so the form is reachable in every dashboard state. It
       * stays labelled but disabled when the caller supplies no destination,
       * exactly as the empty state's action does. WT-2's picker joins it only
       * once there are rabbits to choose.
       */}
      <div className="bs-dashboard__toolbar">
        {data === undefined ? null : (
          <RabbitPicker onChoose={logWeight} rows={data} />
        )}
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
              flag={
                dashboardRowHasDropAlert(row) ? (
                  <Badge tone="warning">{t('weight.dropAlert')}</Badge>
                ) : undefined
              }
              onLogWeight={() => {
                logWeight(row.rabbit.id);
              }}
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
