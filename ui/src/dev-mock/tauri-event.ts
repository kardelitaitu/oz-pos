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
 * True when running inside a real Tauri webview (packaged app or `tauri dev`).
 *
 * The mock is aliased in for the dev server, but a real webview provides
 * `window.__TAURI_INTERNALS__` — in that case we MUST delegate to the actual
 * Rust event system instead of the in-memory pub/sub (the Jul 2026
 * regression where the unconditional alias shipped mock IPC into
 * production builds).
 */
function hasTauriInternals(): boolean {
  try {
    return (
      typeof window !== 'undefined' &&
      typeof (window as unknown as { __TAURI_INTERNALS__?: { invoke?: unknown } })
        .__TAURI_INTERNALS__?.invoke === 'function'
    );
  } catch {
    return false;
  }
}

/** Real event-plugin listener id, as resolved by the Rust backend. */
interface TauriInternals {
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
  transformCallback: (cb: (event: unknown) => void, once?: boolean) => { id: number };
}

/** Event target, mirroring the real @tauri-apps/api/event EventTarget shape. */
export interface EventTargetOption {
  kind: 'Any' | 'AnyLabel';
  label?: string;
}

/** Event listening options, mirroring the real API's EventOptions. */
export interface EventOptions {
  target?: string | EventTargetOption;
}

/**
 * Resolve the event target the way @tauri-apps/api/event does: a plain
 * string becomes { kind: 'AnyLabel', label }, anything else is used as-is.
 */
function resolveTarget(target?: string | EventTargetOption): EventTargetOption {
  if (typeof target === 'string') return { kind: 'AnyLabel', label: target };
  return target ?? { kind: 'Any' };
}

/**
 * Listen for an event and return an unlisten function. Mirrors the Tauri
 * v2 signature (`listen(event, handler, options?)` resolves to `UnlistenFn`).
 */
export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
  options?: EventOptions,
): Promise<UnlistenFn> {
  if (hasTauriInternals()) {
    const internals = (window as unknown as { __TAURI_INTERNALS__: TauriInternals }).__TAURI_INTERNALS__;
    // Mirrors @tauri-apps/api/event listen(): register the handler callback
    // with the Rust backend and resolve the event id for unlisten.
    const cb = internals.transformCallback((e: unknown) =>
      handler({ payload: (e as { payload: T }).payload }),
    );
    const eventId = (await internals.invoke('plugin:event|listen', {
      event,
      target: resolveTarget(options?.target),
      handler: cb,
    })) as number;
    return async () => {
      // Mirrors _unlisten() in @tauri-apps/api/event.
      (window as unknown as {
        __TAURI_EVENT_PLUGIN_INTERNALS__?: { unregisterListener: (e: string, id: number) => void };
      }).__TAURI_EVENT_PLUGIN_INTERNALS__?.unregisterListener(event, eventId);
      await internals.invoke('plugin:event|unlisten', { event, eventId });
    };
  }

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
  options?: EventOptions,
): Promise<UnlistenFn> {
  const unlisten = await listen<T>(event, (e) => {
    unlisten();
    handler(e);
  }, options);
  return unlisten;
}

/** Emit an event to all local listeners (dev-only — no IPC needed). */
export async function emit(event: string, payload?: unknown): Promise<void> {
  if (hasTauriInternals()) {
    const internals = (window as unknown as { __TAURI_INTERNALS__: TauriInternals }).__TAURI_INTERNALS__;
    await internals.invoke('plugin:event|emit', { event, payload });
    return;
  }

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
  if (hasTauriInternals()) {
    const internals = (window as unknown as { __TAURI_INTERNALS__: TauriInternals }).__TAURI_INTERNALS__;
    await internals.invoke('plugin:event|emit_to', {
      target: { kind: 'AnyLabel', label: target },
      event,
      payload,
    });
    return;
  }

  void target;
  return emit(event, payload);
}
