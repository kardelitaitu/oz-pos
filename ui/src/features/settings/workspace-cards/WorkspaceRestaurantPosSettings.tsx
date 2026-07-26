import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import { useSettings } from '@/contexts/SettingsContext';
import { useTerminalHardware } from '@/hooks/useTerminalHardware';
import { setReceiptSettings, getSetting, setSetting } from '@/api/settings';
import SettingsSelect from '../SettingsSelect';
import type { WorkspaceCardProps } from './types';
import { hasChanges } from './helpers';

// ── Component ────────────────────────────────────────────────────────

/**
 * Workspace card for Restaurant/POS settings: kitchen printers, table
 * management toggle, and course firing rules.
 *
 * Consumes `useSettings()` for shared config and
 * `useTerminalHardware(terminalId)` for register-local kitchen printer.
 */
export function WorkspaceRestaurantPosSettings({
  terminalId,
  userId,
  variant = 'full-page',
  onSaved,
}: WorkspaceCardProps) {
  const { settings } = useSettings();
  const hw = useTerminalHardware(terminalId ?? '', settings.store.currency);

  // ── Draft state ──────────────────────────────────────────────

  const [tableManagement, setTableManagement] = useState(false);
  const [courseFiring, setCourseFiring] = useState(false);

  const [saving, setSaving] = useState(false);

  // Originals for dirty tracking — captured after initial load
  const originalsRef = useRef<Record<string, unknown>>({ tableManagement, courseFiring });
  const [originalsLoaded, setOriginalsLoaded] = useState(false);

  const dirty = useMemo(() => hasChanges(
    { tableManagement, courseFiring } as Record<string, unknown>,
    originalsRef.current,
  ), [tableManagement, courseFiring, originalsLoaded]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Initialise ───────────────────────────────────────────────

  useEffect(() => {
    // Only seed initial values once; subsequent re-runs must not
    // overwrite user edits (e.g. when settings.receipt changes).
    if (originalsLoaded) return;

    setTableManagement(settings.receipt.showTableNumber);

    // Load courseFiring from the backend settings table.
    let cancelled = false;
    getSetting('restaurant.course_firing').then((raw) => {
      if (cancelled) return;
      const loaded = raw === 'true';
      setCourseFiring(loaded);
      originalsRef.current = { tableManagement: settings.receipt.showTableNumber, courseFiring: loaded };
      setOriginalsLoaded(true);
    }).catch(() => {
      originalsRef.current = { tableManagement: settings.receipt.showTableNumber, courseFiring: false };
      setOriginalsLoaded(true);
    });
    return () => { cancelled = true; };
  }, [settings.receipt, originalsLoaded]);

  // ── Save ─────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const tasks: Promise<unknown>[] = [];

      // Persist table management + course firing to the backend
      tasks.push(
        setReceiptSettings({
          showCurrency: settings.receipt.showCurrency,
          decimalSeparator: settings.receipt.decimalSeparator,
          showTax: settings.receipt.showTax,
          footer: settings.receipt.footer,
          paperWidth: settings.receipt.paperWidth,
          showTableNumber: tableManagement,
          marginTop: settings.receipt.marginTop,
          marginBottom: settings.receipt.marginBottom,
          marginLeft: settings.receipt.marginLeft,
          marginRight: settings.receipt.marginRight,
        }, userId ?? 'default'),
      );
      tasks.push(
        setSetting('restaurant.course_firing', String(courseFiring), userId ?? 'default'),
      );

      if (terminalId && hw.profile) {
        tasks.push(hw.save(userId));
      }

      await Promise.all(tasks);

      originalsRef.current = { tableManagement, courseFiring };

      onSaved?.();
    } catch {
      // Hook handles error state
    } finally {
      setSaving(false);
    }
  }, [terminalId, hw, userId, tableManagement, courseFiring, settings.receipt, onSaved]);

  const isCompact = variant === 'inspector-drawer';

  return (
    <ErrorBoundary>
      {/* Table management */}
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-resto-table-heading">Table Management</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="resto-table-mgmt" className="settings-label">
            <Localized id="workspace-resto-table-enable">Enable Table Layout</Localized>
          </label>
            <span className="settings-toggle">
              <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
              <span className="settings-toggle-switch">
                <input
                  id="resto-table-mgmt"
                  type="checkbox"
                  role="switch"
                  checked={tableManagement}
                  aria-checked={tableManagement}
                  onChange={(e) => setTableManagement(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </span>
          </div>
          {!isCompact && tableManagement && (
            <p className="settings-hint">
              <Localized id="workspace-resto-table-hint">
                <span>Tables appear on the POS screen for dine-in orders</span>
              </Localized>
            </p>
          )}
        </div>
      </Card>

      {/* Course firing */}
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-resto-courses-heading">Course Firing</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          <div className="settings-field settings-field--horizontal">
          <label htmlFor="resto-course-firing" className="settings-label">
            <Localized id="workspace-resto-courses-enable">Enable Course Firing</Localized>
          </label>
            <span className="settings-toggle">
              <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
              <span className="settings-toggle-switch">
                <input
                  id="resto-course-firing"
                  type="checkbox"
                  role="switch"
                  checked={courseFiring}
                  aria-checked={courseFiring}
                  onChange={(e) => setCourseFiring(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </span>
          </div>
          {!isCompact && courseFiring && (
            <p className="settings-hint">
              <Localized id="workspace-resto-courses-hint">
                <span>Send appetizers, mains, and desserts to the kitchen in sequence</span>
              </Localized>
            </p>
          )}
        </div>
      </Card>

      {/* Kitchen printer */}
      {terminalId && (
        <Card
          shadow="sm"
          header={
            <h2 className="settings-section-title">
              <Localized id="workspace-resto-kitchen-printer-heading">Kitchen Printer</Localized>
            </h2>
          }
        >
          <div className="settings-form">
            <div className="settings-field settings-field--horizontal">
              <label htmlFor="resto-kp-conn" className="settings-label">
                <Localized id="workspace-resto-kp-connection">Connection</Localized>
              </label>
              <SettingsSelect
                id="resto-kp-conn"
                value={hw.profile?.hardware.printer.connection ?? 'auto'}
                onChange={(v) => hw.updatePrinter({ connection: v as 'network' | 'usb' | 'serial' | 'auto' })}
                options={[
                  { value: 'auto', label: 'Auto' },
                  { value: 'network', label: 'Network' },
                  { value: 'usb', label: 'USB' },
                ]}
              />
            </div>
            {hw.profile?.hardware.printer.connection === 'network' && (
              <div className="settings-field settings-field--horizontal">
                <label htmlFor="resto-kp-ip" className="settings-label">
                  <Localized id="workspace-resto-kp-ip">Kitchen Printer IP</Localized>
                </label>
                <input
                  id="resto-kp-ip"
                  type="text"
                  className="settings-input"
                  value={hw.profile?.hardware.printer.devicePath ?? ''}
                  onChange={(e) => {
                    hw.updatePrinter({ devicePath: e.target.value });
                  }}
                  placeholder="192.168.1.50"
                />
              </div>
            )}
          </div>
        </Card>
      )}

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
