import { useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Hook that provides a toggleFullscreen function and sets up a global F11
 * keydown listener to toggle fullscreen mode via the Tauri window API.
 *
 * The handler prevents default browser F11 behavior and calls
 * `getCurrentWindow().setFullscreen()` to toggle.
 *
 * @param onToggle Optional callback fired after the fullscreen state changes.
 *   Receives `true` when entering fullscreen, `false` when exiting.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   useFullscreen(); // just sets up the F11 listener
 *   // ...
 * }
 *
 * function MyOtherComponent() {
 *   const { addToast } = useToast();
 *   useFullscreen((isFullscreen) => {
 *     addToast({ type: 'info', message: isFullscreen ? 'Fullscreen mode enabled' : 'Fullscreen mode disabled' });
 *   });
 * }
 * ```
 */
export function useFullscreen(onToggle?: (isFullscreen: boolean) => void) {
  const toggleRef = useRef<(() => void) | null>(null);
  // Keep the latest callback in a ref so the effect closure always calls the newest version.
  const onToggleRef = useRef(onToggle);
  onToggleRef.current = onToggle;

  useEffect(() => {
    const toggle = () => {
      try {
        const win = getCurrentWindow();
        win.isFullscreen().then((fs) => {
          const newState = !fs;
          win.setFullscreen(newState);
          onToggleRef.current?.(newState);
        });
      } catch {
        // Not running in Tauri — ignore
      }
    };
    toggleRef.current = toggle;

    const handler = (e: KeyboardEvent) => {
      if (e.key === 'F11') {
        e.preventDefault();
        toggle();
      }
    };

    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, []);

  const toggleFullscreen = useCallback(() => {
    toggleRef.current?.();
  }, []);

  return { toggleFullscreen };
}
