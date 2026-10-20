/*
 * The read side of a command's `Result` union (task 3.2).
 *
 * An `api.ts` wrapper resolves to
 * `{ status: 'ok', data } | { status: 'error', error }` for a command failure,
 * handing the backend's `CommandError` through untouched; the one rejection it
 * does not convert is an `Error`, which it re-throws instead (the exact split
 * is in `src/lib/request-id.ts`). A TanStack Query `queryFn` has to be a
 * promise that *rejects* on failure, so this is the one place that turns the
 * union into that shape.
 *
 * The rejection value is the error slot's value as it stands rather than a
 * wrapper `Error`, which is what keeps `cause`, `retryable`, `message_key` and
 * `request_id` intact for the failure surface (NFR-8) — a consumer renders the
 * same cause-specific copy the backend named, with nothing parsed out of a
 * message. `CommandError` there is a nominal type, not a runtime guarantee: an
 * unknown or denied command can put a plain string in the slot, so a caller
 * checks before it reads `cause` (see `request-id.ts`). The value is therefore
 * *not* an `Error` instance: it belongs in a query's or a mutation's error
 * state, not in a React error boundary.
 */
import type { CommandResult } from './request-id';

/** The value of a successful `Result`, e.g. `CommandData<Awaited<ReturnType<…>>>`. */
export type CommandData<R> = R extends { status: 'ok'; data: infer D }
  ? D
  : never;

/** The typed failure of a `Result`, derived from a wrapper's own signature. */
export type CommandFailure<R> = R extends { status: 'error'; error: infer E }
  ? E
  : never;

/** The command's value, or its typed failure thrown unchanged. */
export function unwrapCommandResult<T, E>(result: CommandResult<T, E>): T {
  if (result.status === 'error') {
    throw result.error;
  }

  return result.data;
}
