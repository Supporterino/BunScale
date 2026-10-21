/*
 * The design-system primitives at the component layer, at both baselines
 * through `describeAtBothBaselines` (TOOL-5's first two scenarios, and TOOL-6's
 * inventory).
 *
 * What this file is for: the placeholder screen is one composition of the 23
 * primitives, so a failure there says "the screen broke" and not which contract
 * broke. These are the contracts the component layer can honestly check — a
 * control's accessible name, the role a tone maps to, the state a disabled or
 * busy control exposes, whether a label is actually associated with its input —
 * and each one is asserted directly so the failure names the primitive.
 *
 * What it deliberately does not check is what jsdom cannot answer, and
 * `src/test/setup.ts` states the measured detail: no geometry (there is no
 * layout engine, so `getBoundingClientRect()` returns zeroes, and the 44 pt
 * touch-target rule is an E2E subject), no density or pointer behaviour, no
 * `visualViewport`, and no adaptive arrangement, since no feature query in
 * `src/styles/**` ever applies here and the app's stylesheet is not loaded in a
 * Vitest run at all.
 */
import { render, screen, within } from '@testing-library/react';
import { expect, it } from 'vitest';
import { describeAtBothBaselines } from '../../test/setup';
import { Button } from './Button';
import { ErrorState } from './ErrorState';
import { IconButton } from './IconButton';
import { ListRow } from './ListRow';
import { LoadingIndicator } from './LoadingIndicator';
import { Notice } from './Notice';
import { SegmentedControl } from './SegmentedControl';
import { TextField } from './TextField';

describeAtBothBaselines('design-system primitives', () => {
  it('Button announces a busy state and stays unreachable while busy', () => {
    render(<Button loading>Save</Button>);

    const save = screen.getByRole('button', { name: 'Save' });

    expect(save).toBeDisabled();
    expect(save).toHaveAttribute('aria-busy', 'true');
  });

  it('IconButton carries the accessible name a glyph cannot', () => {
    render(
      <IconButton label="Reminders" pressed>
        <svg aria-hidden="true" />
      </IconButton>,
    );

    const reminders = screen.getByRole('button', { name: 'Reminders' });

    expect(reminders).toHaveAccessibleName('Reminders');
    expect(reminders).toHaveAttribute('aria-pressed', 'true');
  });

  it('Notice announces a failure assertively and the rest politely', () => {
    render(
      <>
        <Notice tone="danger" title="Save failed">
          The shared service rejected the request.
        </Notice>
        <Notice tone="warning" title="You are offline">
          Reads use the last synced data.
        </Notice>
      </>,
    );

    const alert = screen.getByRole('alert');
    const status = screen.getByRole('status');

    expect(alert).toHaveTextContent('Save failed');
    expect(status).toHaveTextContent('You are offline');
  });

  it('LoadingIndicator announces the label it is given, and only that', () => {
    const { container } = render(
      <>
        <LoadingIndicator label="Loading the household" />
        <LoadingIndicator decorative />
      </>,
    );

    const statuses = screen.getAllByRole('status');

    expect(statuses).toHaveLength(1);
    expect(statuses[0]).toHaveTextContent('Loading the household');

    // `decorative` is the caller-announces-it form: `aria-hidden`, no role and
    // no copy of its own, so it cannot talk over the caller's own status.
    const decorative = container.querySelector(
      '[aria-hidden="true"].bs-loading-indicator',
    );

    expect(decorative).not.toBeNull();
    expect(decorative?.textContent).toBe('');

    // The removed fallback: an unlabelled ring used to announce a hard-coded
    // "Loading". No English copy lives in this primitive any more.
    expect(screen.queryByText('Loading')).toBeNull();
  });

  it('LoadingIndicator cannot announce anything outside the catalog', () => {
    // The type is the assertion: `label` is required unless `decorative` is
    // true, so the old bare `<LoadingIndicator />` — which fell back to a
    // hard-coded English string — no longer compiles (task 7.6, Q48).
    // @ts-expect-error - a non-decorative spinner requires a label.
    render(<LoadingIndicator />);

    // And if a cast got past the type, the live region would stay silent rather
    // than announce copy the catalog does not hold.
    expect(screen.getByRole('status').textContent).toBe('');
  });

  it('SegmentedControl is a radiogroup of real radios', () => {
    const options = [
      { label: 'kg', value: 'kg' },
      { label: 'lb', value: 'lb' },
    ];

    render(
      <SegmentedControl
        onChange={() => undefined}
        options={options}
        value="kg"
      />,
    );

    const group = screen.getByRole('radiogroup');
    const kilograms = within(group).getByRole('radio', { name: 'kg' });
    const pounds = within(group).getByRole('radio', { name: 'lb' });

    expect(kilograms).toBeChecked();
    expect(pounds).not.toBeChecked();
  });

  it('TextField links its label, hint and error to the input', () => {
    render(
      <TextField
        error="That name is taken."
        hint="Shown on the dashboard."
        label="Household name"
      />,
    );

    const input = screen.getByLabelText('Household name');
    const describedBy = input.getAttribute('aria-describedby') ?? '';
    const ids = describedBy.split(' ').filter((id) => id !== '');

    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(ids).toHaveLength(2);

    for (const id of ids) {
      expect(document.getElementById(id)).not.toBeNull();
    }
  });

  it('ErrorState is an assertive alert with a retry and a reference', () => {
    render(
      <ErrorState
        onRetry={() => undefined}
        requestId="req_8f2c41"
        title="Nothing could be loaded"
      >
        The shared service is unreachable.
      </ErrorState>,
    );

    const alert = screen.getByRole('alert');
    const retry = screen.getByRole('button', { name: 'Try again' });
    const reference = screen.getByText('Reference: req_8f2c41');

    expect(alert).toHaveTextContent('Nothing could be loaded');
    expect(retry).toBeEnabled();
    expect(reference).toBeVisible();
  });

  it('ListRow is only interactive in the modes that make it so', () => {
    render(
      <ul>
        <ListRow accessory="chevron" detail="2 rabbits" title="Our warren" />
        <ListRow onClick={() => undefined} title="Weight reminders" />
      </ul>,
    );

    const staticRow = screen.queryByRole('link', { name: /Our warren/ });
    const row = screen.getByRole('button', { name: /Weight reminders/ });

    expect(staticRow).toBeNull();
    expect(row).toBeEnabled();
    expect(screen.getByText('Our warren')).toBeVisible();
  });
});
