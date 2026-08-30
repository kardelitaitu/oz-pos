import { t } from '../../i18n';
import { statusLabel } from './accountShared';

/**
 * Tenant profile card — avatar, email, status, email-verification badge,
 * and the sign-out action. Pure presentational: the parent passes the
 * tenant + a logout handler so the session lifecycle stays in one place.
 */
interface Props {
  locale: string;
  tenant: { email: string; emailVerified: boolean; status: string };
  onLogout: () => void;
}

export default function AccountProfile({ locale, tenant, onLogout }: Props) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 rounded-xl border border-ink/10 bg-surface/50 p-5 backdrop-blur-sm shadow-sm">
      <div className="flex items-center gap-3.5">
        <div className="w-11 h-11 rounded-full bg-accent/15 text-accent font-bold flex items-center justify-center text-lg shadow-inner">
          {tenant.email.charAt(0).toUpperCase()}
        </div>
        <div>
          <p className="font-semibold text-ink text-base">{tenant.email}</p>
          <div className="flex items-center gap-2 mt-0.5 text-xs text-muted">
            <span className="capitalize">{statusLabel(locale, tenant.status)}</span>
            <span>•</span>
            {tenant.emailVerified ? (
              <span className="text-success font-medium inline-flex items-center gap-1" title={t(locale, 'account.emailVerified')} aria-label={t(locale, 'account.emailVerified')}>
                <span aria-hidden="true">✓</span> {t(locale, 'account.verified')}
              </span>
            ) : (
              <span className="text-muted inline-flex items-center gap-1" title={t(locale, 'account.notVerified')} aria-label={t(locale, 'account.notVerified')}>
                <span aria-hidden="true">○</span> {t(locale, 'account.notVerified')}
              </span>
            )}
          </div>
        </div>
      </div>
      <button
        type="button"
        onClick={onLogout}
        className="self-start sm:self-auto rounded-lg border border-ink/15 bg-surface px-3 py-1.5 text-xs font-medium text-muted transition hover:text-ink hover:bg-ink/5"
      >
        {t(locale, 'account.logout')}
      </button>
    </div>
  );
}
