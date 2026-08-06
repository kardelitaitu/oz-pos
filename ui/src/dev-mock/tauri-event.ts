/**
 * Dev-mode mock for @tauri-apps/api/event
 *
 * Provides a tiny in-memory pub/sub so the app's `listen()` calls (e.g.
 * `kds:orders-changed`, hardware scan events) work in the browser without
 * the Rust backend — and so dev-mock Tauri commands can `emit()` the same
 * events to drive the full E2E lifecycle (KDS ticket advance → refresh).
 *
 * Aliased in vite.config.ts:
 *   { find: /^@tauri-apps\/api\/event$/, replacement: '/src/dev-mock/tauri-event.ts' }
 *
 * Surface mirrors the Tauri v2 API subset actually imported by the app:
 *   listen, once, emit, emitTo, UnlistenFn
 */

/** A listener registered for a given event name. */
type EventListener = (payload: unknown) => void;

const listeners = new Map<string, Set<EventListener>>();

/** Remove a previously-registered listener. */
export type UnlistenFn = () => void;

/**
 * Listen for an event and return an unlisten function. Mirrors the Tauri
 * v2 signature (`listen(event, handler)` resolves to `UnlistenFn`).
 */
export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  const wrapped: EventListener = (payload: unknown) => handler({ payload: payload as T });
  const set = listeners.get(event) ?? new Set<EventListener>();
  set.add(wrapped);
  listeners.set(event, set);
  return () => {
    set.delete(wrapped);
  };
}

/** Listen once — auto-unsubscribes after the first delivery. */
export async function once<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen<T>(event, (e) => {
    unlisten();
    handler(e);
  });
  return unlisten;
}

/** Emit an event to all local listeners (dev-only — no IPC needed). */
export async function emit(event: string, payload?: unknown): Promise<void> {
  const set = listeners.get(event);
  if (!set) return;
  // Copy so a listener can safely unlisten during iteration.
  for (const fn of [...set]) {
    fn(payload);
  }
}

/** Emit to a specific target (dev mock treats all targets as local). */
export async function emitTo(
  target: string,
  event: string,
  payload?: unknown,
): Promise<void> {
  void target;
  return emit(event, payload);
}
