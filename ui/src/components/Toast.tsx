import {
  createContext,
  useContext,
  useCallback,
  useState,
  useEffect,
  useRef,
  type ReactNode,
} from 'react';

// ── Types ──────────────────────────────────────────────────────────

export type ToastType = 'success' | 'error' | 'warning' | 'info';

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
}

// ── Context ────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue | null>(null);

// ── Hook ────────────────────────────────────────────────────────────

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

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = useCallback(
    (t: Omit<Toast, 'id'> & { id?: string }) => {
      const id = t.id ?? generateId();
      setToasts((prev) => [...prev, { ...t, id }]);
      return id;
    },
    [],
  );

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <ToastContext.Provider value={{ addToast, removeToast }}>
      {children}
      <ToastContainer toasts={toasts} onDismiss={removeToast} />
    </ToastContext.Provider>
  );
}

// ── Individual Toast ────────────────────────────────────────────────

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: string) => void;
}) {
  const { id, type, message, duration = 4000 } = toast;
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (duration <= 0) return;
    timerRef.current = setTimeout(() => onDismiss(id), duration);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [id, duration, onDismiss]);

  return (
    <div
      className={`toast toast--${type}`}
      role="alert"
      aria-live="assertive"
    >
      <span className="toast__message">{message}</span>
      <button
        type="button"
        className="toast__dismiss"
        onClick={() => onDismiss(id)}
        aria-label="Dismiss notification"
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
  toasts,
  onDismiss,
}: {
  toasts: Toast[];
  onDismiss: (id: string) => void;
}) {
  if (toasts.length === 0) return null;

  return (
    <div className="toast-container" aria-label="Notifications">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onDismiss={onDismiss} />
      ))}
    </div>
  );
}
