import { useNavigate } from '@tanstack/react-router';
import { Button } from '../../../components/ui/Button';
import { EmptyState } from '../../../components/ui/EmptyState';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import { Screen } from '../../../components/ui/Screen';
import { GlyphHousehold } from './glyphs';

/*
 * The root not-found surface, presented from the design-system primitives.
 *
 * `src/routes/__root.tsx` declares it as `notFoundComponent`, so every
 * unmatched path in the tree lands here — which is exactly why it must not be
 * bare scaffolding: it is reachable from anywhere, including a mistyped deep
 * link in the webview.
 *
 * The composition is deliberately a standalone `Screen` rather than an
 * `AppShell`: there is no destination to be current, and a navigation landmark
 * with nothing in it is worse than none. It keeps a real `h1` (the navigation
 * bar), an explanatory body and ONE clear way forward — the start screen, by
 * route rather than by `history.back()`, because a cold launch from a bad URL
 * has no history to go back to. That is the same rule the router conventions set
 * for pushed screens: an in-UI control, never a reliance on the platform's back
 * gesture.
 */

export function NotFoundScreen() {
  const navigate = useNavigate();

  return (
    <Screen as="main">
      <NavigationBar title="Not found" />
      <EmptyState
        action={
          <Button
            onClick={() => {
              void navigate({ to: '/' });
            }}
            variant="secondary"
          >
            Go to the start screen
          </Button>
        }
        icon={<GlyphHousehold />}
        title="This screen does not exist"
      >
        The address you opened is not part of BunScale. It may have been
        removed, or it may belong to a screen that arrives in a later version.
      </EmptyState>
    </Screen>
  );
}
