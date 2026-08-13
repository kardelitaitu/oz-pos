import { useRef, useCallback } from 'react';

/**
 * Action cooldown hook for KDS touch interaction safety.
 *
 * Prevents accidental double-taps by enforcing a minimum delay (default 200ms)
 * between successive invocations of the same action. Returns a wrapped callback
 * that only fires if the cooldown has elapsed since the last invocation.
 *
 * During the cooldown period, `cooldownActive` is `true` — consumers can use
 * this to provide visual feedback (e.g. reduced opacity).
 *
 * @param cooldownMs — minimum milliseconds between action invocations (default 200)
 */
export function useActionCooldown<T extends (...args: never[]) => void>(
  action: T,
  cooldownMs = 200,
): { debouncedAction: T; cooldownActive: boolean } {
  const lastFired = useRef(0);
  const cooldownActive = useRef(false);

  const debouncedAction = useCallback(
    (...args: Parameters<T>) => {
      const now = Date.now();
      if (now - lastFired.current < cooldownMs) return;
      lastFired.current = now;
      cooldownActive.current = true;
      action(...args);
      // Reset cooldown flag after the cooldown window.
      setTimeout(() => {
        cooldownActive.current = false;
      }, cooldownMs);
    },
    [action, cooldownMs],
  ) as T;

  return { debouncedAction, cooldownActive: cooldownActive.current };
}

/**
 * Creates a stable cooldown wrapper for a callback. Unlike `useActionCooldown`,
 * this does not need to be called inside a component — it returns a wrapper
 * function suitable for use inside `useCallback` or `useMemo`.
 *
 * Usage:
 * ```
 * const handleClick = useMemo(
 *   () => createCooldownWrapper(() => onAdvance(order), 200),
 *   [onAdvance, order],
 * );
 * ```
 */
export function createCooldownWrapper<T extends (...args: never[]) => void>(
  action: T,
  cooldownMs = 200,
): T {
  let lastFired = 0;
  return ((...args: Parameters<T>) => {
    const now = Date.now();
    if (now - lastFired < cooldownMs) return;
    lastFired = now;
    action(...args);
  }) as T;
}
