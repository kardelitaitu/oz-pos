/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';

import { useLocalization } from '@fluent/react';

// ── Types ──────────────────────────────────────────────────────────

/** Visual variant for a toast notification. */
export type ToastVariant = 'success' | 'error' | 'warning' | 'info';

/** A single toast notification with auto-incremented id. */
export interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
}

interface ToastContextValue {
  toasts: Toast[];
  addToast: (message: string, variant?: ToastVariant) => void;
  removeToast: (id: string) => void;
}

// ── Context ────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue | null>(null);

// ── Provider ───────────────────────────────────────────────────────

let toastCounter = 0;

/**
 * Provides toast notification context to the component tree.
 * Renders a `<div className="toast-container">` portal for displaying
 * toasts with auto-dismiss after 4 seconds.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const { l10n } = useLocalization();
  const [toasts, setToasts] = useState<Toast[]>([]);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const addToast = useCallback(
    (message: string, variant: ToastVariant = 'info') => {
      const id = `toast-${++toastCounter}`;
      setToasts((prev) => [...prev, { id, message, variant }]);
      // Auto-dismiss after 4 seconds.
      setTimeout(() => removeToast(id), 4000);
    },
    [removeToast],
  );

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      {/* Toast container */}
      <div className="toast-container" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast--${t.variant}`}>
            <span className="toast__message">{t.message}</span>
            <button
              type="button"
              className="toast__dismiss"
              onClick={() => removeToast(t.id)}
              aria-label={l10n.getString('toast-dismiss-aria')}
            >
              &times;
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

// ── Hook ───────────────────────────────────────────────────────────

/** Access the toast context. Must be used within a `<ToastProvider>`. */
export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error('useToast must be used within a <ToastProvider>');
  }
  return ctx;
}
