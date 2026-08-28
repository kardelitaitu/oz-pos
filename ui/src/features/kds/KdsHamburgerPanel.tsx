import { useState, useRef, useEffect, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useOptionalTheme } from '@/frontend/shell/ThemeProvider';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import type { DisplayDensity, KdsSettings } from '@/features/kds/KdsSettingsPanel';

interface KdsHamburgerPanelProps {
  settings: KdsSettings;
  onChangeSound: (enabled: boolean) => void;
  onChangeYellowThreshold: (minutes: number) => void;
  onChangeRedThreshold: (minutes: number) => void;
  onChangeAutoAcknowledge: (enabled: boolean) => void;
  onChangeDensity: (density: DisplayDensity) => void;
  showOrderId: boolean;
  showTableNumber: boolean;
  onToggleOrderId: (show: boolean) => void;
  onToggleTableNumber: (show: boolean) => void;
}

/**
 * KdsHamburgerPanel — hamburger icon button that opens the prototype
 * settings panel (``.kds-hamburger-panel``) with two sections:
 *
 * **Display** — theme (light/dark), density, order ID, table number
 * **Behaviour** — sound, auto-acknowledge, yellow/red thresholds
 *
 * Uses the prototype CSS classes added in Phase 1: ``.kds-hamburger-panel``,
 * ``.kds-panel-body``, ``.kds-panel-section``, ``.kds-setting-card``,
 * ``.kds-setting-row``, ``.kds-switch``, ``.kds-theme-toggle``, etc.
 */
export function KdsHamburgerPanel({
  settings,
  onChangeSound,
  onChangeYellowThreshold,
  onChangeRedThreshold,
  onChangeAutoAcknowledge,
  onChangeDensity,
  showOrderId,
  showTableNumber,
  onToggleOrderId,
  onToggleTableNumber,
}: KdsHamburgerPanelProps) {
  const { l10n } = useLocalization();
  const themeCtx = useOptionalTheme();
  const [open, setOpen] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setOpen(false), []);

  useFocusTrap(panelRef, open, close);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node) &&
        btnRef.current &&
        !btnRef.current.contains(e.target as Node)
      ) {
        close();
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open, close]);

  return (
    <>
      <button
        ref={btnRef}
        className="kds-btn kds-btn--icon"
        onClick={() => setOpen((p) => !p)}
        aria-label={l10n.getString('kds-settings-aria') ?? 'Settings'}
        aria-haspopup="true"
        aria-expanded={open}
        data-testid="kds-topbar-settings"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
          <path d="M4 6h16M4 12h16M4 18h16" />
        </svg>
      </button>

      {open && (
        <div
          ref={panelRef}
          className="kds-hamburger-panel"
          role="dialog"
          aria-modal="true"
          aria-label={l10n.getString('kds-settings-aria') ?? 'Settings'}
        >
          <div className="kds-panel-body">
            {/* ── Display ──────────────────────────────────── */}
            <div className="kds-panel-section">
              <h3>Display</h3>
              <div className="kds-setting-card">
                {themeCtx && (
                  <div className="kds-setting-row">
                    <span className="kds-setting-label"><Localized id="kds-settings-theme">Theme</Localized></span>
                    <button
                      className="kds-theme-toggle"
                      onClick={() => themeCtx.setTheme(themeCtx.theme === 'dark' ? 'light' : 'dark')}
                      title={l10n.getString('kds-settings-theme-toggle-aria') ?? 'Toggle light/dark'}
                      aria-label={l10n.getString('kds-settings-theme-toggle-aria') ?? 'Toggle light or dark theme'}
                      data-testid="kds-settings-theme-toggle"
                    >
                      <span className="kds-theme-indicator" style={{ width: themeCtx.theme === 'dark' ? '33px' : '33px', left: themeCtx.theme === 'dark' ? '3px' : '36px' }} />
                      <span className={`kds-theme-option${themeCtx.theme === 'dark' ? ' on' : ''}`} aria-label="Dark theme">
                        <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36a5.39 5.39 0 0 1-4.4 2.26 5.4 5.4 0 0 1-3.14-9.8c-.44-.06-.9-.1-1.36-.1z" /></svg>
                      </span>
                      <span className={`kds-theme-option${themeCtx.theme === 'light' ? ' on' : ''}`} aria-label="Light theme">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" /></svg>
                      </span>
                    </button>
                  </div>
                )}

                <div className="kds-setting-row">
                  <span className="kds-setting-label"><Localized id="kds-settings-density">Density</Localized></span>
                  <div className="kds-zoom-row">
                    {(['comfortable', 'compact'] as const).map((d) => (
                      <button
                        key={d}
                        className={`kds-btn kds-btn--muted kds-zoom-btn${d === settings.density ? ' is-active' : ''}`}
                        onClick={() => onChangeDensity(d)}
                        aria-pressed={d === settings.density}
                      >
                        <Localized id={d === 'comfortable' ? 'kds-settings-density-comfortable' : 'kds-settings-density-compact'}>{d}</Localized>
                      </button>
                    ))}
                  </div>
                </div>

                <div className="kds-setting-row">
                  <div className="kds-setting-text">
                    <span className="kds-setting-label"><Localized id="kds-layout-order-id">Order ID</Localized></span>
                    <span className="kds-setting-caption"><Localized id="kds-layout-order-id-caption">Show order number on cards</Localized></span>
                  </div>
                  <button
                    className={`kds-switch${showOrderId ? ' on' : ''}`}
                    role="switch"
                    aria-checked={showOrderId}
                    onClick={() => onToggleOrderId(!showOrderId)}
                    aria-label={l10n.getString('kds-layout-order-id') ?? 'Order ID'}
                  />
                </div>

                <div className="kds-setting-row">
                  <div className="kds-setting-text">
                    <span className="kds-setting-label"><Localized id="kds-layout-table-number">Table Number</Localized></span>
                    <span className="kds-setting-caption"><Localized id="kds-layout-table-number-caption">Show table number on cards</Localized></span>
                  </div>
                  <button
                    className={`kds-switch${showTableNumber ? ' on' : ''}`}
                    role="switch"
                    aria-checked={showTableNumber}
                    onClick={() => onToggleTableNumber(!showTableNumber)}
                    aria-label={l10n.getString('kds-layout-table-number') ?? 'Table Number'}
                  />
                </div>
              </div>
            </div>

            {/* ── Behaviour ──────────────────────────────── */}
            <div className="kds-panel-section">
              <h3>Behaviour</h3>
              <div className="kds-setting-card">
                <div className="kds-setting-row">
                  <div className="kds-setting-text">
                    <span className="kds-setting-label"><Localized id="kds-settings-sound">Sound</Localized></span>
                    <span className="kds-setting-caption"><Localized id="kds-settings-sound-caption">Chime when an order arrives</Localized></span>
                  </div>
                  <button
                    className={`kds-switch${settings.soundEnabled ? ' on' : ''}`}
                    role="switch"
                    aria-checked={settings.soundEnabled}
                    onClick={() => onChangeSound(!settings.soundEnabled)}
                    aria-label={l10n.getString('kds-settings-sound') ?? 'Sound'}
                  />
                </div>

                <div className="kds-setting-row">
                  <div className="kds-setting-text">
                    <span className="kds-setting-label"><Localized id="kds-settings-auto-ack">Auto-acknowledge</Localized></span>
                    <span className="kds-setting-caption"><Localized id="kds-settings-auto-ack-caption">New orders appear without tapping Accept</Localized></span>
                  </div>
                  <button
                    className={`kds-switch${settings.autoAcknowledge ? ' on' : ''}`}
                    role="switch"
                    aria-checked={settings.autoAcknowledge}
                    onClick={() => onChangeAutoAcknowledge(!settings.autoAcknowledge)}
                    aria-label={l10n.getString('kds-settings-auto-ack') ?? 'Auto-acknowledge'}
                  />
                </div>

                {/* SLA thresholds */}
                <div className="kds-setting-row">
                  <span className="kds-setting-label"><Localized id="kds-settings-yellow" vars={{ min: settings.yellowThresholdMin }}>{`Yellow at ${settings.yellowThresholdMin} min`}</Localized></span>
                  <input
                    type="range"
                    className="kds-settings-slider"
                    min={3}
                    max={10}
                    step={1}
                    value={settings.yellowThresholdMin}
                    onChange={(e) => onChangeYellowThreshold(Number(e.target.value))}
                    aria-label={l10n.getString('kds-settings-yellow-aria') ?? 'Yellow threshold minutes'}
                  />
                </div>

                <div className="kds-setting-row">
                  <span className="kds-setting-label"><Localized id="kds-settings-red" vars={{ min: settings.redThresholdMin }}>{`Red at ${settings.redThresholdMin} min`}</Localized></span>
                  <input
                    type="range"
                    className="kds-settings-slider"
                    min={Math.max(settings.yellowThresholdMin + 1, 6)}
                    max={15}
                    step={1}
                    value={settings.redThresholdMin}
                    onChange={(e) => onChangeRedThreshold(Number(e.target.value))}
                    aria-label={l10n.getString('kds-settings-red-aria') ?? 'Red threshold minutes'}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}