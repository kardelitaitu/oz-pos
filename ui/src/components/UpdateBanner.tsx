import { useState, useEffect, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useExitAnimation } from '@/hooks/useExitAnimation';
import './UpdateBanner.css';
import type * as UpdaterModule from '@tauri-apps/plugin-updater';

// ── Types ──────────────────────────────────────────────────────────

interface UpdateInfo {
  /** Whether an update is available. */
  available: boolean;
  /** The new version string (e.g. "0.1.0"). */
  version?: string;
  /** Release notes / description. */
  notes?: string | undefined;
  /** Whether the update is mandatory. */
  mandatory?: boolean;
}

// ── Hook to check for updates ──────────────────────────────────────

interface UpdateState {
  info: UpdateInfo;
  instance: Awaited<ReturnType<typeof UpdaterModule.check>> | null;
}

function useUpdateCheck(): UpdateState {
  const [state, setState] = useState<UpdateState>({
    info: { available: false },
    instance: null,
  });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const updater = await import('@tauri-apps/plugin-updater');
        const update = await updater.check();
        if (!cancelled && update) {
          setState({
            info: {
              available: true,
              version: update.version,
              notes: update.body ?? undefined,
              mandatory: false,
            },
            instance: update,
          });
        }
      } catch {
        // Tauri updater plugin not available (dev mode or browser).
        // Silently return no update available.
      }
    })();
    return () => { cancelled = true; };
  }, []);

  return state;
}

// ── Component ──────────────────────────────────────────────────────

/**
 * A non-intrusive update notification banner.
 *
 * Automatically checks for updates via the Tauri updater plugin on
 * mount. When an update is available, shows a dismissible banner at
 * the top of the content area with an "Install" action.
 */
export default function UpdateBanner() {
  const { l10n } = useLocalization();
  const { info: update, instance: updateInstance } = useUpdateCheck();
  const [dismissed, setDismissed] = useState(false);
  const [installing, setInstalling] = useState(false);

  const handleInstall = useCallback(async () => {
    if (!updateInstance) return;
    setInstalling(true);
    try {
      await updateInstance.downloadAndInstall();
    } catch {
      // Installation failed silently — banner stays visible.
      setInstalling(false);
    }
  }, [updateInstance]);

  // Mirror the entry keyframe with a 200ms exit fade so the × dismiss
  // doesn't snap. Install path bypasses the hook entirely: Tauri
  // either restarts (banner would unmount via the parent route
  // unmount) or fails (banner stays visible, no dismiss needed).
  // Per project exit-animation-pattern skill (commit 2d8bab9 sibling).
  const exit = useExitAnimation(
    update.available && !dismissed,
    () => setDismissed(true),
  );

  if (!exit.shouldRender) {
    return null;
  }

  return (
    <div
      className={`update-banner${exit.exiting ? ' update-banner--exiting' : ''}`}
      role="alert"
      aria-live="polite"
    >
      <div className="update-banner-content">
        <svg
          className="update-banner-icon"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
          <polyline points="17 6 23 6 23 12" />
        </svg>
        <span className="update-banner-text">
          <Localized id="update-banner-title"><strong>Update available:</strong></Localized>{' '}
          {update.version ? `v${update.version}` : l10n.getString('update-banner-new-version')}
          {update.notes && <span className="update-banner-notes"> — {update.notes}</span>}
        </span>
      </div>
      <div className="update-banner-actions">
        <button
          type="button"
          className="update-banner-btn update-banner-btn--primary"
          onClick={handleInstall}
          disabled={installing}
          aria-label={l10n.getString(installing ? 'update-banner-installing-aria' : 'update-banner-install-aria')}
        >
          {l10n.getString(installing ? 'update-banner-installing' : 'update-banner-install')}
        </button>
        <button
          type="button"
          className="update-banner-btn update-banner-btn--dismiss"
          onClick={() => exit.requestClose()}
          disabled={exit.exiting}
          aria-label={l10n.getString('update-banner-dismiss-aria')}
        >
          <svg
            width="14"
            height="14"
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
    </div>
  );
}
