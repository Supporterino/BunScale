import type { ReactNode } from 'react';
import { classNames } from './class-names';
import type { NavigationDestination } from './navigation';
import { Sidebar } from './Sidebar';
import { TabBar } from './TabBar';

/*
 * `AppShell` — the app frame: one `main`, one navigation landmark and the skip
 * link that gets a keyboard user past the navigation.
 *
 * The adaptive arrangement, and why it is CSS rather than JavaScript. One route
 * tree renders for every form factor; the shell renders BOTH arrangements from
 * the same `destinations` array and each stylesheet band lays out exactly one of
 * them:
 *
 *   - compact (base, 0-767 px): the sidebar is `display: none`, the tab bar is
 *     the bottom bar, and `main` reserves the tab bar's height so nothing ends
 *     up under it;
 *   - medium and expanded (768 px and up): the tab bar is `display: none`, the
 *     sidebar becomes a 260 pt column and `main` becomes the scrolling region
 *     beside it.
 *
 * `display: none` removes the hidden landmark from the accessibility tree and
 * from the focus order *in a browser*, so a screen reader user and a keyboard
 * user each meet exactly one arrangement at any width, while the DOM holds both.
 * That is a rendering fact, not a DOM fact — and the reason it cannot be read
 * in jsdom (the Vitest component layer) is *not* that jsdom has no CSS
 * pipeline: the pinned jsdom 30.1.3 has one. Two stronger reasons hold, and
 * `src/test/setup.ts` states both in detail: (1) jsdom's media evaluation
 * matches plain media **types** only (`all`, `screen`), so every feature query
 * this stylesheet uses — `min-width`, `prefers-color-scheme`,
 * `hover`/`pointer`, `prefers-reduced-motion` — never matches, which makes even
 * a `display` read return the base rule at both baselines; and (2) the app's
 * stylesheet is not loaded in a Vitest run at all (`css` is off in
 * `vitest.config.ts`; `src/styles/index.css` is imported only by
 * `src/main.tsx`). Both landmarks are therefore in the DOM and a role query
 * finds both. The arrangement is asserted in E2E, and a component spec scopes
 * by the `data-navigation-arrangement` attribute or `within(...)` instead of by
 * the landmark's role name — see `./navigation.ts`, which states the same rule.
 *
 * `data-navigation-arrangement` on each landmark is a constant that identifies
 * which landmark is which (see `./navigation.ts`); it is not a report of which
 * one the stylesheet laid out.
 *
 * The rejected alternative is a `matchMedia` hook that renders one arrangement
 * instead of both: it would put a second copy of the 768 px breakpoint in
 * JavaScript, would flash the wrong arrangement on the first frame, and
 * `frontend-conventions` asks for CSS adaptation for exactly that reason.
 *
 * The skip link is the first focusable element on the screen and is visible only
 * while focused; it targets `main`, which takes `tabIndex={-1}` so the focus
 * actually moves there rather than only scrolling (a negative index is not a
 * tab stop, so it does not add one to the tab order).
 */

export type AppShellProps = {
  children: ReactNode;
  /** The same destinations feed both arrangements. */
  destinations: NavigationDestination[];
  /** Accessible name of the navigation landmark. Defaults to `Primary`. */
  navigationLabel?: string;
  /** `id` of the `main` region, and the skip link's target. */
  mainId?: string;
  className?: string;
};

export function AppShell({
  children,
  destinations,
  navigationLabel = 'Primary',
  mainId = 'bs-main',
  className,
}: AppShellProps) {
  return (
    <div className={classNames('bs-app-shell', className)}>
      <a className="bs-skip-link" href={`#${mainId}`}>
        Skip to content
      </a>
      <Sidebar
        className="bs-app-shell__sidebar"
        destinations={destinations}
        label={navigationLabel}
      />
      <main className="bs-app-shell__main" id={mainId} tabIndex={-1}>
        {children}
      </main>
      <TabBar
        className="bs-app-shell__tab-bar"
        destinations={destinations}
        label={navigationLabel}
      />
    </div>
  );
}
