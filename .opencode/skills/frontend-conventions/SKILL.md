---
name: frontend-conventions
description: React + TypeScript conventions for the mobile-first Tauri frontend: structure, state, IPC, platform handling, lifecycle, accessibility. Load for any frontend work.
---
# Frontend conventions

## Structure

```
src/routes/              file routes only (thin)
src/features/<name>/
  api.ts                 ONLY place importing bindings
  queries.ts             TanStack Query hooks
  components/            feature UI
  *.test.tsx
src/components/ui/       design-system primitives
src/lib/platform.ts      platform + pointer detection helpers
```

## State

| Kind | Tool |
|---|---|
| Backend/IPC data | TanStack Query |
| Local UI state | `useState` / `useReducer` |
| Cross-component client state | small store (Zustand) only when justified |
| URL state | router search params |

No derived state stored. No prop drilling beyond 2 levels.

## IPC

- Components never call `invoke` or `bindings` directly.
- Mutations invalidate affected query keys; keys centralized per feature.
- Handle `AppError.kind` explicitly; user messages from a map.

## Mobile-first CSS

- Base styles = compact. Scale up with `min-width` queries. Never `max-width` for the mobile case.
- `100dvh`, safe-area insets on fixed bars, `overscroll-behavior: contain` in scroll regions.
- Hover and shortcut styles only inside `@media (hover: hover) and (pointer: fine)`.
- Long lists virtualized. Avoid layout thrash on scroll; animate `transform`/`opacity` only.

## Platform handling

- `src/lib/platform.ts` exposes `isMobile`, `isMacOS`, `isTouch`. No scattered UA sniffing.
- Platform-specific UI via small adapter components, not forked screens.

## Lifecycle and connectivity

- App can be suspended at any time on iOS. Persist drafts and in-flight state; refetch on foreground (`visibilitychange` / Tauri window focus event).
- Tolerate offline and flaky network: optimistic updates with rollback, clear error states.

## Events

```ts
useEffect(() => {
  const un = events.importProgress.listen(e => ...);
  return () => { un.then(f => f()); };
}, []);
```

Always clean up.

## TypeScript and a11y

- `strict`. No `any`; `unknown` + narrowing. Zod for untrusted data.
- Semantic elements, labels, AA contrast, Dynamic Type friendly units, VoiceOver names. Keyboard operable on desktop.
