/*
 * The explicit connectivity-required state (task 4.4; SYNC-3).
 *
 * The entry guard reaches this surface when the device's household book holds no
 * household **and** the service/account cannot confirm that: nothing was ever
 * loaded on this device and there is no usable live answer. It is therefore not
 * "no household" and not empty data — the screen names the actual cause and never
 * presents an empty collection as the household's current data.
 *
 * **Copy branches on the live status**, because "connect to the internet" is
 * false for a person who is online and whose account the service refused:
 *   - `offline` — the required-connectivity title and body, plus the `Offline`
 *     badge;
 *   - `account-unusable` — the neutral `connectivity.account.title` heading with
 *     `error.no_account` as the body: the service answered and refused/blocked
 *     the account;
 *   - `account-undetermined` — the same neutral heading, with the cause's copy
 *     (rate-limited, timeout, or `error.unknown` when no cause was classified) as
 *     the body;
 *   - `available` — the transient window after a successful probe, before the
 *     guard's re-decision lands: a checking state only, because the
 *     required-connectivity copy would be false for an online, identified person;
 *   - everything else (not observed yet, pending, failed) — the
 *     required-connectivity copy, which is the honest starting state.
 *
 * **One press, one check.** The probe's own result decides what happens next: if
 * the tracker recorded an observation (`observed_at` non-null) the router is
 * invalidated so the entry guard re-decides from that status. Below the
 * backend's three-failure threshold the tracker leaves `observed_at` null; then
 * nothing is invalidated, because the guard would see "not observed" and probe
 * again — one tap must not become two network checks and two increments of the
 * failure count.
 *
 * **The outcome is announced once.** After a press the result is written to a
 * polite `role="status"` region (`VisuallyHidden` inside the live wrapper), so
 * "still offline" is not silent when the screen does not change. When the probe
 * itself fails, the danger `Notice` (`role="alert"`) is the announcement and the
 * polite region stays empty — one failure is not spoken twice. On success the app
 * navigates away and the region unmounts with the screen.
 *
 * The surface is a standalone `Screen as="main"`: the route renders it with no
 * `AppShell` (there is no destination to be current), so it owns its safe areas
 * and the entry-state attributes the specs assert.
 */
import { useRouter } from '@tanstack/react-router';
import { useState } from 'react';
import { Badge } from '../../../components/ui/Badge';
import { Button } from '../../../components/ui/Button';
import { Heading } from '../../../components/ui/Heading';
import { LoadingIndicator } from '../../../components/ui/LoadingIndicator';
import { Notice } from '../../../components/ui/Notice';
import { Screen } from '../../../components/ui/Screen';
import { Text } from '../../../components/ui/Text';
import { VisuallyHidden } from '../../../components/ui/VisuallyHidden';
import { t } from '../../i18n';
import { describeSaveFailure, SAVE_CAUSE_MESSAGE_KEYS } from '../failure';
import type { ServiceStatus } from '../status';
import { useProbeServiceReachability, useServiceStatusView } from '../status';

export type ConnectivityRequiredScreenProps = {
  /** The status the entry guard decided on (`null` when none was consulted). */
  status: ServiceStatus | null;
};

export function ConnectivityRequiredScreen({
  status,
}: ConnectivityRequiredScreenProps) {
  const view = useServiceStatusView();
  const probe = useProbeServiceReachability();
  const router = useRouter();
  const [checked, setChecked] = useState(false);

  const live = view.kind === 'ready' ? view.view : null;
  const offline = live?.kind === 'offline';

  const accountCopy =
    live?.kind === 'account-unusable'
      ? t('error.no_account')
      : live?.kind === 'account-undetermined'
        ? t(
            live.cause === null
              ? 'error.unknown'
              : SAVE_CAUSE_MESSAGE_KEYS[live.cause],
          )
        : null;

  const heading =
    accountCopy === null
      ? t('connectivity.required.title')
      : t('connectivity.account.title');
  const body = accountCopy ?? t('connectivity.required.body');

  const probeFailure =
    probe.error === null ? null : describeSaveFailure(probe.error);

  /*
   * A rejected probe is announced by the danger `Notice` (`role="alert"`); the
   * polite region stays empty so the same failure is not heard twice.
   */
  const announcement =
    !checked || probeFailure !== null
      ? ''
      : (accountCopy ?? t('connectivity.required.body'));

  async function checkAgain() {
    setChecked(false);

    try {
      const probed = await probe.mutateAsync();
      setChecked(true);

      /*
       * A recorded observation is what the guard can decide on. `observed_at`
       * null means the tracker did not record an answer (below the failure
       * threshold); invalidating then would make the guard probe a second time.
       */
      if (probed.observed_at !== null) {
        await router.invalidate();
      }
    } catch {
      setChecked(true);
    }
  }

  /*
   * `available` is the transient window after a successful probe, before the
   * guard's re-decision lands. It is not a connectivity failure, so the
   * required-connectivity copy must not appear: show the checking state, which
   * is also the polite status announcement.
   */
  if (live?.kind === 'available') {
    return (
      <Screen
        as="main"
        className="bs-connectivity"
        data-household-entry="service-required"
        data-service-cause={status?.cause ?? 'none'}
        data-service-state={status?.state ?? 'unobserved'}
      >
        <LoadingIndicator label={t('connectivity.checking')} variant="screen" />
      </Screen>
    );
  }

  return (
    <Screen
      as="main"
      className="bs-connectivity"
      data-household-entry="service-required"
      data-service-cause={status?.cause ?? 'none'}
      data-service-state={status?.state ?? 'unobserved'}
    >
      <div className="bs-connectivity__intro">
        <Heading as="h1" variant="large-title">
          {heading}
        </Heading>

        <Text tone="secondary">{body}</Text>

        {offline ? (
          <Badge tone="warning">{t('connectivity.offline')}</Badge>
        ) : null}

        {probeFailure === null ? null : (
          <Notice tone="danger">{t(probeFailure.messageKey)}</Notice>
        )}

        <Button
          fullWidth
          loading={probe.isPending}
          onClick={() => {
            void checkAgain();
          }}
        >
          {probe.isPending
            ? t('connectivity.checking')
            : t('connectivity.checkAgain')}
        </Button>

        <span role="status">
          <VisuallyHidden>{announcement}</VisuallyHidden>
        </span>
      </div>
    </Screen>
  );
}
