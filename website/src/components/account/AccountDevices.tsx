import { t } from '../../i18n';
import { fmtDate } from './accountShared';

/** A registered terminal/device from GET /api/v1/web/devices. */
export interface Device {
  /** PocketBase record id — used as the revoke target. */
  id?: string;
  machine_id: string;
  created?: string;
  revoked_at?: string | null;
  status?: string;
}

interface Props {
  locale: string;
  devices: Device[] | null;
  /** License tier used for the "unlimited" entitlement hint when no live count exists. */
  licenseTierKey?: string;
  revokingId: string | null;
  revokeError: string | null;
  onRevoke: (device: Device) => void;
}

/**
 * Device / terminal management — live count badge, up to 5 most recent
 * registrations, per-row revoke, and the empty-state "terminal slots" hint.
 * Presentational: revoke is a callback so the API + session lifecycle stays
 * in the parent.
 */
export default function AccountDevices({ locale, devices, licenseTierKey, revokingId, revokeError, onRevoke }: Props) {
  return (
    <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.devices')}>
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{t(locale, 'account.devices')}</h2>
        <span className="rounded-full bg-accent/15 px-2.5 py-0.5 text-xs font-semibold text-link">
          {devices !== null
            ? t(locale, 'account.terminalCountLive').replace('{count}', String(devices.length))
            : licenseTierKey === 'pro' || licenseTierKey === 'enterprise' || licenseTierKey === 'premium'
              ? t(locale, 'account.terminalUnlimited')
              : t(locale, 'account.terminalCount')}
        </span>
      </div>
      <p className="mt-1 text-sm text-muted">{t(locale, 'account.devicesHint')}</p>
      {devices && devices.length > 0 ? (
        <div className="mt-4 space-y-2">
          {devices.slice(0, 5).map((d) => (
            <div key={d.machine_id} className="rounded-lg border border-ink/10 bg-surface p-3 flex items-center justify-between">
              <div className="flex items-center gap-3 min-w-0">
                <div className="w-8 h-8 rounded-lg bg-ink/5 flex items-center justify-center text-muted flex-shrink-0">
                  <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                    <line x1="8" y1="21" x2="16" y2="21" />
                    <line x1="12" y1="17" x2="12" y2="21" />
                  </svg>
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-ink truncate">{d.machine_id}</p>
                  <p className="text-xs text-muted">{d.created ? fmtDate(d.created, locale) : '—'}</p>
                </div>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                  d.revoked_at ? 'bg-danger/15 text-danger' : 'bg-success/15 text-success'
                }`}>
                  {d.revoked_at ? t(locale, 'account.statusRevoked') : t(locale, 'account.statusActive')}
                </span>
                {!d.revoked_at && d.id && (
                  <button
                    type="button"
                    onClick={() => onRevoke(d)}
                    disabled={revokingId === d.id}
                    className="inline-flex items-center gap-1 rounded border border-ink/15 bg-surface px-2 py-1 text-xs font-medium text-ink transition hover:bg-ink/5 hover:border-danger/40 disabled:opacity-50"
                  >
                    {revokingId === d.id ? '…' : t(locale, 'account.revokeDevice')}
                  </button>
                )}
              </div>
            </div>
          ))}
          {revokeError && (
            <p className="text-xs text-danger" role="alert">{revokeError}</p>
          )}
          {devices.length > 5 && (
            <p className="text-xs text-muted text-center pt-1">+{devices.length - 5} more</p>
          )}
        </div>
      ) : (
        <div className="mt-4 rounded-lg border border-ink/10 bg-surface p-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-ink/5 flex items-center justify-center text-muted">
              <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
              </svg>
            </div>
            <div>
              <p className="text-sm font-medium text-ink">{t(locale, 'account.terminalSlots')}</p>
              <p className="text-xs text-muted">{t(locale, 'account.unbindHint')}</p>
            </div>
          </div>
          <a
            href={`/${locale}/docs/activation`}
            className="rounded-md border border-ink/15 bg-surface px-2.5 py-1 text-xs font-medium text-ink transition hover:bg-ink/5 flex-shrink-0 ml-2"
          >
            {t(locale, 'account.activationGuide')}
          </a>
        </div>
      )}
    </section>
  );
}
