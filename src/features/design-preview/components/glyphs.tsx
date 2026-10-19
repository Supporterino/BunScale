/*
 * Decorative glyphs for the placeholder screen.
 *
 * They live in the feature rather than in `src/components/ui/` on purpose: the
 * inventory is a closed list of 23 primitives and an icon set is not one of them
 * (a `Tooltip`, an `Avatar` and a chart primitive are the deferred neighbours of
 * that decision). A glyph is not a component with a contract — it is 20 px of
 * `currentColor` — and each screen supplies its own until a real icon set earns
 * a place in the inventory.
 *
 * Every glyph is `aria-hidden="true"`: the accessible name of an icon action is
 * the `label` on its `IconButton`, and the name of a navigation destination is
 * its visible text. A glyph that carried meaning by itself would be a
 * colour-and-shape-only signal, which the accessibility contract forbids.
 */

const GLYPH_SIZE = 20;

export function GlyphDashboard() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={GLYPH_SIZE}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 20 20"
      width={GLYPH_SIZE}
    >
      <rect height="6" rx="1.5" width="6" x="2.5" y="2.5" />
      <rect height="6" rx="1.5" width="6" x="11.5" y="2.5" />
      <rect height="6" rx="1.5" width="6" x="2.5" y="11.5" />
      <rect height="6" rx="1.5" width="6" x="11.5" y="11.5" />
    </svg>
  );
}

export function GlyphSettings() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={GLYPH_SIZE}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 20 20"
      width={GLYPH_SIZE}
    >
      <circle cx="10" cy="10" r="2.75" />
      <path d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M15.3 4.7l-1.4 1.4M6.1 13.9l-1.4 1.4" />
    </svg>
  );
}

export function GlyphBack() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={GLYPH_SIZE}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 20 20"
      width={GLYPH_SIZE}
    >
      <polyline points="12,4 6,10 12,16" />
    </svg>
  );
}

export function GlyphBell() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={GLYPH_SIZE}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 20 20"
      width={GLYPH_SIZE}
    >
      <path d="M5.5 8.5a4.5 4.5 0 0 1 9 0v3l1.5 2.5H4l1.5-2.5z" />
      <path d="M8.5 16.5a1.75 1.75 0 0 0 3 0" />
    </svg>
  );
}

export function GlyphTrash() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height={GLYPH_SIZE}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 20 20"
      width={GLYPH_SIZE}
    >
      <path d="M3.5 5.5h13" />
      <path d="M5.5 5.5V16a1.5 1.5 0 0 0 1.5 1.5h6a1.5 1.5 0 0 0 1.5-1.5V5.5" />
      <path d="M7.5 5.5V3.5h5v2" />
      <path d="M8.5 9v5M11.5 9v5" />
    </svg>
  );
}

export function GlyphHousehold() {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height="48"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 48 48"
      width="48"
    >
      <path d="M8 21 24 7l16 14" />
      <path d="M12 20v18a2 2 0 0 0 2 2h20a2 2 0 0 0 2-2V20" />
      <path d="M20 40V29h8v11" />
    </svg>
  );
}
