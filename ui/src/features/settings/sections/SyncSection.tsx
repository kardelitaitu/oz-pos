import { Localized } from '@fluent/react';
import type { ReactLocalization } from '@fluent/react';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import type {
  SyncSettingsDto,
  SyncAttemptResult,
  PullResult,
  PingResult,
  TokenResult,
} from '@/api/offline';

/** Structured expiry info for a JWT token. */
interface ExpiryInfo {
  fluentKey: string;
  fluentArgs: Record<string, number | string>;
  tone: 'good' | 'warn' | 'critical';
}

/** Compute a localisable expiry label and urgency colour for a JWT token. */
function formatTokenExpiry(expiresAt: string | null): ExpiryInfo | null {
  if (!expiresAt) return null;
  const now = Date.now();
  const expiry = Date.parse(expiresAt);
  if (Number.isNaN(expiry)) {
    return { fluentKey: 'settings-sync-expiry-fallback', fluentArgs: { iso: expiresAt }, tone: 'warn' };
  }
  const diffMs = expiry - now;
  if (diffMs <= 0) {
    return { fluentKey: 'settings-sync-expiry-expired', fluentArgs: {}, tone: 'critical' };
  }
  const mins = Math.floor(diffMs / 60_000);
  const hours = Math.floor(diffMs / 3_600_000);
  const days = Math.floor(diffMs / 86_400_000);
  const tone: 'good' | 'warn' | 'critical' =
    hours < 1 ? 'critical' : hours < 24 ? 'warn' : 'good';
  if (days >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-days', fluentArgs: { count: days }, tone };
  }
  if (hours >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-hours', fluentArgs: { count: hours }, tone };
  }
  if (mins >= 1) {
    return { fluentKey: 'settings-sync-expiry-in-minutes', fluentArgs: { count: mins }, tone };
  }
  return { fluentKey: 'settings-sync-expiry-less-than-minute', fluentArgs: {}, tone: 'critical' };
}

export interface SyncSectionProps {
  sync: SyncSettingsDto;
  setSync: (s: SyncSettingsDto | ((prev: SyncSettingsDto) => SyncSettingsDto)) => void;
  syncServerUrl: string;
  setSyncServerUrl: (v: string) => void;
  syncApiKey: string;
  setSyncApiKey: (v: string) => void;
  syncApiKeyVisible: boolean;
  setSyncApiKeyVisible: (v: boolean | ((prev: boolean) => boolean)) => void;
  syncing: boolean;
  setSyncing: (v: boolean) => void;
  pulling: boolean;
  setPulling: (v: boolean) => void;
  syncResult: SyncAttemptResult | null;
  setSyncResult: (r: SyncAttemptResult | null) => void;
  pullResult: PullResult | null;
  setPullResult: (r: PullResult | null) => void;
  pendingCount: number | null;
  testing: boolean;
  setTesting: (v: boolean) => void;
  pingResult: PingResult | null;
  setPingResult: (r: PingResult | null) => void;
  requesting: boolean;
  setRequesting: (v: boolean) => void;
  tokenExpiresAt: string | null;
  setTokenExpiresAt: (v: string | null) => void;
  // eslint-disable-next-line @typescript-eslint/consistent-type-imports -- spread onto <input> elements
  cmInput: React.HTMLAttributes<HTMLInputElement>;
  markDirty: () => void;
  refreshPendingCount: () => Promise<void>;
  testSyncConnection: (url?: string) => Promise<PingResult>;
  syncRun: () => Promise<SyncAttemptResult>;
  syncPull: () => Promise<PullResult>;
  requestSyncToken: (url?: string) => Promise<TokenResult>;
  l10n: ReactLocalization;
  addToast: (opts: { message: string; type: 'success' | 'error' | 'info' }) => void;
}

export default function SyncSection({
  sync,
  setSync,
  syncServerUrl,
  setSyncServerUrl,
  syncApiKey,
  setSyncApiKey,
  syncApiKeyVisible,
  setSyncApiKeyVisible,
  syncing,
  setSyncing,
  pulling,
  setPulling,
  syncResult,
  setSyncResult,
  pullResult,
  setPullResult,
  pendingCount,
  testing,
  setTesting,
  pingResult,
  setPingResult,
  requesting,
  setRequesting,
  tokenExpiresAt,
  setTokenExpiresAt,
  cmInput,
  markDirty,
  refreshPendingCount,
  testSyncConnection,
  syncRun,
  syncPull,
  requestSyncToken,
  l10n,
  addToast,
}: SyncSectionProps) {
  return (
    <Card
      shadow="sm"
      header={<Localized id="settings-section-sync"><h2 className="settings-section-title">Cloud Sync</h2></Localized>}
    >
      <div className="settings-form">
        {sync.serverUrl === null && !sync.enabled && (
          <p className="settings-hint">
            <Localized id="settings-sync-not-configured">
              <span>Sync is not configured. Enter a server URL and enable sync.</span>
            </Localized>
          </p>
        )}

        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-field-server-url" className="settings-label">
            {l10n.getString('settings-sync-server-url')}
          </label>
          <span className="settings-field-input-wrap">
            <Localized id="settings-server-url-placeholder" attrs={{ placeholder: true }}>
              <input
                className="settings-input" {...cmInput}
                type="url"
                id="settings-field-server-url"
                placeholder="https://api.example.com"
                value={syncServerUrl}
                onChange={(e) => { setSyncServerUrl(e.target.value); setPingResult(null); setTokenExpiresAt(null); markDirty(); }}
              />
            </Localized>
          </span>
        </div>

        <div className="settings-field settings-field--horizontal">
          <label htmlFor="settings-field-api-key" className="settings-label">
            {l10n.getString('settings-sync-api-key')}
          </label>
          <span className="settings-field-input-wrap">
            <div className="settings-input-wrap">
              <Localized id={sync.hasApiKey ? 'settings-api-key-masked' : 'settings-api-key-placeholder'} attrs={{ placeholder: true }}>
                <input
                  className="settings-input" {...cmInput}
                  type={syncApiKeyVisible ? 'text' : 'password'}
                  id="settings-field-api-key"
                  placeholder={sync.hasApiKey ? '••••••••' : 'Enter API key'}
                  value={syncApiKey}
                  onChange={(e) => { setSyncApiKey(e.target.value); markDirty(); }}
                />
              </Localized>
              {/* Only show the eye toggle when there is text to reveal. */}
              {syncApiKey && (
              <button
                type="button"
                className="settings-input-toggle"
                onClick={() => setSyncApiKeyVisible((v) => !v)}
                aria-label={l10n.getString(syncApiKeyVisible ? 'settings-api-key-hide-aria' : 'settings-api-key-show-aria')}
                tabIndex={-1}
              >
                {syncApiKeyVisible ? (
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
              )}
            </div>
            <p className="settings-hint">
              <Localized id="settings-sync-token-hint">
                <span>Enter a JWT token from the cloud server. Generate one via POST /api/v1/tokens</span>
              </Localized>
            </p>
            <div className="settings-sync-token-actions">
              <Button
                variant="ghost"
                loading={requesting}
                onClick={async () => {
                  setRequesting(true);
                  try {
                    const result = await requestSyncToken(syncServerUrl || undefined);
                    if (result.ok && result.token) {
                      setSyncApiKey(result.token);
                      setSyncApiKeyVisible(false);
                      setTokenExpiresAt(result.expiresAt ?? null);
                      markDirty();
                      addToast({ message: result.status, type: 'success' });
                    } else {
                      addToast({ message: result.status, type: 'error' });
                    }
                  } catch {
                    addToast({ message: l10n.getString('settings-sync-token-request-failed'), type: 'error' });
                  } finally {
                    setRequesting(false);
                  }
                }}
              >
                <Localized id={requesting ? 'settings-sync-requesting' : 'settings-sync-request-token'}>
                  <span>{requesting ? 'Requesting…' : 'Request Token'}</span>
                </Localized>
              </Button>
            </div>
            {(() => {
              const expiry = formatTokenExpiry(tokenExpiresAt);
              if (!expiry) return null;
              return (
                <span className={`settings-sync-expiry-badge settings-sync-expiry-badge--${expiry.tone}`}>
                  {l10n.getString(expiry.fluentKey, expiry.fluentArgs)}
                </span>
              );
            })()}
          </span>
        </div>

        <div className="settings-field settings-field--horizontal">
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- @fluent/react Localized wrapper */}
          <label htmlFor="sync-enabled" className="settings-label">
            <Localized id="settings-sync-enabled">
              <span>Enable Cloud Sync</span>
            </Localized>
          </label>
          <span className="settings-field-input-wrap">
            <label className="settings-toggle" htmlFor="sync-enabled">
              <span className="sr-only">Toggle</span>
              <span className="settings-toggle-switch">
                <input
                  id="sync-enabled"
                  type="checkbox"
                  role="switch"
                  checked={sync.enabled}
                  aria-checked={sync.enabled}
                  onChange={(e) => { setSync({ ...sync, enabled: e.target.checked }); markDirty(); }}
                />
                <span className="settings-toggle-slider" />
              </span>
            </label>
          </span>
        </div>

        {(sync.serverUrl !== null || sync.enabled) && (
          <>
            {/* ── Status indicator ──────────────────── */}
            <div className="settings-sync-status">
              <span
                className={`settings-sync-dot${syncResult && !syncResult.error ? ' settings-sync-dot--ok' : ''}${syncResult?.error ? ' settings-sync-dot--err' : ''}`}
                aria-hidden="true"
              />
              <span className="settings-sync-status-text">
                {syncResult === null
                  ? (pingResult
                    ? pingResult.status
                    : l10n.getString('settings-sync-status-idle'))
                  : syncResult.error
                    ? syncResult.error
                    : l10n.getString('settings-sync-status-ok')}
              </span>
              {pendingCount !== null && pendingCount > 0 && (
                <span className="settings-sync-pending-badge">
                  {l10n.getString('settings-sync-pending-count', { count: pendingCount })}
                </span>
              )}
            </div>

            <div className="settings-actions">
              <Button
                variant="ghost"
                loading={testing}
                onClick={async () => {
                  setTesting(true);
                  setPingResult(null);
                  try {
                    const result = await testSyncConnection(syncServerUrl || undefined);
                    setPingResult(result);
                    if (result.ok) {
                      addToast({ message: result.status, type: 'success' });
                    } else {
                      addToast({ message: result.status, type: 'error' });
                    }
                  } catch {
                    setPingResult({ ok: false, status: l10n.getString('settings-sync-test-failed'), latencyMs: null });
                    addToast({ message: l10n.getString('settings-sync-test-failed'), type: 'error' });
                  } finally {
                    setTesting(false);
                  }
                }}
              >
                <Localized id={testing ? 'settings-sync-testing' : 'settings-sync-test-connection'}>
                  <span>{testing ? 'Testing…' : 'Test Connection'}</span>
                </Localized>
              </Button>
              <Button
                variant="secondary"
                loading={syncing}
                onClick={async () => {
                  setSyncing(true);
                  setSyncResult(null);
                  try {
                    const result = await syncRun();
                    setSyncResult(result);
                    refreshPendingCount();
                    if (result.error) {
                      addToast({ message: result.error, type: 'error' });
                    } else if (result.synced > 0 || result.failed > 0) {
                      addToast({
                        message: l10n.getString('settings-sync-success', { synced: result.synced, failed: result.failed }),
                        type: 'success',
                      });
                    } else {
                      addToast({
                        message: l10n.getString('settings-sync-nothing'),
                        type: 'info',
                      });
                    }
                  } catch {
                    const errMsg = l10n.getString('settings-sync-error');
                    setSyncResult({ synced: 0, failed: 0, error: errMsg });
                    addToast({ message: errMsg, type: 'error' });
                  } finally {
                    setSyncing(false);
                  }
                }}
              >
                <Localized id={syncing ? 'settings-sync-syncing' : 'settings-sync-sync-now'}>
                  <span>{syncing ? 'Syncing…' : 'Sync Now'}</span>
                </Localized>
              </Button>
              <Button
                variant="ghost"
                loading={pulling}
                onClick={async () => {
                  setPulling(true);
                  setPullResult(null);
                  try {
                    const result = await syncPull();
                    setPullResult(result);
                    if (result.error) {
                      addToast({ message: result.error, type: 'error' });
                    } else if (result.productsPulled > 0 || result.taxRatesPulled > 0 || result.usersPulled > 0) {
                      addToast({
                        message: l10n.getString('settings-sync-pull-toast-success', { products: result.productsPulled, tax_rates: result.taxRatesPulled, users: result.usersPulled }),
                        type: 'success',
                      });
                    } else {
                      addToast({
                        message: l10n.getString('settings-sync-pull-empty'),
                        type: 'info',
                      });
                    }
                  } catch {
                    const errMsg = l10n.getString('settings-sync-error');
                    setPullResult({ productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: errMsg });
                    addToast({ message: errMsg, type: 'error' });
                  } finally {
                    setPulling(false);
                  }
                }}
              >
                <Localized id={pulling ? 'settings-sync-pulling' : 'settings-sync-pull'}>
                  <span>{pulling ? 'Pulling…' : 'Pull from Server'}</span>
                </Localized>
              </Button>
            </div>

            {syncResult && (
              <div className="settings-sync-result-block">
                <p className="settings-hint">
                  <Localized
                    id="settings-sync-result"
                    vars={{ synced: syncResult.synced, failed: syncResult.failed }}
                  >
                    <span>Last sync: {syncResult.synced} synced, {syncResult.failed} failed</span>
                  </Localized>
                </p>
                {syncResult.error && (
                  <p className="settings-hint settings-hint--error">{syncResult.error}</p>
                )}
              </div>
            )}

            {pullResult && (
              <div className="settings-sync-result-block">
                <p className="settings-hint">
                  <Localized
                    id="settings-sync-pull-result"
                    vars={{ products: pullResult.productsPulled, tax_rates: pullResult.taxRatesPulled, users: pullResult.usersPulled }}
                  >
                    <span>Last pull: {pullResult.productsPulled} products, {pullResult.taxRatesPulled} tax rates, {pullResult.usersPulled} users</span>
                  </Localized>
                </p>
                {pullResult.error && (
                  <p className="settings-hint settings-hint--error">{pullResult.error}</p>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </Card>
  );
}
