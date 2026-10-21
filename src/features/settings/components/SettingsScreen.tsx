/*
 * Settings & Profile (task 4.3; HS-4, SET-2, SET-4).
 *
 * The screen SET-1's gear opens. It is a grouped-list screen in the system's
 * language: a read-only name, the held households with exactly one active and
 * switching, an inline "create another" form that never sends the person back
 * through onboarding, and the per-device display unit. It keeps the adaptive
 * frame (tab bar compact, sidebar expanded) and a Back control, so SET-1's
 * mobile rule (a way back) and desktop rule (the dashboard remains reachable)
 * are both true.
 *
 * **Data, and what each section trusts.**
 *
 *   - `useSettingsHouseholdList()` is the household set: it marks exactly one row
 *     active from the active read and reports which source produced the flag.
 *     Switching goes through `useSwitchActiveHousehold`, whose mutation variable
 *     is the `setActiveHousehold(id)` handle, so "Try again" re-sends the same
 *     `request_id` (D0.14). The screen never writes the active marker itself; it
 *     moves when the backend confirms.
 *   - `useIdentityName()` is SET-2's chain already applied. It is read-only by
 *     construction: a static `ListRow`, no control, no chevron. `failed` and the
 *     placeholder tier both show `settings.identity.you`, as SET-2 requires, and
 *     neither disables the rest of the screen.
 *   - `useDisplayWeightUnit()` / `useSetDisplayWeightUnit()` are SET-4. The
 *     selected segment is the backend's *effective* unit, never the requested
 *     one; while the write runs the control is disabled and "Saving…" is shown,
 *     and a refused write leaves the confirmed unit in place with its cause
 *     below. A retry re-runs the handle (same `request_id`).
 *   - `useServiceStatusView()` is read for one thing only: an offline `Notice`,
 *     so a failing write's cause has context. The connectivity-required and
 *     local-copy surfaces are task 4.4's, not duplicated here.
 *
 * **Read failures carry a cause and, when the backend says the same attempt can
 * succeed, a re-read.** A failed household or unit read runs through
 * `describeSaveFailure` like a write; the notice shows `failure.messageKey`, and
 * its action refetches the same query key. A cause marked non-retryable gets no
 * action, because no re-read is promised to fix it.
 *
 * **Arriving here moves focus.** SET-1's gear is a route change; the screen
 * focuses its `main` on mount so a keyboard or screen-reader user lands inside
 * the new surface instead of on `<body>`. The `main` is `tabIndex={-1}` and its
 * first heading is the screen title, so the arrival is announced in reading
 * order. This is the screen's effect, not a change to `AppShell`.
 *
 * **Creating another household without onboarding.** The form is inline in the
 * Households section, and `useCreateHousehold` (added in parallel with this
 * screen) updates the active read and the held list; no navigation happens. The
 * form keeps the handle discipline itself — one operation per intent, reused on
 * retry while the name is unchanged, a new handle for a corrected name — because
 * only the caller knows which input the operation carries (D0.14). When the held
 * set is empty the section renders an `EmptyState` with the create action; when
 * the set failed to load the create action stays available, so the failure does
 * not remove the only way forward.
 *
 * **Copy.** Every string comes from `t()`. The dashboard destination is labelled
 * `dashboard.title` ("Your household") in both screens, so one navigation item
 * has one name. The create form's cancel and the empty held set use
 * `common.cancel` and `settings.households.emptyTitle`/`emptyBody`.
 */
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { AppShell } from '../../../components/ui/AppShell';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { EmptyState } from '../../../components/ui/EmptyState';
import { FormField } from '../../../components/ui/FormField';
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
import { TextField } from '../../../components/ui/TextField';
import { useAppEvents } from '../../app-events';
import {
  GlyphBack,
  GlyphDashboard,
} from '../../design-preview/components/glyphs';
import { createHousehold, setActiveHousehold } from '../../households/api';
import {
  useCreateHousehold,
  useHouseholdList,
  useSwitchActiveHousehold,
} from '../../households/queries';
import { t } from '../../i18n';
import { describeSaveFailure } from '../../sync/failure';
import { useServiceStatusView } from '../../sync/status';
import { setDisplayWeightUnit } from '../api';
import {
  useDisplayWeightUnit,
  useSetDisplayWeightUnit,
  type WeightUnit,
} from '../display-unit';
import { useSettingsHouseholdList } from '../households';
import { useIdentityName } from '../identity';

/** The `main` region's id, and the focus target on mount. */
const SETTINGS_MAIN_ID = 'bs-settings-main';

/** One create intent: the handle and the name it was created for. */
type CreateOperation = ReturnType<typeof createHousehold>;

type CreateAttempt = {
  readonly operation: CreateOperation;
  readonly name: string;
};

export function SettingsScreen() {
  // Task 3.6's subscription. The dashboard shell mounts it too; only one route
  // is mounted at a time, so this keeps live updates while Settings is open
  // without two listeners. The root `_app` layout is where it belongs once a
  // route can host it (see task 4.2's record).
  useAppEvents();

  const navigate = useNavigate();

  /*
   * SET-1's gear swap changes routes; without this the browser moves focus to
   * `document.body` and a keyboard or screen-reader user starts over. The
   * screen's `main` is the focus target (`AppShell` gives it `tabIndex={-1}`),
   * so the arrival is announced as a new region and its title follows in reading
   * order. Done from the screen, not the primitive: the shell does not own this
   * screen's arrival.
   */
  useEffect(() => {
    document.getElementById(SETTINGS_MAIN_ID)?.focus();
  }, []);

  const destinations: NavigationDestination[] = [
    {
      to: '/',
      label: t('dashboard.title'),
      icon: <GlyphDashboard />,
    },
  ];

  return (
    <AppShell destinations={destinations} mainId={SETTINGS_MAIN_ID}>
      <NavigationBar
        leadingAction={
          <IconButton
            label={t('settings.back')}
            onClick={() => {
              void navigate({ to: '/' });
            }}
          >
            <GlyphBack />
          </IconButton>
        }
        title={t('settings.title')}
      />
      <Screen className="bs-settings">
        <OfflineNotice />
        <ProfileSection />
        <HouseholdsSection />
        <DisplayUnitSection />
      </Screen>
    </AppShell>
  );
}

/*
 * The status surface this screen owns. Only `offline` is shown: it is the state
 * that explains a refused write. The other two connectivity surfaces (nothing
 * ever loaded, a local copy being served) belong to task 4.4.
 */
function OfflineNotice() {
  const status = useServiceStatusView();

  if (status.kind !== 'ready' || status.view.kind !== 'offline') {
    return null;
  }

  return (
    <Notice title={t('connectivity.offline')} tone="warning">
      {t('error.connectivity')}
    </Notice>
  );
}

function ProfileSection() {
  const identity = useIdentityName();

  const value =
    identity.kind === 'pending' ? (
      <LoadingIndicator decorative />
    ) : identity.kind === 'failed' ? (
      t('settings.identity.you')
    ) : identity.displayName.kind === 'name' ? (
      identity.displayName.name
    ) : (
      t(identity.displayName.messageKey)
    );

  return (
    <ListSection header={t('settings.identity.heading')}>
      <List>
        <ListRow
          control={<span className="bs-settings__value">{value}</span>}
          title={t('settings.identity.label')}
        />
      </List>
    </ListSection>
  );
}

function HouseholdsSection() {
  const households = useSettingsHouseholdList();
  const listRead = useHouseholdList();
  const switchHousehold = useSwitchActiveHousehold();
  const [creating, setCreating] = useState(false);

  const switchFailure =
    switchHousehold.error === null
      ? null
      : describeSaveFailure(switchHousehold.error);
  const lastSwitch = switchHousehold.variables;

  /*
   * The read's own failure is a `HouseholdListFailure`, so it goes through the
   * same cause map as a write. It is not "something went wrong": the copy is
   * `failure.messageKey`, and the retry re-reads this book (`refetch` on the
   * same key `useSettingsHouseholdList` subscribed to). A cause the backend
   * marked non-retryable gets no button — there is nothing the retry could do.
   */
  const readFailure =
    households.kind === 'failed' ? describeSaveFailure(households.error) : null;

  /*
   * Only `validation` has screen-specific copy ("that household is no longer
   * available"); every other cause keeps its `failure.ts` mapping. A retryable
   * cause re-runs the stored handle (same `request_id`). A `validation` refusal
   * is not retryable, so its action is the re-read the copy names: refresh the
   * held set, drop the stale failure and let the person pick a row that exists.
   */
  const failureCopy =
    switchFailure === null
      ? null
      : switchFailure.cause === 'validation'
        ? t('settings.households.switchFailed')
        : t(switchFailure.messageKey);

  const empty =
    households.kind === 'ready' && households.households.length === 0;

  function refreshHouseholds() {
    void listRead.refetch();
  }

  function switchTo(householdId: string) {
    if (switchHousehold.isPending) {
      return;
    }

    switchHousehold.mutate(setActiveHousehold(householdId));
  }

  return (
    <ListSection header={t('settings.households.heading')}>
      {households.kind === 'pending' ? <LoadingIndicator /> : null}

      {readFailure === null ? null : (
        <Notice
          action={
            readFailure.retryable ? (
              <Button
                onClick={refreshHouseholds}
                size="compact"
                variant="secondary"
              >
                {t('error.retry')}
              </Button>
            ) : undefined
          }
          tone="danger"
        >
          {t(readFailure.messageKey)}
        </Notice>
      )}

      {empty ? (
        <EmptyState
          action={
            <Button onClick={() => setCreating(true)} variant="secondary">
              {t('household.create')}
            </Button>
          }
          title={t('settings.households.emptyTitle')}
        >
          {t('settings.households.emptyBody')}
        </EmptyState>
      ) : null}

      {households.kind === 'ready' && !empty ? (
        <List>
          {households.households.map((household) =>
            household.isActive ? (
              <ListRow
                control={
                  <span className="bs-settings__markers">
                    <Badge tone="success">
                      {t('settings.households.active')}
                    </Badge>
                    {household.is_owner ? (
                      <Badge>{t('settings.households.owner')}</Badge>
                    ) : null}
                  </span>
                }
                key={household.id}
                selected
                title={household.name}
              />
            ) : (
              <ListRow
                accessory="chevron"
                control={
                  household.is_owner ? (
                    <Badge>{t('settings.households.owner')}</Badge>
                  ) : undefined
                }
                detail={t('settings.households.makeActive')}
                key={household.id}
                onClick={() => {
                  switchTo(household.id);
                }}
                title={household.name}
              />
            ),
          )}
        </List>
      ) : null}

      {failureCopy === null ? null : (
        <Notice
          action={
            switchFailure?.retryable === true && lastSwitch !== undefined ? (
              <Button
                onClick={() => {
                  switchHousehold.mutate(lastSwitch);
                }}
                size="compact"
                variant="secondary"
              >
                {t('error.retry')}
              </Button>
            ) : switchFailure?.cause === 'validation' ? (
              <Button
                onClick={() => {
                  switchHousehold.reset();
                  refreshHouseholds();
                }}
                size="compact"
                variant="secondary"
              >
                {t('error.retry')}
              </Button>
            ) : undefined
          }
          title={t('error.notSaved')}
          tone="danger"
        >
          {failureCopy}
        </Notice>
      )}

      {switchHousehold.isPending ? <LoadingIndicator /> : null}

      {households.kind === 'pending' ? null : creating ? (
        <CreateHouseholdForm onClose={() => setCreating(false)} />
      ) : empty ? null : (
        <Button fullWidth onClick={() => setCreating(true)} variant="secondary">
          {t('settings.households.createAnother')}
        </Button>
      )}
    </ListSection>
  );
}

/*
 * Creating an additional household in place. `useCreateHousehold` (added by
 * task 4.2's follow-up) owns the cache update; the form owns only the intent:
 * one handle per name, reused by "Try again" while the name is unchanged, and a
 * new handle for a corrected name. The backend makes the new household active,
 * so the list and the active read both move without leaving this screen (no
 * onboarding, no route change).
 */
function CreateHouseholdForm({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState('');
  const [prompted, setPrompted] = useState(false);
  const attempt = useRef<CreateAttempt | null>(null);

  const creation = useCreateHousehold();

  const failure =
    creation.error === null ? null : describeSaveFailure(creation.error);
  const validationFailure =
    failure !== null && failure.cause === 'validation' ? failure : null;
  const otherFailure =
    failure !== null && failure.cause !== 'validation' ? failure : null;

  const fieldError =
    prompted || validationFailure !== null
      ? t('household.blankName')
      : undefined;

  function run(operation: CreateOperation) {
    creation.mutate(operation, { onSuccess: onClose });
  }

  function submit() {
    const trimmed = name.trim();

    if (trimmed === '') {
      setPrompted(true);
      creation.reset();
      return;
    }

    setPrompted(false);

    const last = attempt.current;

    if (last !== null && last.name === trimmed) {
      run(last.operation);
      return;
    }

    const operation = createHousehold(trimmed);
    attempt.current = { operation, name: trimmed };
    run(operation);
  }

  const pending = creation.isPending;

  return (
    <form
      className="bs-settings__create"
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <TextField
        autoComplete="off"
        disabled={pending}
        enterKeyHint="done"
        error={fieldError}
        label={t('household.nameLabel')}
        name="new-household-name"
        onChange={(next) => {
          setName(next);
          setPrompted(false);

          if (validationFailure !== null) {
            creation.reset();
          }
        }}
        placeholder={t('household.namePlaceholder')}
        required
        value={name}
      />

      {otherFailure === null ? null : (
        <Notice
          action={
            otherFailure.retryable ? (
              <Button onClick={submit} size="compact" variant="secondary">
                {t('error.retry')}
              </Button>
            ) : undefined
          }
          title={t('error.notSaved')}
          tone="danger"
        >
          {t(otherFailure.messageKey)}
        </Notice>
      )}

      <div className="bs-settings__actions">
        <Button onClick={onClose} variant="plain">
          {t('common.cancel')}
        </Button>
        <Button loading={pending} type="submit">
          {pending ? t('household.creating') : t('household.create')}
        </Button>
      </div>
    </form>
  );
}

function DisplayUnitSection() {
  const unit = useDisplayWeightUnit();
  const setUnit = useSetDisplayWeightUnit();

  const failure =
    setUnit.error === null ? null : describeSaveFailure(setUnit.error);
  const lastChange = setUnit.variables;

  /*
   * A section with a real `h2`, so heading navigation reaches it like Profile and
   * Households. The `FormField` label stays (it is the radiogroup's accessible
   * name); only its visual copy is suppressed inside this section by
   * `screens.css`, because the heading above already says the same words.
   */
  if (unit.data === undefined) {
    const readFailure =
      unit.error === null ? null : describeSaveFailure(unit.error);

    return (
      <ListSection header={t('settings.displayUnit.heading')}>
        {readFailure === null ? (
          <LoadingIndicator />
        ) : (
          <Notice
            action={
              readFailure.retryable ? (
                <Button
                  onClick={() => {
                    void unit.refetch();
                  }}
                  size="compact"
                  variant="secondary"
                >
                  {t('error.retry')}
                </Button>
              ) : undefined
            }
            tone="danger"
          >
            {t(readFailure.messageKey)}
          </Notice>
        )}
      </ListSection>
    );
  }

  const effective = unit.data;

  /*
   * Built per render, not at module scope: the labels are copy, and a later
   * locale switch must re-resolve them rather than freeze the first locale at
   * import time.
   */
  const options: Array<SegmentedControlOption<WeightUnit>> = [
    { value: 'grams', label: t('settings.displayUnit.grams') },
    { value: 'kilograms', label: t('settings.displayUnit.kilograms') },
  ];

  return (
    <ListSection header={t('settings.displayUnit.heading')}>
      <div className="bs-settings__unit">
        <FormField
          className="bs-settings__unit-form"
          hint={t('settings.displayUnit.description')}
          label={t('settings.displayUnit.heading')}
        >
          {(field) => (
            <SegmentedControl
              describedBy={field.describedBy}
              disabled={setUnit.isPending}
              id={field.controlId}
              invalid={field.invalid}
              labelId={field.labelId}
              onChange={(next) => {
                if (next === effective) {
                  return;
                }

                setUnit.mutate(setDisplayWeightUnit(next));
              }}
              options={options}
              required={field.required}
              value={effective}
            />
          )}
        </FormField>

        {setUnit.isPending ? (
          <LoadingIndicator label={t('settings.displayUnit.saving')} />
        ) : null}

        {failure === null ? null : (
          <Notice
            action={
              failure.retryable && lastChange !== undefined ? (
                <Button
                  onClick={() => {
                    setUnit.mutate(lastChange);
                  }}
                  size="compact"
                  variant="secondary"
                >
                  {t('error.retry')}
                </Button>
              ) : undefined
            }
            title={t('error.notSaved')}
            tone="danger"
          >
            {t(failure.messageKey)}
          </Notice>
        )}
      </div>
    </ListSection>
  );
}
