import { useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Hook that provides a toggleFullscreen function and sets up a global F11
 * keydown listener to toggle fullscreen mode via the Tauri window API.
 *
 * The handler prevents default browser F11 behavior and calls
 * `getCurrentWindow().setFullscreen()` to toggle.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   useFullscreen(); // just sets up the F11 listener
 *   // ...
 * }
 * ```
 */
export function useFullscreen() {
  const toggleRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const toggle = () => {
      try {
        const win = getCurrentWindow();
        win.isFullscreen().then((fs) => win.setFullscreen(!fs));
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
