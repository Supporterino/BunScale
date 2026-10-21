/*
 * The local-copy indication (task 4.4; SYNC-3).
 *
 * A collection read reports where its answer came from. `source: local` with a
 * `loaded_at` means the device served its previously loaded copy because the
 * refresh could not reach the service — the data on screen is real, but it may
 * be behind changes made by other members. This surface says so, non-blocking:
 * a polite `Notice` above the content, with no action, because there is nothing
 * to fix on this screen and the read keeps working.
 *
 * It is deliberately not shown for `loaded_at: null` together with
 * `source: local`: that is the "nothing was ever loaded" state, which is the
 * connectivity-required screen's job (SYNC-3) and must never be presented as a
 * loaded copy.
 *
 * Mounting it costs one household read — the same read the collection screen
 * already makes, keyed by household, so it shares that cache entry. It is mounted
 * in the dashboard shell for P0 (where the active household is known); when P1
 * renders rabbit data it belongs directly above that data, not in the shell.
 */
import { Notice } from '../../../components/ui/Notice';
import {
  collectionPresence,
  useHouseholdCollection,
} from '../../households/queries';
import { t } from '../../i18n';

export type LocalCopyNoticeProps = {
  householdId: string;
};

export function LocalCopyNotice({ householdId }: LocalCopyNoticeProps) {
  const collection = useHouseholdCollection(householdId);

  if (collection.data === undefined) {
    return null;
  }

  const presence = collectionPresence(collection.data);

  if (presence.kind !== 'loaded' || presence.source !== 'local') {
    return null;
  }

  return <Notice tone="info">{t('connectivity.localCopy')}</Notice>;
}
