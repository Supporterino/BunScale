/*
 * One dashboard rabbit row (task 4.1; DASH-1, DASH-2, DASH-3).
 *
 * The row presents exactly what DASH-1 asks for and nothing derived here:
 * the rabbit's name, its thumbnail with the "no photo" placeholder when the
 * payload carries no reference, and its last-weighed date or DASH-2's
 * never-weighed indication. The ordering is not this component's either — the
 * read model sorts (DASH-3, task 2.14) and the list renders the payload in the
 * order it arrives.
 *
 * **The never-weighed indication is the contract's flag, read as text.** DASH-2
 * asks for an indication *in place of* a date, so a never-weighed row prints
 * `dashboard.neverWeighed` where a weighed row prints `dashboard.lastWeighed`
 * and a `<time>`; it never depends on a colour or an icon, and it carries no
 * date of its own. The date is the device's locale at medium length: the app
 * has no date formatter of its own (P0 formats none), and a hand-written pattern
 * would be a second, worse locale decision than `Intl`'s.
 *
 * **The row's classes are P0's list row, not a copy of it.** `ListRow` has no
 * slot for a leading image, so this row composes the primitive's own class
 * contract (`.bs-list-row`, `__target`, `__text`, `__title`, `__detail`,
 * `__control`) rather than restyling a second list surface: height, separators,
 * pressed state and the trailing control slot stay the ones `ui.css` already
 * defines. Only the thumbnail is RabbitRow's own, in `screens.css`.
 *
 * **A control only where there is a destination.** `onOpen` turns the row into
 * a real `<button>` with the row's text as its accessible name; without it the
 * row is static content. There is no rabbit-dashboard route in P1 until task
 * 4.11, and a focusable button that cannot go anywhere is a worse lie than a
 * row that is honestly not interactive yet — the caller supplies the handler
 * when the destination exists. The static mode also omits `ListRow`'s
 * `--interactive` target class, so it paints no pressed or hover surface.
 *
 * **DASH-9's quick-weight control is the row's sibling, never its child**
 * (task 4.7; DASH-9, WT-2, NFR-9, D1.10). It is a directly visible,
 * always-present control in the row's trailing control slot — not a long press
 * and not a hidden menu (NFR-9). Because it is a target of its own it cannot
 * live inside the row's open-dashboard button: nesting interactive controls
 * makes one of them unreachable by touch, so the two siblings both stay
 * operable. The control navigates to the weight-form route with the rabbit
 * bound through typed search parameters (task 4.18), so the route owns the
 * form, the household read and the return; this row only names the rabbit. Its
 * visible label is the catalog's `weight.log`, and `aria-labelledby` adds the
 * row title's id so every row's control is distinguishable — "Hazel Log
 * weight" — without a new key or a second visible label. The compact size keeps
 * the system's 44 pt minimum without stretching the row, and `data-quick-weight`
 * marks the slot for the adjacent-target spacing.
 *
 * The control composes `Button`'s class contract rather than using the
 * primitive because `Button` does not forward ARIA attributes and this name
 * needs `aria-labelledby`. It is the same composition this row already makes
 * for the list row, whose primitive has no leading-image slot; no style is
 * copied, only the class names.
 *
 * **DASH-4's drop flag is a second line in the text column, not a trailing
 * slot** (task 4.12; DASH-4, WT-7). Two reasons. The trailing control slot is
 * already the quick-weight control's (4.7), so a badge there would be a second
 * flex item competing with it; and the badge's copy is a fixed phrase, so
 * sharing the row's width would squeeze the rabbit's name and date at 390 px
 * instead of adding a line. Under the detail it never touches a 44 pt target,
 * it stays in the row button's reading order, and it is the caller's `flag`
 * only when the payload flag is set — the row re-derives nothing
 * (`dashboardRowHasDropAlert` is the selector, in `RabbitList`).
 */
import { type ReactNode, useId, useState } from 'react';
import { t } from '../../i18n';
import type { DashboardRow } from '../queries';

/**
 * The last-weighed date, at medium length in the device's locale. Created once
 * at module scope: an `Intl.DateTimeFormat` holds parsed options, so building
 * one per row per render would redo that work on every refetch.
 */
const lastWeighedFormat = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
});

export type RabbitRowProps = {
  /** One row of the dashboard read, in the backend's order (DASH-1 … DASH-3). */
  row: DashboardRow;
  /**
   * Opens this rabbit's dashboard (DASH-6). Supplied by the screen once the
   * rabbit route exists (task 4.11); absent, the row is static.
   */
  onOpen?: () => void;
  /**
   * DASH-4's drop flag, rendered under the row's detail when the payload flags
   * a drop. Supplied by the dashboard list; absent, the row presents nothing.
   */
  flag?: ReactNode;
  /**
   * Opens the weight form bound to this rabbit (DASH-9, WT-2, D1.10). Supplied
   * by the dashboard list, which navigates to the weight-form route; absent, the
   * row carries no quick-weight control.
   */
  onLogWeight?: () => void;
};

export function RabbitRow({ row, onOpen, flag, onLogWeight }: RabbitRowProps) {
  const { rabbit } = row;
  const titleId = useId();
  const quickWeightLabelId = useId();

  /*
   * The accessible name is this content, in reading order: the rabbit's name,
   * the placeholder's own name when there is no photo, and the last-weighed
   * date or the never-weighed indication. No `aria-label` restates it, so the
   * spoken name and the visible row cannot drift apart.
   */
  const content = (
    <>
      <span className="bs-rabbit-row__thumbnail">
        <RabbitThumbnail reference={rabbit.thumbnail_ref} />
      </span>
      <span className="bs-list-row__text">
        <span className="bs-list-row__title" id={titleId}>
          {rabbit.name}
        </span>
        <span className="bs-list-row__detail">{lastWeighedDetail(row)}</span>
        {flag === undefined ? null : (
          <span className="bs-rabbit-row__flag">{flag}</span>
        )}
      </span>
    </>
  );

  return (
    <li className="bs-list-row bs-rabbit-row" data-rabbit-id={rabbit.id}>
      {onOpen === undefined ? (
        <span className="bs-list-row__target">{content}</span>
      ) : (
        <button
          className="bs-list-row__target bs-list-row__target--interactive"
          onClick={onOpen}
          type="button"
        >
          {content}
        </button>
      )}
      {onLogWeight === undefined ? null : (
        /*
         * The control slot centres the 44 pt control inside the row instead of
         * stretching it to the row's full height, so the pressed surface stays
         * the size of the control and the thumbnail keeps the row's rhythm.
         */
        <span className="bs-list-row__control" data-quick-weight="">
          <button
            aria-labelledby={`${titleId} ${quickWeightLabelId}`}
            className="bs-button bs-button--plain bs-button--compact"
            onClick={onLogWeight}
            type="button"
          >
            <span className="bs-button__label" id={quickWeightLabelId}>
              {t('weight.log')}
            </span>
          </button>
        </span>
      )}
    </li>
  );
}

/**
 * DASH-1's date, or DASH-2's never-weighed indication in its place.
 *
 * The flag decides, not the absence of a date: `never_weighed` is the
 * contract's explicit indication and `last_weighed_at` is redundant with it
 * (DASH-2), so a row the backend marks never-weighed never prints a date.
 */
function lastWeighedDetail(row: DashboardRow): ReactNode {
  if (row.never_weighed || row.last_weighed_at === null) {
    return t('dashboard.neverWeighed');
  }

  return (
    <>
      {t('dashboard.lastWeighed')}{' '}
      <time dateTime={row.last_weighed_at}>
        {lastWeighedFormat.format(new Date(row.last_weighed_at))}
      </time>
    </>
  );
}

/**
 * The rabbit's thumbnail, or RAB-1's placeholder.
 *
 * P1 carries an opaque reference but no way to populate one — choosing a photo
 * is PHOTO-2 in P2 (D1.11) — so in P1's own flows this always renders the
 * placeholder. A supplied reference is presented as an image and falls back to
 * the placeholder when it cannot be loaded, because an opaque reference is not
 * yet a URL this build can resolve: the fallback has to be decided by the image
 * itself, not by the reference being present. The photo is decorative (the row
 * is named by its text); the placeholder carries `rabbit.thumbnailPlaceholder`
 * as its accessible name, which is the key's stated purpose.
 */
function RabbitThumbnail({ reference }: { reference: string | null }) {
  const [failed, setFailed] = useState(false);

  if (reference === null || failed) {
    return (
      <span
        aria-label={t('rabbit.thumbnailPlaceholder')}
        className="bs-rabbit-row__placeholder"
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

  return (
    <img
      alt=""
      className="bs-rabbit-row__photo"
      onError={() => setFailed(true)}
      src={reference}
    />
  );
}
