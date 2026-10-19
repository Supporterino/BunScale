import type { ReactNode } from 'react';
import { useState } from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Card } from '../../../components/ui/Card';
import { Divider } from '../../../components/ui/Divider';
import { EmptyState } from '../../../components/ui/EmptyState';
import { ErrorState } from '../../../components/ui/ErrorState';
import { FormField } from '../../../components/ui/FormField';
import { Heading } from '../../../components/ui/Heading';
import { IconButton } from '../../../components/ui/IconButton';
import { List } from '../../../components/ui/List';
import { ListRow } from '../../../components/ui/ListRow';
import { ListSection } from '../../../components/ui/ListSection';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { NavigationBar } from '../../../components/ui/NavigationBar';
import { Notice } from '../../../components/ui/Notice';
import type { NavigationDestination } from '../../../components/ui/navigation';
import { Screen } from '../../../components/ui/Screen';
import type { SegmentedControlOption } from '../../../components/ui/SegmentedControl';
import { SegmentedControl } from '../../../components/ui/SegmentedControl';
import { Text } from '../../../components/ui/Text';
import { TextField } from '../../../components/ui/TextField';
import { VisuallyHidden } from '../../../components/ui/VisuallyHidden';
import {
  GlyphBack,
  GlyphBell,
  GlyphDashboard,
  GlyphHousehold,
  GlyphSettings,
  GlyphTrash,
} from './glyphs';

/*
 * The placeholder screen of task 0.21, and the only styled surface in the app
 * until the P0 screens land.
 *
 * What it is for: task 0.25 asserts the adaptive navigation arrangement at
 * 390 x 844 and at 1280 x 800 rather than only that content rendered, so the
 * screen has to be composed the way a real screen is composed — through
 * `AppShell`, with one `main`, one navigation landmark named "Primary" and the
 * same destinations rendered as tabs or as a sidebar by width alone.
 *
 * What it is not: a feature. It holds no IPC, no query and no router logic
 * (`frontend-conventions` puts those in `frontend/api.ts`, `queries.ts` and the
 * route tree), and the only state it keeps is the state a demonstration needs —
 * the selected segment, the two row toggles and the notice behind the settings
 * action. Every control is live: pressed, focused, selected and disabled states
 * are the real ones, and the nav bar's gear is wired to real feedback so it is
 * not chrome that does nothing.
 *
 * One destination is declared, not three. A destination has to be a real route —
 * `to` is typed as the router's own path union — and the scaffold has exactly
 * one (`/`). Three links to it would put `aria-current` on three elements and
 * describe a navigation that does not exist; the array grows as the P0 screens
 * add routes, and the primitives already take a list of up to five.
 */

type WeightUnit = 'kg' | 'lb';

const DESTINATIONS: NavigationDestination[] = [
  {
    to: '/',
    label: 'Design system',
    icon: <GlyphDashboard />,
  },
];

const WEIGHT_UNITS: Array<SegmentedControlOption<WeightUnit>> = [
  { label: 'kg', value: 'kg' },
  { label: 'lb', value: 'lb' },
];

export function DesignSystemPreview() {
  const [unit, setUnit] = useState<WeightUnit>('kg');
  const [remindersOn, setRemindersOn] = useState(true);
  const [settingsNoteOpen, setSettingsNoteOpen] = useState(false);
  const [retryAttempts, setRetryAttempts] = useState(0);

  return (
    <AppShell destinations={DESTINATIONS}>
      <NavigationBar
        leadingAction={
          <IconButton disabled label="Back">
            <GlyphBack />
          </IconButton>
        }
        title="Design system"
        trailingActions={
          <IconButton
            label="Settings"
            onClick={() => {
              setSettingsNoteOpen((open) => !open);
            }}
            pressed={settingsNoteOpen}
          >
            <GlyphSettings />
          </IconButton>
        }
      />
      <Screen>
        <Text
          className="bs-preview__intro"
          tone="secondary"
          variant="subheadline"
        >
          Every control here is live: press feedback, focus, selection and
          disabled states are the real ones, and the navigation is the app's own
          — tabs on a phone, a sidebar on a wider window. Nothing is wired to a
          service yet; the first real screens replace this one.
        </Text>

        {settingsNoteOpen ? (
          <Notice
            className="bs-preview__intro"
            title="Settings are not here yet"
            tone="info"
          >
            This notice is what the gear does until the settings screen exists.
            It appears after an action, so it announces itself politely.
          </Notice>
        ) : null}

        <PreviewSection title="Typography">
          <Text variant="large-title">Large title</Text>
          <Text variant="title-1">Title 1</Text>
          <Text variant="title-2">Title 2</Text>
          <Text variant="title-3">Title 3</Text>
          <Text variant="headline">Headline</Text>
          <Text variant="body">Body — the default for copy.</Text>
          <Text variant="callout">Callout</Text>
          <Text variant="subheadline">Subheadline</Text>
          <Text variant="footnote">Footnote</Text>
          <Text variant="caption-1">Caption 1</Text>
          <Text variant="caption-2">Caption 2</Text>
          <Heading variant="title-3">A heading inside the screen</Heading>
          <Heading variant="section-header">Section header style</Heading>
          <Text tone="secondary" variant="body">
            Secondary body — 7.3:1 on the background.
          </Text>
          <Text tone="tertiary" variant="body">
            Tertiary body — 5.1:1 on the background.
          </Text>
          <Text tone="danger" variant="body">
            Danger body — 6.2:1 on the background.
          </Text>
          <Text tone="secondary" variant="footnote">
            The next phrase is announced but not drawn:
            <VisuallyHidden> no eye sees this</VisuallyHidden>, which is how an
            icon-only control or a live region is named.
          </Text>
        </PreviewSection>

        <PreviewSection title="Buttons">
          <div className="bs-preview__row">
            <Button>Create household</Button>
            <Button variant="secondary">Add a rabbit</Button>
            <Button variant="plain">Not now</Button>
            <Button variant="destructive">Delete household</Button>
          </div>
          <div className="bs-preview__row">
            <Button size="compact" variant="secondary">
              Compact
            </Button>
            <Button loading>Save</Button>
            <Button disabled>Disabled</Button>
          </div>
          <Button fullWidth variant="secondary">
            Full width
          </Button>
          <div className="bs-preview__row">
            <IconButton label="Reminders" pressed={remindersOn}>
              <GlyphBell />
            </IconButton>
            <IconButton label="Settings">
              <GlyphSettings />
            </IconButton>
            <IconButton disabled label="Delete">
              <GlyphTrash />
            </IconButton>
          </div>
        </PreviewSection>

        <PreviewSection title="Form fields">
          <TextField
            autoComplete="organization"
            hint="Shown on the dashboard and to everyone in it."
            label="Household name"
            placeholder="Our warren"
          />
          <TextField
            error="A household with this name already exists."
            label="Household name"
          />
          <TextField defaultValue="kg" disabled label="Weight unit" />
          <TextField
            enterKeyHint="done"
            hint="One decimal place is enough."
            inputMode="decimal"
            label="Weight"
            type="number"
          />
          <FormField
            hint="Used for every weight in the app."
            label="Weight unit"
          >
            {(field) => (
              <SegmentedControl
                describedBy={field.describedBy}
                id={field.controlId}
                invalid={field.invalid}
                labelId={field.labelId}
                onChange={setUnit}
                options={WEIGHT_UNITS}
                required={field.required}
                value={unit}
              />
            )}
          </FormField>
        </PreviewSection>

        <PreviewSection title="Lists">
          <ListSection header="Households">
            <List>
              <ListRow
                accessory="chevron"
                detail="2 rabbits"
                title="Our warren"
                to="/"
              />
              <ListRow detail="Current household" selected title="The Burrow" />
              <ListRow
                accessory="checkmark"
                detail="Every day at 08:00"
                onClick={() => {
                  setRemindersOn((on) => !on);
                }}
                selected={remindersOn}
                title="Weight reminders"
              />
              <ListRow
                control={<Badge tone="success">Connected</Badge>}
                detail="Last synced just now"
                title="Household sync"
              />
            </List>
          </ListSection>
        </PreviewSection>

        <PreviewSection title="Grouping">
          <Card heading="This week">
            <Text variant="title-1">+12 g</Text>
            <Text tone="secondary" variant="subheadline">
              Average gain across two rabbits.
            </Text>
            <Divider />
            <div className="bs-preview__row">
              <Badge tone="success">Connected</Badge>
              <Badge tone="warning">Offline</Badge>
              <Badge tone="danger">Save failed</Badge>
              <Badge tone="info">Syncing</Badge>
              <Badge>Neutral</Badge>
            </div>
          </Card>
        </PreviewSection>

        <PreviewSection title="Status">
          <Notice title="Saved" tone="success">
            The household was saved, and the other devices will see it shortly.
          </Notice>
          <Notice title="You are offline" tone="warning">
            Reads use the last synced data. A save waits until the connection is
            back.
          </Notice>
          <Notice title="The save did not go through" tone="danger">
            The shared service rejected the request, so nothing was written and
            the value on screen is still the old one.
          </Notice>
          <Notice title="Two devices are syncing" tone="info">
            The list below may be a moment behind.
          </Notice>
          <div className="bs-preview__row">
            <LoadingIndicator />
            <LoadingIndicator label="Loading the household" />
          </div>
          <LoadingIndicator label="Loading every household" variant="screen" />
        </PreviewSection>

        <PreviewSection title="States">
          <EmptyState
            action={<Button variant="secondary">Create a household</Button>}
            icon={<GlyphHousehold />}
            title="No households yet"
          >
            A household is what a rabbit's record is shared in. Create one, then
            invite the people who care for the rabbit with you.
          </EmptyState>
          <ErrorState
            onRetry={() => {
              setRetryAttempts((attempts) => attempts + 1);
            }}
            requestId="req_8f2c41"
            title="Nothing could be loaded"
          >
            The shared service is unreachable, so there is nothing to show yet.
            This is not an empty household — nothing was ever loaded.
          </ErrorState>
          <Text tone="tertiary" variant="caption-1">
            Retry pressed {retryAttempts} times in this placeholder screen.
          </Text>
        </PreviewSection>
      </Screen>
    </AppShell>
  );
}

/*
 * Section scaffolding for this screen only: a real `h2` (the screen's `h1` is
 * the navigation bar's title) and a vertical stack. It is not an inventory
 * primitive — nothing in the app composes a "preview section" — so it lives with
 * the screen it belongs to and reads the same spacing tokens every other
 * component reads.
 */
function PreviewSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="bs-preview__section">
      <Heading variant="title-2">{title}</Heading>
      <div className="bs-preview__stack">{children}</div>
    </section>
  );
}
