/**
 * Animation helpers — centralises duration constants and respects
 * `prefers-reduced-motion` so that JS-scheduled callbacks (e.g.
 * unmount after exit animation) fire immediately when motion is
 * disabled.
 */

const mql = typeof window !== 'undefined'
  ? window.matchMedia('(prefers-reduced-motion: reduce)')
  : null;

/** `true` when the user has requested reduced motion at the OS level. */
export const prefersReducedMotion = (): boolean => mql?.matches ?? false;

/**
 * Returns `0` when the user prefers reduced motion, otherwise
 * returns the given `ms` value. Use as the delay for `setTimeout`
 * that waits for a CSS exit animation to finish.
 *
 * @example
 * ```ts
 * const timer = setTimeout(cb, animDuration(MS_200));
 * ```
 */
export const animDuration = (ms: number): number =>
  prefersReducedMotion() ? 0 : ms;
