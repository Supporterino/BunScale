/*
 * `data-*` attributes a screen may put on a primitive's own element.
 *
 * The primitives do not spread arbitrary HTML props: their surface is the
 * declared props, so a behavior cannot be changed from a call site by accident.
 * A `data-*` attribute is the one thing a caller genuinely needs to add — the
 * household entry guard publishes `data-household-entry` on its `main`, which is
 * the stable name of the decision its specs assert — and without this type the
 * feature would either fork the primitive or wrap a second landmark around it.
 *
 * It lives in its own module because `Screen` and `AppShell` both take one (the
 * screen's root element and the shell's `main`), and two copies of the
 * template-literal signature would drift. It is an internal module, not an
 * inventory primitive: no screen composes it directly.
 */
export type DataAttributes = {
  [key: `data-${string}`]: string | undefined;
};
