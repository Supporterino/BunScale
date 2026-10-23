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
 * only drawn. Every row is at least 44 pt tall, with a hairline separator drawn
 * by the stylesheet. The pressed and hover surfaces belong to the link and
 * button branches only: the static mode renders no affordance at all — including
 * no painted press — because a row that lights up and does nothing is the
 * invented affordance decision 7 rules out (global convention 8). The
 * stylesheet's hook for that is the `--interactive` modifier below.
 *
 * The accessory marks are drawn here rather than demanded from the caller
 * because they are part of the row's contract; they are `aria-hidden`, since
 * they restate the row's semantics rather than carry any.
 */

export type ListRowAccessory = 'none' | 'chevron' | 'checkmark';

/**
 * The target's class contract. `--interactive` is what marks a real control for
 * the stylesheet's pressed and hover rules; the static mode renders without it,
 * so it cannot paint an affordance. A feature row that composes these classes
 * for its own control sets the same modifier (see `RabbitRow`).
 */
const TARGET_CLASS = 'bs-list-row__target';
const INTERACTIVE_TARGET_CLASS = `${TARGET_CLASS} ${TARGET_CLASS}--interactive`;

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
          className={INTERACTIVE_TARGET_CLASS}
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
          className={INTERACTIVE_TARGET_CLASS}
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
      <span className={TARGET_CLASS}>{content}</span>
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
