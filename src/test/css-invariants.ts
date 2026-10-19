/*
 * Token invariants for `src/styles/*.css` (task 0.24, TOOL-6).
 *
 * Task 0.20 measured four properties with a throwaway script that no longer
 * exists, and nothing at build time guards them. Vite does not resolve custom
 * properties: a misspelt `var(--colour-primary)` is emitted verbatim, the build
 * stays green and the browser silently falls back. This module is the check
 * that survives, and it asserts four separate invariants:
 *
 * 1. every `var(--x)` reference — including `var(--x, fallback)` — names a
 *    custom property some stylesheet defines;
 * 2. no custom property's value refers, directly or through a chain, back to
 *    itself;
 * 3. no custom property is defined twice **in one cascade context**. This is
 *    the check that has to be written carefully: `semantic.css` deliberately
 *    redefines ~47 names inside `prefers-color-scheme: dark`,
 *    `prefers-reduced-motion: reduce`, `@supports` and the breakpoint blocks. A
 *    naive first-definition-wins check would fail on the very file it guards.
 *    The context here is the file plus the enclosing at-rule chain plus the
 *    selector, so a dark-theme override is a different context from the base
 *    rule, while two definitions of one name inside a single block — or in two
 *    blocks with the same file, conditions and selector — are a duplicate;
 * 4. the documented AA contrast pairs still hold, computed from the resolved
 *    hex values in both themes.
 *
 * Parsing limits, stated rather than implied: this is not a CSS parser. It
 * strips comments, then walks balanced `{…}` blocks, tracking quotes and
 * parentheses. It does not support CSS nesting (it fails loudly on a nested
 * rule rather than mis-reading one), `@layer`, or custom properties defined
 * from a dynamic value. Resolution follows `var()` aliases only; a token whose
 * value is a `calc()` or `env()` is not a colour and is only checked for a
 * defined name, not resolved to a literal.
 */

export type CssDeclaration = {
  readonly property: string;
  readonly value: string;
  /** Basename of the stylesheet, for legible messages. */
  readonly file: string;
  readonly line: number;
  readonly selector: string;
  readonly atRules: readonly string[];
};

export type CssReference = {
  readonly name: string;
  readonly file: string;
  readonly line: number;
  /** The declaration that carries the reference. */
  readonly from: string;
};

export type Stylesheet = {
  readonly file: string;
  /** The source with comments blanked out, offsets preserved. */
  readonly source: string;
  readonly declarations: readonly CssDeclaration[];
  readonly customProperties: readonly CssDeclaration[];
  readonly references: readonly CssReference[];
};

export type Rgb = {
  readonly r: number;
  readonly g: number;
  readonly b: number;
};

export type ContrastPair = {
  /** The semantic token names, with the `--color-` prefix restored. */
  readonly foreground: string;
  readonly background: string;
  readonly light: number;
  readonly dark: number;
  readonly minimum: number;
};

/**
 * Replaces comments with spaces (newlines kept) so offsets and line numbers
 * survive. Strings are copied as-is: `//` inside a string is not a comment.
 */
export function stripComments(text: string): string {
  let result = '';
  let quote: string | undefined;

  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];

    if (quote !== undefined) {
      result += character;

      if (character === '\\' && index + 1 < text.length) {
        result += text[index + 1];
        index += 1;
      } else if (character === quote) {
        quote = undefined;
      }

      continue;
    }

    if (character === '"' || character === "'") {
      quote = character;
      result += character;
      continue;
    }

    if (character === '/' && text[index + 1] === '*') {
      result += '  ';
      index += 2;

      while (
        index < text.length &&
        !(text[index] === '*' && text[index + 1] === '/')
      ) {
        result += text[index] === '\n' ? '\n' : ' ';
        index += 1;
      }

      if (index < text.length) {
        result += '  ';
        index += 1;
      }

      continue;
    }

    if (character === '/' && text[index + 1] === '/') {
      while (index < text.length && text[index] !== '\n') {
        result += ' ';
        index += 1;
      }

      if (index < text.length) {
        result += '\n';
      }

      continue;
    }

    result += character;
  }

  return result;
}

function lineIndex(text: string): (offset: number) => number {
  const starts = [0];

  for (let index = 0; index < text.length; index += 1) {
    if (text[index] === '\n') {
      starts.push(index + 1);
    }
  }

  return (offset: number): number => {
    let low = 0;
    let high = starts.length - 1;

    while (low < high) {
      const middle = Math.ceil((low + high) / 2);

      if (starts[middle] <= offset) {
        low = middle;
      } else {
        high = middle - 1;
      }
    }

    return low + 1;
  };
}

/** The index of the `}` matching the `{` at `open`, or -1. */
function closingBrace(text: string, open: number): number {
  let depth = 0;
  let quote: string | undefined;

  for (let index = open; index < text.length; index += 1) {
    const character = text[index];

    if (quote !== undefined) {
      if (character === '\\') {
        index += 1;
      } else if (character === quote) {
        quote = undefined;
      }

      continue;
    }

    if (character === '"' || character === "'") {
      quote = character;
    } else if (character === '{') {
      depth += 1;
    } else if (character === '}') {
      depth -= 1;

      if (depth === 0) {
        return index;
      }
    }
  }

  return -1;
}

/** The first `:` outside strings and parentheses, or -1. */
function topLevelColon(chunk: string): number {
  let quote: string | undefined;
  let parentheses = 0;

  for (let index = 0; index < chunk.length; index += 1) {
    const character = chunk[index];

    if (quote !== undefined) {
      if (character === '\\') {
        index += 1;
      } else if (character === quote) {
        quote = undefined;
      }

      continue;
    }

    if (character === '"' || character === "'") {
      quote = character;
    } else if (character === '(') {
      parentheses += 1;
    } else if (character === ')') {
      parentheses -= 1;
    } else if (character === ':' && parentheses === 0) {
      return index;
    }
  }

  return -1;
}

function parseDeclarations(
  text: string,
  start: number,
  end: number,
  file: string,
  selector: string,
  atRules: readonly string[],
  declarations: CssDeclaration[],
  lineOf: (offset: number) => number,
): void {
  let chunkStart = start;
  let index = start;
  let quote: string | undefined;
  let parentheses = 0;

  const flush = (stop: number): void => {
    const chunk = text.slice(chunkStart, stop);
    const colon = topLevelColon(chunk);

    if (colon === -1) {
      if (chunk.trim() !== '') {
        throw new Error(
          `${file}:${lineOf(chunkStart)} cannot read a declaration from "${chunk.trim()}"`,
        );
      }

      return;
    }

    const property = chunk.slice(0, colon).trim();
    const value = chunk.slice(colon + 1).trim();

    if (property === '' || value === '') {
      return;
    }

    declarations.push({
      property,
      value,
      file,
      line: lineOf(chunkStart + chunk.indexOf(property)),
      selector,
      atRules,
    });
  };

  while (index < end) {
    const character = text[index];

    if (quote !== undefined) {
      if (character === '\\') {
        index += 2;
        continue;
      }

      if (character === quote) {
        quote = undefined;
      }

      index += 1;
      continue;
    }

    if (character === '"' || character === "'") {
      quote = character;
      index += 1;
      continue;
    }

    if (character === '(') {
      parentheses += 1;
      index += 1;
      continue;
    }

    if (character === ')') {
      parentheses = Math.max(0, parentheses - 1);
      index += 1;
      continue;
    }

    if (character === '{' && parentheses === 0) {
      throw new Error(
        `${file}:${lineOf(index)} has a nested rule; this parser does not support CSS nesting`,
      );
    }

    if (character === ';' && parentheses === 0) {
      flush(index);
      index += 1;
      chunkStart = index;
      continue;
    }

    index += 1;
  }

  flush(end);
}

function parseRuleList(
  text: string,
  start: number,
  end: number,
  file: string,
  atRules: readonly string[],
  declarations: CssDeclaration[],
  lineOf: (offset: number) => number,
): void {
  let index = start;

  while (index < end) {
    while (index < end && /[\s;]/.test(text[index])) {
      index += 1;
    }

    if (index >= end) {
      return;
    }

    const preludeStart = index;
    let quote: string | undefined;

    while (index < end) {
      const character = text[index];

      if (quote !== undefined) {
        if (character === '\\') {
          index += 2;
          continue;
        }

        if (character === quote) {
          quote = undefined;
        }

        index += 1;
        continue;
      }

      if (character === '"' || character === "'") {
        quote = character;
        index += 1;
        continue;
      }

      if (character === '{' || character === ';' || character === '}') {
        break;
      }

      index += 1;
    }

    if (index >= end) {
      return;
    }

    const prelude = text.slice(preludeStart, index).trim();

    if (text[index] === ';' || text[index] === '}') {
      index += 1;
      continue;
    }

    const close = closingBrace(text, index);

    if (close === -1) {
      throw new Error(
        `${file}:${lineOf(index)} has unbalanced braces after "${prelude}"`,
      );
    }

    if (prelude.startsWith('@')) {
      parseRuleList(
        text,
        index + 1,
        close,
        file,
        [...atRules, prelude],
        declarations,
        lineOf,
      );
    } else if (prelude !== '') {
      parseDeclarations(
        text,
        index + 1,
        close,
        file,
        prelude,
        atRules,
        declarations,
        lineOf,
      );
    }

    index = close + 1;
  }
}

/** Parses one stylesheet's declarations and custom-property definitions. */
export function parseStylesheet(file: string, raw: string): Stylesheet {
  const source = stripComments(raw);
  const lineOf = lineIndex(source);
  const declarations: CssDeclaration[] = [];

  parseRuleList(source, 0, source.length, file, [], declarations, lineOf);

  const references: CssReference[] = [];

  for (const declaration of declarations) {
    for (const match of declaration.value.matchAll(
      /var\(\s*(--[A-Za-z0-9_-]+)/g,
    )) {
      references.push({
        name: match[1],
        file: declaration.file,
        line: declaration.line,
        from: declaration.property,
      });
    }
  }

  return {
    file,
    source,
    declarations,
    customProperties: declarations.filter((declaration) =>
      declaration.property.startsWith('--'),
    ),
    references,
  };
}

/**
 * Every unresolved reference, cycle and same-context duplicate. Empty is green.
 *
 * The cycle walk is over the **union** of definitions: a name is checked
 * against every definition of it, across themes, because a cycle introduced by
 * only the dark-theme branch is still a cycle.
 */
export function checkTokenInvariants(sheets: readonly Stylesheet[]): string[] {
  const issues: string[] = [];
  const definitions = sheets.flatMap((sheet) => sheet.customProperties);
  const references = sheets.flatMap((sheet) => sheet.references);
  const definedNames = new Set(
    definitions.map((definition) => definition.property),
  );

  for (const reference of references) {
    if (!definedNames.has(reference.name)) {
      issues.push(
        `${reference.file}:${reference.line} ${reference.from} references var(${reference.name}), which no stylesheet defines (Vite emits it verbatim and the browser falls back silently)`,
      );
    }
  }

  const contexts = new Map<string, Map<string, CssDeclaration[]>>();

  for (const definition of definitions) {
    const context = [
      definition.file,
      ...definition.atRules,
      definition.selector,
    ].join('\u0000');
    const names = contexts.get(context) ?? new Map<string, CssDeclaration[]>();
    contexts.set(context, names);

    const list = names.get(definition.property) ?? [];
    names.set(definition.property, list);
    list.push(definition);
  }

  for (const names of contexts.values()) {
    for (const [name, list] of names) {
      if (list.length > 1) {
        const where = list
          .map((definition) => `${definition.file}:${definition.line}`)
          .join(', ');

        issues.push(
          `${list[0].file}:${list[0].line} ${name} is defined ${list.length} times in the same cascade context (${where}); the later declaration silently wins`,
        );
      }
    }
  }

  const edges = new Map<string, Set<string>>();

  for (const definition of definitions) {
    const targets = edges.get(definition.property) ?? new Set<string>();
    edges.set(definition.property, targets);

    for (const match of definition.value.matchAll(
      /var\(\s*(--[A-Za-z0-9_-]+)/g,
    )) {
      targets.add(match[1]);
    }
  }

  const state = new Map<string, 'visiting' | 'done'>();
  const reported = new Set<string>();
  const stack: string[] = [];

  const visit = (name: string): void => {
    const current = state.get(name);

    if (current === 'done') {
      return;
    }

    if (current === 'visiting') {
      const cycle = [...stack.slice(stack.indexOf(name)), name];
      const key = [...cycle].sort().join('|');

      if (!reported.has(key)) {
        reported.add(key);
        issues.push(`reference cycle: ${cycle.join(' -> ')}`);
      }

      return;
    }

    state.set(name, 'visiting');
    stack.push(name);

    for (const target of edges.get(name) ?? []) {
      visit(target);
    }

    stack.pop();
    state.set(name, 'done');
  };

  for (const name of edges.keys()) {
    visit(name);
  }

  return issues;
}

/**
 * Names defined in more than one cascade context, with those contexts.
 *
 * This is the positive half of the duplicate check: it shows that the
 * deliberately overridden names (dark theme, reduced motion, breakpoints) are
 * seen as separate contexts rather than collapsed into a duplicate.
 */
export function crossContextRedefinitions(
  sheets: readonly Stylesheet[],
): Map<string, string[]> {
  const contextsByName = new Map<string, Set<string>>();

  for (const sheet of sheets) {
    for (const definition of sheet.customProperties) {
      const context = [
        definition.file,
        ...definition.atRules,
        definition.selector,
      ].join(' | ');
      const seen = contextsByName.get(definition.property) ?? new Set();
      contextsByName.set(definition.property, seen);
      seen.add(context);
    }
  }

  const redefinitions = new Map<string, string[]>();

  for (const [name, seen] of contextsByName) {
    if (seen.size > 1) {
      redefinitions.set(name, [...seen]);
    }
  }

  return redefinitions;
}

/**
 * Reads a colour literal as sRGB bytes.
 *
 * Alpha is refused rather than approximated: a translucent colour's contrast
 * depends on what is behind it, and compositing is not modelled here. The
 * documented pairs use opaque values, so a pair that starts resolving through
 * alpha is a change the check should report, not guess at.
 */
export function parseColourLiteral(value: string): Rgb | undefined {
  const hex = /^#([0-9a-fA-F]{3,8})$/.exec(value);

  if (hex !== null) {
    const digits = hex[1];

    if (![3, 4, 6, 8].includes(digits.length)) {
      return undefined;
    }

    const expanded =
      digits.length <= 4
        ? [...digits].map((digit) => digit + digit).join('')
        : digits;

    if (expanded.length === 8 && expanded.slice(6, 8) !== 'ff') {
      throw new Error(
        `alpha colour ${value} needs compositing, which this check does not model`,
      );
    }

    return {
      r: Number.parseInt(expanded.slice(0, 2), 16),
      g: Number.parseInt(expanded.slice(2, 4), 16),
      b: Number.parseInt(expanded.slice(4, 6), 16),
    };
  }

  const rgb =
    /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)\s*(?:[,/]\s*([\d.]+)(%?))?\s*\)$/.exec(
      value,
    );

  if (rgb === null) {
    return undefined;
  }

  if (rgb[4] !== undefined) {
    const alpha = rgb[5] === '%' ? Number(rgb[4]) / 100 : Number(rgb[4]);

    if (alpha !== 1) {
      throw new Error(
        `alpha colour ${value} needs compositing, which this check does not model`,
      );
    }
  }

  return {
    r: Number(rgb[1]),
    g: Number(rgb[2]),
    b: Number(rgb[3]),
  };
}

function linearChannel(value: number): number {
  const channel = value / 255;

  return channel <= 0.04045
    ? channel / 12.92
    : ((channel + 0.055) / 1.055) ** 2.4;
}

function relativeLuminance({ r, g, b }: Rgb): number {
  return (
    0.2126 * linearChannel(r) +
    0.7152 * linearChannel(g) +
    0.0722 * linearChannel(b)
  );
}

/** The WCAG 2.1 contrast ratio, 1:1 to 21:1. */
export function contrastRatio(first: Rgb, second: Rgb): number {
  const a = relativeLuminance(first);
  const b = relativeLuminance(second);
  const lighter = Math.max(a, b);
  const darker = Math.min(a, b);

  return (lighter + 0.05) / (darker + 0.05);
}

/** Which theme a definition applies to, for colour resolution. */
export type Theme = 'light' | 'dark';

/**
 * Builds a name → colour resolver for one theme.
 *
 * `sheets` must be in cascade order (the order `index.css` imports them). A
 * definition inside `prefers-color-scheme: dark` is skipped for the light theme
 * and applies for the dark one; every other context is applied, which is exact
 * for the colour tokens (none of them is redefined inside a breakpoint or
 * `@supports` block). The winning definition is the last one in cascade order,
 * which is what the browser does for the same specificity.
 */
export function createThemeResolver(
  sheets: readonly Stylesheet[],
  theme: Theme,
): (name: string) => { colour: Rgb; chain: readonly string[] } {
  const winners = new Map<string, CssDeclaration>();

  for (const sheet of sheets) {
    for (const definition of sheet.customProperties) {
      const dark = definition.atRules.some((atRule) =>
        /prefers-color-scheme\s*:\s*dark/i.test(atRule),
      );

      if (dark && theme === 'light') {
        continue;
      }

      winners.set(definition.property, definition);
    }
  }

  return (name: string) => {
    const chain: string[] = [];
    let current = name;

    for (let step = 0; step <= winners.size; step += 1) {
      const definition = winners.get(current);

      if (definition === undefined) {
        throw new Error(`--${current} is not defined in the ${theme} theme`);
      }

      chain.push(current);
      const value = definition.value.trim();
      const alias = /^var\(\s*(--[A-Za-z0-9_-]+)\s*(?:,[\s\S]*)?\)$/.exec(
        value,
      );

      if (alias !== null) {
        current = alias[1];
        continue;
      }

      const colour = parseColourLiteral(value);

      if (colour === undefined) {
        throw new Error(
          `--${current} resolves to "${value}" in the ${theme} theme, which is not a colour literal the contrast check can read`,
        );
      }

      return { colour, chain };
    }

    throw new Error(`--${name} did not resolve (reference cycle?)`);
  };
}

/**
 * Parses the "Measured AA pairs" table out of the design-system record.
 *
 * The pair column names semantic tokens without the `--color-` prefix
 * (`primary` on `surface-sunken`); the prefix is restored here. The documented
 * ratio columns are parsed too, so the spec can also report a ratio that still
 * passes AA but no longer matches the record.
 */
export function parseMeasuredContrastPairs(
  skillMarkdown: string,
): ContrastPair[] {
  const pairs: ContrastPair[] = [];

  for (const line of skillMarkdown.split('\n')) {
    const match =
      /^\|\s*`([^`]+)`\s+on\s+`([^`]+)`\s*\|\s*([\d.]+):1\s*\|\s*([\d.]+):1\s*\|\s*([\d.]+)\s*\|/.exec(
        line,
      );

    if (match === null) {
      continue;
    }

    pairs.push({
      foreground: `--color-${match[1]}`,
      background: `--color-${match[2]}`,
      light: Number(match[3]),
      dark: Number(match[4]),
      minimum: Number(match[5]),
    });
  }

  return pairs;
}
