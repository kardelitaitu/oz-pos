import { useEffect, useRef } from 'react';

const STORAGE_KEY = 'auto-lock-minutes';
const DEFAULT_MINUTES = 5;

function getMinutes(): number {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v) {
      const n = parseInt(v, 10);
      if (Number.isFinite(n) && n >= 1) return n;
    }
  } catch { /* ignore */ }
  return DEFAULT_MINUTES;
}

const ACTIVITY_EVENTS = ['mousedown', 'keydown', 'touchstart', 'scroll', 'wheel'] as const;

/**
 * Monitor user activity and fire `onIdle` after a configurable period
 * of inactivity. Resets the timer on mousedown, keydown, touchstart,
 * scroll, or wheel events. The idle duration is read from localStorage
 * (`auto-lock-minutes`, default 5 minutes).
 */
export function useIdleTimer(onIdle: () => void) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onIdleRef = useRef(onIdle);
  onIdleRef.current = onIdle;

  const reset = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    const ms = getMinutes() * 60 * 1000;
    timerRef.current = setTimeout(() => {
      onIdleRef.current();
    }, ms);
  };

  useEffect(() => {
    reset();
    const handler = () => reset();
    for (const ev of ACTIVITY_EVENTS) {
      window.addEventListener(ev, handler, { passive: true });
    }
    return () => {
      for (const ev of ACTIVITY_EVENTS) {
        window.removeEventListener(ev, handler);
      }
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);
}

/** Read the persisted auto-lock idle duration in minutes. */
export function getAutoLockMinutes(): number {
  return getMinutes();
}

/** Persist the auto-lock idle duration (clamped 1–120 minutes). */
export function setAutoLockMinutes(minutes: number) {
  localStorage.setItem(STORAGE_KEY, String(Math.max(1, Math.min(120, minutes))));
}
