import { useEffect, useState, useCallback } from 'react';
import { Localized } from '@fluent/react';
import {
  getBrandSettings,
  setBrandPrimaryColour,
  setBrandLogoPath,
  setBrandStoreName,
  pickLogoFile,
} from '@/api/branding';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './AppearanceSettings.css';

export function AppearanceSettings() {
  const [colour, setColour] = useState('#4f46e5');
  const [logoPath, setLogoPath] = useState<string | null>(null);
  const [storeName, setStoreName] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getBrandSettings().then((s) => {
      setColour(s.primary_colour);
      setLogoPath(s.logo_path);
      setStoreName(s.store_name);
    });
  }, []);

  const applyColour = useCallback((c: string) => {
    setColour(c);
    document.documentElement.style.setProperty('--color-accent', c);
  }, []);

  const handlePickLogo = useCallback(async () => {
    const path = await pickLogoFile();
    if (path) {
      setLogoPath(path);
      await setBrandLogoPath(path);
    }
  }, []);

  const save = useCallback(async () => {
    setSaving(true);
    await setBrandPrimaryColour(colour);
    await setBrandStoreName(storeName);
    setSaving(false);
  }, [colour, storeName]);

  return (
    <Card shadow="sm">
      <h2 className="settings-section-title">
        <Localized id="settings-appearance">Appearance</Localized>
      </h2>

      <div className="settings-form">
        <div className="appearance-field">
          <label htmlFor="brand-colour" className="settings-label">
            <Localized id="appearance-primary-colour">Primary Colour</Localized>
          </label>
          <div className="appearance-colour-row">
            <input
              id="brand-colour"
              type="color"
              value={colour}
              onChange={(e) => applyColour(e.target.value)}
              aria-label="Primary colour picker"
              className="appearance-colour-picker"
            />
            <input
              type="text"
              value={colour}
              onChange={(e) => applyColour(e.target.value)}
              className="appearance-colour-hex settings-input"
              aria-label="Colour hex value"
            />
          </div>
        </div>

        <div className="appearance-field">
          <span className="settings-label">
            <Localized id="appearance-logo">Store Logo</Localized>
          </span>
          <div className="appearance-logo-row">
            {logoPath && (
              <img
                src={`file://${logoPath}`}
                alt="Store logo"
                className="appearance-logo-preview"
              />
            )}
            <Button variant="secondary" onClick={handlePickLogo} aria-label="Pick logo file">
              <Localized id="appearance-choose-logo">Choose Logo</Localized>
            </Button>
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
            value={storeName}
            onChange={(e) => setStoreName(e.target.value)}
            className="settings-input"
          />
        </div>

        <div className="appearance-preview">
          <h3 className="appearance-preview-heading">
            <Localized id="appearance-preview">Preview</Localized>
          </h3>
          <div
            className="appearance-preview-box"
            style={{ '--preview-colour': colour } as React.CSSProperties}
          >
            <span className="appearance-preview-text" style={{ color: colour }}>
              {storeName || 'OZ-POS'}
            </span>
          </div>
        </div>

        <div className="settings-actions">
          <Button variant="primary" onClick={save} disabled={saving} aria-label="Save appearance">
            <Localized id="save">Save</Localized>
          </Button>
        </div>
      </div>
    </Card>
  );
}
