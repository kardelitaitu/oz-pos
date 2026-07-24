import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import { useTerminalHardware } from '@/hooks/useTerminalHardware';
import type { WorkspaceCardProps } from './types';
import { hasChanges } from './helpers';

// ── Component ────────────────────────────────────────────────────────

/**
 * Terminal-local preferences card: sound volume, dark mode toggle,
 * and scale auto-zero behaviour.
 *
 * Consumes `useTerminalHardware(terminalId)` for register-local
 * preferences stored in `terminal_profile.json`.
 */
export function TerminalPreferencesCard({
  terminalId,
  variant = 'full-page',
  onSaved,
}: WorkspaceCardProps) {
  const hw = useTerminalHardware(terminalId ?? '');

  // ── Draft state derived from hardware profile ────────────────

  const [soundVolume, setSoundVolume] = useState(80);
  const [darkMode, setDarkMode] = useState(false);
  const [scaleAutoZero, setScaleAutoZero] = useState(true);
  const [saving, setSaving] = useState(false);

  const originalsRef = useRef<Record<string, unknown>>({
    soundVolume, darkMode, scaleAutoZero,
  });

  const dirty = useMemo(() => hasChanges(
    { soundVolume, darkMode, scaleAutoZero } as Record<string, unknown>,
    originalsRef.current,
  ), [soundVolume, darkMode, scaleAutoZero]);

  // ── Sync state with hardware profile on load ─────────────────

  useEffect(() => {
    if (hw.profile) {
      const lp = hw.profile.localPrefs;
      setSoundVolume(lp.soundVolume);
      setDarkMode(lp.darkMode);
      setScaleAutoZero(lp.scaleAutoZero);
      originalsRef.current = { soundVolume: lp.soundVolume, darkMode: lp.darkMode, scaleAutoZero: lp.scaleAutoZero };
    }
  }, [hw.profile]);

  // Update helpers call both local state and hw.updateLocalPrefs.

  const updateSoundVolume = useCallback((v: number) => {
    setSoundVolume(v);
    hw.updateLocalPrefs({ soundVolume: v });
  }, [hw]);

  const updateDarkMode = useCallback((v: boolean) => {
    setDarkMode(v);
    hw.updateLocalPrefs({ darkMode: v });
  }, [hw]);

  const updateScaleAutoZero = useCallback((v: boolean) => {
    setScaleAutoZero(v);
    hw.updateLocalPrefs({ scaleAutoZero: v });
  }, [hw]);

  // ── Save ─────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      if (terminalId && hw.profile) {
        await hw.save();
      }
      onSaved?.();
    } catch {
      // Hook handles error state
    } finally {
      setSaving(false);
    }
  }, [terminalId, hw, onSaved]);

  const isCompact = variant === 'inspector-drawer';

  return (
    <ErrorBoundary>
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-terminal-prefs-heading">Terminal Preferences</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          {/* Sound volume */}
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="term-sound" className="settings-label">
              <Localized id="workspace-terminal-sound">Sound Volume</Localized>
            </label>
            <input
              id="term-sound"
              type="range"
              className="settings-range"
              min={0}
              max={100}
              step={5}
              value={soundVolume}
              onChange={(e) => updateSoundVolume(Number(e.target.value))}
              aria-label="Sound volume"
            />
            {!isCompact && (
              <span className="settings-range-value">{soundVolume}%</span>
            )}
          </div>

          {/* Dark mode */}
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="term-dark-mode" className="settings-label">
            <Localized id="workspace-terminal-dark-mode">Dark Mode</Localized>
          </label>
            <span className="settings-toggle">
              <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
              <span className="settings-toggle-switch">
                <input
                  id="term-dark-mode"
                  type="checkbox"
                  role="switch"
                  checked={darkMode}
                  aria-checked={darkMode}
                  onChange={(e) => updateDarkMode(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </span>
          </div>

          {/* Scale auto-zero */}
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="term-scale-zero" className="settings-label">
            <Localized id="workspace-terminal-scale-zero">Auto-Zero Scale on Boot</Localized>
          </label>
            <span className="settings-toggle">
              <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
              <span className="settings-toggle-switch">
                <input
                  id="term-scale-zero"
                  type="checkbox"
                  role="switch"
                  checked={scaleAutoZero}
                  aria-checked={scaleAutoZero}
                  onChange={(e) => updateScaleAutoZero(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </span>
          </div>
        </div>
      </Card>

      {hw.error && (
        <div className="settings-error-banner" role="alert">
          {hw.error}
        </div>
      )}

      {/* Save button */}
      {variant !== 'inspector-drawer' && (
        <div className="settings-actions">
          <Button variant="primary" onClick={handleSave} disabled={!dirty || saving}>
            <Localized id="save">Save</Localized>
          </Button>
        </div>
      )}
    </ErrorBoundary>
  );
}
