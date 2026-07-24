/**
 * Shared nested-modal depth tracker for WorkspaceSettingsModal (ADR #22 Phase 4).
 *
 * Separated from the component to satisfy react-refresh/only-export-components
 * (Fast Refresh only works when a file exports components exclusively).
 */

let nestedModalDepth = 0;
const listeners = new Set<(depth: number) => void>();

/** Subscribe to depth changes. Returns unsubscribe function. */
export function onNestedDepthChange(cb: (depth: number) => void): () => void {
  listeners.add(cb);
  return () => { listeners.delete(cb); };
}

/** Increment nested depth (called by inner SettingsPopup on open). */
export function enterNestedModal(): void {
  nestedModalDepth += 1;
  listeners.forEach((cb) => cb(nestedModalDepth));
}

/** Decrement nested depth (called by inner SettingsPopup on close). */
export function exitNestedModal(): void {
  nestedModalDepth = Math.max(0, nestedModalDepth - 1);
  listeners.forEach((cb) => cb(nestedModalDepth));
}

/** Get current nested modal depth. */
export function getNestedDepth(): number {
  return nestedModalDepth;
}
