import { QueryClient } from '@tanstack/react-query';

/*
 * The application's single query cache, created once at module scope rather
 * than per render: the router carries this instance as its `context` and the
 * provider in `src/main.tsx` hands the same one to the component tree, so a
 * cache created inside a component would be replaced on every render and would
 * disagree with the router's. Keeping the creation in its own module also
 * avoids an import cycle between `src/main.tsx` and `src/router.tsx`.
 *
 * The defaults below are the ones that carry meaning for this app's read/write
 * model; everything else stays at the library default deliberately:
 *
 * - `retry: false` on both queries and mutations. The bounded attempts and the
 *   timeouts are owned by the Rust side (`REACHABILITY_PROBE_TIMEOUT`,
 *   `SAVE_ATTEMPT_TIMEOUT`, design.md D0.13), and a failure is meant to reach
 *   the person with its cause and an explicit retry (NFR-8, D0.6). An
 *   automatic cache-level retry would stack a second, invisible attempt policy
 *   on top of those bounds and would hide the cause the failure surface exists
 *   to show. That the test harness task 0.23 also uses `retry: false` is a
 *   separate reason — deterministic tests — not the reason here.
 * - `refetchOnWindowFocus: false`. Freshness is owned by the locally kept copy
 *   plus the explicit refresh on start and on foreground return (D0.2, SYNC-3),
 *   driven by the Rust events task 3.6 subscribes to; a focus refetch would be a
 *   second, invisible refresh policy that P6 (`add-read-path-performance`) would
 *   then have to reason about.
 * - No `staleTime` override: reads resolve against the in-process local copy
 *   (D0.2), so revalidating on mount is cheap, and invalidation is what task 3.2
 *   and 3.6 use to drive a refresh once a load or a `household:changed` event
 *   has landed. Tuning staleness is P6's subject.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: false,
    },
  },
});
