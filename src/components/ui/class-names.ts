/*
 * Class-name composition for the design-system primitives.
 *
 * There is no CSS-in-JS and no atomic CSS utility in this app (rule 6 and
 * TOOL-8: no new dependency), so a primitive's appearance comes from a class in
 * `src/styles/ui.css` and its per-instance variation comes from a small set of
 * modifier classes. This helper joins those classes and drops the falsy ones,
 * which is all a primitive needs; it does not sort, dedupe or parse.
 *
 * It is deliberately not exported from a barrel: a barrel would add an import
 * graph edge with no behaviour, and every consumer imports the primitive it
 * uses directly, so the bundler keeps route-level code splitting honest.
 */
export type ClassName = string | false | null | undefined;

export function classNames(...values: ClassName[]): string {
  return values.filter((value) => typeof value === 'string').join(' ');
}
