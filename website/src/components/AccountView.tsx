import { useEffect, useState } from 'react';
import { t } from '../i18n';
import { pricingFor } from '../content/pricing';
import { getSessionEmail, openPaddleCheckout } from './paddle';

/**
 * Account dashboard (website-plan.md §8/§11). Reads the session token from
 * sessionStorage (set by AuthForm) and fetches /api/v1/web/me from the
 * license server. Shows the license + subscription state; when there is no
 * active subscription yet it renders the subscribe buttons, which open the
 * Paddle checkout prefilled with the account email (register-first flow —
 * the account must exist before payment). Graceful in every failure mode:
 * no token, API unset, server error.
 */
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

interface MeResponse {
  tenant?: {
    email: string;
    emailVerified: boolean;
    status: string;
  };
  license?: {
    key: string;
    tierKey: string;
    status: string;
    expiresAt?: string;
  };
  subscription?: {
    tierKey: string;
    status: string;
    startsAt?: string;
    expiresAt?: string;
    graceUntil?: string;
  };
}

interface Props {
  locale: string;
}

export default function AccountView({ locale }: Props) {
  const [state, setState] = useState<'loading' | 'anon' | 'error' | 'ready'>('loading');
  const [me, setMe] = useState<MeResponse | null>(null);
  const [subscribing, setSubscribing] = useState<string | null>(null);
  const [subscribeError, setSubscribeError] = useState(false);

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
        if (res.status === 401) {
          // Expired/revoked session — clear the stored token and show the
          // signed-out state instead of a confusing generic error.
          sessionStorage.removeItem('oz_session');
          if (!cancelled) setState('anon');
          return;
        }
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

  const subscribe = async (priceId: string, tierKey: string) => {
    setSubscribing(tierKey);
    setSubscribeError(false);
    try {
      const email = await getSessionEmail();
      if (!email) throw new Error('no session email');
      await openPaddleCheckout(priceId, email);
    } catch {
      setSubscribeError(true);
    } finally {
      setSubscribing(null);
    }
  };

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

  const { tenant, license, subscription } = me ?? {};
  // Subscribe options from the locale's pricing content (pro + premium
  // have real Paddle price ids; trial/enterprise do not).
  const subscribable = (pricingFor(locale) ?? [])
    .filter((tier) => tier.priceId && (tier.tierKey === 'pro' || tier.tierKey === 'premium'))
    .map((tier) => ({ tierKey: tier.tierKey, name: tier.name, price: tier.price, period: tier.period, priceId: tier.priceId! }));

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
            <div>
              <dt className="text-muted">{t(locale, 'account.emailVerified')}</dt>
              <dd>
                {tenant.emailVerified ? (
                  <span className="inline-flex items-center gap-1.5 text-link">
                    <span aria-hidden="true">✓</span>
                    {t(locale, 'account.verified')}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1.5 text-muted">
                    <span aria-hidden="true">○</span>
                    {t(locale, 'account.notVerified')}
                  </span>
                )}
              </dd>
            </div>
          </dl>
        </section>
      )}

      {subscription ? (
        <section
          className="rounded-xl border border-ink/10 bg-surface/40 p-6"
          aria-label={t(locale, 'account.subscription')}
        >
          <h2 className="text-lg font-semibold">{t(locale, 'account.subscription')}</h2>
          <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
            <div>
              <dt className="text-muted">{t(locale, 'account.tier')}</dt>
              <dd className="capitalize">{subscription.tierKey}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.status')}</dt>
              <dd className="capitalize">{subscription.status}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.starts')}</dt>
              <dd>{subscription.startsAt ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.expires')}</dt>
              <dd>{subscription.expiresAt ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.grace')}</dt>
              <dd>{subscription.graceUntil ?? '—'}</dd>
            </div>
          </dl>
        </section>
      ) : (
        <section className="rounded-xl border border-accent/40 bg-surface/40 p-6" aria-label={t(locale, 'account.subscribe')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.subscribe')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.noSubscription')}</p>
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            {subscribable.map((plan) => (
              <div key={plan.tierKey} className="rounded-lg border border-ink/10 p-4">
                <div className="flex items-baseline justify-between">
                  <span className="font-semibold">{plan.name}</span>
                  <span className="text-sm text-muted">
                    {plan.price}
                    {plan.period && <span> {plan.period}</span>}
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() => void subscribe(plan.priceId, plan.tierKey)}
                  disabled={subscribing !== null}
                  className="mt-3 block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
                >
                  {subscribing === plan.tierKey ? '…' : t(locale, 'account.subscribe')}
                </button>
              </div>
            ))}
          </div>
          {subscribeError && (
            <p className="mt-3 text-sm text-link" role="alert">
              {t(locale, 'checkout.error')}
            </p>
          )}
        </section>
      )}

      <button
        type="button"
        onClick={async () => {
          // Best-effort server-side invalidation; the local token is always
          // cleared regardless of network outcome so the user is never
          // stuck signed in.
          const token = sessionStorage.getItem('oz_session');
          if (API && token) {
            try {
              await fetch(`${API}/api/v1/web/logout`, {
                method: 'POST',
                headers: { Authorization: `Bearer ${token}` },
              });
            } catch {
              // Ignore network errors — logout is idempotent server-side.
            }
          }
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
