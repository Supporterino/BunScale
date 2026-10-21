/*
 * Settings & Profile as its own screen (task 4.3; SET-1, HS-4, SET-2, SET-4).
 *
 * The dashboard's gear navigates here (task 4.2's `onOpenSettings` seam, wired
 * in `./index.tsx`). The route is thin on purpose: it renders the screen and
 * declares no loader, because the screen's reads are TanStack Query hooks that
 * render the cached value first and revalidate on mount — a loader would
 * duplicate that and hold the navigation on the service. `tanstack-router-
 * conventions` keeps route files to `createFileRoute` plus the component; the
 * layout and the copy belong to `SettingsScreen`.
 *
 * No `beforeLoad` guard: SET-1 opens Settings from the dashboard, and a deep
 * link to `/settings` is not a path P0 defines. The screen's own reads still
 * show the book's answer or the explicit empty state, so nothing here invents
 * a redirect the product spec does not ask for.
 */
import { createFileRoute } from '@tanstack/react-router';
import { SettingsScreen } from '../features/settings/components/SettingsScreen';

export const Route = createFileRoute('/settings')({
  component: SettingsScreen,
});
