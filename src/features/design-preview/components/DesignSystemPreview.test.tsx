/*
 * The component layer's half of TOOL-5 for the placeholder screen of task 0.21
 * (TOOL-5's first two scenarios are the baselines themselves, and this spec runs
 * at both of them through `describeAtBothBaselines`).
 *
 * What belongs here and what does not is the whole point of this file, so it is
 * stated rather than implied. The screen renders **both** navigation
 * arrangements at every width — `Sidebar` and `TabBar` from one destinations
 * array — and the stylesheet decides which of them is laid out. jsdom cannot
 * answer that question: its media evaluation matches plain media **types** only,
 * so no `min-width` query in `src/styles/**` ever applies, and the app's
 * stylesheet is not loaded in a Vitest run at all (`css` is off in
 * `vitest.config.ts`; `src/styles/index.css` is imported only by
 * `src/main.tsx`). Both landmarks are therefore in the DOM, and
 * `getAllByRole('navigation', { name: 'Primary' })` returns two — which this
 * spec asserts as the reason the arrangement is asserted in Playwright
 * (`e2e/adaptive-navigation.spec.ts`) instead of being faked here
 * (`src/components/ui/navigation.ts`, `src/test/setup.ts`).
 *
 * So this file asserts only what the DOM itself says: the surface composes, the
 * primitives carry accessible names and states, the live controls respond, and
 * both arrangements exist. No geometry (`getBoundingClientRect()` returns zeroes
 * under jsdom), no density, no pointer behaviour, no arrangement.
 *
 * The screen is reached through the app's own route tree — `createTestRouter`
 * over `src/routes/design-preview.tsx` — rather than by rendering the component
 * in isolation, because the screen's links and the nav bar's actions need a
 * router in context and a spec that rendered it bare would have to invent one.
 * The route is `/design-preview` since task 3.3 gave `/` to the household entry;
 * the preview is the scaffold's own surface and keeps its own path.
 */
import { RouterProvider } from '@tanstack/react-router';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it } from 'vitest';
import type { NavigationArrangement } from '../../../components/ui/navigation';
import {
  COMPONENT_BASELINES,
  createTestRouter,
  currentComponentViewport,
  describeAtBothBaselines,
} from '../../../test/setup';

const START_TITLE = 'Design system';
const SKIP = 'Skip to content';
const UNIT = 'Weight unit';
const BACK = 'Back';
const SAVE = 'Save';
const NOTICE = 'Settings are not here yet';
const REMINDERS = /Weight reminders/;

/** The attribute `TabBar` and `Sidebar` each tag their landmark with. */
const ARRANGEMENT_ATTR = 'data-navigation-arrangement';

/** The seven section headings `PreviewSection` renders, in order. */
const PREVIEW_SECTIONS = [
  'Typography',
  'Buttons',
  'Form fields',
  'Lists',
  'Grouping',
  'Status',
  'States',
];

async function renderPreviewScreen(): Promise<void> {
  const router = createTestRouter({ initialEntries: ['/design-preview'] });

  /*
   * Loaded before the first render, and the render is then awaited through a
   * query: the router's own `Transitioner` loads on mount, and letting it do so
   * inside `act` is what keeps the assertions below synchronous rather than
   * racing a load nobody waited for.
   */
  await router.load();
  render(<RouterProvider router={router} />);
  await screen.findByRole('heading', { name: START_TITLE });
}

/*
 * One arrangement's landmark. Scoped by `data-navigation-arrangement`, never by
 * a class name: the attribute is a constant per primitive and says *which*
 * landmark this is, not which one the stylesheet is laying out — the distinction
 * `src/components/ui/navigation.ts` is at pains to make, and the reason this
 * helper can be used here without pretending to know the arrangement.
 */
function navigationLandmark(arrangement: NavigationArrangement): HTMLElement {
  const landmarks = screen.getAllByRole('navigation');
  const landmark = landmarks.find(
    (candidate) => candidate.getAttribute(ARRANGEMENT_ATTR) === arrangement,
  );

  if (landmark === undefined) {
    throw new Error(`no ${arrangement} navigation landmark was rendered`);
  }

  return landmark;
}

describeAtBothBaselines('DesignSystemPreview', (baseline) => {
  it(`TOOL-5: Component tests exercise the ${baseline} baseline`, () => {
    const expected = COMPONENT_BASELINES[baseline];

    expect(currentComponentViewport()).toEqual(expected);
  });

  it('Renders the navigation bar and every section', async () => {
    await renderPreviewScreen();

    const title = screen.getByRole('heading', { name: START_TITLE });

    expect(title).toBeVisible();

    for (const section of PREVIEW_SECTIONS) {
      const heading = screen.getByRole('heading', { name: section });

      expect(heading).toBeVisible();
    }
  });

  it('Renders both arrangements from one destinations array', async () => {
    await renderPreviewScreen();

    const tabs = navigationLandmark('tabs');
    const sidebar = navigationLandmark('sidebar');
    const landmarks = [tabs, sidebar];

    for (const landmark of landmarks) {
      const destination = within(landmark).getByRole('link', {
        name: START_TITLE,
      });

      expect(destination).toBeVisible();
    }

    /*
     * Two landmarks, not one: the `display: none` that removes the hidden
     * arrangement from a browser's accessibility tree never applies here, so the
     * count is the honest sum of the DOM rather than a claim about layout.
     */
    expect(screen.getAllByRole('navigation', { name: 'Primary' })).toHaveLength(
      2,
    );
  });

  it('Exposes the primitives with their names and states', async () => {
    await renderPreviewScreen();

    const skip = screen.getByRole('link', { name: SKIP });
    const group = screen.getByRole('radiogroup', { name: UNIT });
    const kilograms = within(group).getByRole('radio', { name: 'kg' });
    const pounds = within(group).getByRole('radio', { name: 'lb' });
    const back = screen.getByRole('button', { name: BACK });
    const save = screen.getByRole('button', { name: SAVE });
    const skipTarget = skip.getAttribute('href')?.replace(/^#/, '') ?? '';
    const skipTargetElement = document.getElementById(skipTarget);
    const focusable = [
      ...document.querySelectorAll<HTMLElement>(
        'a[href], button, input, select, textarea, [tabindex]',
      ),
    ].filter(
      (element) =>
        !element.hasAttribute('disabled') &&
        element.getAttribute('tabindex') !== '-1',
    );

    /*
     * `.bs-skip-link` is translated off-screen until `:focus-visible` in the
     * app stylesheet, so `toBeVisible()` here would pass only because that
     * stylesheet is absent from a Vitest run. Assert the facts the DOM does
     * carry instead: the link targets a real element (the `main` it skips to)
     * and it is the first focusable element in document order.
     */
    expect(skipTarget).not.toBe('');
    expect(skipTargetElement).toBe(screen.getByRole('main'));
    expect(focusable[0]).toBe(skip);
    expect(kilograms).toBeChecked();
    expect(pounds).not.toBeChecked();
    expect(back).toBeDisabled();
    expect(save).toHaveAttribute('aria-busy', 'true');
    expect(save).toBeDisabled();
  });

  it('Responds to a live control', async () => {
    await renderPreviewScreen();

    const user = userEvent.setup();
    const row = screen.getByRole('button', { name: REMINDERS });

    expect(row).toHaveAttribute('aria-current', 'true');

    await user.click(row);

    expect(row).not.toHaveAttribute('aria-current', 'true');
  });

  it('Opens the navigation bar action without a service', async () => {
    await renderPreviewScreen();

    const user = userEvent.setup();
    const actions = screen.getAllByRole('button', { name: 'Settings' });
    const gear = actions.find((item) => item.hasAttribute('aria-pressed'));

    expect(screen.queryByText(NOTICE)).toBeNull();

    if (gear === undefined) {
      throw new Error('the navigation bar settings action was not rendered');
    }

    await user.click(gear);

    expect(screen.getByText(NOTICE)).toBeVisible();
  });
});
