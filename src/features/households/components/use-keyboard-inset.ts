/*
 * The one assignment of `--keyboard-inset` (task 4.1; Q21).
 *
 * `semantic.css` defines the token, `Screen` reserves it (padding and scroll
 * padding at the bottom of the scroll region), and until this task nothing wrote
 * it, so it stayed `0px`. This hook measures the software keyboard from
 * `visualViewport` and writes the token on the root element; it is mounted by
 * the first real form screen, the first-run household creation screen, and by no
 * other surface.
 *
 * **Why `visualViewport` and not `window.innerHeight` alone.** On iOS the
 * keyboard does not shrink the layout viewport — `100vh`/`innerHeight` keep the
 * full height — while the visual viewport becomes the part actually visible
 * above the keyboard. The overlap is therefore
 *
 *   innerHeight − visualViewport.height − visualViewport.offsetTop
 *
 * `offsetTop` matters as well: iOS may pan the layout viewport to keep the
 * focused input visible, and the panned distance is not keyboard.
 *
 * **Zero whenever no keyboard is reported.** The result is clamped at zero, and a
 * pinch-zoom (`scale !== 1`) is treated as no keyboard: a zoom shrinks the visual
 * viewport for a reason that is not a keyboard, and padding a screen for it would
 * be wrong. On a desktop window the visual viewport equals the layout viewport,
 * so the token is pinned at `0px` — the state `semantic.css` documents.
 *
 * **Event discipline.** Both `resize` and `scroll` fire repeatedly while the
 * keyboard animates (and `scroll` fires on every pan), so writes are coalesced
 * into one animation frame; one listener pair is registered on mount and removed
 * on unmount, together with the frame and the property it wrote. Removing the
 * property restores the stylesheet's `0px` rather than leaving a stale value
 * behind for the next screen.
 *
 * `visualViewport` is absent in jsdom (and in older WebViews), where the hook
 * does nothing — a component spec therefore sees the token's defined value, not
 * a measurement it cannot make.
 */
import { useEffect } from 'react';

export function useKeyboardInset(): void {
  useEffect(() => {
    const viewport: VisualViewport | null = window.visualViewport ?? null;

    if (viewport === null) {
      return;
    }

    const root = document.documentElement;
    let frame: number | undefined;

    const write = () => {
      frame = undefined;

      const overlap =
        viewport.scale === 1
          ? Math.round(
              window.innerHeight - viewport.height - viewport.offsetTop,
            )
          : 0;

      root.style.setProperty(
        '--keyboard-inset',
        `${Math.max(0, overlap).toString()}px`,
      );
    };

    const schedule = () => {
      if (frame !== undefined) {
        return;
      }

      frame = window.requestAnimationFrame(write);
    };

    // The initial value matters on a remount while the keyboard is already open.
    write();
    viewport.addEventListener('resize', schedule);
    viewport.addEventListener('scroll', schedule);

    return () => {
      viewport.removeEventListener('resize', schedule);
      viewport.removeEventListener('scroll', schedule);

      if (frame !== undefined) {
        window.cancelAnimationFrame(frame);
      }

      root.style.removeProperty('--keyboard-inset');
    };
  }, []);
}
