//! Feature flag management screen — Settings → Features
//!
//! Displays all 32 feature flags grouped by category with toggle
//! switches. Users can enable/disable features after the initial
//! Setup Wizard. Dependencies are resolved automatically: when
//! enabling a feature, required dependencies are also enabled.
//! When disabling, only the selected feature is turned off.

import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/Button';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import './FeatureToggleScreen.css';

// ── Types ──────────────────────────────────────────────────────────

interface FeatureInfo {
  key: string;
  name: string;
  description: string;
  group: string;
  enabled: boolean;
  dependencies: string[];
}

interface ListAllFeaturesResult {
  features: FeatureInfo[];
}

interface SetFeatureResult {
  success: boolean;
  features: FeatureInfo[];
  auto_enabled: string[];
}

// ── Group ordering ─────────────────────────────────────────────────

const GROUP_ORDER: string[] = [
  'Core',
  'Payments',
  'Products',
  'Staff',
  'Hardware',
  'Business Rules',
  'Restaurant',
  'Scaling',
  'Reporting',
  'Advanced',
];

// ── IPC wrappers (inline to avoid circular deps) ───────────────────

async function listAllFeatures(): Promise<ListAllFeaturesResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ListAllFeaturesResult>('list_all_features');
}

async function setFeature(key: string, enabled: boolean): Promise<SetFeatureResult> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SetFeatureResult>('set_feature', { args: { key, enabled } });
}

// ── Helpers ─────────────────────────────────────────────────────────

function getGroupIcon(group: string): string {
  switch (group) {
    case 'Core': return '⚙️';
    case 'Payments': return '💳';
    case 'Products': return '📦';
    case 'Staff': return '👤';
    case 'Hardware': return '🖨️';
    case 'Business Rules': return '📋';
    case 'Restaurant': return '🍽️';
    case 'Scaling': return '📈';
    case 'Reporting': return '📊';
    case 'Advanced': return '🔧';
    default: return '📌';
  }
}

// ── Component ──────────────────────────────────────────────────────

export default function FeatureToggleScreen() {
  const [features, setFeatures] = useState<FeatureInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [toast, setToast] = useState<{ message: string; variant: 'success' | 'error' } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await listAllFeatures();
      setFeatures(result.features);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load features');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleToggle = useCallback(async (key: string, current: boolean) => {
    const newValue = !current;
    setToggling(key);
    try {
      const result = await setFeature(key, newValue);
      setFeatures(result.features);

      if (newValue && result.auto_enabled.length > 0) {
        setToast({
          message: `Auto-enabled dependencies: ${result.auto_enabled.join(', ')}`,
          variant: 'success',
        });
      } else {
        setToast({
          message: newValue ? 'Feature enabled' : 'Feature disabled',
          variant: 'success',
        });
      }
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : 'Failed to toggle feature',
        variant: 'error',
      });
    } finally {
      setToggling(null);
    }
  }, []);

  // ── Group features ────────────────────────────────────────────

  const grouped = GROUP_ORDER
    .map((group) => ({
      group,
      features: features.filter((f) => f.group === group),
    }))
    .filter((g) => g.features.length > 0);

  // ── Render ────────────────────────────────────────────────────

  return (
    <div className="feature-toggle">
      <div className="feature-toggle-header">
        <h1 className="feature-toggle-title">Feature Toggles</h1>
        <span className="feature-toggle-subtitle">
          {features.length > 0
            ? `${features.filter((f) => f.enabled).length} / ${features.length} enabled`
            : ''}
        </span>
      </div>

      {loading && (
        <div className="feature-toggle-loading">
          <Spinner size="md" />
          <p>Loading features…</p>
        </div>
      )}

      {error && (
        <div className="feature-toggle-error" role="alert">
          <p>Error: {error}</p>
          <Button variant="secondary" onClick={load}>Retry</Button>
        </div>
      )}

      {!loading && !error && grouped.length === 0 && (
        <Card shadow="sm">
          <div className="feature-toggle-empty">
            <p>No features found.</p>
          </div>
        </Card>
      )}

      {!loading && !error && grouped.map(({ group, features: groupFeatures }) => (
        <div key={group} className="feature-toggle-group">
          <h2 className="feature-toggle-group-title">
            <span className="feature-toggle-group-icon">{getGroupIcon(group)}</span>
            {group}
            <span className="feature-toggle-group-count">
              {groupFeatures.filter((f) => f.enabled).length}/{groupFeatures.length}
            </span>
          </h2>

          <Card shadow="xs">
            <div className="feature-toggle-list" role="group" aria-label={`${group} features`}>
              {groupFeatures.map((feat) => {
                const depNames = feat.dependencies
                  .map((dk) => features.find((f) => f.key === dk)?.name ?? dk)
                  .join(', ');

                return (
                  <div key={feat.key} className="feature-toggle-item">
                    <div className="feature-toggle-item-info">
                      <span className="feature-toggle-item-name">{feat.name}</span>
                      <span className="feature-toggle-item-desc">{feat.description}</span>
                      {feat.dependencies.length > 0 && (
                        <span className="feature-toggle-item-deps">
                          Requires: {depNames}
                        </span>
                      )}
                    </div>
                    <label className="feature-toggle-switch" aria-label={`Toggle ${feat.name}`}>
                      <input
                        type="checkbox"
                        checked={feat.enabled}
                        disabled={toggling === feat.key}
                        onChange={() => handleToggle(feat.key, feat.enabled)}
                        aria-describedby={`desc-${feat.key}`}
                      />
                      <span className="feature-toggle-slider" />
                    </label>
                  </div>
                );
              })}
            </div>
          </Card>
        </div>
      ))}

      {toast && (
        <button
          type="button"
          className={toast.variant === 'success' ? 'feature-toggle-toast feature-toggle-toast--success' : 'feature-toggle-toast feature-toggle-toast--error'}
          onClick={() => setToast(null)}
          aria-label="Dismiss notification"
        >
          {toast.message}
        </button>
      )}
    </div>
  );
}
