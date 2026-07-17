import { useEffect, useCallback, useRef, type RefObject } from 'react';

/**
 * Reusable focus-trap hook for modal dialogs.
 *
 * When `active` is true, the hook:
 * 1. Auto-focuses the first focusable element inside the panel.
 * 2. Traps Tab / Shift+Tab cycling within the panel.
 * 3. Calls `onEscape` when Escape is pressed.
 * 4. Locks body scroll while the trap is active.
 *
 * Uses a ref for `onEscape` to avoid re-attaching the event listener
 * (and the associated body-scroll-lock flicker) when the callback
 * reference changes across renders.
 *
 * Mirrors the pattern used in `Modal.tsx`, `SettingsPopup.tsx`, and
 * the shared `frontend/shared/Modal.tsx`.
 *
 * @param panelRef - Ref to the dialog panel DOM element.
 * @param active   - Whether the trap should be active (typically `open && !exiting`).
 * @param onEscape - Called when Escape is pressed while the trap is active.
 */
export function useFocusTrap(
  panelRef: RefObject<HTMLElement | null>,
  active: boolean,
  onEscape: () => void,
): void {
  // Keep onEscape in a ref so handleKeyDown never needs to change,
  // preventing the effect from re-running (and scroll-lock flickering)
  // just because the parent passed a new inline callback.
  const onEscapeRef = useRef(onEscape);
  onEscapeRef.current = onEscape;

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onEscapeRef.current();
        return;
      }
      if (e.key !== 'Tab' || !panelRef.current) return;

      const focusable = panelRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;

      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    },
    [panelRef],
  );

  useEffect(() => {
    if (!active) return;

    const panel = panelRef.current;
    if (!panel) return;

    // Auto-focus the first focusable element inside the panel.
    const focusable = panel.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    focusable?.focus();

    // Lock body scroll.
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    // Listen for keyboard events.
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [active, panelRef, handleKeyDown]);
}
