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
 * DASH-4's drop flag is 4.12's; this row only carries `flag` in the trailing
 * slot P0's row already has (`dashboardRowHasDropAlert` picks it, 4.12 paints
 * it).
 */
import { type ReactNode, useState } from 'react';
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
   * Trailing indicator, rendered in the list row's control slot. DASH-4's
   * weight-drop flag is task 4.12's to compose and style.
   */
  flag?: ReactNode;
};

export function RabbitRow({ row, onOpen, flag }: RabbitRowProps) {
  const { rabbit } = row;

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
        <span className="bs-list-row__title">{rabbit.name}</span>
        <span className="bs-list-row__detail">{lastWeighedDetail(row)}</span>
      </span>
      {flag === undefined ? null : (
        <span className="bs-list-row__control">{flag}</span>
      )}
    </>
  );

  return (
    <li className="bs-list-row bs-rabbit-row">
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
