/*
 * TOOL-5: "E2E runs in WebKit with mocked IPC" (task 0.25, design.md D0.24,
 * Q23) — the mechanical half of the shared fixture.
 *
 * `e2e/fixtures.ts` is an `{ auto: true }` fixture, so the mock assertion runs
 * before every test whose spec uses the `test` it exports. That does not make a
 * spec use it: `import { test } from '@playwright/test'` typechecks, lints and
 * runs with the fixture silently absent, restoring exactly the unmocked-green
 * state Q23 exists to prevent. This spec is the rule, stated mechanically:
 *
 *   - every `e2e/*.spec.ts` MUST value-import both `test` and `expect` from
 *     `./fixtures`;
 *   - no `e2e/*.spec.ts` may value-import **anything** from `@playwright/test`;
 *   - no **other** file under `e2e/` may match Playwright's default
 *     `testMatch`, so a nested or `*.test.ts` spec fails loudly instead of
 *     being run by Playwright and silently skipped by this guard.
 *
 * A **type-only** import from `@playwright/test` is allowed: `import type {
 * Page }` is erased, selects no runner and cannot bypass the fixture. The same
 * permission extends to an inline `type` specifier. The restriction is on
 * values, because only a value can choose Playwright's runner over this module;
 * forbidding the whole module rather than only `test`/`expect` also closes the
 * namespace shape (`import * as pw from '@playwright/test'`), which would
 * otherwise select Playwright's runner without naming either export.
 *
 * The rule is read from the files rather than inferred from the compiler: the
 * plugin-less Vitest program has no way to say "this spec must use that test
 * object", which is why a spec that forgot the fixture stayed green before this
 * file existed. The file check is deliberately vacuous-proof — the discovery
 * test below asserts at least one spec was found — so renaming `e2e/` cannot
 * turn the guard into a scan over nothing. It also asserts that `fixtures.ts`
 * still exports both names; a re-export accidentally dropped would otherwise
 * make every spec fail at typecheck with no statement of the rule that broke.
 *
 * Limits: the reader parses import declarations from comment-stripped source.
 * It does not evaluate TypeScript, so a dynamic import or a re-export chain is
 * out of scope. Those are named here rather than left implicit; the static form
 * is the one every E2E spec in this repository uses and the one a reviewer can
 * see.
 *
 * Discovery is deliberately narrow (`e2e/*.spec.ts`) and is paired with a
 * recursive complement check: every file under `e2e/` whose basename matches
 * Playwright's default `testMatch` must be one of the discovered specs. A spec
 * in a subdirectory (`e2e/flows/preview.spec.ts`) or a `.test.ts` variant is
 * therefore a loud failure rather than a file Playwright runs unguarded. The
 * root is `e2e/`, which is `playwright.config.ts`'s `testDir`; that agreement is
 * kept by hand. The complement check only sees files beneath this root, so if
 * `testDir` ever moves, this guard must move with it rather than pass over the
 * new directory.
 */
import { readdirSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { REPOSITORY_ROOT, readAbsoluteFile } from './scaffold-files';

const E2E_DIRECTORY = resolve(REPOSITORY_ROOT, 'e2e');
const FIXTURES_MODULE = './fixtures';
const PLAYWRIGHT_MODULE = '@playwright/test';

/**
 * Playwright's default `testMatch`: `**\/*.@(spec|test).?(c|m)[jt]s?(x)`.
 * Matched against the basename, since `**\/` accepts any depth and Playwright
 * runs a matching file wherever it sits beneath `testDir`.
 */
const PLAYWRIGHT_TEST_MATCH = /\.(?:spec|test)\.(?:c|m)?[jt]sx?$/;

/** Every file beneath `directory`, recursively, sorted for stable messages. */
function collectFiles(directory: string): string[] {
  const files: string[] = [];

  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const entryPath = resolve(directory, entry.name);

    if (entry.isDirectory()) {
      files.push(...collectFiles(entryPath));
    } else if (entry.isFile()) {
      files.push(entryPath);
    }
  }

  return files.sort();
}

/** Every E2E spec, sorted so a failure list has a stable order. */
const SPEC_FILES = readdirSync(E2E_DIRECTORY)
  .filter((entry) => entry.endsWith('.spec.ts'))
  .map((entry) => resolve(E2E_DIRECTORY, entry))
  .sort();

/**
 * Files Playwright would run that the narrow discovery above does not check.
 * Empty is the invariant; a non-empty list is the escape Q23's guard exists to
 * prevent.
 */
const UNGUARDED_PLAYWRIGHT_FILES = collectFiles(E2E_DIRECTORY).filter(
  (file) =>
    PLAYWRIGHT_TEST_MATCH.test(basename(file)) && !SPEC_FILES.includes(file),
);

type ImportBinding = {
  /** The name the module exports, before any `as` alias. */
  imported: string;
  /** `import type` or an inline `type` specifier: erased at runtime. */
  typeOnly: boolean;
};

type ImportDeclaration = {
  module: string;
  bindings: ImportBinding[];
};

/**
 * Blank comments first, so a commented-out import neither satisfies nor trips
 * the rule. Import declarations never contain a block comment in this
 * repository; the substitution is what keeps a rule about code from reading
 * prose about the rule.
 */
function stripComments(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^[ \t]*\/\/.*$/gm, '');
}

function parseSpecifier(
  specifier: string,
  declarationTypeOnly: boolean,
): ImportBinding | undefined {
  const trimmed = specifier.trim();

  if (trimmed === '') {
    return undefined;
  }

  let rest = trimmed;
  let typeOnly = declarationTypeOnly;

  if (rest.startsWith('type ')) {
    typeOnly = true;
    rest = rest.slice('type '.length).trim();
  }

  const [imported] = rest.split(/\s+as\s+/);
  const name = imported?.trim() ?? '';

  return name === '' ? undefined : { imported: name, typeOnly };
}

function parseImportBindings(
  clause: string,
  declarationTypeOnly: boolean,
): ImportBinding[] {
  const named = clause.match(/\{([\s\S]*)\}/);
  const beforeBraces = named === null ? clause : clause.slice(0, named.index);
  const bindings: ImportBinding[] = [];

  /*
   * A default or namespace binding is a value even though it names no export
   * we require from `./fixtures`; recording it is what lets the
   * `@playwright/test` rule catch `import Foo from …` and `import * as pw from
   * …` as well as a named `test`/`expect`.
   */
  if (!declarationTypeOnly && /^\s*[A-Za-z_$][\w$]*/.test(beforeBraces)) {
    bindings.push({ imported: 'default', typeOnly: false });
  }

  if (!declarationTypeOnly && /\*\s+as\s+[A-Za-z_$][\w$]*/.test(beforeBraces)) {
    bindings.push({ imported: '*', typeOnly: false });
  }

  if (named !== null) {
    bindings.push(
      ...named[1]
        .split(',')
        .map((specifier) => parseSpecifier(specifier, declarationTypeOnly))
        .filter((binding): binding is ImportBinding => binding !== undefined),
    );
  }

  return bindings;
}

/**
 * The import declarations in a comment-stripped source file. The lazy body
 * stops at the first `from '<module>'`, which is the only shape an import
 * declaration in these specs takes.
 */
function parseImports(source: string): ImportDeclaration[] {
  const declarations: ImportDeclaration[] = [];
  const pattern =
    /^[ \t]*import\s+(type\s+)?([\s\S]*?)\s+from\s+['"]([^'"]+)['"]\s*;?/gm;

  for (const match of source.matchAll(pattern)) {
    declarations.push({
      module: match[3] ?? '',
      bindings: parseImportBindings(match[2] ?? '', match[1] !== undefined),
    });
  }

  return declarations;
}

function hasValueImport(
  declarations: readonly ImportDeclaration[],
  module: string,
  name: string,
): boolean {
  return declarations.some(
    (declaration) =>
      declaration.module === module &&
      declaration.bindings.some(
        (binding) => binding.imported === name && !binding.typeOnly,
      ),
  );
}

/**
 * Whether the module exports `name` as a value. Named export lists are parsed
 * rather than substring-matched, so `export { expect as somethingElse }` does
 * not count as exporting `expect`.
 */
function exportsValue(source: string, name: string): boolean {
  const clean = stripComments(source);

  for (const block of clean.matchAll(/export\s*\{([\s\S]*?)\}/g)) {
    const specifiers = (block[1] ?? '').split(',');

    for (const specifier of specifiers) {
      const [imported, exported = imported] = specifier.split(/\s+as\s+/);
      const importedName = imported?.trim() ?? '';
      const exportedName = exported?.trim() ?? '';

      if (importedName === name && exportedName === name) {
        return true;
      }
    }
  }

  const declarations = clean.matchAll(
    /export\s+(?:const|let|var|function|class)\s+(\w+)/g,
  );

  return [...declarations].some((match) => match[1] === name);
}

describe('TOOL-5: E2E runs in WebKit with mocked IPC — every spec consumes the fixture', () => {
  it('Discovers at least one E2E spec to check', () => {
    expect(
      SPEC_FILES.length,
      `no e2e/*.spec.ts files were found under ${E2E_DIRECTORY}`,
    ).toBeGreaterThan(0);
  });

  it('No other file under e2e/ matches Playwright testMatch', () => {
    expect(
      UNGUARDED_PLAYWRIGHT_FILES,
      [
        'these files are run by Playwright (default testMatch) but are not checked by this guard:',
        ...UNGUARDED_PLAYWRIGHT_FILES.map((file) => `  ${file}`),
        `move each one to e2e/<name>.spec.ts, or widen this guard's discovery`,
      ].join('\n'),
    ).toEqual([]);
  });

  it('The shared fixture exports test and expect as values', () => {
    const fixtures = readAbsoluteFile(resolve(E2E_DIRECTORY, 'fixtures.ts'));

    expect(
      exportsValue(fixtures, 'test'),
      `${FIXTURES_MODULE} no longer exports \`test\``,
    ).toBe(true);
    expect(
      exportsValue(fixtures, 'expect'),
      `${FIXTURES_MODULE} no longer exports \`expect\``,
    ).toBe(true);
  });

  it.each(SPEC_FILES)(
    '%s imports test and expect from ./fixtures',
    (specPath) => {
      const declarations = parseImports(
        stripComments(readAbsoluteFile(specPath)),
      );

      for (const name of ['test', 'expect']) {
        expect(
          hasValueImport(declarations, FIXTURES_MODULE, name),
          `${specPath} must value-import \`${name}\` from '${FIXTURES_MODULE}' (task 0.25; Q23)`,
        ).toBe(true);
      }
    },
  );

  it.each(SPEC_FILES)(
    '%s has no value import from @playwright/test',
    (specPath) => {
      const declarations = parseImports(
        stripComments(readAbsoluteFile(specPath)),
      );
      const valueImports = declarations
        .filter((declaration) => declaration.module === PLAYWRIGHT_MODULE)
        .flatMap((declaration) => declaration.bindings)
        .filter((binding) => !binding.typeOnly)
        .map((binding) => binding.imported);

      expect(
        valueImports,
        `${specPath} must not value-import from '${PLAYWRIGHT_MODULE}'; import \`test\` and \`expect\` from '${FIXTURES_MODULE}' (task 0.25; Q23)`,
      ).toEqual([]);
    },
  );
});
