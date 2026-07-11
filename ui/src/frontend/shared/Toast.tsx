/* eslint-disable react-refresh/only-export-components */
import {

  createContext,
  useContext,
  useCallback,
  type ReactNode,
} from 'react';
import { useLocalization } from '@fluent/react';
import { useAnimatedToastQueue } from '@/hooks/useAnimatedToastQueue';

// ── Types ──────────────────────────────────────────────────────────

/** Visual variant for a toast notification in the animated queue. */
export type ToastType = 'success' | 'error' | 'warning' | 'info';

/** A single toast in the animated queue with auto-dismiss support. */
export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  /** Auto-dismiss duration in ms. 0 = persistent. @default 4000 */
  duration?: number;
}

interface ToastContextValue {
  addToast: (toast: Omit<Toast, 'id'> & { id?: string }) => string;
  removeToast: (id: string) => void;
  /**
   * Race-safe dismiss-all with coordinated exit fade. Items
   * enqueued during the fade (whose ids are not in the snapshot)
   * survive. Useful for "reset / restart" UX or any action that
   * needs to clear the entire notification queue in one go.
   */
  clearToasts: () => void;
}

// ── Context ────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue | null>(null);

// ── Hook ────────────────────────────────────────────────────────────

/** Access the animated toast context. Must be used within a `<ToastProvider>`. */
export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return ctx;
}

// ── ID generator ────────────────────────────────────────────────────

let toastCounter = 0;
function generateId(): string {
  toastCounter += 1;
  return `toast-${toastCounter}-${Date.now()}`;
}

// ── Provider ────────────────────────────────────────────────────────

/**
 * ToastProvider — owns the animated toast queue, centralised timer
 * cleanup, and per-item isExiting state via `useAnimatedToastQueue`.
 *
 * Each toast added via `addToast()` schedules an auto-dismiss timer
 * (default 4000 ms, or the per-item `duration`). User-initiated
 * dismissal (× click) OR auto-expiry both flow through `removeToast`
 * → `queue.dismiss(id)` → adds the id to `exitingIds` → 200 ms
 * mirror CSS fade → final unmount.
 *
 * `clearToasts()` triggers the race-safe collective fade: snapshots
 * current ids, fades them all, and on the timer fire removes only
 * snapshot ids. Items enqueued DURING the fade (not in snapshot)
 * survive, matching the undo-pill's race-safety contract.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const queue = useAnimatedToastQueue<Toast>({
    getId: (t) => t.id,
    getAutoDismissMs: (t) => t.duration ?? 4000,
  });

  const addToast = useCallback(
    (t: Omit<Toast, 'id'> & { id?: string }) => {
      const id = t.id ?? generateId();
      queue.enqueue({ ...t, id });
      return id;
    },
    [queue],
  );

  const removeToast = useCallback(
    (id: string) => {
      queue.dismiss(id);
    },
    [queue],
  );

  const clearToasts = useCallback(() => {
    queue.clearAll();
  }, [queue]);

  return (
    <ToastContext.Provider value={{ addToast, removeToast, clearToasts }}>
      {children}
      <ToastContainer
        items={queue.items}
        exitingIds={queue.exitingIds}
        onDismiss={removeToast}
      />
    </ToastContext.Provider>
  );
}

// ── Individual Toast ────────────────────────────────────────────────

function ToastItem({
  toast,
  isExiting,
  onDismiss,
}: {
  toast: Toast;
  isExiting: boolean;
  onDismiss: (id: string) => void;
}) {
  const { l10n } = useLocalization();
  const { id, type, message } = toast;

  return (
    <div
      className={`toast toast--${type}${isExiting ? ' toast--exiting' : ''}`}
      role="alert"
      aria-live="assertive"
      aria-busy={isExiting}
      data-toast-id={id}
    >
      <span className="toast__message">{message}</span>
      <button
        type="button"
        className="toast__dismiss"
        onClick={() => onDismiss(id)}
        disabled={isExiting}
        aria-label={l10n.getString('toast-dismiss-aria')}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  );
}

// ── Toast Container ─────────────────────────────────────────────────

function ToastContainer({
  items,
  exitingIds,
  onDismiss,
}: {
  items: readonly Toast[];
  exitingIds: ReadonlySet<string>;
  onDismiss: (id: string) => void;
}) {
  const { l10n } = useLocalization();
  if (items.length === 0) return null;

  return (
    <div className="toast-container" aria-label={l10n.getString('toast-notifications-aria')}>
      {items.map((t) => (
        <ToastItem
          key={t.id}
          toast={t}
          isExiting={exitingIds.has(t.id)}
          onDismiss={onDismiss}
        />
      ))}
    </div>
  );
}
