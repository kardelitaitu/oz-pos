import { useState, useRef, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';
import './KdsSettingsPanel.css';

/** Display density for KDS ticket cards. */
export type DisplayDensity = 'comfortable' | 'compact';

export interface KdsSettings {
  /** Whether new-ticket sound is enabled. */
  soundEnabled: boolean;
  /** Escalation threshold in minutes before a ticket turns yellow. */
  yellowThresholdMin: number;
  /** Escalation threshold in minutes before a ticket turns red. */
  redThresholdMin: number;
  /** Whether to auto-advance tickets after a configurable delay. */
  autoAcknowledge: boolean;
  /** Ticket card display density. */
  density: DisplayDensity;
}

export const DEFAULT_SETTINGS: KdsSettings = {
  soundEnabled: true,
  yellowThresholdMin: 5,
  redThresholdMin: 10,
  autoAcknowledge: false,
  density: 'comfortable',
};

interface KdsSettingsPanelProps {
  settings: KdsSettings;
  onChangeSound: (enabled: boolean) => void;
  onChangeYellowThreshold: (minutes: number) => void;
  onChangeRedThreshold: (minutes: number) => void;
  onChangeAutoAcknowledge: (enabled: boolean) => void;
  onChangeDensity: (density: DisplayDensity) => void;
}

/**
 * KdsSettingsPanel — gear icon button that opens a popover with KDS
 * settings: sound toggle, escalation thresholds, auto-acknowledge,
 * and display density. Follows the same portal + close-on-escape +
 * click-outside pattern as KdsLayoutSwitcher.
 */
export function KdsSettingsPanel({
  settings,
  onChangeSound,
  onChangeYellowThreshold,
  onChangeRedThreshold,
  onChangeAutoAcknowledge,
  onChangeDensity,
}: KdsSettingsPanelProps) {
  const [open, setOpen] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        btnRef.current &&
        !btnRef.current.contains(e.target as Node)
      ) {
        close();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [open, close]);

  return (
    <>
      <button
        ref={btnRef}
        className="kds-settings-btn"
        onClick={() => setOpen((p) => !p)}
        aria-label="KDS settings"
        aria-expanded={open}
      >
        <svg className="kds-settings-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
        </svg>
      </button>
      {open && createPortal(
        <div
          ref={popoverRef}
          className="kds-settings-popover"
          role="dialog"
          aria-label="KDS settings"
        >
          {/* Sound toggle */}
          <label className="kds-settings-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={settings.soundEnabled}
              onChange={(e) => onChangeSound(e.target.checked)}
            />
            <span className="kds-settings-toggle-label">Sound</span>
          </label>

          {/* Yellow threshold slider */}
          <div className="kds-settings-slider-group">
            <span className="kds-settings-slider-label">
              Yellow at {settings.yellowThresholdMin} min
            </span>
            <input
              type="range"
              className="kds-settings-slider"
              min={3}
              max={10}
              step={1}
              value={settings.yellowThresholdMin}
              onChange={(e) => onChangeYellowThreshold(Number(e.target.value))}
              aria-label="Yellow escalation threshold in minutes"
            />
          </div>

          {/* Red threshold slider */}
          <div className="kds-settings-slider-group">
            <span className="kds-settings-slider-label">
              Red at {settings.redThresholdMin} min
            </span>
            <input
              type="range"
              className="kds-settings-slider"
              min={Math.max(settings.yellowThresholdMin + 1, 6)}
              max={15}
              step={1}
              value={settings.redThresholdMin}
              onChange={(e) => onChangeRedThreshold(Number(e.target.value))}
              aria-label="Red escalation threshold in minutes"
            />
          </div>

          {/* Auto-acknowledge toggle */}
          <label className="kds-settings-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={settings.autoAcknowledge}
              onChange={(e) => onChangeAutoAcknowledge(e.target.checked)}
            />
            <span className="kds-settings-toggle-label">Auto-acknowledge</span>
          </label>

          {/* Display density */}
          <div className="kds-settings-density">
            <span className="kds-settings-slider-label">Density</span>
            <div className="kds-settings-density-options">
              {(['comfortable', 'compact'] as const).map((d) => (
                <button
                  key={d}
                  className={`kds-settings-density-btn ${d === settings.density ? 'kds-settings-density-btn--active' : ''}`}
                  onClick={() => onChangeDensity(d)}
                  aria-pressed={d === settings.density}
                >
                  {d}
                </button>
              ))}
            </div>
          </div>
        </div>,
        document.body,
      )}
    </>
  );
}
