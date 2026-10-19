import type { ReactNode } from 'react';
import { classNames } from './class-names';
import { LoadingIndicator } from './LoadingIndicator';

/*
 * `Button` — the app's single action control: create a household, retry, save.
 *
 * `button`, never a styled `div`: it is focusable, it answers Space and Enter,
 * and it carries the disabled semantics. The visible label always stays (an
 * icon-only action is `IconButton`, which requires its own label), and the label
 * also stays while loading, so the reader still knows what is in flight;
 * `aria-busy` is what says that it is.
 *
 * `loading` implies `disabled`, because a busy control that can be activated a
 * second time is a duplicate-submit bug; passing `disabled` is still the way to
 * disable a control that is not busy.
 *
 * Touch: `--button-height` is 50 px, above the 44 pt baseline, and the
 * interactive states are pressed, focused and disabled — exactly three, and
 * hover is deliberately not a fourth. `src/styles/ui.css` has no
 * `.bs-button:hover` rule (the navigational rows do have one, because a plain
 * text row has to advertise that it is tappable); a filled or bordered button
 * already advertises itself, its pressed state is the feedback, and every action
 * is reachable by touch and keyboard without a pointer. Adding a hover state
 * would mean a hover colour per variant — four more tokens for an enhancement
 * nothing depends on. Disabled uses the real `disabled` attribute rather than
 * opacity alone, so it is genuinely unreachable and still labelled.
 */

export type ButtonVariant = 'primary' | 'secondary' | 'plain' | 'destructive';

export type ButtonSize = 'regular' | 'compact';

export type ButtonProps = {
  children: ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  disabled?: boolean;
  fullWidth?: boolean;
  onClick?: () => void;
  type?: 'button' | 'submit';
  className?: string;
};

export function Button({
  children,
  variant = 'primary',
  size = 'regular',
  loading = false,
  disabled = false,
  fullWidth = false,
  onClick,
  type = 'button',
  className,
}: ButtonProps) {
  return (
    <button
      aria-busy={loading ? true : undefined}
      className={classNames(
        'bs-button',
        `bs-button--${variant}`,
        `bs-button--${size}`,
        fullWidth ? 'bs-button--full-width' : null,
        className,
      )}
      disabled={disabled || loading}
      onClick={onClick}
      type={type}
    >
      {loading ? <LoadingIndicator decorative /> : null}
      <span className="bs-button__label">{children}</span>
    </button>
  );
}
