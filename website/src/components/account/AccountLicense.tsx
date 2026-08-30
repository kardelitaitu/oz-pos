import { useState } from 'react';
import { t } from '../../i18n';
import { statusLabel, statusPillClass, fmtDate } from './accountShared';

/**
 * License card — shows the activated license key (mono, select-all), a copy
 * button with transient "Copied!" feedback, tier, status pill, and expiry.
 * Presentational; the copy feedback is local state.
 */
interface License {
  key: string;
  tierKey: string;
  status: string;
  expiresAt?: string;
}

interface Props {
  locale: string;
  /** Tenant status used as a fallback when the license has no status. */
  tenantStatus: string;
  license?: License;
}

export default function AccountLicense({ locale, tenantStatus, license }: Props) {
  return (
    <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.license')}>
      <h2 className="text-lg font-semibold">{t(locale, 'account.license')}</h2>
      <dl className="mt-4 grid gap-3.5 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-muted">{t(locale, 'account.licenseKey')}</dt>
          <dd className="mt-1 flex items-center gap-2">
            <span className="font-mono bg-ink/5 px-2.5 py-1 rounded text-xs select-all border border-ink/10">
              {license?.key ?? '—'}
            </span>
            {license?.key && <CopyKeyButton locale={locale} licenseKey={license.key} />}
          </dd>
        </div>
        <div>
          <dt className="text-muted">{t(locale, 'account.tier')}</dt>
          <dd className="mt-1 font-medium capitalize">{license?.tierKey ?? '—'}</dd>
        </div>
        <div>
          <dt className="text-muted">{t(locale, 'account.status')}</dt>
          <dd className="mt-1 capitalize">
            <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${statusPillClass(license?.status ?? tenantStatus)}`}>
              {statusLabel(locale, license?.status ?? tenantStatus)}
            </span>
          </dd>
        </div>
        <div>
          <dt className="text-muted">{t(locale, 'account.expires')}</dt>
          <dd className="mt-1">{fmtDate(license?.expiresAt, locale)}</dd>
        </div>
      </dl>
    </section>
  );
}

/** Copy-key button with a transient success label. */
function CopyKeyButton({ locale, licenseKey }: { locale: string; licenseKey: string }) {
  const [copiedKey, setCopiedKey] = useState(false);
  return (
    <button
      type="button"
      onClick={() => {
        void navigator.clipboard?.writeText(licenseKey);
        setCopiedKey(true);
        setTimeout(() => setCopiedKey(false), 2500);
      }}
      className="inline-flex items-center gap-1 rounded border border-ink/15 bg-surface px-2 py-1 text-xs font-medium text-ink transition hover:bg-ink/5"
      aria-label={t(locale, 'account.copyKey')}
    >
      {copiedKey ? (
        <span className="text-success font-semibold">{t(locale, 'account.copied')}</span>
      ) : (
        <span>{t(locale, 'account.copyKey')}</span>
      )}
    </button>
  );
}
