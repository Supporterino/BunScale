/*
 * The weight entry detail route (task 4.19; WT-8, D1.2).
 *
 * One entry, named by the route's typed `entryId` parameter, with the context
 * the detail cannot derive carried in typed search parameters — validated by
 * `validateWeightEntrySearch` in `../features/weight/search`, beside task
 * 4.18's `validateWeightFormSearch` and for the same reason:
 *
 * - `rabbit` is the entry's rabbit. The detail read (`get_weight_entry`) takes
 *   the entry id only and answers the entry, but this feature's cache entry is
 *   keyed under the rabbit (`../features/weight/keys`), because a rabbit write's
 *   threshold edit invalidates that rabbit's series and nothing else — so the
 *   scope is the caller's, and a detail opened from a row already knows it.
 * - `from` is the surface Back (and later Edit and Delete) return to, with the
 *   dashboard's two windows (`trend`, `history`) so a return lands on the view
 *   the person left (WT-6, D1.4). A blank `rabbit` falls back to the list, the
 *   way 4.18's form does: without a rabbit there is no rabbit dashboard to
 *   return to.
 *
 * Validation never fails the navigation: an absent or misspelled value becomes
 * the ordinary default instead of an error page over a stale deep link. The
 * range pair reuses the frozen `validateWeightRangeSearch` rather than restating
 * its four windows. A blank `rabbit` is a documented limit, not a fix here: the
 * detail is then keyed outside every `series(rabbitId)` invalidation, and task
 * 4.21 must read the rabbit from the entry before trusting its drop flag
 * (`../features/weight/search` and `../features/weight/keys` say why).
 *
 * No loader and no guard, matching the weight form route's reasoning: the
 * screen's read is a TanStack Query hook that presents cached data first and
 * revalidates on mount, and a failure is the screen's own read surface. A blank
 * or unknown entry id is the backend's `validation` refusal rather than a
 * redirect the product spec does not state.
 *
 * It is not under an adaptive layout fork: `WeightEntryDetailScreen` renders the
 * same `AppShell` every other surface uses, so iOS and macOS share this node.
 */
import { createFileRoute } from '@tanstack/react-router';
import { WeightEntryDetailScreen } from '../features/weight/components/WeightEntryDetailScreen';
import { validateWeightEntrySearch } from '../features/weight/search';

export const Route = createFileRoute('/weights/$entryId')({
  validateSearch: validateWeightEntrySearch,
  component: WeightEntryDetailRoute,
});

function WeightEntryDetailRoute() {
  const { entryId } = Route.useParams();
  const search = Route.useSearch();

  return (
    <WeightEntryDetailScreen
      entryId={entryId}
      from={search.from}
      history={search.history}
      rabbitId={search.rabbit}
      trend={search.trend}
    />
  );
}
