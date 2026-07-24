import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import { useSettings } from '@/contexts/SettingsContext';
import { useTerminalHardware } from '@/hooks/useTerminalHardware';
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
  ), [tableManagement, courseFiring, originalsLoaded]);

  // ── Initialise ───────────────────────────────────────────────

  useEffect(() => {
    setTableManagement(settings.receipt.showTableNumber);
    if (!originalsLoaded) {
      originalsRef.current = { tableManagement: settings.receipt.showTableNumber, courseFiring };
      setOriginalsLoaded(true);
    }
  }, [settings.receipt, originalsLoaded, courseFiring]);

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
