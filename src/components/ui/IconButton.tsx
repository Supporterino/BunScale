import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `IconButton` — a glyph action: the navigation bar's gear, a close, a retry.
 *
 * `label` is required by the type, which is the whole point: an icon-only
 * control has no accessible name unless one is given, and a glyph is not a name.
 * The label is the button's `aria-label` (there is no tooltip-only path, and no
 * `title`-attribute fallback, neither of which exists on touch); pass the glyph
 * as `children` with `aria-hidden` on its `<svg>` and the hit area is still
 * 44 x 44 even when the glyph is 20 px (`--icon-button-size`).
 *
 * The inventory's `default` / `pressed` / `disabled` states map onto real
 * semantics rather than painted ones:
 *   - `default` — an ordinary button;
 *   - `pressed` — a toggle, exposed as `aria-pressed` (not just a background),
 *     so a screen reader hears the state;
 *   - `disabled` — the `disabled` attribute, still labelled.
 */

export type IconButtonProps = {
  /** Accessible name of the control. Required. */
  label: string;
  /** The glyph; pass a decorative `<svg aria-hidden="true">`. */
  children: ReactNode;
  /** Toggle state: paints the pressed appearance and sets `aria-pressed`. */
  pressed?: boolean;
  disabled?: boolean;
  onClick?: () => void;
  className?: string;
};

export function IconButton({
  label,
  children,
  pressed,
  disabled = false,
  onClick,
  className,
}: IconButtonProps) {
  return (
    <button
      aria-label={label}
      aria-pressed={pressed}
      className={classNames('bs-icon-button', className)}
      disabled={disabled}
      onClick={onClick}
      type="button"
    >
      {children}
    </button>
  );
}
