import type { ElementType, ReactNode } from 'react';
import { classNames } from './class-names';

/*
 * `Text` — every piece of copy that is not owned by a component with its own
 * type tokens (the navigation bar title, a list row, a notice and the two state
 * surfaces read their `--<component>-*` sizes instead).
 *
 * The type roles come from the semantic layer: `--text-<style>-size` is
 * `calc(rem x --text-scale)`, so a preferred text size scales every role
 * without a component setting a font size of its own, and the line heights are
 * unitless so they follow the same scale.
 *
 * `as` deliberately defaults to `p` — copy is a paragraph unless the caller
 * says otherwise — and there is NO heading semantics here: a screen's headings
 * are `Heading`, which is what keeps the outline honest. Inside a control
 * (a `ListRow` title, a tab label) pass `as="span"`; a `<p>` inside a
 * `<button>` is not valid phrasing content.
 */

export type TextVariant =
  | 'large-title'
  | 'title-1'
  | 'title-2'
  | 'title-3'
  | 'headline'
  | 'body'
  | 'callout'
  | 'subheadline'
  | 'footnote'
  | 'caption-1'
  | 'caption-2';

/*
 * Tone is never the only signal (`Badge`, `Notice` and the state surfaces all
 * carry text as well); it is a supplementary emphasis, which is why the default
 * is the full-contrast `primary`.
 */
export type TextTone = 'primary' | 'secondary' | 'tertiary' | 'danger';

export type TextProps = {
  children: ReactNode;
  /** Text style from the semantic layer. Defaults to `body`. */
  variant?: TextVariant;
  /** Colour role. Defaults to `primary` (`--color-text`). */
  tone?: TextTone;
  /** Element to render; defaults to `p`. */
  as?: ElementType;
  className?: string;
  id?: string;
};

export function Text({
  children,
  variant = 'body',
  tone = 'primary',
  as: Element = 'p',
  className,
  id,
}: TextProps) {
  return (
    <Element
      className={classNames(
        'bs-text',
        `bs-type--${variant}`,
        `bs-text--${tone}`,
        className,
      )}
      id={id}
    >
      {children}
    </Element>
  );
}
