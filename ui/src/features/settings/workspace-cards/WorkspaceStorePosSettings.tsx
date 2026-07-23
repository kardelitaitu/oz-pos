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

/**
 * Workspace card for Store/POS settings: receipt layout, printer config,
 * barcode scanner, weight scale, and workspace presets.
 *
 * Consumes `useSettings()` for shared store configuration and
 * `useTerminalHardware(terminalId)` for register-local hardware bindings.
 */
export function WorkspaceStorePosSettings({
  terminalId,
  variant = 'full-page',
  onSaved,
}: WorkspaceCardProps) {
  const { settings } = useSettings();
  const hw = useTerminalHardware(terminalId ?? '', settings.store.currency);

  // ── Draft state ──────────────────────────────────────────────

  const [paperWidth, setPaperWidth] = useState('standard');
  const [showCurrency, setShowCurrency] = useState(false);
  const [showTax, setShowTax] = useState(true);
  const [showTableNumber, setShowTableNumber] = useState(false);
  const [footer, setFooter] = useState('');
  const [saving, setSaving] = useState(false);

  // Original values for dirty tracking — captured after initial load
  const originalsRef = useRef<Record<string, unknown>>({});
  const [originalsLoaded, setOriginalsLoaded] = useState(false);

  const dirty = useMemo(() => hasChanges(
    { paperWidth, showCurrency, showTax, showTableNumber, footer } as Record<string, unknown>,
    originalsRef.current,
  ), [paperWidth, showCurrency, showTax, showTableNumber, footer, originalsLoaded]);

  // ── Initialise from context ──────────────────────────────────

  useEffect(() => {
    setPaperWidth(settings.receipt.paperWidth);
    setShowCurrency(settings.receipt.showCurrency);
    setShowTax(settings.receipt.showTax);
    setShowTableNumber(settings.receipt.showTableNumber);
    setFooter(settings.receipt.footer);
    if (!originalsLoaded) {
      originalsRef.current = {
        paperWidth: settings.receipt.paperWidth,
        showCurrency: settings.receipt.showCurrency,
        showTax: settings.receipt.showTax,
        showTableNumber: settings.receipt.showTableNumber,
        footer: settings.receipt.footer,
      };
      setOriginalsLoaded(true);
    }
  }, [settings.receipt, originalsLoaded]);

  // ── Save ─────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      // Save terminal hardware if available
      if (terminalId && hw.profile) {
        await hw.save();
      }
      // TODO (Phase 2): Call IPC to save store-level receipt settings
      onSaved?.();
    } catch {
      // Error handled by hook's error state
    } finally {
      setSaving(false);
    }
  }, [terminalId, hw, onSaved]);

  // ── Variant classes ──────────────────────────────────────────

  const isCompact = variant === 'inspector-drawer';

  // ── Receipt section ──────────────────────────────────────────

  const receiptSection = (
    <Card
      shadow="sm"
      header={
        <h2 className="settings-section-title">
          <Localized id="workspace-pos-receipt-heading">Receipt Settings</Localized>
        </h2>
      }
    >
      <div className="settings-form">
        {/* Paper width */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-paper-width" className="settings-label">
            <Localized id="workspace-pos-paper-width">Paper Width</Localized>
          </label>
          <SettingsSelect
            id="pos-paper-width"
            value={paperWidth}
            onChange={setPaperWidth}
            options={[
              { value: 'standard', label: '80 mm' },
              { value: 'narrow', label: '58 mm' },
            ]}
          />
        </div>

        {/* Show currency */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-show-currency" className="settings-label">
            <Localized id="workspace-pos-show-currency">Show Currency</Localized>
          </label>
          <label className="settings-toggle" htmlFor="pos-show-currency">
            <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
            <span className="settings-toggle-switch">
              <input
                id="pos-show-currency"
                type="checkbox"
                role="switch"
                checked={showCurrency}
                aria-checked={showCurrency}
                onChange={(e) => setShowCurrency(e.target.checked)}
              />
              <span className="settings-toggle-slider" />
            </span>
          </label>
        </div>

        {/* Show tax */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-show-tax" className="settings-label">
            <Localized id="workspace-pos-show-tax">Show Tax</Localized>
          </label>
          <label className="settings-toggle" htmlFor="pos-show-tax">
            <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
            <span className="settings-toggle-switch">
              <input
                id="pos-show-tax"
                type="checkbox"
                role="switch"
                checked={showTax}
                aria-checked={showTax}
                onChange={(e) => setShowTax(e.target.checked)}
              />
              <span className="settings-toggle-slider" />
            </span>
          </label>
        </div>

        {/* Show table number */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-show-table" className="settings-label">
            <Localized id="workspace-pos-show-table">Show Table Number</Localized>
          </label>
          <label className="settings-toggle" htmlFor="pos-show-table">
            <span className="sr-only"><Localized id="toggle">Toggle</Localized></span>
            <span className="settings-toggle-switch">
              <input
                id="pos-show-table"
                type="checkbox"
                role="switch"
                checked={showTableNumber}
                aria-checked={showTableNumber}
                onChange={(e) => setShowTableNumber(e.target.checked)}
              />
              <span className="settings-toggle-slider" />
            </span>
          </label>
        </div>

        {!isCompact && (
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="pos-footer" className="settings-label">
              <Localized id="workspace-pos-footer">Receipt Footer</Localized>
            </label>
            <textarea
              id="pos-footer"
              className="settings-input"
              value={footer}
              onChange={(e) => setFooter(e.target.value)}
              rows={2}
              maxLength={200}
            />
          </div>
        )}
      </div>
    </Card>
  );

  // ── Printer section ──────────────────────────────────────────

  const printerSection = terminalId ? (
    <Card
      shadow="sm"
      header={
        <h2 className="settings-section-title">
          <Localized id="workspace-pos-printer-heading">Printer</Localized>
        </h2>
      }
    >
      <div className="settings-form">
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-printer-conn" className="settings-label">
            <Localized id="workspace-pos-printer-connection">Connection</Localized>
          </label>
          <SettingsSelect
            id="pos-printer-conn"
            value={hw.profile?.hardware.printer.connection ?? 'auto'}
            onChange={(v) => hw.updatePrinter({ connection: v as 'network' | 'usb' | 'serial' | 'auto' })}
            options={[
              { value: 'auto', label: 'Auto' },
              { value: 'network', label: 'Network' },
              { value: 'usb', label: 'USB' },
              { value: 'serial', label: 'Serial' },
            ]}
          />
        </div>
        {hw.profile?.hardware.printer.connection === 'network' && (
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="pos-printer-ip" className="settings-label">
              <Localized id="workspace-pos-printer-ip">IP Address</Localized>
            </label>
            <input
              id="pos-printer-ip"
              type="text"
              className="settings-input"
              value={hw.profile.hardware.printer.devicePath}
              onChange={(e) => hw.updatePrinter({ devicePath: e.target.value })}
            />
          </div>
        )}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-printer-paper" className="settings-label">
            <Localized id="workspace-pos-printer-paper-size">Paper Size</Localized>
          </label>
          <SettingsSelect
            id="pos-printer-paper"
            value={hw.profile?.hardware.printer.paperSize ?? '80'}
            onChange={(v) => hw.updatePrinter({ paperSize: v as '58' | '80' | 'a4' | 'letter' })}
            options={[
              { value: '80', label: '80 mm' },
              { value: '58', label: '58 mm' },
              { value: 'a4', label: 'A4' },
              { value: 'letter', label: 'Letter' },
            ]}
          />
        </div>
      </div>
    </Card>
  ) : null;

  // ── Scanner section ──────────────────────────────────────────

  const scannerSection = terminalId ? (
    <Card
      shadow="sm"
      header={
        <h2 className="settings-section-title">
          <Localized id="workspace-pos-scanner-heading">Barcode Scanner</Localized>
        </h2>
      }
    >
      <div className="settings-form">
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="pos-scanner-mode" className="settings-label">
            <Localized id="workspace-pos-scanner-mode">Input Mode</Localized>
          </label>
          <SettingsSelect
            id="pos-scanner-mode"
            value={hw.profile?.hardware.scanner.mode ?? 'auto'}
            onChange={(v) => hw.updateScanner({ mode: v as 'keyboard' | 'serial' | 'auto' })}
            options={[
              { value: 'auto', label: 'Auto' },
              { value: 'keyboard', label: 'Keyboard Wedge' },
              { value: 'serial', label: 'Serial' },
            ]}
          />
        </div>
        {!isCompact && (
          <div className="settings-field settings-field--horizontal">
            <label htmlFor="pos-scanner-device" className="settings-label">
              <Localized id="workspace-pos-scanner-device">Device ID</Localized>
            </label>
            <input
              id="pos-scanner-device"
              type="text"
              className="settings-input"
              value={hw.profile?.hardware.scanner.deviceId ?? ''}
              onChange={(e) => hw.updateScanner({ deviceId: e.target.value })}
            />
          </div>
        )}
      </div>
    </Card>
  ) : null;

  // ── Save button ──────────────────────────────────────────────

  const saveButton = variant !== 'inspector-drawer' ? (
    <div className="settings-actions">
      <Button variant="primary" onClick={handleSave} disabled={!dirty || saving}>
        <Localized id="save">Save</Localized>
      </Button>
    </div>
  ) : null;

  return (
    <ErrorBoundary>
      {receiptSection}
      {printerSection}
      {scannerSection}
      {hw.error && (
        <div className="settings-error-banner" role="alert">
          {hw.error}
        </div>
      )}
      {saveButton}
    </ErrorBoundary>
  );
}
