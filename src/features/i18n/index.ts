/*
 * The i18n feature's public surface (task 3.7).
 *
 * Screens import `t` and `MessageKey` from here; they do not reach into
 * `./messages`. Keeping the entry point separate means a later locale switch
 * (P4/NFR-5) can add state behind the same import without touching call sites.
 *
 * German is not shipped in P0 — see `./messages` for why the scaffold stays
 * single-locale rather than half-translated.
 */
export type { MessageKey } from './messages';
export { P0_MESSAGES, t } from './messages';
