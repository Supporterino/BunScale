import type { ReactNode } from 'react';
import { useId } from 'react';
import { classNames } from './class-names';
import { Heading } from './Heading';

/*
 * `Card` — a grouped surface: the dashboard summary, a settings group.
 *
 * A card separates content with a token-defined surface and border rather than
 * with a floating website panel (`--card-*` in `src/styles/components.css`:
 * surface, hairline border, 16 pt radius, one shadow). It is not a link: if the
 * whole card is one control, the caller wraps it in the control, because a card
 * that is partly a link and partly text is the classic ambiguous affordance.
 *
 * With a heading it is a named `<section>` — a region a screen reader can jump
 * to and announce by name; without one it is an unnamed group and adds no
 * landmark to the rotor.
 */

export type CardProps = {
  children: ReactNode;
  /** Card title. When present, the card is labelled by it. */
  heading?: string;
  /** Heading level; the default follows the screen's `h1`. */
  headingLevel?: 'h2' | 'h3';
  className?: string;
};

export function Card({
  children,
  heading,
  headingLevel = 'h2',
  className,
}: CardProps) {
  const headingId = useId();

  return (
    <section
      aria-labelledby={heading === undefined ? undefined : headingId}
      className={classNames('bs-card', className)}
    >
      {heading === undefined ? null : (
        <Heading
          as={headingLevel}
          className="bs-card__heading"
          id={headingId}
          variant="title-3"
        >
          {heading}
        </Heading>
      )}
      {children}
    </section>
  );
}
