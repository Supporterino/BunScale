/*
 * The config-agreement check for task 0.14's rule (task 0.24, TOOL-1/TOOL-2).
 *
 * `src-tauri/tauri.conf.json` and `vite.config.ts` each state the dev-server
 * address and the build output directory, and nothing reconciles them but a
 * human: task 0.14 requires "change whichever side is wrong, never let the two
 * drift". This module reads both files and reports any disagreement.
 *
 * `vite.config.ts` is TypeScript, and it is read **as text**, not imported.
 * That is the honest choice for two reasons: importing it would execute the
 * router plugin inside the test process, and it would resolve `server.host`
 * through `process.env.TAURI_DEV_HOST` — so an assertion about the *wiring*
 * ("host is still wired to the variable") would become an assertion about
 * whatever the test process's environment happened to hold. Reading the source
 * sees the wiring itself.
 *
 * What the textual read can and cannot see: it strips comments, then reads the
 * literal `server` and `build` object blocks, `port: <number>`,
 * `outDir: '<string>'` and the identifier assigned from
 * `process.env.TAURI_DEV_HOST`. It cannot evaluate a spread, a function call, a
 * value imported from another module, an environment interpolation inside the
 * literal, or a second `server`/`build` block merged by `mergeConfig`; those
 * would either fail to match (reported as an issue) or be read as written.
 * Brace matching does not skip a `{`/`}` inside a string; neither block has
 * one. The stripper is quote-aware for `'` and `"` but not for backticks, and
 * the brace walk does not track strings at all, so a template literal anywhere
 * above the two blocks could be mis-stripped or mis-matched. Rather than read
 * one wrong, a backtick in the stripped source is refused loudly — neither this
 * config nor the contract it states needs one.
 */
import { resolve } from 'node:path';
import { stripComments } from './css-invariants';
import { REPOSITORY_ROOT } from './scaffold-files';

export type TauriBuildTarget = {
  readonly devUrl: string;
  /** The explicit port from `devUrl`; a URL without one is a failure. */
  readonly port: string;
  /** `build.frontendDist` resolved against `src-tauri/`, as Tauri does. */
  readonly frontendDist: string;
  readonly frontendDistRaw: string;
};

export type ViteConfigFacts = {
  readonly hasServerBlock: boolean;
  readonly port?: number;
  /** The variable assigned from `process.env.TAURI_DEV_HOST`, if any. */
  readonly hostVariable?: string;
  /** Whether `server.host` is that variable (or reads the env directly). */
  readonly hostWired: boolean;
  /** `build.outDir` resolved against the repository root, where Vite resolves it. */
  readonly outDir?: string;
};

/**
 * Returns the inside of `key: { … }`, brace-balanced, or `undefined`.
 *
 * `source` must already have had its comments stripped — by the quote-aware
 * scanner in `css-invariants.ts`, not by a regex pair. That is a measured
 * choice: a regex that opens a block comment at the first slash-star is fooled
 * by the glob `src/routes/**` inside a line comment in this very config, and it
 * then blanks everything up to the star-slash in the `src-tauri` ignore glob —
 * the whole `server` block included. The scanner tracks strings and line
 * comments, so it stops where the language does.
 */
function objectBlock(source: string, key: string): string | undefined {
  const pattern = new RegExp(`\\b${key}\\s*:\\s*\\{`);
  const opener = pattern.exec(source);

  if (opener === null) {
    return undefined;
  }

  const open = opener.index + opener[0].length - 1;
  let depth = 0;

  for (let index = open; index < source.length; index += 1) {
    const character = source[index];

    if (character === '{') {
      depth += 1;
    } else if (character === '}') {
      depth -= 1;

      if (depth === 0) {
        return source.slice(open + 1, index);
      }
    }
  }

  return undefined;
}

/** Reads the two build values this check needs out of `tauri.conf.json`. */
export function parseTauriBuildTarget(tauriConf: string): TauriBuildTarget {
  const config = JSON.parse(tauriConf) as {
    build?: { devUrl?: unknown; frontendDist?: unknown };
  };
  const devUrl = config.build?.devUrl;
  const frontendDist = config.build?.frontendDist;

  if (typeof devUrl !== 'string') {
    throw new Error('tauri.conf.json has no string build.devUrl');
  }

  if (typeof frontendDist !== 'string') {
    throw new Error('tauri.conf.json has no string build.frontendDist');
  }

  const url = new URL(devUrl);

  if (url.port === '') {
    throw new Error(
      `tauri.conf.json's build.devUrl (${devUrl}) carries no explicit port, so there is nothing for server.port to agree with`,
    );
  }

  return {
    devUrl,
    port: url.port,
    frontendDistRaw: frontendDist,
    frontendDist: resolve(REPOSITORY_ROOT, 'src-tauri', frontendDist),
  };
}

/** Reads the `server.port`/`server.host` and `build.outDir` facts as text. */
export function parseViteConfig(source: string): ViteConfigFacts {
  const stripped = stripComments(source);

  // `stripComments` is quote-aware for `'` and `"` but not for backticks, and
  // `objectBlock` does not track strings at all, so a template literal above (or
  // inside) either block could break the comment stripping or the brace
  // matching without any assertion noticing. Refuse it loudly instead of
  // reading it wrong: a false green here is worse than a red that names the
  // construct.
  if (stripped.includes('`')) {
    throw new Error(
      'vite.config.ts contains a backtick; this check reads the file as text and does not model template literals (a `//` or `}` inside one would break the comment stripping or the brace matching)',
    );
  }

  const server = objectBlock(stripped, 'server');
  const build = objectBlock(stripped, 'build');

  const portMatch =
    server === undefined ? null : /\bport\s*:\s*(\d+)\b/.exec(server);
  const port = portMatch === null ? undefined : Number(portMatch[1]);

  const wiring =
    /(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*process\.env\.TAURI_DEV_HOST/.exec(
      stripped,
    );
  const hostVariable = wiring?.[1];

  let hostWired = false;

  if (server !== undefined) {
    if (hostVariable !== undefined) {
      const escaped = hostVariable.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      // A shorthand `host` may be the last property in the block, so the end
      // of the block closes it as legitimately as a comma or a colon.
      hostWired = new RegExp(`(?:^|[\\s,{])${escaped}\\s*(?:,|:|$)`).test(
        server,
      );
    } else {
      hostWired = /\bhost\s*:\s*process\.env\.TAURI_DEV_HOST/.test(server);
    }
  }

  const outDirMatch =
    build === undefined ? null : /\boutDir\s*:\s*(['"])([^'"]+)\1/.exec(build);

  return {
    hasServerBlock: server !== undefined,
    port,
    hostVariable,
    hostWired,
    outDir:
      outDirMatch === null
        ? undefined
        : resolve(REPOSITORY_ROOT, outDirMatch[2]),
  };
}

/** Every way the two configs disagree. An empty list is green. */
export function checkConfigAgreement(
  tauriConf: string,
  viteSource: string,
): string[] {
  let target: TauriBuildTarget;

  try {
    target = parseTauriBuildTarget(tauriConf);
  } catch (error) {
    return [error instanceof Error ? error.message : String(error)];
  }

  const vite = parseViteConfig(viteSource);
  const issues: string[] = [];

  if (!vite.hasServerBlock) {
    issues.push(
      'vite.config.ts has no literal `server: { … }` block, so server.port and server.host cannot be read',
    );
  }

  if (vite.port === undefined) {
    issues.push(
      `vite.config.ts has no literal server.port; tauri.conf.json's build.devUrl expects port ${target.port}`,
    );
  } else if (String(vite.port) !== target.port) {
    issues.push(
      `vite.config.ts server.port is ${vite.port} but tauri.conf.json's build.devUrl (${target.devUrl}) expects ${target.port}`,
    );
  }

  if (!vite.hostWired) {
    issues.push(
      'vite.config.ts server.host is not wired to process.env.TAURI_DEV_HOST (the iOS simulator and LAN devices cannot reach the dev server without it)',
    );
  }

  if (vite.outDir === undefined) {
    issues.push(
      'vite.config.ts has no literal build.outDir, so it cannot be compared with tauri.conf.json build.frontendDist',
    );
  } else if (vite.outDir !== target.frontendDist) {
    issues.push(
      `vite.config.ts build.outDir resolves to ${vite.outDir} but tauri.conf.json's build.frontendDist ("${target.frontendDistRaw}") resolves to ${target.frontendDist}`,
    );
  }

  return issues;
}
