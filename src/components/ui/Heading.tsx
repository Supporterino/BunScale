import type { ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `Heading` — screen and section titles.
 *
 * Only `h1`–`h3` are offered: the outline of this app is shallow by design
 * (one screen title, then section headings), and offering `h4`–`h6` would
 * invite a level that no screen has. Levels must not skip, and a screen has
 * exactly one `h1` — the navigation bar's title, which renders its own `h1` for
 * that reason. A `ListSection` header is a real `h2` here, not a styled `div`,
 * so VoiceOver's heading rotor can jump between sections.
 *
 * The default `title-2` + `h2` pair is the common case: a section title inside
 * a `Screen` whose `h1` is in the navigation bar.
 */

export type HeadingVariant =
  | 'large-title'
  | 'title-1'
  | 'title-2'
  | 'title-3'
  | 'section-header';

export type HeadingLevel = 'h1' | 'h2' | 'h3';

export type HeadingProps = {
  children: ReactNode;
  variant?: HeadingVariant;
  as?: HeadingLevel;
  className?: string;
  id?: string;
};

export function Heading({
  children,
  variant = 'title-2',
  as: Level = 'h2',
  className,
  id,
}: HeadingProps) {
  return (
    <Level
      className={classNames('bs-heading', `bs-type--${variant}`, className)}
      id={id}
    >
      {children}
    </Level>
  );
}
