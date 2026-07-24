/** Dirty-track: returns true if any key in draft differs from original. */
export function hasChanges<T extends Record<string, unknown>>(
  draft: T,
  original: T,
): boolean {
  return Object.keys(draft).some((k) => draft[k] !== original[k]);
}
