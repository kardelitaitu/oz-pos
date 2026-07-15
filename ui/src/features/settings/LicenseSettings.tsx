import { useState, useEffect, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { getLicenseStatus, checkLicenseStatus, type ServerLicenseStatus } from '@/api/license';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { useToast } from '@/frontend/shared/Toast';
import './LicenseSettings.css';

/** Parsed subscription payload from the license server's signed JSON. */
interface LicensePayload {
  tenant_id: string;
  tier_key: string;
  status: string;
  max_stores: number;
  max_pos_instances: number;
  allowed_types: string[];
  starts_at: string;
  expires_at: string;
  grace_until: string;
  issued_at: string;
}

/** Format an RFC 3339 date string for display. */
function formatDate(rfc3339: string): string {
  try {
    const d = new Date(rfc3339);
    return d.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  } catch {
    return rfc3339;
  }
}

/** Human-readable tier label via l10n. */
function tierLabel(tier: string, l10n: ReturnType<typeof useLocalization>['l10n']): string {
  const key = `settings-license-tier-${tier}`;
  const v = l10n.getString(key);
  return v !== key ? v : tier;
}

/** Human-readable labels for workspace type slugs via l10n. */
function workspaceTypeLabel(type: string, l10n: ReturnType<typeof useLocalization>['l10n']): string {
  const key = `settings-license-ws-${type}`;
  const v = l10n.getString(key);
  return v !== key ? v : type;
}

/** License settings section — displays tier, expiry, grace period, and quotas. */
export default function LicenseSettings() {
  const { l10n } = useLocalization();
  const { addToast } = useToast();

  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [payload, setPayload] = useState<LicensePayload | null>(null);
  const [serverStatus, setServerStatus] = useState<ServerLicenseStatus | null>(null);
  const [checkingServer, setCheckingServer] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const status = await getLicenseStatus();
      if (status.payload) {
        const parsed: LicensePayload = JSON.parse(status.payload);
        setPayload(parsed);
      }
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : l10n.getString('settings-license-load-failed'));
    } finally {
      setLoading(false);
    }
  }, [l10n]);

  useEffect(() => { load(); }, [load]);

  const handleCheckServer = useCallback(async () => {
    setCheckingServer(true);
    try {
      const status = await checkLicenseStatus();
      setServerStatus(status);
      addToast({ type: 'info', message: l10n.getString('settings-license-server-status-retrieved') });
    } catch (err) {
      const msg = err instanceof Error ? err.message : l10n.getString('settings-license-server-check-failed');
      addToast({ type: 'error', message: msg });
    } finally {
      setCheckingServer(false);
    }
  }, [addToast, l10n]);

  // ── Loading / Error states ──────────────────────────────────
  if (loading) {
    return (
      <Card shadow="sm" header={<Localized id="settings-section-license"><h2 className="settings-section-title">License</h2></Localized>}>
        <div className="settings-license-skeleton" role="status" aria-live="polite" aria-label={l10n.getString('settings-loading')}>
          <div className="settings-license-skeleton-row">
            <span className="settings-license-skeleton-label" />
            <span className="settings-license-skeleton-value" />
          </div>
          <div className="settings-license-skeleton-row">
            <span className="settings-license-skeleton-label" />
            <span className="settings-license-skeleton-value" />
          </div>
          <div className="settings-license-skeleton-row">
            <span className="settings-license-skeleton-label" />
            <span className="settings-license-skeleton-value" />
          </div>
          <div className="settings-license-skeleton-row">
            <span className="settings-license-skeleton-label" />
            <span className="settings-license-skeleton-value" />
          </div>
        </div>
      </Card>
    );
  }

  if (loadError) {
    return (
      <Card shadow="sm" header={<Localized id="settings-section-license"><h2 className="settings-section-title">License</h2></Localized>}>
        <div className="settings-form">
          <div className="settings-error" role="alert">
            <p>{loadError}</p>
            <Button variant="secondary" onClick={() => { setLoadError(null); load(); }} aria-label={l10n.getString('settings-retry')}>
              <Localized id="settings-retry"><span>Retry</span></Localized>
            </Button>
          </div>
        </div>
      </Card>
    );
  }

  if (!payload) {
    return (
      <Card shadow="sm" header={<Localized id="settings-section-license"><h2 className="settings-section-title">License</h2></Localized>}>
        <div className="settings-license-empty" role="status">
          <svg className="settings-license-empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0110 0v4" />
          </svg>
          <Localized id="settings-license-not-activated">
            <p className="settings-license-empty-text">No license activated. Activate a license to see details here.</p>
          </Localized>
        </div>
      </Card>
    );
  }

  // ── Main render ─────────────────────────────────────────────
  return (
    <Card shadow="sm" header={<Localized id="settings-section-license"><h2 className="settings-section-title">License</h2></Localized>}>
      <div className="settings-form settings-license-section" role="region" aria-label={l10n.getString('settings-section-license')}>

        {/* ── Subscription details from local payload ── */}
        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-tier"><span>Tier</span></Localized>
          </span>
          <span className={`settings-license-value settings-license-value--tier settings-license-value--tier-${payload.tier_key}`}>
            {tierLabel(payload.tier_key, l10n)}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-status-label"><span>Status</span></Localized>
          </span>
          <span className={`settings-license-value${payload.status === 'active' ? ' settings-license-value--active' : ' settings-license-value--warning'}`}>
            {payload.status === 'active' ? l10n.getString('settings-license-status-active') : payload.status}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-expires"><span>Expires</span></Localized>
          </span>
          <span className="settings-license-value">
            {formatDate(payload.expires_at)}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-grace"><span>Grace Period Until</span></Localized>
          </span>
          <span className="settings-license-value">
            {formatDate(payload.grace_until)}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-max-stores"><span>Max Stores</span></Localized>
          </span>
          <span className="settings-license-value settings-license-value--mono">
            {payload.max_stores === 0 ? l10n.getString('settings-license-unlimited') : String(payload.max_stores)}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-max-pos"><span>Max POS Instances</span></Localized>
          </span>
          <span className="settings-license-value settings-license-value--mono">
            {payload.max_pos_instances === 0 ? l10n.getString('settings-license-unlimited') : String(payload.max_pos_instances)}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-tenant-id"><span>Tenant ID</span></Localized>
          </span>
          <span className="settings-license-value settings-license-value--mono">
            {payload.tenant_id}
          </span>
        </div>

        <div className="settings-license-row">
          <span className="settings-license-label">
            <Localized id="settings-license-allowed-types"><span>Allowed Workspace Types</span></Localized>
          </span>
          <span className="settings-license-value">
            {(payload.allowed_types ?? []).length === 0
              ? (<Localized id="settings-license-allowed-types-all"><span>All</span></Localized>)
              : payload.allowed_types.map((t) => workspaceTypeLabel(t, l10n)).join(', ')}
          </span>
        </div>

        {/* ── Server status check ── */}
        <div className="settings-license-row settings-license-row--actions">
          <Button
            variant="secondary"
            loading={checkingServer}
            onClick={handleCheckServer}
            aria-label={l10n.getString('settings-license-check-server')}
          >
            <Localized id="settings-license-check-server">
              <span>Check Server Status</span>
            </Localized>
          </Button>
        </div>

        {serverStatus && (
          <div className="settings-license-server-section" role="region" aria-label={l10n.getString('settings-license-server-results')}>
            <div className="settings-license-row">
              <span className="settings-license-label">
                <Localized id="settings-license-server-tier"><span>Server Tier</span></Localized>
              </span>
              <span className={`settings-license-value settings-license-value--tier settings-license-value--tier-${serverStatus.tier}`}>
                {tierLabel(serverStatus.tier, l10n)}
              </span>
            </div>
            <div className="settings-license-row">
              <span className="settings-license-label">
                <Localized id="settings-license-server-active"><span>Server Active</span></Localized>
              </span>
              <span className={`settings-license-value${serverStatus.active ? ' settings-license-value--active' : ' settings-license-value--warning'}`}>
                {serverStatus.active ? l10n.getString('settings-license-yes') : l10n.getString('settings-license-no')}
              </span>
            </div>
            {serverStatus.expiresAt && (
              <div className="settings-license-row">
                <span className="settings-license-label">
                  <Localized id="settings-license-server-expires"><span>Server Expires</span></Localized>
                </span>
                <span className="settings-license-value">
                  {formatDate(serverStatus.expiresAt)}
                </span>
              </div>
            )}
          </div>
        )}
      </div>
    </Card>
  );
}
