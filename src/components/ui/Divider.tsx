import { classNames } from './class-names';

/*
 * `Divider` — a hairline rule inside a card or between sections.
 *
 * It is a real `<hr>`, which is a separator in the accessibility tree and needs
 * no role of its own. It deliberately does not declare `aria-hidden`: the lint
 * rule for `aria-hidden` on an element the browser can focus refuses it on
 * `<hr>`, and the honest reading of that is that a divider is not hidden — it is
 * simply not load-bearing. Grouping and hierarchy are carried by headings,
 * spacing and surfaces, so nothing is lost when a screen reader skips it, and
 * nothing is lost either when a reader is told there is a separator.
 *
 * A divider that means something (a resize handle, a section boundary a user has
 * to know about) is a different primitive, and none of those exists in P0.
 */

export type DividerProps = {
  className?: string;
};

export function Divider({ className }: DividerProps) {
  return <hr className={classNames('bs-divider', className)} />;
}
