import { useEffect, useState, useCallback, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  getBrandSettings,
  setBrandPrimaryColour,
  setBrandLogoPath,
  setBrandStoreName,
  pickLogoFile,
} from '@/api/branding';
import { useBrand } from '@/contexts/BrandContext';
import { deriveAccentPalette, applyAccentPalette } from '@/utils/color';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { useAppZoom } from '@/contexts/ZoomContext';
import type { ZoomLevel } from '@/contexts/ZoomContext';
import { useToast } from '@/frontend/shared/Toast';
import './AppearanceSettings.css';

// ── Helpers ──────────────────────────────────────────────────────────

const DEFAULT_COLOUR = '#10b981';

/**
 * Normalise a hex colour string to `#rrggbb` lowercase format.
 * Accepts shorthand `#fff`, with or without `#`, and strips invalid characters.
 * Returns `null` if the input is completely unparseable.
 */
function normaliseHex(raw: string): string | null {
  let hex = raw.replace(/[^0-9a-fA-F]/g, '');
  if (hex.length === 0) return null;
  if (hex.length <= 3) {
    // Expand shorthand: 'fff' → 'ffffff'
    hex = hex.split('').map((c) => c + c).join('');
  }
  if (hex.length > 6) hex = hex.slice(0, 6);
  if (hex.length < 6) hex = hex.padEnd(6, '0');
  return `#${hex.toLowerCase()}`;
}

interface AppearanceSettingsProps {
  embedded?: boolean;
  colour?: string;
  storeName?: string;
  onColourChange?: (c: string) => void;
  onStoreNameChange?: (n: string) => void;
}

/** Brand appearance panel — primary colour picker, logo upload, store name, interface zoom, and a live preview of the resulting palette. */
export function AppearanceSettings({
  embedded = false,
  colour: colourProp,
  storeName: storeNameProp,
  onColourChange,
  onStoreNameChange,
}: AppearanceSettingsProps) {
  const { refreshBrandSettings } = useBrand();
  const [colour, setColour] = useState('#10b981');
  const [logoPath, setLogoPath] = useState<string | null>(null);
  const [storeName, setStoreName] = useState('');
  const [saving, setSaving] = useState(false);
  const [resetting, setResetting] = useState(false);
  const { zoomLevel, setZoomLevel } = useAppZoom();
  const { addToast } = useToast();

  useEffect(() => {
    if (embedded) return;
    getBrandSettings().then((s) => {
      setColour(s.primary_colour);
      setLogoPath(s.logo_path);
      setStoreName(s.store_name);
    });
  }, [embedded]);

  const activeColour = embedded ? (colourProp ?? colour) : colour;
  const activeStoreName = embedded ? (storeNameProp ?? storeName) : storeName;

  // Contrast text is absolute — light accent needs dark text, dark accent needs
  // light text, regardless of theme. Centralised as CSS variables instead of
  // duplicated inline styles.
  const isLightBg = parseInt(activeColour.slice(1), 16) > 0x7fffff;
  const previewBtnText = isLightBg ? '#0a0a0a' : '#ffffff';

  const updateColour = useCallback((c: string) => {
    if (embedded) {
      onColourChange?.(c);
    } else {
      setColour(c);
    }
    const palette = deriveAccentPalette(c);
    applyAccentPalette(palette);
  }, [embedded, onColourChange]);

  // ── Localized helper for reset button tooltip ─────────────
  const { l10n } = useLocalization();

  const updateStoreName = useCallback((n: string) => {
    if (embedded) {
      onStoreNameChange?.(n);
    } else {
      setStoreName(n);
    }
  }, [embedded, onStoreNameChange]);

  const handlePickLogo = useCallback(async () => {
    const path = await pickLogoFile();
    if (path) {
      setLogoPath(path);
      await setBrandLogoPath(path);
      refreshBrandSettings();
    }
  }, [refreshBrandSettings]);

  const colourRef = useRef(activeColour);
  colourRef.current = activeColour;
  const nameRef = useRef(activeStoreName);
  nameRef.current = activeStoreName;

  const save = useCallback(async () => {
    setSaving(true);
    await setBrandPrimaryColour(colourRef.current);
    await setBrandStoreName(nameRef.current);
    refreshBrandSettings();
    setSaving(false);
  }, [refreshBrandSettings]);

  const handleResetAll = useCallback(async () => {
    if (!window.confirm(l10n.getString('appearance-reset-all-confirm'))) return;
    setResetting(true);
    try {
      // Reset in-memory state immediately so the UI updates.
      setColour(DEFAULT_COLOUR);
      setLogoPath(null);
      setStoreName('');

      // Persist changes via backend.
      await setBrandPrimaryColour(DEFAULT_COLOUR);
      await setBrandStoreName('');
      await setBrandLogoPath('');

      // Refresh brand context and apply palette.
      refreshBrandSettings();
      const palette = deriveAccentPalette(DEFAULT_COLOUR);
      applyAccentPalette(palette);

      addToast({ message: l10n.getString('appearance-reset-all-success'), type: 'success' });
    } catch {
      addToast({ message: l10n.getString('appearance-reset-all-failed'), type: 'error' });
    } finally {
      setResetting(false);
    }
  }, [refreshBrandSettings, addToast, l10n]);

  const content = (
    <div className="settings-form">
      <div className="appearance-field">
        <label htmlFor="brand-colour" className="settings-label">
          <Localized id="appearance-primary-colour">Primary Colour</Localized>
        </label>
        <div className="appearance-colour-row">
          <Localized id="appearance-primary-colour-picker-aria" attrs={{ 'aria-label': true }}>
            <input
              id="brand-colour"
              type="color"
              value={activeColour}
              onChange={(e) => updateColour(e.target.value)}
              aria-label="Primary colour picker"
              className="appearance-colour-picker"
            />
          </Localized>
          <Localized id="appearance-colour-hex-aria" attrs={{ 'aria-label': true }}>
            <input
              type="text"
              value={activeColour}
              onChange={(e) => {
                const normalised = normaliseHex(e.target.value);
                if (normalised) updateColour(normalised);
              }}
              className="appearance-colour-hex settings-input"
              autoComplete="off"
              aria-label="Colour hex value"
            />
          </Localized>
          <Localized id="appearance-reset-colour-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="appearance-colour-reset"
              onClick={() => updateColour(DEFAULT_COLOUR)}
              aria-label="Reset colour to default"
              title={l10n.getString('appearance-reset-colour')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                <polyline points="1 4 1 10 7 10" />
                <path d="M3.51 15a9 9 0 102.13-9.36L1 10" />
              </svg>
            </button>
          </Localized>
        </div>
      </div>

      <div className="appearance-field">
        <span className="settings-label">
          <Localized id="appearance-logo">Store Logo</Localized>
        </span>
        <div className="appearance-logo-row">
          {logoPath && (
            <Localized id="appearance-logo-alt" attrs={{ alt: true }}>
              <img
                src={`file://${logoPath}`}
                alt="Store logo"
                className="appearance-logo-preview"
              />
            </Localized>
          )}
          <Localized id="appearance-choose-logo-aria" attrs={{ 'aria-label': true }}>
            <Button variant="secondary" onClick={handlePickLogo} aria-label="Pick logo file">
              <Localized id="appearance-choose-logo">Choose Logo</Localized>
            </Button>
          </Localized>
          {logoPath && <span className="appearance-logo-path">{logoPath}</span>}
        </div>
      </div>

      <div className="appearance-field">
        <label htmlFor="store-name-display" className="settings-label">
          <Localized id="appearance-store-name">Display Store Name</Localized>
        </label>
        <input
          id="store-name-display"
          type="text"
          value={activeStoreName}
          onChange={(e) => updateStoreName(e.target.value)}
          className="settings-input"
          autoComplete="off"
        />
      </div>

      <div className="appearance-field">
        <label htmlFor="interface-zoom" className="settings-label">
          <Localized id="appearance-interface-zoom">Interface Zoom</Localized>
        </label>
        <select
          id="interface-zoom"
          value={zoomLevel}
          onChange={(e) => setZoomLevel(e.target.value as ZoomLevel)}
          className="settings-select"
        >
          <option value="auto"><Localized id="appearance-zoom-auto">Automatic (Scale with screen)</Localized></option>
          <option value="100"><Localized id="appearance-zoom-100">100% (Default)</Localized></option>
          <option value="125"><Localized id="appearance-zoom-125">125%</Localized></option>
          <option value="150"><Localized id="appearance-zoom-150">150%</Localized></option>
          <option value="200"><Localized id="appearance-zoom-200">200%</Localized></option>
        </select>
      </div>

      {!embedded && (
        <div className="appearance-reset-actions">
          <Localized id="appearance-reset-all-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="appearance-reset-all-btn"
              onClick={handleResetAll}
              disabled={resetting}
              aria-label="Reset all appearance settings"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
                <polyline points="1 4 1 10 7 10" />
                <path d="M3.51 15a9 9 0 102.13-9.36L1 10" />
              </svg>
              <Localized id="appearance-reset-all">Reset all to defaults</Localized>
            </button>
          </Localized>
        </div>
      )}

      <div className="appearance-preview">
        <h3 className="appearance-preview-heading">
          <Localized id="appearance-preview">Preview</Localized>
        </h3>
        <div
          className="appearance-preview-box"
          style={{
            '--preview-colour': activeColour,
            '--preview-btn-text': previewBtnText,
            '--preview-colour-alpha-10': `${activeColour}1a`,
            '--preview-colour-alpha-20': `${activeColour}33`,
          } as React.CSSProperties}
        >
          <div className="appearance-preview-sample">
            <span className="appearance-preview-text">
              {activeStoreName ? activeStoreName : <Localized id="appearance-store-name-fallback"><span>OZ-POS</span></Localized>}
            </span>
          </div>
          <div className="appearance-preview-elements">
            <button
              type="button"
              className="appearance-preview-btn"
              disabled
            >
              <Localized id="appearance-preview-btn-label">Primary Button</Localized>
            </button>
            <button
              type="button"
              className="appearance-preview-btn-outline"
              disabled
            >
              <Localized id="appearance-preview-btn-outline-label">Secondary</Localized>
            </button>
            <span className="appearance-preview-badge">
              <Localized id="appearance-preview-badge-label">Live</Localized>
            </span>
          </div>
        </div>
      </div>

      {!embedded && (
        <div className="settings-actions">
          <Localized id="appearance-save-aria" attrs={{ 'aria-label': true }}>
            <Button variant="primary" onClick={save} disabled={saving} aria-label="Save appearance">
              <Localized id="save">Save</Localized>
            </Button>
          </Localized>
        </div>
      )}
    </div>
  );

  if (embedded) {
    return content;
  }

  return (
    <Card shadow="sm">
      <h2 className="settings-section-title">
        <Localized id="settings-appearance">Appearance</Localized>
      </h2>
      {content}
    </Card>
  );
}
