import type { LinkProps } from '@tanstack/react-router';
import type { ReactNode } from 'react';

/*
 * The one destination shape both navigation arrangements render.
 *
 * `to` is typed as the router's own path union (`LinkProps['to']`), not
 * `string`, so a destination cannot be a string-built URL: the type widens by
 * itself as routes are added to `src/routes/`, and a destination pointing at a
 * route that does not exist is a compile error rather than a 404 in the
 * webview (`tanstack-router-conventions`, "Routes").
 *
 * `icon` is a caller-supplied glyph. It is rendered `aria-hidden`, because the
 * visible label is the item's accessible name; an icon-only destination would
 * be an unlabelled control (`IconButton` exists for that case and requires its
 * label).
 */
export type NavigationDestination = {
  /** A route path from the generated tree. */
  to: LinkProps['to'];
  /** Visible label, and the item's accessible name. */
  label: string;
  /** Decorative glyph; the caller owns its `<svg aria-hidden="true">`. */
  icon: ReactNode;
};

/*
 * Which arrangement a navigation landmark *is*, in the DOM. The two arrangements
 * render the same destinations through different primitives, and only one of them
 * is laid out at any width (`src/styles/ui.css`): the bottom tab bar is
 * compact-only, the sidebar is medium-and-up.
 *
 * What the attribute does and does not say: it is a **constant per primitive**
 * (`'tabs'` on `TabBar`, `'sidebar'` on `Sidebar`), so it identifies which
 * landmark is which — it does not report which one the stylesheet is laying out
 * at the current width. Both landmarks are in the DOM at every width; `display:
 * none` is what removes the other one from the accessibility tree in a browser.
 *
 * That difference matters for where the arrangement can be asserted:
 *
 *   - **E2E (Playwright) owns it.** The browser applies the stylesheet, so the
 *     visible landmark can be read at 390×844 and 1280×800 — exactly one
 *     `navigation` landmark named "Primary" is exposed, and
 *     `data-navigation-arrangement` says which one it is without a class-name
 *     dependency;
 *   - a **component spec** (Vitest, jsdom) cannot assert it, though *not*
 *     because jsdom lacks a CSS pipeline: the pinned jsdom 30.1.3 has one.
 *     jsdom's media evaluation matches plain media **types** only (`all`,
 *     `screen`), so no feature query this stylesheet uses (`min-width`,
 *     `prefers-color-scheme`, `hover`/`pointer`, `prefers-reduced-motion`)
 *     ever matches and the `display: none` never applies — and the app's
 *     stylesheet is not loaded in a Vitest run at all (`css` is off in
 *     `vitest.config.ts`; `src/styles/index.css` is imported only by
 *     `src/main.tsx`). So **both** landmarks are in the DOM:
 *     `getByRole('navigation', { name: 'Primary' })` matches two elements and
 *     cannot say which is laid out. A component spec must scope by this
 *     attribute (or `within(...)`) and only ever assert what the DOM itself
 *     says — `src/test/setup.ts` states the measured detail.
 */
export type NavigationArrangement = 'tabs' | 'sidebar';
