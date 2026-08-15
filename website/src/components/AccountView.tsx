import { useEffect, useState } from 'react';
import { t } from '../i18n';

/**
 * Account page (website-plan.md §8/§11). Reads the session token from
 * sessionStorage (set by AuthForm) and fetches /api/v1/web/me from the
 * license server. Graceful in every failure mode: no token, API unset,
 * server error.
 */
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

interface MeResponse {
  tenant?: {
    email: string;
    status: string;
  };
  license?: {
    key: string;
    tierKey: string;
    status: string;
    expiresAt?: string;
  };
}

interface Props {
  locale: string;
}

export default function AccountView({ locale }: Props) {
  const [state, setState] = useState<'loading' | 'anon' | 'error' | 'ready'>('loading');
  const [me, setMe] = useState<MeResponse | null>(null);

  useEffect(() => {
    if (!API) {
      setState('error');
      return;
    }
    const token = sessionStorage.getItem('oz_session');
    if (!token) {
      setState('anon');
      return;
    }
    let cancelled = false;
    fetch(`${API}/api/v1/web/me`, {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then(async (res) => {
        if (!res.ok) throw new Error('me failed');
        const data = (await res.json()) as MeResponse;
        if (!cancelled) {
          setMe(data);
          setState('ready');
        }
      })
      .catch(() => {
        if (!cancelled) setState('error');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (state === 'loading') return <p className="text-muted">{t(locale, 'account.loading')}</p>;

  if (state === 'anon') {
    return (
      <div className="rounded-xl border border-ink/10 bg-surface/40 p-6 text-center">
        <p className="text-muted">{t(locale, 'account.notSignedIn')}</p>
        <a
          href={`/${locale}/login`}
          className="mt-4 inline-block rounded-md bg-accent px-5 py-2.5 text-sm font-semibold text-black transition hover:opacity-90"
        >
          {t(locale, 'account.signIn')}
        </a>
      </div>
    );
  }

  if (state === 'error') {
    return (
      <p className="rounded-md border border-ink/10 p-4 text-sm text-muted">
        {API ? t(locale, 'account.error') : t(locale, 'account.notConfigured')}
      </p>
    );
  }

  const { tenant, license } = me ?? {};

  return (
    <div className="mx-auto max-w-2xl space-y-4">
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6" aria-label={t(locale, 'account.license')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.license')}</h2>
          <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
            <div>
              <dt className="text-muted">{t(locale, 'account.licenseKey')}</dt>
              <dd className="font-mono">{license?.key ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.tier')}</dt>
              <dd className="capitalize">{license?.tierKey ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.status')}</dt>
              <dd className="capitalize">{license?.status ?? tenant.status}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.expires')}</dt>
              <dd>{license?.expiresAt ?? '—'}</dd>
            </div>
          </dl>
        </section>
      )}
      <button
        type="button"
        onClick={() => {
          sessionStorage.removeItem('oz_session');
          window.location.href = `/${locale}`;
        }}
        className="text-sm text-muted transition hover:text-ink"
      >
        {t(locale, 'account.logout')}
      </button>
    </div>
  );
}
