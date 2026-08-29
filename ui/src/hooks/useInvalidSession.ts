//! Detect when a Tauri command fails with `InvalidSession` and surface
//! a dismissible call-to-action banner so the user can re-authenticate
//! instead of seeing "Your session has expired" on every card.

import { useEffect, useRef, useState } from 'react';
import { onIpcError } from '@/utils/app-error';

/**
 * Track whether any recent IPC command failed with `InvalidSession`.
 *
 * Returns `true` for ~5 seconds after the first `invalidSession` error,
 * then auto-clears. This avoids a persistent banner that never goes away
 * if the user has already signed in again (the new session's commands
 * succeed, so the old error is no longer relevant).
 */
export function useInvalidSession(): boolean {
  const [triggered, setTriggered] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const unsub = onIpcError((event) => {
      if (event.error.kind !== 'invalidSession') return;
      setTriggered(true);
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        setTriggered(false);
      }, 5000);
    });
    return () => {
      unsub();
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  return triggered;
}