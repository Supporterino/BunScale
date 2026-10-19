/*
 * TOOL-6's token invariants (task 0.24).
 *
 * Task 0.20 measured these with a throwaway script that no longer exists, and
 * nothing at build time guards them. This is the assertion that matters most in
 * this task for a reason worth stating: **Vite does not resolve custom
 * properties**. A misspelt `var(--colour-primary)` is emitted verbatim, the
 * build stays green and the browser silently falls back to nothing. No other
 * gate command would catch it.
 *
 * Four invariants are checked over every `src/styles/*.css` file:
 *
 *   1. every `var()` reference — `var(--x, fallback)` included — names a custom
 *      property that is defined somewhere;
 *   2. no reference cycle;
 *   3. no custom property is defined twice **in one cascade context**. This is
 *      the one that has to understand `semantic.css`: it deliberately redefines
 *      ~47 names inside `prefers-color-scheme: dark`,
 *      `prefers-reduced-motion: reduce`, `@supports` and the two breakpoint
 *      blocks. A naive first-definition-wins check would fail on the very file
 *      it guards, so the context is (file, enclosing at-rule chain, selector);
 *   4. the documented AA pairs still hold, computed from the resolved hex
 *      values in both themes, with the pair list taken from the design system's
 *      own record rather than invented here.
 *
 * The checkers are pure functions over parsed text; `css-invariants.ts` states
 * the parsing limits (no CSS nesting, no dynamic values, `var()` aliases only).
 */
import { readdirSync, readFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  checkTokenInvariants,
  contrastRatio,
  createThemeResolver,
  crossContextRedefinitions,
  parseMeasuredContrastPairs,
  parseStylesheet,
  type Stylesheet,
  type Theme,
} from './css-invariants';
import { REPOSITORY_ROOT, readRepositoryFile } from './scaffold-files';

const STYLE_DIRECTORY = resolve(REPOSITORY_ROOT, 'src/styles');
const DESIGN_SYSTEM_RECORD = '.opencode/skills/design-system/SKILL.md';

/**
 * Reads every stylesheet, **in cascade order**: the order `index.css` imports
 * them, then any file it does not import (so nothing is left unchecked). The
 * theme resolver needs that order — the dark block must be applied after the
 * base rules, exactly as the browser does.
 */
function loadStylesheets(): Stylesheet[] {
  const entry = readFileSync(resolve(STYLE_DIRECTORY, 'index.css'), 'utf8');
  const imported = [...entry.matchAll(/@import\s+['"]([^'"]+)['"]/g)].map(
    (match) => resolve(STYLE_DIRECTORY, match[1]),
  );
  const all = readdirSync(STYLE_DIRECTORY)
    .filter((file) => file.endsWith('.css'))
    .map((file) => resolve(STYLE_DIRECTORY, file));
  const cascade = [
    ...imported.filter((path) => all.includes(path)),
    ...all.filter((path) => !imported.includes(path)),
  ];

  return cascade.map((path) =>
    parseStylesheet(basename(path), readFileSync(path, 'utf8')),
  );
}

const stylesheets = loadStylesheets();

const THEMES: readonly Theme[] = ['light', 'dark'];

describe('TOOL-6: the design system’s token invariants', () => {
  it('Every var() reference resolves to a defined custom property', () => {
    const issues = checkTokenInvariants(stylesheets).filter((issue) =>
      issue.includes('references var('),
    );

    expect(
      issues,
      'a reference names a custom property no stylesheet defines; Vite emits it verbatim and the browser falls back silently',
    ).toEqual([]);
  });

  it('No custom property reference cycle', () => {
    const issues = checkTokenInvariants(stylesheets).filter((issue) =>
      issue.includes('reference cycle'),
    );

    expect(issues).toEqual([]);
  });

  it('No custom property is defined twice in the same cascade context', () => {
    const issues = checkTokenInvariants(stylesheets).filter((issue) =>
      issue.includes('same cascade context'),
    );

    expect(
      issues,
      'a genuine duplicate is two definitions of one name in one block (or in two blocks with the same file, conditions and selector)',
    ).toEqual([]);
  });

  it('The intended theme, motion and breakpoint overrides are not duplicates', () => {
    const redefinitions = crossContextRedefinitions(stylesheets);

    // The positive half of the check above: these names *are* defined more than
    // once, deliberately, in different contexts. If the parser collapsed
    // contexts they would have failed the duplicate test; asserting they are
    // seen keeps that test from passing vacuously over a single-context file.
    for (const name of [
      '--color-background',
      '--duration-fast',
      '--spacing-inline',
      '--layout-viewport-height',
    ]) {
      expect(
        redefinitions.has(name),
        `${name} should be redefined in a second context (dark theme, reduced motion, a breakpoint or @supports); the duplicate check cannot distinguish contexts if it is not`,
      ).toBe(true);
    }

    // Today 46 names are redefined across contexts (the record says "around
    // 47"). The floor is not a target: it makes a parser that lost the
    // at-rule chain fail here rather than pass the duplicate check by accident.
    expect(redefinitions.size).toBeGreaterThanOrEqual(40);
  });

  it('The documented AA pairs still hold in both themes', () => {
    const pairs = parseMeasuredContrastPairs(
      readRepositoryFile(DESIGN_SYSTEM_RECORD),
    );

    expect(
      pairs,
      'the design-system record’s "Measured AA pairs" table could not be read, so this test would pass vacuously',
    ).toHaveLength(25);

    const failures: string[] = [];

    for (const pair of pairs) {
      for (const theme of THEMES) {
        const resolveToken = createThemeResolver(stylesheets, theme);
        const foreground = resolveToken(pair.foreground);
        const background = resolveToken(pair.background);
        const ratio = contrastRatio(foreground.colour, background.colour);
        const documented = theme === 'light' ? pair.light : pair.dark;
        const label = `${theme}: ${pair.foreground} on ${pair.background}`;

        if (ratio < pair.minimum) {
          failures.push(
            `${label} is ${ratio.toFixed(2)}:1, below the required ${pair.minimum}:1`,
          );
        } else if (Math.abs(ratio - documented) > 0.02) {
          failures.push(
            `${label} is ${ratio.toFixed(2)}:1 but the record documents ${documented}:1; update the palette or the record`,
          );
        }
      }
    }

    expect(failures).toEqual([]);
  });

  it('The two tightest text pairs named in the record still hold', () => {
    const pairs = parseMeasuredContrastPairs(
      readRepositoryFile(DESIGN_SYSTEM_RECORD),
    );
    const tight = [
      {
        foreground: '--color-primary',
        background: '--color-primary-subtle-pressed',
      },
      { foreground: '--color-primary', background: '--color-surface-sunken' },
    ] as const;

    // Named separately from the loop above so a palette regression fails with
    // the pair the palette was chosen around, not with a row number.
    for (const required of tight) {
      const pair = pairs.find(
        (candidate) =>
          candidate.foreground === required.foreground &&
          candidate.background === required.background,
      );

      expect(
        pair,
        `${required.foreground} on ${required.background} is no longer documented in the AA table`,
      ).toBeDefined();

      for (const theme of THEMES) {
        const resolveToken = createThemeResolver(stylesheets, theme);
        const ratio = contrastRatio(
          resolveToken(required.foreground).colour,
          resolveToken(required.background).colour,
        );

        expect(
          ratio,
          `${theme}: ${required.foreground} on ${required.background} must stay at or above 4.5:1`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('The design-system status is recorded', () => {
    const record = readRepositoryFile(DESIGN_SYSTEM_RECORD);
    const frontmatter = /^---\n([\s\S]*?)\n---/.exec(record)?.[1] ?? '';

    // TOOL-6's own scenario: the machine-readable line the orchestrator's UI
    // precondition reads. It must be in the frontmatter, not in prose below it.
    expect(frontmatter).toMatch(/^status:\s*frozen$/m);
  });
});
