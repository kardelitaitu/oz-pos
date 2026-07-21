import { useState, useEffect, useCallback } from 'react';

/**
 * Current screen orientation state.
 */
export interface OrientationState {
  /** Whether the screen is currently in landscape orientation. */
  isLandscape: boolean;
  /** Current angle in degrees (0, 90, 180, 270). */
  angle: number;
  /** Screen.width or screen.height depending on orientation. */
  viewportWidth: number;
  /** Screen.height or screen.width depending on orientation. */
  viewportHeight: number;
}

/**
 * Result of useOrientation hook.
 */
export interface OrientationResult {
  /** Current orientation state. */
  orientation: OrientationState;
  /** True while the lock request is in-flight on supported browsers. */
  locking: boolean;
  /** Whether the screen orientation API is available on this device. */
  supported: boolean;
  /**
   * Attempt to lock the screen to a specific orientation.
   * On unsupported browsers this is a no-op.
   */
  lock: (type: string) => Promise<void>;
  /** Unlock the screen orientation. */
  unlock: () => void;
}

/** Known shape of the ScreenOrientation API (not fully typed in TS DOM lib). */
interface ScreenOrientationAPI {
  lock?: (type: string) => Promise<void>;
  unlock?: () => void;
  angle?: number;
  type?: string;
}

/**
 * Helper to access the ScreenOrientation API with proper typing.
 * The TypeScript DOM lib types don't include `lock`/`unlock` on
 * ScreenOrientation, so we use a locally-defined interface.
 */
function getScreenOrientation(): ScreenOrientationAPI | null {
  try {
    const orient = (window.screen as { orientation?: ScreenOrientationAPI }).orientation;
    if (!orient) return null;
    const result: ScreenOrientationAPI = {};
    if (typeof orient.lock === 'function') {
      result.lock = orient.lock.bind(orient);
    }
    if (typeof orient.unlock === 'function') {
      result.unlock = orient.unlock.bind(orient);
    }
    if (typeof orient.angle === 'number') {
      result.angle = orient.angle;
    }
    return result;
  } catch {
    return null;
  }
}

/**
 * Hook that tracks screen orientation and provides lock/unlock control.
 *
 * On tablet POS screens, use:
 *   useOrientation('landscape-primary');
 *
 * The hook also listens for `orientationchange` and `resize` events so
 * the consuming component can reflow its layout when the device rotates.
 * Only the side effect (locking + listening) is needed in most cases;
 * consume the returned `orientation` state only if the component needs
 * to reflow on orientation change.
 */
export function useOrientation(
  /** Optional orientation to lock on mount. */
  initialLock?: string,
): OrientationResult {
  const [locking, setLocking] = useState(false);
  const [supported, setSupported] = useState(false);

  const getOrientationState = useCallback((): OrientationState => {
    const isLandscape = window.innerWidth > window.innerHeight;
    const screenOrientation = getScreenOrientation();
    const angle = screenOrientation?.angle ?? 0;
    return {
      isLandscape,
      angle,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    };
  }, []);

  const [orientation, setOrientation] = useState<OrientationState>(
    getOrientationState,
  );

  // Check if ScreenOrientation API is available.
  useEffect(() => {
    const so = getScreenOrientation();
    setSupported(typeof so?.lock === 'function');
  }, []);

  // Lock to initial orientation on mount.
  useEffect(() => {
    if (!initialLock) return;
    if (!supported) return;

    const screenOrientation = getScreenOrientation();
    if (!screenOrientation?.lock) return;

    setLocking(true);
    screenOrientation
      .lock(initialLock)
      .catch(() => {
        // Orientation lock may fail on devices that don't support it
        // (browser permission, iframe, etc.). Silently degrade.
      })
      .finally(() => setLocking(false));

    return () => {
      // Unlock on unmount.
      try {
        getScreenOrientation()?.unlock?.();
      } catch {
        // Ignore unlock failures.
      }
    };
  }, [initialLock, supported]);

  // Listen for orientation changes + window resize.
  useEffect(() => {
    const handleChange = () => {
      setOrientation(getOrientationState());
    };

    window.addEventListener('orientationchange', handleChange);
    window.addEventListener('resize', handleChange);
    return () => {
      window.removeEventListener('orientationchange', handleChange);
      window.removeEventListener('resize', handleChange);
    };
  }, [getOrientationState]);

  const lock = useCallback(
    async (type: string) => {
      if (!supported) return;
      const screenOrientation = getScreenOrientation();
      if (!screenOrientation?.lock) return;
      setLocking(true);
      try {
        await screenOrientation.lock(type);
        setOrientation(getOrientationState());
      } finally {
        setLocking(false);
      }
    },
    [supported, getOrientationState],
  );

  const unlock = useCallback(() => {
    if (!supported) return;
    try {
      getScreenOrientation()?.unlock?.();
    } catch {
      // Ignore.
    }
  }, [supported]);

  return { orientation, locking, supported, lock, unlock };
}
