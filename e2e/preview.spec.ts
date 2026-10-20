/*
 * The placeholder surface of task 0.21, driven as a smoke test rather than as a
 * render check: this file asserts that the screen composes (every section is
 * there, under the navigation bar's `h1`) **and** that the primitives on it are
 * live — the split TOOL-5 asks for, with the arrangement half asserted in
 * `./adaptive-navigation.spec.ts` where the viewport is a real browser viewport.
 *
 * The screen is `src/features/design-preview/components/DesignSystemPreview.tsx`
 * and it is the only styled surface in the app until the P0 screens land, so it
 * is what the E2E layer can honestly exercise today. It holds no IPC, no query
 * and no router logic of its own; the interactions below are local state, which
 * is exactly why they are a *smoke* test: they prove the app boots, mounts and
 * responds in WebKit with the IPC layer mocked, and nothing more.
 *
 * The stretched, unconcerned list of assertions is deliberate. A section list
 * that were shorter would pass on a screen whose `Lists` or `States` section had
 * failed to compose; the seven headings are the screen's own structure, and a
 * failure names the section that is missing.
 */
import { expect, test } from './fixtures';

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

/*
 * The preview's own route, stated by the spec rather than inherited from the
 * fixture's start path: `/` is the household entry guard since task 3.3, so the
 * screen under test is reached at `/#/design-preview` and nowhere else.
 */
test.beforeEach(async ({ page }) => {
  await page.goto('/#/design-preview');
});

test('The preview renders every section', async ({ page }) => {
  const screenTitle = page.getByRole('heading', {
    level: 1,
    name: 'Design system',
  });

  await expect(screenTitle).toBeVisible();

  for (const section of PREVIEW_SECTIONS) {
    const heading = page.getByRole('heading', { name: section });

    await expect(heading).toBeVisible();
  }
});

test('The interactive primitives respond', async ({ page }) => {
  const kilograms = page.getByRole('radio', { name: 'kg' });
  const pounds = page.getByRole('radio', { name: 'lb' });
  const reminders = page.getByRole('button', { name: /Weight reminders/ });

  await expect(kilograms).toBeChecked();

  /*
   * `press` rather than `check`: the radio is the design system's visually
   * hidden 1x1 clip-rect control (`src/styles/ui.css`, `.bs-segmented__input`),
   * so pointer actionability is not what this asserts — the browser's own radio
   * activation, and through it the primitive's `onChange`, is.
   */
  await pounds.press('Space');

  await expect(pounds).toBeChecked();

  /*
   * An actionable `ListRow` is a real `button` whose selected state is exposed
   * as `aria-current`, so the row's state is announced and not only drawn. It
   * flips on a press and comes back on the next one.
   */
  await expect(reminders).toHaveAttribute('aria-current', 'true');

  await reminders.click();

  await expect(reminders).not.toHaveAttribute('aria-current', 'true');

  // The navigation bar's leading action is a disabled `IconButton`: present,
  // named and genuinely unreachable.
  await expect(page.getByRole('button', { name: 'Back' })).toBeDisabled();
});
