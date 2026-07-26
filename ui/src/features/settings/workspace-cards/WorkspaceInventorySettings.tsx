import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import { getSetting, setSetting } from '@/api/settings';
import type { WorkspaceCardProps } from './types';
import { hasChanges } from './helpers';

// ── Component ────────────────────────────────────────────────────────

/**
 * Workspace card for Inventory settings: low stock threshold and
 * deduction location priority rules.
 *
 * Consumes `useSettings()` for store-level inventory configuration.
 */
export function WorkspaceInventorySettings({
  userId,
  locationId,
  variant = 'full-page',
  onSaved,
}: WorkspaceCardProps) {
  // ── Draft state ──────────────────────────────────────────────

  const [lowStockThreshold, setLowStockThreshold] = useState(10);
  const [deductionPreferWarehouse, setDeductionPreferWarehouse] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loaded, setLoaded] = useState(false);

  const originalsRef = useRef<Record<string, unknown>>({
    lowStockThreshold, deductionPreferWarehouse,
  });

  const dirty = useMemo(() => hasChanges(
    { lowStockThreshold, deductionPreferWarehouse } as Record<string, unknown>,
    originalsRef.current,
  ), [lowStockThreshold, deductionPreferWarehouse, loaded]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Load from backend ───────────────────────────────────────

  useEffect(() => {
    if (loaded) return;

    Promise.all([
      getSetting('inventory.low_stock_threshold'),
      getSetting('inventory.deduction_prefer_warehouse'),
    ]).then(([thresholdRaw, preferWhRaw]) => {
      const t = parseInt(thresholdRaw ?? '', 10);
      if (!isNaN(t) && t >= 0) setLowStockThreshold(t);
      setDeductionPreferWarehouse(preferWhRaw === 'true');
      originalsRef.current = {
        lowStockThreshold: !isNaN(t) && t >= 0 ? t : 10,
        deductionPreferWarehouse: preferWhRaw === 'true',
      };
    }).catch(() => {
      originalsRef.current = { lowStockThreshold: 10, deductionPreferWarehouse: false };
    }).finally(() => {
      setLoaded(true);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded]);

  // ── Save ─────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const entries: Record<string, string> = {
        'inventory.low_stock_threshold': String(lowStockThreshold),
        'inventory.deduction_prefer_warehouse': String(deductionPreferWarehouse),
      };
      await Promise.all(
        Object.entries(entries).map(([key, value]) =>
          setSetting(key, value, userId ?? 'default'),
        ),
      );
      originalsRef.current = { lowStockThreshold, deductionPreferWarehouse };
      onSaved?.();
    } catch {
      // Hook handles error
    } finally {
      setSaving(false);
    }
  }, [userId, lowStockThreshold, deductionPreferWarehouse, onSaved]);

  const isCompact = variant === 'inspector-drawer';

  return (
    <ErrorBoundary>
      {/* Low stock threshold */}
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-inv-threshold-heading">Stock Thresholds</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="inv-low-stock" className="settings-label">
              <Localized id="workspace-inv-low-stock">Low Stock Alert At</Localized>
            </label>
            <input
              id="inv-low-stock"
              type="number"
              className="settings-input"
              min={0}
              max={999}
              value={lowStockThreshold}
              onChange={(e) => setLowStockThreshold(Math.max(0, parseInt(e.target.value, 10) || 0))}
            />
            {!isCompact && (
              <span className="settings-range-value">
                <Localized id="workspace-inv-units" vars={{ count: lowStockThreshold }}>
                  items
                </Localized>
              </span>
            )}
          </div>
          {!isCompact && (
            <p className="settings-hint">
              <Localized id="workspace-inv-threshold-hint">
                <span>Alert when stock falls below this quantity</span>
              </Localized>
            </p>
          )}
        </div>
      </Card>

      {/* Deduction rules */}
      {locationId && (
        <Card
          shadow="sm"
          header={
            <h2 className="settings-section-title">
              <Localized id="workspace-inv-deduction-heading">Deduction Rules</Localized>
            </h2>
          }
        >
          <div className="settings-form">
            <div className="settings-field settings-field--horizontal">
          <label htmlFor="inv-deduction-wh" className="settings-label">
            <Localized id="workspace-inv-deduction-warehouse">Prefer Warehouse First</Localized>
          </label>
              <span className="settings-toggle">
                <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
                <span className="settings-toggle-switch">
                  <input
                    id="inv-deduction-wh"
                    type="checkbox"
                    role="switch"
                    checked={deductionPreferWarehouse}
                    aria-checked={deductionPreferWarehouse}
                    onChange={(e) => setDeductionPreferWarehouse(e.target.checked)}
                  />
                  <span className="settings-toggle-slider" />
                </span>
              </span>
            </div>
            {!isCompact && (
              <p className="settings-hint">
                <Localized id="workspace-inv-deduction-hint">
                  <span>When enabled, stock is deducted from warehouse before store shelves</span>
                </Localized>
              </p>
            )}
          </div>
        </Card>
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
