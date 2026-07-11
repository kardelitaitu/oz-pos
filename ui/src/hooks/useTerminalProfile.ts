import { useState, useEffect, useMemo } from 'react';
import { getTerminalProfile, listTerminals, type TerminalProfileDto } from '@/api/terminals';

/** Return type of the `useTerminalProfile` hook. */
export interface UseTerminalProfileResult {
  /** The loaded terminal profile, or null if not set / not found. */
  profile: TerminalProfileDto | null;
  /** True while the profile is being loaded. */
  loading: boolean;
  /** True if the terminal is in kds_kiosk lockdown mode. */
  isKdsKiosk: boolean;
  /** Error message if profile loading failed. */
  error: string | null;
}

/**
 * Load the terminal profile for the current device.
 *
 * Determines the current terminal by listing all terminals and matching
 * the hostname-env device ID (or falling back to listing).
 *
 * If the profile's `profileType` is `'kds_kiosk'`, the front-end should
 * force the KDS route and hide all navigation.
 */
export function useTerminalProfile(): UseTerminalProfileResult {
  const [profile, setProfile] = useState<TerminalProfileDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        // First, try to get the current terminal ID from env or match by device.
        // We list terminals and pick the first one (this is a simplification —
        // in production, the terminal ID is passed as a Tauri env var).
        const terminals = await listTerminals();
        if (cancelled) return;

        if (terminals.length === 0) {
          setProfile(null);
          setLoading(false);
          return;
        }

        // Use the first active terminal's ID to look up its profile.
        const activeTerminal = terminals.find((t) => t.isActive) ?? terminals[0];
        if (!activeTerminal) {
          setProfile(null);
          setLoading(false);
          return;
        }
        const terminalProfile = await getTerminalProfile(activeTerminal.id);

        if (!cancelled) {
          setProfile(terminalProfile);
        }
      } catch (err) {
        if (!cancelled) {
          setError(
            err instanceof Error ? err.message : 'Failed to load terminal profile',
          );
          setProfile(null);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const isKdsKiosk = profile?.profileType === 'kds_kiosk';

  return useMemo(
    () => ({ profile, loading, isKdsKiosk, error }),
    [profile, loading, isKdsKiosk, error],
  );
}
