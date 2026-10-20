/*
 * Settings & Profile commands (SET-2, SET-4, NFR-3): the person's identity, the
 * per-device display unit and the redacted diagnostics summary. The only module
 * that imports these generated bindings.
 *
 * `set_display_weight_unit` is the one state-changing call here and follows the
 * same discipline as every other (design.md D0.14): the handle mints a request
 * id and a retry sends that same id. The change is local to this device and no
 * service ledger sees it, so the value names the operation without a
 * deduplication step. The command returns the unit it read back rather than
 * echoing its argument, so a write that did not land is never reported as
 * stored (SET-4).
 *
 * `get_identity` does not fail for an unresolved identity: `resolved: false`
 * with `display_name: null` is a normal answer. The localized "You"-style
 * placeholder is frontend copy (task 3.7), so nothing here substitutes one.
 *
 * `get_diagnostics_summary` lives here rather than in a diagnostics feature
 * because support diagnostics is reached from Settings & Profile; no P0 screen
 * shows it, and it carries identifiers, counts and instants only (NFR-3).
 */
import type {
  CommandError,
  DiagnosticsSummary,
  Identity,
  WeightUnit,
} from '../../bindings';
import { commands } from '../../bindings';
import type { CommandResult, RequestOperation } from '../../lib/request-id';
import { requestOperation } from '../../lib/request-id';

/** One read's outcome: the contract's `Result`, typed to this domain. */
type Outcome<T> = Promise<CommandResult<T, CommandError>>;

/** A state-changing command's handle; `run()` again re-uses the request id. */
type Operation<T> = RequestOperation<T, CommandError>;

/**
 * The account id and display name as Settings & Profile shows them (SET-2).
 *
 * The backend resolves SET-2's fallback chain: the account's display name, then
 * the locally known member name, then no name. An unresolved identity is not a
 * failure and must not disturb the screen.
 */
export function getIdentity(): Outcome<Identity> {
  return commands.getIdentity();
}

/**
 * The effective display unit: the person's choice, otherwise the unit the
 * device's region states (SET-4).
 *
 * A local read that touches no household collection. `cause: unknown` is its
 * only failure, meaning the preference file could not be read or holds content
 * this version cannot interpret.
 */
export function getDisplayWeightUnit(): Outcome<WeightUnit> {
  return commands.getDisplayWeightUnit();
}

/**
 * Store this device's display unit and return the effective one (SET-4).
 *
 * A display preference only: no stored value is read or written in that unit
 * (D0.4), which is why the command carries a request id without a service-side
 * deduplication step.
 */
export function setDisplayWeightUnit(unit: WeightUnit): Operation<WeightUnit> {
  return requestOperation<WeightUnit, CommandError>((requestId) =>
    commands.setDisplayWeightUnit(unit, requestId),
  );
}

/**
 * A redacted summary of the diagnostics sink: counts, never content (NFR-3).
 *
 * Idempotent, so it takes no request id; its only failure is an unreadable
 * sink, reported as `cause: unknown` with `request_id: null`.
 */
export function getDiagnosticsSummary(): Outcome<DiagnosticsSummary> {
  return commands.getDiagnosticsSummary();
}
