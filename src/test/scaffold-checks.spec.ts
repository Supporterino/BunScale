/*
 * The negative half of task 0.24's checks.
 *
 * This file covers four checkers, each against the drift it exists to catch:
 * `checkNpmPins`/`collectNpmPins`, `checkCargoPins`/`collectCargoPins`,
 * `checkTokenInvariants` and `checkConfigAgreement`. The other exported helpers
 * are not exercised here: `parseDesignPinTables` is covered positively in
 * `exact-pins.spec.ts`, and `crossContextRedefinitions` and
 * `parseMeasuredContrastPairs` in `design-tokens.spec.ts`.
 *
 * A green suite that cannot fail is the same problem as a lint configuration
 * that matches nothing, so each checker above is also exercised against the
 * drift — or the unreadable input — it exists to catch. These are in-memory
 * fixtures: they never touch the repository, they are fast, and they keep
 * proving the failure paths long after the one-off perturbation run that
 * produced this task's evidence is forgotten.
 */
import { describe, expect, it } from 'vitest';
import { checkConfigAgreement } from './config-agreement';
import {
  checkTokenInvariants,
  contrastRatio,
  parseColourLiteral,
  parseStylesheet,
  type Rgb,
} from './css-invariants';
import {
  checkCargoPins,
  checkNpmPins,
  collectCargoPins,
  collectNpmPins,
} from './manifest-pins';

function rgb(value: string): Rgb {
  const colour = parseColourLiteral(value);

  if (colour === undefined) {
    throw new Error(`fixture colour "${value}" did not parse`);
  }

  return colour;
}

describe('the scaffold checks fail on the drift they exist to catch', () => {
  it('The npm pin check flags an inexact range, a missing pin and an extra pin', () => {
    const expected = new Map([
      ['react', '19.3.1'],
      ['vite', '8.3.4'],
    ]);

    const issues = checkNpmPins(
      expected,
      collectNpmPins(
        JSON.stringify({
          dependencies: { react: '^19.3.1' },
          devDependencies: { '@tanstack/react-query': '5.104.3' },
        }),
      ),
    );
    const message = issues.join('\n');

    expect(message).toContain('"react" is "^19.3.1"');
    expect(message).toContain('not an exact version');
    expect(message).toContain('D0.8 pins "vite" but the manifest does not');
    expect(message).toContain('"@tanstack/react-query" is not in D0.8');
  });

  it('The npm pin check flags a pin that differs from D0.8 and a duplicate declaration', () => {
    const expected = new Map([['react', '19.3.1']]);
    const issues = checkNpmPins(
      expected,
      collectNpmPins(
        JSON.stringify({
          dependencies: { react: '19.3.0' },
          devDependencies: { react: '19.3.1' },
        }),
      ),
    );
    const message = issues.join('\n');

    expect(message).toContain('"react" is "19.3.0" but D0.8 pins "19.3.1"');
    expect(message).toContain(
      '"react" is declared twice, in dependencies and devDependencies',
    );
  });

  it('The cargo pin check flags a non-= version, a missing pin and an unreadable version', () => {
    const expected = new Map([
      ['serde', '=1.0.229'],
      ['tauri', '=2.12.4'],
    ]);
    const issues = checkCargoPins(
      expected,
      collectCargoPins(
        [
          '[dependencies]',
          'serde = { version = "^1.0.229", features = ["derive"] }',
          'specta = "=2.0.0-rc.25"',
        ].join('\n'),
      ),
    );
    const message = issues.join('\n');

    expect(message).toContain('does not begin with "="');
    expect(message).toContain('D0.8 pins "tauri" but the manifest does not');
    expect(message).toContain('"specta" is not in D0.8');

    expect(() =>
      collectCargoPins(
        ['[dependencies]', 'tauri = { workspace = true }'].join('\n'),
      ),
    ).toThrow(/without a literal version/);
  });

  it('The cargo pin check flags a pin that differs from D0.8 and a duplicate declaration', () => {
    const expected = new Map([['serde', '=1.0.229']]);
    const issues = checkCargoPins(
      expected,
      collectCargoPins(
        [
          '[dependencies]',
          'serde = "=1.0.100"',
          '[dev-dependencies]',
          'serde = "=1.0.100"',
        ].join('\n'),
      ),
    );
    const message = issues.join('\n');

    expect(message).toContain('"serde" is "=1.0.100" but D0.8 pins "=1.0.229"');
    expect(message).toContain(
      '"serde" is declared twice, in dependencies and dev-dependencies',
    );
  });

  it('The cargo parser rejects a sub-table with no literal version at EOF', () => {
    const toml = [
      '[dependencies]',
      '[dependencies.some-crate]',
      'git = "https://example.invalid/some-crate"',
    ].join('\n');

    expect(() => collectCargoPins(toml)).toThrow(
      'Cargo.toml:2 declares "[dependencies.some-crate]" without a literal version',
    );
  });

  it('The cargo parser rejects a sub-table with no literal version at the next header', () => {
    expect(() =>
      collectCargoPins(
        [
          '[dependencies.foo]',
          'git = "https://example.invalid/foo"',
          '[dependencies.serde]',
          'version = "=1.0.229"',
        ].join('\n'),
      ),
    ).toThrow(/declares "\[dependencies\.foo\]" without a literal version/);
  });

  it('The cargo parser keeps a sub-table in scope, so a field after version is not read as a top-level entry', () => {
    const pins = collectCargoPins(
      [
        '[dependencies]',
        '[dependencies.serde]',
        'version = "=1.0.229"',
        'features = ["derive"]',
      ].join('\n'),
    );

    expect(pins).toEqual([
      { name: 'serde', version: '=1.0.229', section: 'dependencies' },
    ]);
  });

  it('The token check flags an unresolved reference, a same-context duplicate and a cycle', () => {
    const sheet = parseStylesheet(
      'fixture.css',
      [
        ':root {',
        '  --a: 1px;',
        '  --a: 2px;',
        '  --loop-a: var(--loop-b);',
        '  --loop-b: var(--loop-a);',
        '  color: var(--missing, red);',
        '}',
      ].join('\n'),
    );
    const issues = checkTokenInvariants([sheet]);
    const message = issues.join('\n');

    expect(message).toContain('references var(--missing)');
    expect(message).toContain(
      '--a is defined 2 times in the same cascade context',
    );
    expect(message).toContain(
      'reference cycle: --loop-a -> --loop-b -> --loop-a',
    );
  });

  it('The token check does not flag the intended theme overrides as duplicates', () => {
    const sheet = parseStylesheet(
      'fixture.css',
      [
        ':root { --surface: #ffffff; }',
        '@media (prefers-color-scheme: dark) {',
        '  :root { --surface: #000000; }',
        '}',
        '@media (min-width: 768px) {',
        '  :root { --surface: #fafaf9; }',
        '}',
      ].join('\n'),
    );

    expect(checkTokenInvariants([sheet])).toEqual([]);
  });

  it('The contrast check separates a passing pair from a failing one', () => {
    expect(contrastRatio(rgb('#000000'), rgb('#ffffff'))).toBeCloseTo(21, 1);
    expect(contrastRatio(rgb('#777777'), rgb('#ffffff'))).toBeLessThan(4.5);
    expect(
      contrastRatio(rgb('#000000'), rgb('#ffffff')),
    ).toBeGreaterThanOrEqual(4.5);
  });

  it('The config agreement check flags a port, a host and an outDir mismatch', () => {
    const tauri = JSON.stringify({
      build: { devUrl: 'http://localhost:1420', frontendDist: '../dist' },
    });
    const vite = [
      'const host = process.env.TAURI_DEV_HOST;',
      'export default {',
      '  server: { port: 9999, strictPort: true },',
      "  build: { outDir: 'build' },",
      '};',
    ].join('\n');
    const issues = checkConfigAgreement(tauri, vite).join('\n');

    expect(issues).toContain('server.port is 9999');
    expect(issues).toContain('server.host is not wired');
    expect(issues).toContain('build.outDir resolves to');
  });

  it('The config agreement check refuses a vite config with a template literal', () => {
    const tauri = JSON.stringify({
      build: { devUrl: 'http://localhost:1420', frontendDist: '../dist' },
    });
    // A backtick is neither comment-stripped nor string-tracked by the reader;
    // `//` and `}` inside one would be mis-read, so it must be refused.
    const vite = [
      'const note = `// not a comment, } not a brace`;',
      'export default {',
      '  server: { port: 1420, host: note },',
      "  build: { outDir: 'dist' },",
      '};',
    ].join('\n');

    expect(() => checkConfigAgreement(tauri, vite)).toThrow(/backtick/);
  });
});
