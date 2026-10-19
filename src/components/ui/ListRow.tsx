import type { LinkProps } from '@tanstack/react-router';
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `ListRow` — one row of a `List`: a household, a settings entry, a unit choice.
 *
 * The row is only interactive when the caller makes it so, and then it is a real
 * control rather than a clickable `<div>`:
 *
 *   - `to` — a navigational row, rendered as a router `Link` (never a
 *     string-built `<a href>`), with `aria-current="page"` on the destination the
 *     router considers current;
 *   - `onClick` — an actionable row, rendered as a `<button type="button">`;
 *   - neither — a static row, rendered as a `<span>`: no focus stop, no invented
 *     affordance, and the `accessory` (a chevron or a checkmark) is not painted
 *     *even when a caller passes one*, because a chevron on a non-navigational
 *     row promises a screen that does not exist. The suppression is enforced
 *     here, not left to the caller's restraint: a mode's contract that only
 *     holds when nobody asks is not a contract.
 *
 * `selected` is the app's own idea of the current item in a set (the active
 * household) and is exposed as `aria-current`, so the state is announced and not
 * only drawn. Every row is at least 44 pt tall with an 8 pt gap to the next
 * (the separators are decorative and drawn by the stylesheet), and the pressed
 * state exists on touch — hover is a fine-pointer enhancement only.
 *
 * The accessory marks are drawn here rather than demanded from the caller
 * because they are part of the row's contract; they are `aria-hidden`, since
 * they restate the row's semantics rather than carry any.
 */

export type ListRowAccessory = 'none' | 'chevron' | 'checkmark';

export type ListRowProps = {
  title: string;
  /** Secondary line: a count, a timestamp, a unit. */
  detail?: string;
  /** Turns the row into a navigational `Link`. */
  to?: LinkProps['to'];
  /** Turns the row into a `button`. */
  onClick?: () => void;
  accessory?: ListRowAccessory;
  /** The current item of a set; exposed as `aria-current`. */
  selected?: boolean;
  /** Trailing control (a `SegmentedControl`, a badge) instead of an accessory. */
  control?: ReactNode;
  className?: string;
};

export function ListRow({
  title,
  detail,
  to,
  onClick,
  accessory = 'none',
  selected = false,
  control,
  className,
}: ListRowProps) {
  /*
   * The accessory is an affordance, so it only exists where the row is a
   * control: `to` (a link) or `onClick` (a button). In the static mode the row
   * is a `<span>` and the mark is dropped, whatever the caller asked for.
   */
  const isInteractive = to !== undefined || onClick !== undefined;
  const accessoryToRender: Exclude<ListRowAccessory, 'none'> | null =
    isInteractive && accessory !== 'none' ? accessory : null;

  const content = (
    <>
      <span className="bs-list-row__text">
        <span className="bs-list-row__title">{title}</span>
        {detail === undefined ? null : (
          <span className="bs-list-row__detail">{detail}</span>
        )}
      </span>
      {control === undefined ? null : (
        <span className="bs-list-row__control">{control}</span>
      )}
      {accessoryToRender === null ? null : (
        <AccessoryMark accessory={accessoryToRender} />
      )}
    </>
  );
  const classes = classNames('bs-list-row', className);
  const current = selected ? ('true' as const) : undefined;

  if (to !== undefined) {
    return (
      <li className={classes}>
        <Link
          activeProps={{ 'aria-current': 'page' }}
          className="bs-list-row__target"
          to={to}
        >
          {content}
        </Link>
      </li>
    );
  }

  if (onClick !== undefined) {
    return (
      <li className={classes}>
        <button
          aria-current={current}
          className="bs-list-row__target"
          onClick={onClick}
          type="button"
        >
          {content}
        </button>
      </li>
    );
  }

  return (
    <li aria-current={current} className={classes}>
      <span className="bs-list-row__target">{content}</span>
    </li>
  );
}

function AccessoryMark({
  accessory,
}: {
  accessory: Exclude<ListRowAccessory, 'none'>;
}) {
  return (
    <span aria-hidden="true" className="bs-list-row__accessory">
      <svg
        aria-hidden="true"
        fill="none"
        height="16"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        viewBox="0 0 16 16"
        width="16"
      >
        {accessory === 'chevron' ? (
          <polyline points="5.5,2.5 11,8 5.5,13.5" />
        ) : (
          <polyline points="2.5,8.5 6.5,12.5 13.5,3.5" />
        )}
      </svg>
    </span>
  );
}
