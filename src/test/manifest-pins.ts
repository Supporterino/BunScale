/*
 * Manifest pin checks for TOOL-8's "Versions are exact" scenario (task 0.24).
 *
 * The expectation is **D0.8's pin tables in `design.md`**, parsed at test time,
 * not a third copy written into this file. That is deliberate: a hard-coded
 * list would be one more hand-reconciled statement to rot, and the requirement
 * is that the committed manifests match the recorded pins. When D0.8 changes
 * legitimately, the tables change and the manifests must follow.
 *
 * What the parsers can and cannot see is stated beside each one, because a
 * check that quietly skips a form is worse than no check.
 */
/** One direct dependency as a manifest declares it. */
export type ManifestPin = {
  readonly name: string;
  readonly version: string;
  /** The manifest section it was found in, for failure messages. */
  readonly section: string;
};

/** D0.8's two tables: crate name → pin, npm package name → pin. */
export type DesignPinTables = {
  readonly crates: ReadonlyMap<string, string>;
  readonly npm: ReadonlyMap<string, string>;
};

/** The four npm dependency sections a direct dependency can hide in. */
const NPM_SECTIONS = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] as const;

/**
 * Bare `x.y.z` (plus an optional prerelease/build suffix) and nothing else.
 *
 * This rejects every range operator the task names — `^`, `~`, `>=`, `*`,
 * `latest` — as well as the forms that look exact but are not: `1.2`, `1.2.x`,
 * `1.2.3 - 2.0.0`, `1.2.3 || 2.0.0`, `file:…`, `workspace:…` and a leading
 * `v`. A prerelease such as `2.0.0-rc.25` is an exact version and passes.
 */
const EXACT_NPM_VERSION =
  /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

/** The three Cargo tables a direct dependency can be declared in. */
const CARGO_SECTIONS = [
  'dependencies',
  'dev-dependencies',
  'build-dependencies',
] as const;

type MarkdownTable = {
  header: string[];
  rows: string[][];
};

function splitRow(line: string): string[] {
  return line
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim());
}

function markdownTables(markdown: string): MarkdownTable[] {
  const tables: MarkdownTable[] = [];
  let current: MarkdownTable | undefined;

  for (const line of markdown.split('\n')) {
    const trimmed = line.trim();

    if (!trimmed.startsWith('|')) {
      current = undefined;
      continue;
    }

    const cells = splitRow(trimmed);

    if (current === undefined) {
      current = { header: cells, rows: [] };
      tables.push(current);
      continue;
    }

    // The `|---|:--:|` separator row carries no pins.
    if (cells.every((cell) => /^:?-{3,}:?$/.test(cell))) {
      continue;
    }

    current.rows.push(cells);
  }

  return tables;
}
/** Every backticked token in a cell, in order. */
function backticked(cell: string): string[] {
  return [...cell.matchAll(/`([^`]+)`/g)].map((match) => match[1]);
}

/**
 * Parses D0.8's two pin tables.
 *
 * This reads Markdown tables whose first column is `Crate` or `Package`; the
 * grouped npm row (`@types/react` / … with three pins) is zipped back into
 * three entries. It cannot see a pin stated in prose outside a table, and it
 * fails loudly if either header appears zero or more than once, rather than
 * silently merging tables.
 */
export function parseDesignPinTables(markdown: string): DesignPinTables {
  const crates = new Map<string, string>();
  const npm = new Map<string, string>();
  let crateTables = 0;
  let npmTables = 0;

  for (const table of markdownTables(markdown)) {
    if (table.header[0] === 'Crate') {
      crateTables += 1;

      for (const row of table.rows) {
        const names = backticked(row[0] ?? '');
        const pins = backticked(row[1] ?? '');

        if (names.length !== 1 || pins.length !== 1) {
          throw new Error(
            `D0.8 crate row is not one name and one pin: ${row.join(' | ')}`,
          );
        }

        crates.set(names[0], pins[0]);
      }
    } else if (table.header[0] === 'Package') {
      npmTables += 1;

      for (const row of table.rows) {
        const names = backticked(row[0] ?? '');
        const pins = backticked(row[1] ?? '');

        if (names.length === 0 || names.length !== pins.length) {
          throw new Error(
            `D0.8 npm row does not pair names with pins: ${row.join(' | ')}`,
          );
        }

        names.forEach((name, index) => {
          npm.set(name, pins[index]);
        });
      }
    }
  }

  if (crateTables !== 1 || npmTables !== 1) {
    throw new Error(
      `expected one D0.8 crate table and one npm table, found ${crateTables} and ${npmTables}`,
    );
  }

  return { crates, npm };
}

/**
 * Reads the direct dependencies `package.json` declares.
 *
 * `dependencies` and `devDependencies` are the two sections D0.8 uses. The
 * other two are read as well so an undeclared pin cannot hide there; the table
 * does not distinguish sections, so only the name and version are checked.
 */
export function collectNpmPins(packageJson: string): ManifestPin[] {
  const manifest = JSON.parse(packageJson) as Record<string, unknown>;
  const pins: ManifestPin[] = [];

  for (const section of NPM_SECTIONS) {
    const dependencies = manifest[section];

    if (dependencies === undefined) {
      continue;
    }

    if (typeof dependencies !== 'object' || dependencies === null) {
      throw new Error(`package.json's "${section}" is not an object`);
    }

    for (const [name, version] of Object.entries(dependencies)) {
      if (typeof version !== 'string') {
        throw new Error(`package.json's "${section}.${name}" is not a string`);
      }

      pins.push({ name, version, section });
    }
  }

  return pins;
}

/**
 * Reads `version = "…"` out of one Cargo dependency value.
 *
 * Handles the two forms the manifest uses — `name = "=1.2.3"` and
 * `name = { version = "=1.2.3", features = […] }`. A value with no literal
 * version (`workspace = true`, a path/git dependency) returns `undefined`, and
 * the caller fails loudly: D0.8 pins an exact version in this manifest, not one
 * inherited from a workspace table this parser does not read.
 */
function versionFromValue(raw: string): string | undefined {
  const value = raw.trim();
  const bare = /^"([^"]+)"/.exec(value);

  if (bare) {
    return bare[1];
  }

  if (value.startsWith('{')) {
    const inline = /\bversion\s*=\s*"([^"]+)"/.exec(value);
    return inline?.[1];
  }

  return undefined;
}

function isCargoSection(section: string): boolean {
  return (CARGO_SECTIONS as readonly string[]).includes(section);
}

/**
 * Reads the direct dependencies `Cargo.toml` declares in its three dependency
 * tables, line by line.
 *
 * **This is a careful line-based parse, not a TOML parser**, because no TOML
 * parser is a pinned dependency and TOOL-8 forbids adding one for a test. What
 * it can see: `name = "=x.y.z"`, `name = { version = "=x.y.z", … }` and the
 * `[dependencies.name]` sub-table form, in `[dependencies]`,
 * `[dev-dependencies]` and `[build-dependencies]`. A sub-table stays in scope
 * until the next `[` header or EOF — its non-`version` keys are ignored rather
 * than read as top-level entries, and one that closes without a literal
 * `version` is reported rather than skipped.
 *
 * What it cannot see, and therefore does not claim: `[patch]`, `[replace]`,
 * `[workspace.dependencies]`, target-specific tables
 * (`[target.'cfg(…)'.dependencies]`), a workspace-inherited `version = true`, a
 * dependency moved into a table whose header a later line redefines, a
 * dependency declared with a dotted key (`dependencies.serde = "=x.y.z"`)
 * instead of under a table header, and a second `version` key in one sub-table
 * (TOML forbids it; the first is read). A form it cannot read is reported as an
 * issue rather than skipped.
 */
export function collectCargoPins(toml: string): ManifestPin[] {
  const pins: ManifestPin[] = [];
  const lines = toml.split('\n');
  let section = '';
  let subtable:
    | {
        readonly name: string;
        readonly line: number;
        readonly table: string;
        versionRead: boolean;
      }
    | undefined;

  // The sub-table stays in scope until the next `[` header or EOF (see the
  // comment above): a field after `version` must not be read as a top-level
  // entry, and a sub-table that never names a literal version is a failure
  // rather than a silent skip.
  const closeSubtable = (): void => {
    if (subtable === undefined) {
      return;
    }

    if (!subtable.versionRead) {
      throw new Error(
        `Cargo.toml:${subtable.line} declares "[${subtable.table}]" without a literal version; D0.8 pins an exact version in this manifest`,
      );
    }

    subtable = undefined;
  };

  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    const trimmed = line.trim();
    const header = /^\[([^\]]+)\]$/.exec(trimmed);

    if (header) {
      closeSubtable();
      section = header[1];

      for (const candidate of CARGO_SECTIONS) {
        if (section.startsWith(`${candidate}.`)) {
          subtable = {
            name: section.slice(candidate.length + 1),
            line: lineNumber,
            table: section,
            versionRead: false,
          };
          section = candidate;
          break;
        }
      }

      return;
    }

    if (!isCargoSection(section)) {
      return;
    }

    if (subtable !== undefined) {
      if (subtable.versionRead) {
        return;
      }

      const versionLine = /^version\s*=\s*(.+)$/.exec(trimmed);

      if (versionLine === null) {
        return;
      }

      const version = versionFromValue(versionLine[1]);

      if (version === undefined) {
        throw new Error(
          `Cargo.toml:${lineNumber} cannot read a literal version for "${subtable.name}"`,
        );
      }

      pins.push({ name: subtable.name, version, section });
      subtable.versionRead = true;
      return;
    }

    if (trimmed === '' || trimmed.startsWith('#')) {
      return;
    }

    const assignment = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(trimmed);

    if (assignment === null) {
      throw new Error(
        `Cargo.toml:${lineNumber} is not a dependency assignment: ${trimmed}`,
      );
    }

    const version = versionFromValue(assignment[2]);

    if (version === undefined) {
      throw new Error(
        `Cargo.toml:${lineNumber} declares "${assignment[1]}" without a literal version; D0.8 pins an exact version in this manifest`,
      );
    }

    pins.push({ name: assignment[1], version, section });
  });

  closeSubtable();

  return pins;
}

function duplicates(pins: readonly ManifestPin[]): string[] {
  const seen = new Map<string, string>();
  const issues: string[] = [];

  for (const pin of pins) {
    const previous = seen.get(pin.name);

    if (previous !== undefined) {
      issues.push(
        `"${pin.name}" is declared twice, in ${previous} and ${pin.section}`,
      );
    }

    seen.set(pin.name, pin.section);
  }

  return issues;
}

/** Every way `package.json` disagrees with D0.8's npm table. Empty is green. */
export function checkNpmPins(
  expected: ReadonlyMap<string, string>,
  actual: readonly ManifestPin[],
): string[] {
  const issues = duplicates(actual);
  const seen = new Set<string>();

  for (const pin of actual) {
    seen.add(pin.name);

    if (!EXACT_NPM_VERSION.test(pin.version)) {
      issues.push(
        `package.json: "${pin.name}" is "${pin.version}", which is not an exact version (D0.8 pins a bare x.y.z, never a range)`,
      );
    }

    const pinned = expected.get(pin.name);

    if (pinned === undefined) {
      issues.push(
        `package.json: "${pin.name}" is not in D0.8's npm pin table (nothing extra)`,
      );
    } else if (pinned !== pin.version) {
      issues.push(
        `package.json: "${pin.name}" is "${pin.version}" but D0.8 pins "${pinned}"`,
      );
    }
  }

  for (const name of expected.keys()) {
    if (!seen.has(name)) {
      issues.push(
        `package.json: D0.8 pins "${name}" but the manifest does not declare it`,
      );
    }
  }

  return issues;
}

/** Every way `Cargo.toml` disagrees with D0.8's crate table. Empty is green. */
export function checkCargoPins(
  expected: ReadonlyMap<string, string>,
  actual: readonly ManifestPin[],
): string[] {
  const issues = duplicates(actual);
  const seen = new Set<string>();

  for (const pin of actual) {
    seen.add(pin.name);

    if (!pin.version.startsWith('=')) {
      issues.push(
        `Cargo.toml: "${pin.name}" is "${pin.version}", which does not begin with "=" (D0.8 pins crates as =x.y.z)`,
      );
    }

    const pinned = expected.get(pin.name);

    if (pinned === undefined) {
      issues.push(
        `Cargo.toml: "${pin.name}" is not in D0.8's crate pin table (nothing extra)`,
      );
    } else if (pinned !== pin.version) {
      issues.push(
        `Cargo.toml: "${pin.name}" is "${pin.version}" but D0.8 pins "${pinned}"`,
      );
    }
  }

  for (const name of expected.keys()) {
    if (!seen.has(name)) {
      issues.push(
        `Cargo.toml: D0.8 pins "${name}" but the manifest does not declare it`,
      );
    }
  }

  return issues;
}
