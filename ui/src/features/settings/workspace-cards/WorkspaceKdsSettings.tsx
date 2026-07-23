import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import { useSettings } from '@/contexts/SettingsContext';
import SettingsSelect from '../SettingsSelect';
import type { WorkspaceCardProps } from './types';
import { hasChanges } from './helpers';

// ── Local types ──────────────────────────────────────────────────────

type DisplayDensity = 'comfortable' | 'compact';

interface KdsDraftState {
  soundEnabled: boolean;
  yellowThresholdMin: number;
  redThresholdMin: number;
  autoAcknowledge: boolean;
  density: DisplayDensity;
}

const DEFAULT_KDS: KdsDraftState = {
  soundEnabled: true,
  yellowThresholdMin: 5,
  redThresholdMin: 10,
  autoAcknowledge: false,
  density: 'comfortable',
};

// ── Component ────────────────────────────────────────────────────────

/**
 * Workspace card for Kitchen Display System settings: SLA escalation
 * thresholds, sound toggle, auto-acknowledge, and ticket display density.
 *
 * Consumes `useSettings()` for shared KDS configuration.
 */
export function WorkspaceKdsSettings({
  variant = 'full-page',
  onSaved,
}: WorkspaceCardProps) {
  const { settings } = useSettings();

  // ── Draft state ──────────────────────────────────────────────

  const [draft, setDraft] = useState<KdsDraftState>(DEFAULT_KDS);
  const [saving, setSaving] = useState(false);

  // Originals for dirty tracking — captured after initial load
  const originalsRef = useRef<KdsDraftState>({ ...draft });
  const [originalsLoaded, setOriginalsLoaded] = useState(false);
  const dirty = useMemo(() => hasChanges(
    draft as unknown as Record<string, unknown>,
    originalsRef.current as unknown as Record<string, unknown>,
  ), [draft, originalsLoaded]);

  // ── Initialise from settings ─────────────────────────────────

  useEffect(() => {
    // Load KDS preferences from settings context (Phase 3 will
    // provide dedicated KDS API; for now use user preferences)
    setDraft((prev) => {
      const updated = {
        ...prev,
        soundEnabled: settings.preferences.fontSmoothing === 'antialiased',
      };
      if (!originalsLoaded) {
        originalsRef.current = updated;
        setOriginalsLoaded(true);
      }
      return updated;
    });
  }, [settings.preferences, originalsLoaded]);

  // ── Update helpers ───────────────────────────────────────────

  const update = useCallback(<K extends keyof KdsDraftState>(key: K, value: KdsDraftState[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  }, []);

  // ── Save ─────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      // TODO (Phase 3): Call dedicated KDS settings IPC
      onSaved?.();
    } catch {
      // Hook handles error
    } finally {
      setSaving(false);
    }
  }, [onSaved]);

  const isCompact = variant === 'inspector-drawer';

  return (
    <ErrorBoundary>
      {/* SLA thresholds */}
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-kds-sla-heading">SLA Escalation</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          {/* Sound toggle */}
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="kds-sound" className="settings-label">
            <Localized id="workspace-kds-sound">New Order Sound</Localized>
          </label>
            <label className="settings-toggle" htmlFor="kds-sound">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="kds-sound"
                  type="checkbox"
                  role="switch"
                  checked={draft.soundEnabled}
                  aria-checked={draft.soundEnabled}
                  onChange={(e) => update('soundEnabled', e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </div>

          {/* Yellow threshold */}
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="kds-yellow" className="settings-label">
              <Localized id="workspace-kds-yellow-threshold">Yellow Alert (min)</Localized>
            </label>
            <input
              id="kds-yellow"
              type="range"
              className="settings-range"
              min={3}
              max={10}
              step={1}
              value={draft.yellowThresholdMin}
              onChange={(e) => update('yellowThresholdMin', Number(e.target.value))}
              aria-label="Yellow escalation threshold in minutes"
            />
            {!isCompact && (
              <span className="settings-range-value">{draft.yellowThresholdMin} min</span>
            )}
          </div>

          {/* Red threshold */}
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="kds-red" className="settings-label">
              <Localized id="workspace-kds-red-threshold">Red Alert (min)</Localized>
            </label>
            <input
              id="kds-red"
              type="range"
              className="settings-range"
              min={Math.max(draft.yellowThresholdMin + 1, 6)}
              max={15}
              step={1}
              value={draft.redThresholdMin}
              onChange={(e) => update('redThresholdMin', Number(e.target.value))}
              aria-label="Red escalation threshold in minutes"
            />
            {!isCompact && (
              <span className="settings-range-value">{draft.redThresholdMin} min</span>
            )}
          </div>
        </div>
      </Card>

      {/* Ticket display */}
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-kds-display-heading">Ticket Display</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          {/* Auto-acknowledge */}
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="kds-auto-ack" className="settings-label">
            <Localized id="workspace-kds-auto-ack">Auto-Acknowledge</Localized>
          </label>
            <label className="settings-toggle" htmlFor="kds-auto-ack">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="kds-auto-ack"
                  type="checkbox"
                  role="switch"
                  checked={draft.autoAcknowledge}
                  aria-checked={draft.autoAcknowledge}
                  onChange={(e) => update('autoAcknowledge', e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </div>

          {/* Density */}
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="kds-density" className="settings-label">
              <Localized id="workspace-kds-density">Density</Localized>
            </label>
            <SettingsSelect
              id="kds-density"
              value={draft.density}
              onChange={(v) => update('density', v as DisplayDensity)}
              options={[
                { value: 'comfortable', label: 'Comfortable' },
                { value: 'compact', label: 'Compact' },
              ]}
            />
          </div>
        </div>
      </Card>

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
