//! Feature flag management screen — Settings → Features
//!
//! Displays all 32 feature flags grouped by category with toggle
//! switches. Users can enable/disable features after the initial
//! Setup Wizard. Dependencies are resolved automatically: when
//! enabling a feature, required dependencies are also enabled.
//! When disabling, only the selected feature is turned off.

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Button } from '@/components/Button';
import { Card } from '@/components/Card';
import { Spinner } from '@/components/Spinner';
import { Localized } from '@/frontend/shared/Localized';
import { useLocalization } from '@fluent/react';
import LiveSetupPreview from '@/features/setup/components/LiveSetupPreview';
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

const GROUP_L10N_IDS: Record<string, string> = {
  'Core': 'feature-toggle-group-core',
  'Payments': 'feature-toggle-group-payments',
  'Products': 'feature-toggle-group-products',
  'Staff': 'feature-toggle-group-staff',
  'Hardware': 'feature-toggle-group-hardware',
  'Business Rules': 'feature-toggle-group-business-rules',
  'Restaurant': 'feature-toggle-group-restaurant',
  'Scaling': 'feature-toggle-group-scaling',
  'Reporting': 'feature-toggle-group-reporting',
  'Advanced': 'feature-toggle-group-advanced',
};

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
  const { l10n } = useLocalization();
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
      setError(err instanceof Error ? err.message : l10n.getString('feature-toggle-error-load'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

  useEffect(() => { load(); }, [load]);

  const handleToggle = useCallback(async (key: string, current: boolean) => {
    const newValue = !current;
    setToggling(key);
    try {
      const result = await setFeature(key, newValue);
      setFeatures(result.features);

      if (newValue && result.auto_enabled.length > 0) {
        setToast({
          message: l10n.getString('feature-toggle-auto-enabled', { list: result.auto_enabled.join(', ') }),
          variant: 'success',
        });
      } else {
        setToast({
          message: l10n.getString(newValue ? 'feature-toggle-enabled' : 'feature-toggle-disabled'),
          variant: 'success',
        });
      }
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : l10n.getString('feature-toggle-error-toggle'),
        variant: 'error',
      });
    }
  }, [l10n]);

  // ── Active feature set for preview ───────────────────────────

  const activeFeatureSet = useMemo(
    () => new Set(features.filter((f) => f.enabled).map((f) => f.key)),
    [features],
  );

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
        <Localized id="feature-toggle-title"><h1 className="feature-toggle-title">Feature Toggles</h1></Localized>
        {features.length > 0 && (
          <Localized
            id="feature-toggle-subtitle"
            vars={{ enabled: features.filter((f) => f.enabled).length, total: features.length }}
          >
            <span className="feature-toggle-subtitle">0 / 0 enabled</span>
          </Localized>
        )}
      </div>

      {loading && (
        <div className="feature-toggle-loading">
          <Spinner size="md" />
          <Localized id="feature-toggle-loading"><p>Loading features…</p></Localized>
        </div>
      )}

      {error && (
        <div className="feature-toggle-error" role="alert">
          <p>{error}</p>
          <Button variant="secondary" onClick={load}>
            <Localized id="feature-toggle-retry"><span>Retry</span></Localized>
          </Button>
        </div>
      )}

      {!loading && !error && grouped.length === 0 && (
        <Card shadow="sm">
          <div className="feature-toggle-empty">
            <Localized id="feature-toggle-empty"><p>No features found.</p></Localized>
          </div>
        </Card>
      )}

      {/* Live Preview */}
      {!loading && !error && features.length > 0 && (
        <div className="feature-toggle-preview">
          <LiveSetupPreview selectedFeatures={activeFeatureSet} />
        </div>
      )}

      {!loading && !error && grouped.map(({ group, features: groupFeatures }) => (
        <div key={group} className="feature-toggle-group">
          <Localized id={GROUP_L10N_IDS[group] ?? ''}>
            <h2 className="feature-toggle-group-title">
              <span className="feature-toggle-group-icon">{getGroupIcon(group)}</span>
              {group}
              <span className="feature-toggle-group-count">
              {groupFeatures.filter((f) => f.enabled).length}/{groupFeatures.length}
            </span>
          </h2>
          </Localized>

          <Card shadow="xs">
            <div className="feature-toggle-list" role="group" aria-label={l10n.getString('feature-toggle-group-aria', { group })}>
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
                          {l10n.getString('feature-toggle-requires', { deps: depNames })}
                        </span>
                      )}
                    </div>
                    <label className="feature-toggle-switch" aria-label={l10n.getString('feature-toggle-toggle-aria', { name: feat.name })}>
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
          aria-label={l10n.getString('feature-toggle-dismiss-aria')}
        >
          {toast.message}
        </button>
      )}
    </div>
  );
}
