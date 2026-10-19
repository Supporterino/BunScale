---
name: design-system
description: Token architecture, component specifications, and slide generation. Three-layer tokens (primitive→semantic→component), CSS variables, spacing/typography scales, component specs, strategic slide creation. Use for design tokens, systematic design, brand-compliant presentations.
argument-hint: "[component or token]"
license: MIT
metadata:
  author: claudekit
  version: "1.0.0"
# BunScale-local status line, added by task 0.20. The upstream frontmatter
# above is unchanged. This is the machine-readable line the orchestrator's UI
# precondition reads (TOOL-6, design.md D0.10); its exact text is
# `status: frozen`.
status: frozen
---

# Design System

Token architecture, component specifications, systematic design, slide generation.

## When to Use

- Design token creation
- Component state definitions
- CSS variable systems
- Spacing/typography scales
- Design-to-code handoff
- Tailwind theme configuration
- **Slide/presentation generation**

## Token Architecture

Load: `references/token-architecture.md`

### Three-Layer Structure

```
Primitive (raw values)
       ↓
Semantic (purpose aliases)
       ↓
Component (component-specific)
```

**Example:**
```css
/* Primitive */
--color-blue-600: #2563EB;

/* Semantic */
--color-primary: var(--color-blue-600);

/* Component */
--button-bg: var(--color-primary);
```

## Quick Start

**Generate tokens:**
```bash
node scripts/generate-tokens.cjs --config tokens.json -o tokens.css
```

**Validate usage:**
```bash
node scripts/validate-tokens.cjs --dir src/
```

## References

| Topic | File |
|-------|------|
| Token Architecture | `references/token-architecture.md` |
| Primitive Tokens | `references/primitive-tokens.md` |
| Semantic Tokens | `references/semantic-tokens.md` |
| Component Tokens | `references/component-tokens.md` |
| Component Specs | `references/component-specs.md` |
| States & Variants | `references/states-and-variants.md` |
| Tailwind Integration | `references/tailwind-integration.md` |

## Component Spec Pattern

| Property | Default | Hover | Active | Disabled |
|----------|---------|-------|--------|----------|
| Background | primary | primary-dark | primary-darker | muted |
| Text | white | white | white | muted-fg |
| Border | none | none | none | muted-border |
| Shadow | sm | md | none | none |

## Scripts

| Script | Purpose |
|--------|---------|
| `generate-tokens.cjs` | Generate CSS from JSON token config |
| `validate-tokens.cjs` | Check for hardcoded values in code |
| `search-slides.py` | BM25 search + contextual recommendations |
| `slide-token-validator.py` | Validate slide HTML for token compliance |
| `fetch-background.py` | Fetch images from Pexels/Unsplash |

## Templates

| Template | Purpose |
|----------|---------|
| `design-tokens-starter.json` | Starter JSON with three-layer structure |

## Integration

**With brand:** Extract primitives from brand colors/typography
**With ui-styling:** Component tokens → Tailwind config

**Skill Dependencies:** brand, ui-styling
**Primary Agents:** ui-ux-designer, frontend-developer

## Slide System

Brand-compliant presentations using design tokens + Chart.js + contextual decision system.

### Source of Truth

| File | Purpose |
|------|---------|
| `docs/brand-guidelines.md` | Brand identity, voice, colors |
| `assets/design-tokens.json` | Token definitions (primitive→semantic→component) |
| `assets/design-tokens.css` | CSS variables (import in slides) |
| `assets/css/slide-animations.css` | CSS animation library |

### Slide Search (BM25)

```bash
# Basic search (auto-detect domain)
python scripts/search-slides.py "investor pitch"

# Domain-specific search
python scripts/search-slides.py "problem agitation" -d copy
python scripts/search-slides.py "revenue growth" -d chart

# Contextual search (Premium System)
python scripts/search-slides.py "problem slide" --context --position 2 --total 9
python scripts/search-slides.py "cta" --context --position 9 --prev-emotion frustration
```

### Decision System CSVs

| File | Purpose |
|------|---------|
| `data/slide-strategies.csv` | 15 deck structures + emotion arcs + sparkline beats |
| `data/slide-layouts.csv` | 25 layouts + component variants + animations |
| `data/slide-layout-logic.csv` | Goal → Layout + break_pattern flag |
| `data/slide-typography.csv` | Content type → Typography scale |
| `data/slide-color-logic.csv` | Emotion → Color treatment |
| `data/slide-backgrounds.csv` | Slide type → Image category (Pexels/Unsplash) |
| `data/slide-copy.csv` | 25 copywriting formulas (PAS, AIDA, FAB) |
| `data/slide-charts.csv` | 25 chart types with Chart.js config |

### Contextual Decision Flow

```
1. Parse goal/context
        ↓
2. Search slide-strategies.csv → Get strategy + emotion beats
        ↓
3. For each slide:
   a. Query slide-layout-logic.csv → layout + break_pattern
   b. Query slide-typography.csv → type scale
   c. Query slide-color-logic.csv → color treatment
   d. Query slide-backgrounds.csv → image if needed
   e. Apply animation class from slide-animations.css
        ↓
4. Generate HTML with design tokens
        ↓
5. Validate with slide-token-validator.py
```

### Pattern Breaking (Duarte Sparkline)

Premium decks alternate between emotions for engagement:
```
"What Is" (frustration) ↔ "What Could Be" (hope)
```

System calculates pattern breaks at 1/3 and 2/3 positions.

### Slide Requirements

**ALL slides MUST:**
1. Import `assets/design-tokens.css` - single source of truth
2. Use CSS variables: `var(--color-primary)`, `var(--slide-bg)`, etc.
3. Use Chart.js for charts (NOT CSS-only bars)
4. Include navigation (keyboard arrows, click, progress bar)
5. Center align content
6. Focus on persuasion/conversion

### Chart.js Integration

```html
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>

<canvas id="revenueChart"></canvas>
<script>
new Chart(document.getElementById('revenueChart'), {
    type: 'line',
    data: {
        labels: ['Sep', 'Oct', 'Nov', 'Dec'],
        datasets: [{
            data: [5, 12, 28, 45],
            borderColor: '#FF6B6B',  // Use brand coral
            backgroundColor: 'rgba(255, 107, 107, 0.1)',
            fill: true,
            tension: 0.4
        }]
    }
});
</script>
```

### Token Compliance

```css
/* CORRECT - uses token */
background: var(--slide-bg);
color: var(--color-primary);
font-family: var(--typography-font-heading);

/* WRONG - hardcoded */
background: #0D0D0D;
color: #FF6B6B;
font-family: 'Space Grotesk';
```

### Reference Implementation

Working example with all features:
```
assets/designs/slides/claudekit-pitch-251223.html
```

### Command

```bash
/slides:create "10-slide investor pitch for ClaudeKit Marketing"
```

## Best Practices

1. Never use raw hex in components - always reference tokens
2. Semantic layer enables theme switching (light/dark)
3. Component tokens enable per-component customization
4. Use HSL format for opacity control
5. Document every token's purpose
6. **Slides must import design-tokens.css and use var() exclusively**

---

## BunScale project record — the design system is frozen (task 0.20)

> **Provenance.** The frontmatter above and the generic guidance in the rest of
> this file are the upstream `claudekit` skill (MIT, v1.0.0). Everything from
> this heading down is **this repository's record**, added by `frontend-ui` in
> task 0.20 of `add-shared-household-foundation`. It does not rewrite the
> upstream text. The one addition to the frontmatter is the status line, and it
> is commented as a BunScale-local line.

### Status line

The machine-readable line is:

```
status: frozen
```

It sits in the YAML frontmatter, after the upstream `metadata` block and before
the closing `---`. This is the line the orchestrator's UI precondition reads
(`design.md` D0.10, TOOL-6). "Frozen" means the three token layers and the
shared component inventory exist; task 0.21 builds primitives from the
inventory, and task 0.22 draws the app mark from the tokens. A new component or
token is added to this record **before** it is built, not after.

### Direction (from `ui-ux-pro-max`, direction only)

BunScale is a **native-feeling iOS-first app that scales to macOS. It is not a
website.** The direction used here:

- **Platform, not page.** One route tree; compact navigation is a bottom tab
  bar, expanded navigation is a sidebar. No marketing hero, no desktop-first
  chrome, no hover-first interaction.
- **Baselines.** Design compact at **390×844** first, then **1280×800**. Medium
  (≥768) and expanded (≥1200) widen gutters and content; they do not fork the
  route tree.
- **Apple HIG language.** SF system UI font, the familiar text-style scale
  (large title → caption 2), a 4/8 pt spacing rhythm, sparse elevation,
  continuous-corner radii, pressed feedback within ~150 ms, safe areas, and a
  44 pt touch baseline.
- **Palette.** Warm carrot orange is the brand accent (rabbit, care, a little
  playfulness); warm neutrals keep long health screens calm; health green,
  alert red, warning amber and information sky carry status. The accent is
  split into a deeper tone for text and filled controls (AA on light) and a
  lighter tone for dark mode (AA on dark).
- **Legibility over decoration.** Health and weight data is the content; the
  system avoids translucency, gradients, decorative loops and ornamental
  shadows that would weaken contrast or motion safety.

The skill's Python search script could not run in this session because the
`python3` shell permission is denied. The direction above therefore comes from
the skill's Quick Reference / Common Rules tables and its local data files
(`data/colors.csv` rows "Pet Tech App" and "Healthcare App",
`data/app-interface.csv` for touch targets, safe areas, Dynamic Type, reduced
motion and bottom-tab limits), read directly. No script output is claimed.

### Source of truth and format

```
src/styles/
├── primitives.css   Layer 1 — raw values only
├── semantic.css     Layer 2 — purpose, themes, platform facts
├── components.css   Layer 3 — component decisions
└── index.css        Entry point; imports the three layers in order
```

**Format decision: hand-authored CSS custom properties.** The reasons:

1. The runtime consumers are CSS and inline `var(--token)` references. Custom
   properties are the native, zero-runtime form; no dependency, no build step
   (rule 6 / TOOL-8), no CSS-in-JS.
2. The files are plain text, so they are inspectable and assertable without a
   browser. Task 0.20's verification parses them, resolves every `var()` chain
   and computes contrast; a later test can do the same.
3. Task 0.22 can read `src/styles/primitives.css` and `semantic.css` in Node and
   copy the exact `--app-mark-*` values into the source artwork. No JS mirror is
   needed for a build-time drawing.
4. A hand-maintained TS mirror would duplicate every value and drift. A
   generated bridge is not added because it would introduce a generator and a
   generated artefact for no current runtime consumer. If a JS consumer later
   needs resolved token values at runtime, add a small typed bridge then, with a
   test that compares it to the stylesheet; do not add a second source now.

**Layer rules** (reviewable by reading the files):

- `primitives.css` is the only file with raw palette colours, type sizes,
  spacing, radii, durations and easing curves. It contains only `:root` and
  custom properties.
- `semantic.css` and `components.css` reference primitives through `var()`.
  They contain no raw palette colour, type size, radius or duration.
- Component tokens may hold a component-specific scalar (an opacity, a 22 px
  badge height, the mark's 160 px inset). That is a component decision, not a
  new palette or type scale.
- No element selectors and no `color-scheme` live in the token layers.
  Importing them must not restyle an unstyled screen. Base/reset styles are
  task 0.21's (`src/styles/base.css`).

**Import status: imported, inside task 0.20's own commit.** `src/styles/index.css`
is the single entry point, and `src/main.tsx` imports it with one line:

```ts
import './styles/index.css';
```

The line is `frontend-logic`'s, not `frontend-ui`'s (hard rule 5: one owner per
path), and the orchestrator had `frontend-logic` land it **in this task's
commit** rather than deferring it: the bundler then validates the whole token
chain inside the task that created it, instead of task 0.21 discovering a broken
chain in an import it was never asked to review. `bun run build` therefore walks
161 modules (160 before the import) and emits `dist/assets/index-*.css` plus a
`<link rel="stylesheet" …>` in `dist/index.html`; `src/styles/index.css`'s own
header describes the same imported state. The layers are still visually inert —
custom properties alone restyle nothing, and no layer holds an element selector
or a reset — so the scaffold's unstyled placeholder markup and the screenshot
debt are unchanged (the first screenshots come from 0.21). Task 0.21 still owns
the element-level layer: `src/styles/base.css` for the reset, `body`
background/text, font smoothing, `color-scheme`, default focus and scroll
behaviour, imported from `index.css` after the three token layers — never into
them.

### Token layers

**Layer 1 — primitives.** Colour scales (`orange`, `neutral` including alpha
steps, `green`, `red`, `amber`, `sky`); spacing `--space-0` … `--space-20` on a
4 pt rhythm; type sizes `--font-size-11` … `--font-size-34` in `rem`; weights
400/500/600/700; unitless line heights and tracking; radii 0 … 20 plus full;
durations 0/150/250/350 ms; four easing curves; three shadows; icon sizes;
44 px touch target; 1 px and 2 px borders; breakpoints 768/1200.

**Layer 2 — semantic.** Colour roles (`background`, `surface`, `surface-sunken`,
`surface-elevated`, `text`, `text-secondary`, `text-tertiary`,
`text-disabled`, `text-inverse`, `border`, `border-control`, `primary`,
`primary-pressed`, `primary-subtle`, `primary-subtle-pressed`,
`primary-subtle-border`, `on-primary`, plus success/warning/danger/info and
their `-subtle`, `-subtle-border` and `on-...-subtle` pairs, `focus-ring`,
`scrim`, `selection-background`, and the fixed `brand-*` trio for the app mark).
Thirteen type styles, each with size/line-height/weight/tracking tokens. Spacing
rhythm. Layout: content max 720 (880 expanded), form max 560, sidebar 260, nav
bar 44, tab bar 49 + bottom safe area. Platform facts: `safe-area-inset-*`,
`keyboard-inset`, `text-scale`, `layout-viewport-height`, `focus-ring`. Dark
theme overrides only semantic colour names; reduced motion overrides only
duration and press-scale tokens; medium/expanded override only spacing and
content width.

**Layer 3 — component.** Shared control behaviour; Button; IconButton;
field/input; segmented control; list/list row/section header; card; navigation
bar; tab bar; sidebar; notice; badge; empty/error state; loading indicator; app
mark. The full list is the set of `--<component>-*` names in
`src/styles/components.css`; a primitive reads those names and does not invent
new values.

### Platform facts (token-level, so 0.21 has nothing to invent)

| Fact | Token(s) | Rule |
|---|---|---|
| iOS safe areas | `--safe-area-inset-top/right/bottom/left` = `env(safe-area-inset-*, 0px)` | Every fixed bar adds the matching inset; every scroll region reserves it. `index.html` already declares `viewport-fit=cover`. |
| Touch target | `--size-touch-target: 44px`, `--size-touch-spacing: 8px` | 44 CSS px = 44 pt on iOS. A 20 px glyph still gets a 44×44 hit area. Adjacent targets keep ≥8 px. macOS keeps the same baseline; a fine pointer may get denser *visual* treatment but not a smaller hit area. |
| Dynamic Type / preferred size | `--text-scale: 1`, applied in every `--text-*-size` via `calc(rem × scale)`; unitless line heights | Components never set a `px` font size. Set `--text-scale` once at the root from the preferred content-size category (or an in-app control). Test at 1.5× and 2.0×; allow wrapping, never fixed-height text boxes. |
| Keyboard inset | `--keyboard-inset: 0px` | Set once by the app shell from `visualViewport` when the iOS keyboard is up; `Screen` pads its bottom by it; focused inputs scroll into view; no fixed footer under the keyboard. No UI component measures the keyboard itself. |
| AA contrast | semantic text pairs | Normal text ≥4.5:1, large text ≥3:1, control boundaries and focus rings ≥3:1. Measured pairs below. Disabled text is exempt but must still expose its label. |
| Hover / keyboard | — | No hover-only affordance. Hover styles only inside `@media (hover: hover) and (pointer: fine)`. Pressed feedback exists on touch. Focus rings use `--focus-ring` + `--focus-ring-offset` with `:focus-visible`; keyboard operation is not an enhancement-only path. |
| Reduced motion | `--duration-*` → 0 ms, `--motion-press-scale` → 1 | Components must read the duration tokens; a literal duration would bypass the override. Loading indicators may loop but must still be legible when motion is reduced. |
| Dark mode | `prefers-color-scheme: dark` overrides semantic names | Components consume semantic/component tokens only; no dark-specific component branch. `color-scheme` is a base-layer declaration in 0.21, not a token. |
| iOS 15.0 floor | — | No `color-mix()`, `oklch()`, `@layer`, `linear()` easing or container queries. `100dvh` is a progressive `@supports` upgrade over `100vh`. |

**Measured AA pairs** (Node-side resolution of the stylesheet; see the
verification record):

| Pair (foreground on background) | Light | Dark | Minimum |
|---|---:|---:|---:|
| `text` on `background` | 16.74:1 | 18.92:1 | 4.5 |
| `text` on `surface` | 17.49:1 | 16.74:1 | 4.5 |
| `text` on `surface-sunken` | 16.03:1 | 18.92:1 | 4.5 |
| `text-secondary` on `background` | 7.30:1 | 13.26:1 | 4.5 |
| `text-secondary` on `surface-sunken` | 6.99:1 | 13.26:1 | 4.5 |
| `text-tertiary` on `background` | 5.12:1 | 7.83:1 | 4.5 |
| `text-tertiary` on `surface` | 5.35:1 | 6.93:1 | 4.5 |
| `text-tertiary` on `surface-sunken` | 4.90:1 | 7.83:1 | 4.5 |
| `primary` on `background` | 4.96:1 | 11.71:1 | 4.5 |
| `primary` on `surface-sunken` | 4.75:1 | 11.71:1 | 4.5 |
| `primary` on `primary-subtle` | 4.88:1 | 9.28:1 | 4.5 |
| `primary` on `primary-subtle-pressed` | 4.52:1 | 5.56:1 | 4.5 |
| `on-primary` on `primary` | 5.18:1 | 11.71:1 | 4.5 |
| `danger` on `background` | 6.19:1 | 7.14:1 | 4.5 |
| `danger` on `surface-sunken` | 5.93:1 | 7.14:1 | 4.5 |
| `on-danger` on `danger` | 6.47:1 | 7.14:1 | 4.5 |
| `success` on `background` | 5.25:1 | 10.28:1 | 4.5 |
| `warning` on `background` | 4.81:1 | 11.83:1 | 4.5 |
| `info` on `background` | 5.68:1 | 11.85:1 | 4.5 |
| `on-success-subtle` on `success-subtle` | 7.29:1 | 6.38:1 | 4.5 |
| `on-danger-subtle` on `danger-subtle` | 7.60:1 | 5.28:1 | 4.5 |
| `on-warning-subtle` on `warning-subtle` | 6.84:1 | 6.29:1 | 4.5 |
| `on-info-subtle` on `info-subtle` | 7.09:1 | 5.67:1 | 4.5 |
| `border-control` on `surface` | 4.80:1 | 3.65:1 | 3.0 |
| `focus-ring` on `background` | 3.92:1 | 11.85:1 | 3.0 |

The tightest normal-text pairs are `primary` on `primary-subtle-pressed`
(4.52:1) and `primary` on `surface-sunken` (4.75:1); both pass AA, but they are
for short accents and labels, not long body copy. Body copy uses `--text-body`
with `--color-text`. `--color-danger` is red-700 in the light theme and
`--color-text-tertiary` is a dedicated neutral-550, which is what makes both
pass on all three light surfaces.

### Shared component inventory for task 0.21

Build these from `src/components/ui/`. Each is a primitive, not a screen; no
feature logic or IPC calls. The "P0 use" column says why it exists now.

**Count: 21 rows covering 23 primitives.** The table below has **21 rows**, and
`List`, `ListSection` and `ListRow` share one of them, so the table names **23**
primitives to build. Task 0.21 checks off the **23 names** — that is the list
that answers "did I build them all?", and it is the only count used here. An
earlier "22 primitives" figure (in task 0.21's brief in `tasks.md`) matches
neither the 21 rows nor the 23 names; this paragraph supersedes it.

| Component | P0 use | Variants / states | Accessibility contract |
|---|---|---|---|
| `Text` | All copy | `large-title`, `title-1/2/3`, `headline`, `body`, `callout`, `subheadline`, `footnote`, `caption-1/2`; tone primary/secondary/tertiary/danger | Renders a semantic element via `as`; no heading semantics by default; tone is not the only signal. |
| `Heading` | Screen and section titles | `large-title`, `title-1/2/3`, `section-header`; `as` h1–h3 | Heading levels must not skip; one h1 per screen; section headers are real headings, not styled divs. |
| `VisuallyHidden` | Icon-button labels, live-region text | — | Screen-reader-only text that never becomes invisible to AT; never used to hide focusable content. |
| `Screen` | Every screen's scroll container | `scroll` / `fixed`; bottom inset from tab bar + keyboard | Uses `--layout-viewport-height`, safe-area and `--keyboard-inset`; `overscroll-behavior: contain`; content is not hidden behind bars. |
| `AppShell` | Dashboard, Settings, onboarding | Compact tab bar; expanded sidebar; skip link | One `<main>`, one `<nav aria-label="Primary">`; skip link is the first focusable element and becomes visible on focus; the same destinations in both arrangements. |
| `NavigationBar` | Screen title, gear, back | Title; leading action; trailing action(s) | `<header>` with the screen's `h1`; every icon action is an `IconButton` with a label; back is labelled "Back". |
| `TabBar` | Compact navigation | 3–5 destinations; active/inactive | `nav` + links with `aria-current="page"`; icon + visible label; ≥44×44 per item; bottom safe area; no more than five. |
| `Sidebar` | Medium/expanded navigation | Same destinations; active/inactive | `nav` + links with `aria-current="page"`; 44 px rows; visible focus ring; no hover-only expansion. |
| `Button` | Create household, retry, save | `primary`, `secondary`, `plain`, `destructive`; regular/compact; `loading`, `disabled`, full-width | Native `button`; visible label (or `aria-label` only when the design truly has no room); `aria-busy` while loading and the label stays; ≥44 px; pressed feedback; disabled uses `disabled`, not just opacity. |
| `IconButton` | Gear, close, back, retry icon | `default`, `pressed`, `disabled` | Required `label` prop → accessible name; glyph `aria-hidden`; 44×44 hit area even for a 20 px icon; no tooltip-only label. |
| `FormField` | Wraps any labelled control | Label, hint, error, required | Real `<label for>`; hint/error linked with `aria-describedby`; `aria-invalid` on error; error text is not colour-only; required is announced. |
| `TextField` | Household name, account name | Text/number/search; hint/error/disabled; `inputMode`, `enterKeyHint`, `autoComplete` | Label always visible (placeholder is not a label); 44 px min; focus ring; error announced; selection colours from tokens; keyboard type set for the data. |
| `SegmentedControl` | Weight unit kg/lb | 2–4 segments; selected/unselected; disabled | `radiogroup` with radio semantics; arrow-key movement on desktop; selected state has a non-colour cue; 44 px tall; label from `FormField`. |
| `List` + `ListSection` + `ListRow` | Households, settings rows | Static / navigational / control rows; checkmark/chevron accessory; selected; separator | Real list semantics; interactive rows are `button`/`a`, not clickable `div`s; 44 px min; `aria-current` for the active household; separators are decorative; no swipe-only action. |
| `Card` | Dashboard summary, grouped surface | Static; optional heading | Surface separation with tokens; if it has a heading, `aria-labelledby`; a card is not a link unless the whole card is one control. |
| `Divider` | Separator inside lists/cards | Horizontal | `<hr>` or `role="separator"`; decorative separators are `aria-hidden`; never the only grouping cue. |
| `Notice` | Offline, save failure, success | `info`, `success`, `warning`, `danger`; title + body; optional action | `role="status"` for non-urgent updates, `role="alert"` for a failed save; icon is decorative; persistent until resolved (no auto-dismiss toast); cause-specific copy, never "something went wrong". |
| `Badge` | Connection state, weight unit | Neutral/success/warning/danger/info | Non-interactive; text label always present (never colour/dot alone); AA text on its subtle background; not a substitute for a notice. |
| `EmptyState` | No households yet | Icon, title, body, optional action | Real heading, explanatory body, one clear action; not an error; announced only when it replaces content after an action. |
| `ErrorState` | "Nothing ever loaded + unreachable" | Cause-specific title/body; retry when retryable; request id | Distinct from `EmptyState`; `role="alert"` when it appears after an action; retry is a real button; request id is available but not the primary copy. |
| `LoadingIndicator` | Initial load, save in progress | Inline / screen; optional label | `role="status"` + `aria-live="polite"`; `aria-busy` on the affected region; reduced-motion-safe; never the only feedback for a button (the button also shows loading). |

#### Deferred primitives (not built in 0.21)

Recorded so a later task adds the inventory entry first, rather than inventing
a component: `Switch`, `Select`/`Picker`, `TextArea`, `Sheet`/`Dialog`/`Alert`,
`Toast`, `Tooltip`, `Avatar`, `ProgressView`, chart primitives, pull-to-refresh.
Each has no P0 screen that needs it; the first real consumer is P1/P3 or later.
A sheet or alert will also need a scrim and focus trap, which is why it is not a
"free" primitive.

### Global conventions for every primitive

1. **Tokens only.** No raw hex, px font size, radius or duration in
   `src/components/ui/`. Use semantic or component tokens. A new value is added
   to the layers first, then recorded here.
2. **Touch.** Every interactive primitive is ≥44×44 px, with ≥8 px between
   adjacent targets. Enlarge the hit area with padding or a pseudo-element when
   the visual is smaller; never overlap a neighbour.
3. **Safe areas.** Fixed bars add `--safe-area-inset-*`; scroll regions reserve
   the same space with padding or `scroll-padding`; no tappable content under
   the notch or home indicator.
4. **Keyboard.** `Screen` accounts for `--keyboard-inset`; inputs scroll into
   view on focus; no fixed action bar hidden behind the keyboard; use
   `enterKeyHint` and `inputMode` for the data.
5. **AA contrast.** Text ≥4.5:1, large text ≥3:1, non-text controls ≥3:1. Every
   state (pressed, selected, error, disabled) uses a defined pair; disabled is
   the only exempt state and must still be labelled.
6. **Screen reader.** Every control has a name; every icon-only control gets it
   from a required `label`; decorative icons are `aria-hidden`; state is
   exposed (`aria-current`, `aria-pressed`, `aria-checked`, `aria-busy`,
   `aria-invalid`), not just painted.
7. **Live updates.** Async status uses `role="status"` (polite); a failed save
   uses `role="alert"` (assertive) and names the cause. Do not announce every
   keystroke.
8. **No hover-only affordance.** Hover is a progressive enhancement under
   `@media (hover: hover) and (pointer: fine)`. The same action is reachable by
   touch and by keyboard. Shortcuts are enhancements, never the only path.
9. **Focus.** `:focus-visible` uses `outline: var(--focus-ring)` with
   `outline-offset: var(--focus-ring-offset)`. Never remove the outline without
   replacing it; focus order follows visual order; no positive `tabindex`.
10. **Motion.** Use `--duration-*` and `--ease-*`; animate `transform` and
    `opacity` (and colour) only; no layout-shifting press effects; no infinite
    decorative loops. Reduced motion is handled by the token override.
11. **Dynamic Type.** Text sizes come from `--text-*`; line heights are
    unitless; no fixed-height text containers; test at 1.5× and 2×; truncation
    must not hide an essential value or label.
12. **Mobile first.** Base styles are compact; scale up with `min-width`. Never
    use `max-width` for the compact case. One route tree; navigation adapts,
    routes do not fork.
13. **Platform.** Detect with `src/lib/platform.ts` when behaviour really
    differs; prefer CSS adaptation. Do not fork a screen for iOS vs macOS.

### App mark contract for task 0.22 (D0.19)

- Source artwork: **1024×1024**, full-bleed square, **no rounded corners** (iOS
  applies its own mask), glyph inside the 160 px clear-space inset. A
  transparent source is allowed, but the tile itself must be opaque.
- Colours, read from the tokens:
  - tile `--app-mark-tile` = `--color-brand-tile` = `#c2410c`
  - glyph `--app-mark-glyph` = `--color-brand-glyph` = `#ffffff`
  - accent detail `--app-mark-accent` = `--color-brand-accent` = `#fed7aa`
- Generate with the pinned CLI: `bun run tauri icon <source-artwork>`. No global
  `cargo tauri`, no new dependency, no new permission.
- These brand tokens are intentionally **fixed** across light/dark: a platform
  icon cannot follow the theme.

### Deliberately left out of the frozen system

- No component implementations and no base/reset: task 0.21 owns both.
- No in-app theme switch or `[data-theme]` override: P0 has no theme setting;
  `prefers-color-scheme` is the only switch. The semantic names are ready for a
  later override.
- No TS token mirror, generated token JSON, Tailwind config or CSS-in-JS: no
  runtime JS consumer exists, and a mirror/generator would add a dependency or a
  drift surface. 0.22 reads the CSS at build time.
- No HSL conversion for opacity: the upstream skill suggests HSL, but the
  palette is authored as exact hex values and alpha is expressed with the
  `--color-neutral-950-alpha-*` rgb primitives. The iOS 15 floor supports
  `rgb(r g b / a)`, and keeping hex makes the brand values unambiguous.
- No web font: the system UI font is the native choice and avoids a bundled
  asset. `--font-family-mono` is only for measurements.
- No `color-mix()`, `oklch()`, `@layer`, container queries or `linear()` easing:
  the iOS 15.0 / macOS 12.0 floor does not support them reliably.
- No translucent bars or material blur: contrast over arbitrary content cannot
  be guaranteed, so bars use solid semantic surfaces.
- No toast, sheet, alert, picker, switch or chart primitive yet: no P0 screen
  needs one (see "Deferred primitives").
- No medium/expanded route variations: the same route tree adapts.
- No screenshots: no rendered surface exists until task 0.21.

### Verification record for this record

- `bun run lint` — 16 checked files, no diagnostics (the four new CSS files are
  inside the allow-list). Run twice with no edit between; identical.
- `bun run typecheck` — exit 0.
- `bun run build` — exit 0; 161 modules transformed, and the token chain is
  bundled rather than inert: `dist/assets/index-*.css` (22.21 kB, 4.06 kB gzip)
  is emitted and linked from `dist/index.html` with `<link rel="stylesheet" …>`.
  `src/routeTree.gen.ts` and `src/bindings.ts` show no diff.
- `bun run gen:bindings` — exit 0; the exporter itself prints nothing on
  success (cargo prints its own build/run lines), and the regenerated file has
  no diff.
- A Node-side parse of the three layer files resolved **484 token entries in
  each theme** with no unresolved `var()`, no missing reference and no cycle;
  the contrast table above is its output. It also read back:
  `--color-primary: #c2410c`, `--color-danger: #b91c1c`,
  `--color-text-tertiary: #6f6a65`, `--text-body-size: calc(1.0625rem * 1)`,
  `--space-4: 16px`, `--radius-card: 16px`, `--duration-fast: 150ms`,
  `--size-touch-target: 44px`, `--safe-area-inset-top: env(safe-area-inset-top,
  0px)`, `--keyboard-inset: 0px`, `--app-mark-tile: #c2410c`.
- **Not verified:** browser rendering, computed styles via `getComputedStyle`,
  screenshots, iOS/macOS native appearance, and the emitted `dist` CSS asset
  itself — its bytes were not read back or rendered, only its emission, its size
  and its `<link>` from `dist/index.html` are recorded above. No visual claim is
  made. (The tokens are imported and bundled — see "Import status" — but nothing
  consumes them visually until 0.21.)
- `bun run test` / `bun run test:e2e` remain red for the empty-suite reasons
  owned by tasks 0.23/0.24/0.25; this task adds no specs.

