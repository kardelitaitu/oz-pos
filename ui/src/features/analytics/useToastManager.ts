import { useCallback, useEffect, useRef, useState } from 'react';
import { animDuration } from '@/utils/animation';

/**
 * Toast feedback for the analytics dashboard — extracted from
 * `AnalyticsScreen.tsx` (Phase 1 split). Owns the toast list, the auto-
 * dismiss timers, and the two-phase exit (fade out, then unmount) so a
 * toast never snaps away.
 */

export interface AnalyticsToast {
  id: number;
  message: string;
  exiting?: boolean;
}

export function useToastManager() {
  const [toasts, setToasts] = useState<AnalyticsToast[]>([]);
  const toastId = useRef(0);
  const toastTimersRef = useRef(new Map<number, ReturnType<typeof setTimeout>>());

  // Transient toast feedback — auto-dismisses per toast. Dismissal runs a
  // two-phase exit (fade out, then unmount) so a toast never snaps away.
  const dismissToast = useCallback((id: number) => {
    // Phase 1: mark exiting → the `--exiting` mirror keyframe runs.
    setToasts((t) => t.map((x) => (x.id === id ? { ...x, exiting: true } : x)));
    // Phase 2: unmount after the exit animation completes.
    const timer = setTimeout(() => {
      setToasts((t) => t.filter((x) => x.id !== id));
      toastTimersRef.current.delete(id);
    }, animDuration(250));
    toastTimersRef.current.set(id, timer);
  }, []);

  const showToast = useCallback(
    (message: string) => {
      const id = ++toastId.current;
      setToasts((t) => [...t.slice(-2), { id, message }]);
      const timer = setTimeout(() => dismissToast(id), 2600);
      toastTimersRef.current.set(id, timer);
    },
    [dismissToast],
  );

  // Cancel any in-flight toast timers on unmount — never setState against
  // an unmounted component.
  useEffect(() => {
    const timers = toastTimersRef.current;
    return () => {
      for (const t of timers.values()) clearTimeout(t);
      timers.clear();
    };
  }, []);

  return { toasts, showToast, dismissToast };
}
