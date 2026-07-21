/**
 * Email Report Settings — SMTP configuration and test email sending.
 *
 * Renders SMTP host, port, credentials, and from-address fields.
 * Persists via the existing `set_setting` / `get_setting` Tauri
 * commands using the `smtp_config` settings key as JSON.
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Localized, useLocalization } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { useToast } from '@/frontend/shared/Toast';
import Tooltip from '@/frontend/shell/Tooltip';

interface SmtpConfigDto {
  host: string;
  port: number;
  username: string | null;
  password: string | null;
  from: string;
  use_tls: boolean;
}

const DEFAULT_SMTP: SmtpConfigDto = {
  host: '',
  port: 587,
  username: null,
  password: null,
  from: '',
  use_tls: true,
};

const SMTP_CONFIG_KEY = 'smtp_config';

export default function EmailReportSettings() {
  const { l10n } = useLocalization();
  const { addToast } = useToast();

  const [config, setConfig] = useState<SmtpConfigDto>(DEFAULT_SMTP);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [sending, setSending] = useState(false);
  const [saved, setSaved] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  const loadConfig = useCallback(async () => {
    try {
      const raw = await invoke<string | null>('get_setting', { key: SMTP_CONFIG_KEY, user_id: '' });
      if (raw) {
        setConfig({ ...DEFAULT_SMTP, ...JSON.parse(raw) });
      }
    } catch {
      // Settings key doesn't exist yet — use defaults
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadConfig(); }, [loadConfig]);

  const saveConfig = useCallback(async () => {
    setSaving(true);
    setSaved(false);
    try {
      // Validate
      if (!config.host.trim()) {
        addToast({ message: l10n.getString('settings-email-host-required'), type: 'error' });
        setSaving(false);
        return;
      }
      if (!config.from.trim() || !config.from.includes('@')) {
        addToast({ message: l10n.getString('settings-email-from-required'), type: 'error' });
        setSaving(false);
        return;
      }

      await invoke('set_setting', {
        key: SMTP_CONFIG_KEY,
        value: JSON.stringify(config),
        user_id: '',
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      addToast({ message: l10n.getString('settings-email-saved'), type: 'success' });
    } catch (err) {
      addToast({ message: l10n.getString('settings-email-save-error'), type: 'error' });
    } finally {
      setSaving(false);
    }
  }, [config, l10n, addToast]);

  const handleSendTest = useCallback(async () => {
    setSending(true);
    try {
      // Dynamically import the email API to avoid circular deps
      const { sendTestReport } = await import('@/api/email');
      const message = await sendTestReport();
      addToast({ message, type: 'success' });
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err : 'Failed to send test email';
      addToast({ message: errorMessage, type: 'error' });
    } finally {
      setSending(false);
    }
  }, [addToast]);

  const updateField = useCallback(
    <K extends keyof SmtpConfigDto>(key: K, value: SmtpConfigDto[K]) => {
      setConfig((prev) => ({ ...prev, [key]: value }));
    },
    [],
  );

  if (loading) {
    return (
      <Card shadow="sm" header={l10n.getString('settings-section-email')}>
        <div className="settings-loading-inline" style={{ padding: '1rem', textAlign: 'center', color: '#9ca3af' }}>
          <Localized id="settings-email-loading">
            <span>Loading email settings…</span>
          </Localized>
        </div>
      </Card>
    );
  }

  return (
    <Card
      shadow="sm"
      header={
        <h2 className="settings-section-title">
          <Localized id="settings-section-email">
            <span>Email Reports</span>
          </Localized>
        </h2>
      }
    >
      <div className="settings-form">
        <p className="settings-hint" style={{ marginBottom: '1rem' }}>
          <Localized id="settings-email-description">
            <span>Configure SMTP to receive scheduled report emails.</span>
          </Localized>
        </p>

        {/* SMTP Host */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-host" className="settings-label">
            {l10n.getString('settings-email-host')}
          </label>
          <span className="settings-field-input-wrap">
            <input
              className="settings-input"
              type="text"
              id="settings-email-host"
              placeholder="smtp.example.com"
              value={config.host}
              onChange={(e) => updateField('host', e.target.value)}
              autoComplete="off"
              data-gramm="false"
            />
          </span>
        </div>

        {/* SMTP Port */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-port" className="settings-label">
            {l10n.getString('settings-email-port')}
          </label>
          <span className="settings-field-input-wrap">
            <input
              className="settings-input"
              type="number"
              id="settings-email-port"
              min={1}
              max={65535}
              value={config.port}
              onChange={(e) => updateField('port', parseInt(e.target.value, 10) || 0)}
              style={{ maxWidth: '120px' }}
            />
          </span>
        </div>

        {/* SMTP Username */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-username" className="settings-label">
            {l10n.getString('settings-email-username')}
          </label>
          <span className="settings-field-input-wrap">
            <input
              className="settings-input"
              type="text"
              id="settings-email-username"
              placeholder={l10n.getString('settings-email-username-placeholder')}
              value={config.username ?? ''}
              onChange={(e) => updateField('username', e.target.value || null)}
              autoComplete="off"
              data-gramm="false"
            />
          </span>
        </div>

        {/* SMTP Password */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-password" className="settings-label">
            {l10n.getString('settings-email-password')}
          </label>
          <span className="settings-field-input-wrap">
            <div className="settings-input-wrap">
              <input
                className="settings-input"
                type={showPassword ? 'text' : 'password'}
                id="settings-email-password"
                placeholder={l10n.getString('settings-email-password-placeholder')}
                value={config.password ?? ''}
                onChange={(e) => updateField('password', e.target.value || null)}
                autoComplete="off"
                data-gramm="false"
              />
              <button
                type="button"
                className="settings-input-toggle"
                onClick={() => setShowPassword((v) => !v)}
                aria-label={l10n.getString(showPassword ? 'settings-email-password-hide' : 'settings-email-password-show')}
                tabIndex={-1}
              >
                {showPassword ? (
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                    <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                    <line x1="1" y1="1" x2="23" y2="23" />
                  </svg>
                ) : (
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </svg>
                )}
              </button>
            </div>
          </span>
        </div>

        {/* From address */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-from" className="settings-label">
            {l10n.getString('settings-email-from')}
          </label>
          <span className="settings-field-input-wrap">
            <input
              className="settings-input"
              type="email"
              id="settings-email-from"
              placeholder="reports@mystore.com"
              value={config.from}
              onChange={(e) => updateField('from', e.target.value)}
              autoComplete="off"
              data-gramm="false"
            />
          </span>
        </div>

        {/* Use TLS toggle */}
        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-email-use-tls" className="settings-label">
            <Localized id="settings-email-use-tls">
              <span>Use STARTTLS</span>
            </Localized>
          </label>
          <span className="settings-field-input-wrap">
            <label className="settings-toggle" htmlFor="settings-email-use-tls">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="settings-email-use-tls"
                  type="checkbox"
                  role="switch"
                  checked={config.use_tls}
                  aria-checked={config.use_tls}
                  onChange={(e) => updateField('use_tls', e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </span>
        </div>

        {/* Action buttons */}
        <div className="settings-field settings-field--horizontal" style={{ borderTop: '1px solid #e5e7eb', paddingTop: '1rem', marginTop: '0.5rem' }}>
          <span />
          <span className="settings-field-input-wrap" style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', flexWrap: 'wrap' }}>
            <Button
              variant="primary"
              onClick={saveConfig}
              disabled={saving}
              aria-label={l10n.getString('settings-email-save-btn')}
            >
              {saving ? (
                <span className="btn-spinner" aria-hidden="true" />
              ) : saved ? (
                <Localized id="settings-email-saved-btn">
                  <span>Saved ✓</span>
                </Localized>
              ) : (
                <Localized id="settings-email-save-btn">
                  <span>Save SMTP Settings</span>
                </Localized>
              )}
            </Button>

            <Tooltip content={l10n.getString('settings-email-test-tooltip')}>
              <Button
                variant="secondary"
                onClick={handleSendTest}
                disabled={sending || !config.host.trim()}
                aria-label={l10n.getString('settings-email-test-btn')}
              >
                {sending ? (
                  <>
                    <span className="btn-spinner" aria-hidden="true" />
                    <Localized id="settings-email-sending">
                      <span> Sending…</span>
                    </Localized>
                  </>
                ) : (
                  <Localized id="settings-email-test-btn">
                    <span>Send Test Report</span>
                  </Localized>
                )}
              </Button>
            </Tooltip>
          </span>
        </div>
      </div>
    </Card>
  );
}
