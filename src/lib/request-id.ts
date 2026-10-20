/*
 * The request-id discipline behind `src/features/*\/api.ts` (task 3.1,
 * design.md D0.14).
 *
 * `src/bindings.ts` already resolves every command to a `Result`-shaped union:
 * its generated `typedError` turns a command failure into
 * `{ status: 'error', error }` instead of a rejection, so a wrapper passes that
 * union through unchanged. A command failure is therefore never re-thrown as a
 * bare `Error` and never dropped — the caller narrows on `status` and renders
 * the backend's cause (NFR-8; the cause-to-copy map is task 3.4).
 *
 * What the generated wrapper cannot supply is the operation identity, because
 * it *receives* one. Each state-changing command takes a `RequestId`, and D0.14
 * makes that value semantic rather than diagnostic: the shared service
 * deduplicates on it, so a retry has to re-send the same value, and only a
 * genuinely new operation may mint a new one. That rule is a shape here instead
 * of a convention. `requestOperation` mints the id once, when the handle is
 * created, and every `run()` sends that same id — retrying is `run()` again,
 * and there is no way to retry through this module with a fresh id. The handle
 * is what a failure surface keeps while it offers the retry, so the id travels
 * with the attempt that has to be re-issued (task 3.4's retry state). With
 * TanStack Query the handle is the mutation's variable and `mutationFn` calls
 * `run()`, so re-submitting the same handle is the retry rather than a new
 * operation.
 *
 * A rejected `Error` instance is deliberately **not** converted into a
 * `CommandError`, and it is the only rejection value the generated `typedError`
 * rethrows; every other rejection value is passed through **untyped**. Tauri
 * answers an unknown or denied command with a plain string, which `typedError`
 * returns as `{ status: 'error', error: <string> }` even though
 * `CommandResult` types that slot as `CommandError`. An `Error` is the IPC call
 * itself failing — an argument that will not serialise, a broken webview
 * bridge — rather than a command reporting a failure, and dressing it up as a
 * `CommandError` would invent a cause and a message key the backend never
 * produced, so it is left to propagate and surface as the defect it is.
 *
 * The consequence is a typing caveat rather than a behaviour: an error leaving
 * this layer is *usually* the backend's `CommandError`, but that is not
 * guaranteed, so `cause`, `retryable` and `message_key` have to be checked for
 * before they are read. A command that actually reported a failure is the only
 * source of a typed `CommandError`.
 */

/**
 * The identity of one client-originated operation — `RequestId` in the
 * contract. Opaque, minted here, and stable across a retry.
 */
export type RequestId = string;

/**
 * The `Result` shape every command resolves to, as `src/bindings.ts` emits it:
 * `ok` carries the value, `error` carries the backend's `CommandError`.
 */
export type CommandResult<T, E> =
  | { status: 'ok'; data: T }
  | { status: 'error'; error: E };

/**
 * One state-changing command, callable more than once under one identity.
 *
 * `run()` is both the first attempt and the retry: each sends `requestId`, so a
 * re-issued operation cannot be mistaken for a new one.
 */
export type RequestOperation<T, E> = {
  /** The operation identity, sent to the command and echoed by its failure. */
  readonly requestId: RequestId;
  /** Call the command. Calling it again retries with the same `requestId`. */
  run(): Promise<CommandResult<T, E>>;
};

/**
 * Mint a fresh operation identity.
 *
 * WebCrypto is the source rather than a counter or a timestamp: the value only
 * has to be unique per account, and 128 random bits are also unique across the
 * devices of one account without any coordination.
 *
 * `getRandomValues` is used rather than `randomUUID` on purpose: `randomUUID`
 * is `[SecureContext]` and the Tauri webview that loads the app from its custom
 * scheme is not a secure context, so it would be missing exactly where the app
 * runs. The contract fixes no format anyway, so plain hex is as valid as a
 * UUID-shaped string.
 */
export function createRequestId(): RequestId {
  const bytes = globalThis.crypto.getRandomValues(new Uint8Array(16));
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0'));

  return hex.join('');
}

/**
 * Create an operation and mint its identity once.
 *
 * The id is minted here rather than inside `run()` so that a second `run()`
 * re-issues the same operation. That distinction is the whole of D0.14: a
 * retry that minted a new id would be a second create, which is exactly the
 * duplicated write R6 exists to prevent. An operation is bound to the input it
 * was created with, so a corrected input is a new operation with its own id.
 */
export function requestOperation<T, E>(
  command: (requestId: RequestId) => Promise<CommandResult<T, E>>,
): RequestOperation<T, E> {
  const requestId = createRequestId();

  return { requestId, run: () => command(requestId) };
}
