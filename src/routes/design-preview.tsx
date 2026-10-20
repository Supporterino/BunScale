/*
 * The design-system preview (task 0.21) — the scaffold's own surface, now on its
 * own route so `/` can be the household entry (task 3.3).
 *
 * It holds no IPC, no query and no router logic (see the component's header); it
 * is a development and design-review surface, kept reachable rather than deleted
 * so the primitives stay exercised until the P0 screens replace it. Task 4.2
 * decides whether the finished app keeps this route.
 */
import { createFileRoute } from '@tanstack/react-router';
import { DesignSystemPreview } from '../features/design-preview/components/DesignSystemPreview';

export const Route = createFileRoute('/design-preview')({
  component: DesignSystemPreview,
});
