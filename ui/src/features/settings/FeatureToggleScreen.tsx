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
import { Localized, useLocalization } from '@fluent/react';
import { useToast } from '@/frontend/shared/Toast';
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
  const [searchQuery, setSearchQuery] = useState('');
  const [toggling, setToggling] = useState<string | null>(null);
  const [togglingBatch, setTogglingBatch] = useState<string | null>(null);
  const { addToast } = useToast();

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
        addToast({
          message: l10n.getString('feature-toggle-auto-enabled', { list: result.auto_enabled.join(', ') }),
          type: 'success',
        });
      } else {
        addToast({
          message: l10n.getString(newValue ? 'feature-toggle-enabled' : 'feature-toggle-disabled'),
          type: 'success',
        });
      }
    } catch (err) {
      addToast({
        message: err instanceof Error ? err.message : l10n.getString('feature-toggle-error-toggle'),
        type: 'error',
      });
    }
  }, [l10n, addToast]);

  // ── Active feature set for preview ───────────────────────────

  const activeFeatureSet = useMemo(
    () => new Set(features.filter((f) => f.enabled).map((f) => f.key)),
    [features],
  );

  // ── Search filter ──────────────────────────────────────────────

  const query = searchQuery.toLowerCase().trim();

  const matchesSearch = (f: FeatureInfo) =>
    !query ||
    f.key.toLowerCase().includes(query) ||
    f.name.toLowerCase().includes(query) ||
    f.description.toLowerCase().includes(query);

  // ── Group features ────────────────────────────────────────────

  const grouped = GROUP_ORDER
    .map((group) => ({
      group,
      features: features.filter((f) => f.group === group && matchesSearch(f)),
    }))
    .filter((g) => g.features.length > 0);

  const hasSearchResults = grouped.length > 0 || !query;

  // ── Bulk toggle handlers ───────────────────────────────────────

  const toggleGroup = useCallback(async (group: string, enable: boolean) => {
    const groupFeatures = features.filter((f) => f.group === group);
    setTogglingBatch(group);
    try {
      // Toggle each feature in sequence (individual IPC calls).
      for (const feat of groupFeatures) {
        if (feat.enabled !== enable) {
          // We don't await result.features after every toggle —
          // just update local state optimistically.
          await setFeature(feat.key, enable);
        }
      }
      // Reload full state after batch completes.
      const result = await listAllFeatures();
      setFeatures(result.features);
      addToast({
        message: l10n.getString(
          enable ? 'feature-toggle-bulk-enabled' : 'feature-toggle-bulk-disabled',
          { group },
        ),
        type: 'success',
      });
    } catch (err) {
      addToast({
        message: err instanceof Error ? err.message : l10n.getString('feature-toggle-error-toggle'),
        type: 'error',
      });
    } finally {
      setTogglingBatch(null);
    }
  }, [features, l10n, addToast]);

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

      {/* Search bar */}
      {!loading && !error && (
        <div className="feature-toggle-search">
          <svg className="feature-toggle-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="16" height="16">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <Localized id="feature-toggle-search-placeholder" attrs={{ placeholder: true }}>
            <input
              type="search"
              className="feature-toggle-search-input"
              placeholder="Search features…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              aria-label={l10n.getString('feature-toggle-search-aria')}
            />
          </Localized>
          {searchQuery && (
            <button
              type="button"
              className="feature-toggle-search-clear"
              onClick={() => setSearchQuery('')}
              aria-label={l10n.getString('feature-toggle-search-clear-aria')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>
      )}

      {!loading && !error && !hasSearchResults && (
        <Card shadow="sm">
          <div className="feature-toggle-empty">
            <Localized id="feature-toggle-empty-search">
              <p>No features match your search.</p>
            </Localized>
          </div>
        </Card>
      )}

      {!loading && !error && grouped.length === 0 && hasSearchResults && features.length === 0 && (
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
          <div className="feature-toggle-group-header">
            <Localized id={GROUP_L10N_IDS[group] ?? ''}>
              <h2 className="feature-toggle-group-title">
                <span className="feature-toggle-group-icon" aria-hidden="true">{getGroupIcon(group)}</span>
                {group}
                <span className="feature-toggle-group-count">
                {groupFeatures.filter((f) => f.enabled).length}/{groupFeatures.length}
              </span>
            </h2>
            </Localized>
            <div className="feature-toggle-bulk-actions">
              <button
                type="button"
                className="feature-toggle-bulk-btn"
                disabled={togglingBatch === group}
                onClick={() => toggleGroup(group, true)}
                aria-label={l10n.getString('feature-toggle-bulk-enable-aria', { group })}
              >
                <Localized id="feature-toggle-bulk-enable"><span>Enable All</span></Localized>
              </button>
              <button
                type="button"
                className="feature-toggle-bulk-btn feature-toggle-bulk-btn--disable"
                disabled={togglingBatch === group}
                onClick={() => toggleGroup(group, false)}
                aria-label={l10n.getString('feature-toggle-bulk-disable-aria', { group })}
              >
                <Localized id="feature-toggle-bulk-disable"><span>Disable All</span></Localized>
              </button>
            </div>
          </div>

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
    </div>
  );
}
