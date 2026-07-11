import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
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
import './AppearanceSettings.css';

interface AppearanceSettingsProps {
  embedded?: boolean;
  colour?: string;
  storeName?: string;
  onColourChange?: (c: string) => void;
  onStoreNameChange?: (n: string) => void;
}

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
  const { zoomLevel, setZoomLevel } = useAppZoom();

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

  const updateColour = useCallback((c: string) => {
    if (embedded) {
      onColourChange?.(c);
    } else {
      setColour(c);
    }
    const palette = deriveAccentPalette(c);
    applyAccentPalette(palette);
  }, [embedded, onColourChange]);

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

  const save = useCallback(async () => {
    setSaving(true);
    await setBrandPrimaryColour(activeColour);
    await setBrandStoreName(activeStoreName);
    refreshBrandSettings();
    setSaving(false);
  }, [activeColour, activeStoreName, refreshBrandSettings]);

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
              onChange={(e) => updateColour(e.target.value)}
              className="appearance-colour-hex settings-input"
              aria-label="Colour hex value"
            />
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
          className="settings-input"
        >
          <option value="auto">Automatic (Scale with screen)</option>
          <option value="100">100% (Default)</option>
          <option value="125">125%</option>
          <option value="150">150%</option>
          <option value="200">200%</option>
        </select>
      </div>

      <div className="appearance-preview">
        <h3 className="appearance-preview-heading">
          <Localized id="appearance-preview">Preview</Localized>
        </h3>
        <div
          className="appearance-preview-box"
          style={{ '--preview-colour': activeColour } as React.CSSProperties}
        >
          <div className="appearance-preview-sample">
            <span className="appearance-preview-text" style={{ color: activeColour }}>
              {activeStoreName ? activeStoreName : <Localized id="appearance-store-name-fallback"><span>OZ-POS</span></Localized>}
            </span>
          </div>
          <div className="appearance-preview-elements">
            <button
              type="button"
              className="appearance-preview-btn"
              style={{
                backgroundColor: activeColour,
                borderColor: activeColour,
                color: parseInt(activeColour.slice(1), 16) > 0x7fffff ? '#0a0a0a' : '#ffffff',
              }}
              disabled
            >
              <Localized id="appearance-preview-btn-label">Primary Button</Localized>
            </button>
            <button
              type="button"
              className="appearance-preview-btn-outline"
              style={{
                borderColor: activeColour,
                color: activeColour,
              }}
              disabled
            >
              <Localized id="appearance-preview-btn-outline-label">Secondary</Localized>
            </button>
            <span
              className="appearance-preview-badge"
              style={{
                backgroundColor: `${activeColour}1a`,
                color: activeColour,
                borderColor: `${activeColour}33`,
              }}
            >
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
