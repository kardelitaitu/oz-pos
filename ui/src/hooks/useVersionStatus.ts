//! `useVersionStatus` — current app version + update availability.
//!
//! Reads the running app version via the Tauri core API and checks for
//! updates via `@tauri-apps/plugin-updater`. Returns a two-state result
//! (latest / update available) plus the version strings for display.

import { useState, useEffect, useRef } from 'react';
import { getVersion } from '@/api/tauri';

export type VersionState = 'checking' | 'latest' | 'update';

export interface VersionStatusInfo {
  state: VersionState;
  /** The currently running app version (e.g. "0.0.34"). */
  currentVersion: string;
  /** The available update version, if any. */
  availableVersion: string | null;
}

/**
 * Check for update availability once on mount.
 *
 * - `checking` — initial state before the updater plugin responds.
 * - `latest` — either no updater plugin (browser/dev) or `check()` returned null.
 * - `update` — `check()` returned an Update with a valid version.
 */
export function useVersionStatus(): VersionStatusInfo {
  const [state, setState] = useState<VersionState>('checking');
  const [currentVersion, setCurrentVersion] = useState('0.0.0');
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    (async () => {
      // Read the running version from the Tauri app API.
      try {
        const ver = await getVersion();
        if (!mountedRef.current) return;
        setCurrentVersion(ver);
      } catch {
        if (!mountedRef.current) return;
        setCurrentVersion('0.0.0');
      }

      // Check for updates via the updater plugin.
      try {
        const updater = await import('@tauri-apps/plugin-updater');
        const update = await updater.check();
        if (!mountedRef.current) return;

        if (update) {
          setAvailableVersion(update.version);
          setState('update');
        } else {
          setState('latest');
        }
      } catch {
        // Updater plugin not available (browser / dev).
        if (!mountedRef.current) return;
        setState('latest');
      }
    })();

    return () => { mountedRef.current = false; };
  }, []);

  return { state, currentVersion, availableVersion };
}