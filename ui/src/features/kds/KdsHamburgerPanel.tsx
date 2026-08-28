import { useState, useRef, useEffect, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useOptionalTheme } from '@/frontend/shell/ThemeProvider';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import type { DisplayDensity, KdsSettings } from '@/features/kds/KdsSettingsPanel';
import { useKdsCardColors } from '@/features/kds/KdsCardColorsContext';

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
  /** Current page zoom percentage (100 = default). */
  pageZoom?: number;
  /** Called when zoom changes (percentage). */
  onChangePageZoom?: (zoom: number) => void;
  /** Current column count override (0 = auto). */
  columns?: number;
  /** Called when column count changes (0 = auto). */
  onChangeColumns?: (cols: number) => void;
  /** Whether card animations are enabled. */
  cardAnimations?: boolean;
  /** Called when card animations toggle changes. */
  onChangeCardAnimations?: (enabled: boolean) => void;
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
  pageZoom = 100,
  onChangePageZoom,
  columns = 0,
  onChangeColumns,
  cardAnimations = true,
  onChangeCardAnimations,
}: KdsHamburgerPanelProps) {
  const { l10n } = useLocalization();
  const themeCtx = useOptionalTheme();
  const [open, setOpen] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  // Card colours from shared context.
  const { colors: cardColors, updateColor, resetColors } = useKdsCardColors();

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

                {onChangePageZoom && (
                  <div className="kds-setting-row">
                    <span className="kds-setting-label"><Localized id="kds-settings-display-scale">Display scale</Localized></span>
                    <div className="kds-zoom-row">
                      <button className="kds-btn kds-btn--muted kds-zoom-btn" onClick={() => onChangePageZoom(Math.max(50, pageZoom - 10))} aria-label="Zoom out" data-testid="kds-settings-zoom-out">−</button>
                      <button className="kds-btn kds-btn--muted kds-zoom-value" onClick={() => onChangePageZoom(100)} title="Reset to 100%" aria-label="Reset zoom to 100%" data-testid="kds-settings-zoom-value">{pageZoom}%</button>
                      <button className="kds-btn kds-btn--muted kds-zoom-btn" onClick={() => onChangePageZoom(Math.min(200, pageZoom + 10))} aria-label="Zoom in" data-testid="kds-settings-zoom-in">+</button>
                    </div>
                  </div>
                )}
                {onChangeColumns && (
                  <div className="kds-setting-row">
                    <span className="kds-setting-label"><Localized id="kds-settings-columns">Columns</Localized></span>
                    <div className="kds-zoom-row">
                      <button className="kds-btn kds-btn--muted kds-zoom-btn" onClick={() => onChangeColumns(Math.max(1, columns - 1))} aria-label="Fewer columns" data-testid="kds-settings-cols-out">−</button>
                      <button className="kds-btn kds-btn--muted kds-zoom-value" onClick={() => onChangeColumns(0)} title="Reset to auto" aria-label="Reset columns to auto" data-testid="kds-settings-cols-value">{columns === 0 ? 'Auto' : columns}</button>
                      <button className="kds-btn kds-btn--muted kds-zoom-btn" onClick={() => onChangeColumns(columns + 1)} aria-label="More columns" data-testid="kds-settings-cols-in">+</button>
                    </div>
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

            {/* ── Card Colours — per-theme pickers (prototype) ── */}
            <div className="kds-panel-section">
              <div className="kds-section-head">
                <h3><Localized id="kds-settings-card-colours">Card Colours</Localized></h3>
                <span className="kds-theme-tag" data-testid="kds-settings-colors-theme-tag">{themeCtx?.theme ?? 'dark'}</span>
              </div>
              <div className="kds-setting-card">
                {([
                  { key: 'dinein' as const, labelId: 'kds-settings-color-dinein' },
                  { key: 'takeaway' as const, labelId: 'kds-settings-color-takeaway' },
                  { key: 'rush' as const, labelId: 'kds-settings-color-rush' },
                  { key: 'processing' as const, labelId: 'kds-settings-color-preparing' },
                  { key: 'prepared' as const, labelId: 'kds-settings-color-ready' },
                  { key: 'complete' as const, labelId: 'kds-settings-color-complete' },
                ]).map(({ key, labelId }) => (
                  <div className="kds-color-group" key={key}>
                    <div className="kds-color-head">
                      <label><Localized id={labelId}>{key}</Localized></label>
                      <input
                        type="color"
                        className="kds-native"
                        value={cardColors[key]}
                        onChange={(e) => updateColor(key, e.target.value)}
                        aria-label={`${labelId} colour picker`}
                        data-testid={`kds-settings-colors-native-${key}`}
                      />
                      <input
                        type="text"
                        className="kds-hex-input"
                        value={cardColors[key]}
                        onChange={(e) => {
                          const v = e.target.value;
                          if (/^#[0-9a-f]{6}$/i.test(v)) updateColor(key, v);
                        }}
                        maxLength={7}
                        data-testid={`kds-settings-colors-hex-${key}`}
                      />
                    </div>
                  </div>
                ))}
              </div>
              <div className="kds-setting-card kds-reset-card">
                <button className="kds-reset-btn" onClick={resetColors} data-testid="kds-settings-colors-reset">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7L3 8" /><path d="M3 3v5h5" /></svg>
                  <Localized id="kds-settings-reset-colours">Reset colours</Localized>
                </button>
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

                {onChangeCardAnimations && (
                  <div className="kds-setting-row">
                    <div className="kds-setting-text">
                      <span className="kds-setting-label"><Localized id="kds-settings-card-animations">Card animations</Localized></span>
                      <span className="kds-setting-caption"><Localized id="kds-settings-card-animations-caption">Spawn and reorder effects</Localized></span>
                    </div>
                    <button
                      className={`kds-switch${cardAnimations ? ' on' : ''}`}
                      role="switch"
                      aria-checked={cardAnimations}
                      onClick={() => onChangeCardAnimations(!cardAnimations)}
                      aria-label={l10n.getString('kds-settings-card-animations') ?? 'Card animations'}
                      data-testid="kds-settings-anim-toggle"
                    />
                  </div>
                )}

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