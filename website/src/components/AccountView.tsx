import { useCallback, useEffect, useRef, useState } from 'react';
import { t } from '../i18n';
import { pricingFor } from '../content/pricing';
import { isStrongPassword, passwordsMatch } from '../lib/passwordPolicy';
import { clearSession, getSessionEmail, isPaddleConfigured, openPaddleCheckout } from './paddle';
import PasswordField from './PasswordField';
import PasswordStrength from './PasswordStrength';
import { licenseApiUrl } from '../lib/runtime-config';

/**
 * Account dashboard (website-plan.md §8/§11). Reads the session token from
 * sessionStorage (set by AuthForm) and fetches /api/v1/web/me from the
 * license server. Shows the license + subscription state; when there is no
 * active subscription yet it renders the subscribe buttons, which open the
 * Paddle checkout prefilled with the account email (register-first flow —
 * the account must exist before payment). Graceful in every failure mode:
 * no token, API unset, server error.
 */
const API = licenseApiUrl();

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

/**
 * Localized label for the raw status values the license server writes
 * (license_keys + subscriptions collections). Unknown values pass through
 * unchanged so a new server status never renders blank.
 */
function statusLabel(locale: string, status: string | undefined): string {
  switch (status) {
    case 'active':
      return t(locale, 'account.statusActive');
    case 'unused':
      return t(locale, 'account.statusUnused');
    case 'grace_period':
      return t(locale, 'account.statusGracePeriod');
    case 'expired':
      return t(locale, 'account.statusExpired');
    case 'revoked':
      return t(locale, 'account.statusRevoked');
    default:
      return status ?? '—';
  }
}

export default function AccountView({ locale }: Props) {
  const [state, setState] = useState<'loading' | 'anon' | 'error' | 'ready'>('loading');
  const [me, setMe] = useState<MeResponse | null>(null);
  const [subscribing, setSubscribing] = useState<string | null>(null);
  const [subscribeError, setSubscribeError] = useState(false);
  // Post-checkout refresh: 'checking' polls /me after a completed purchase;
  // 'pending' means the webhook hasn't provisioned yet after the poll window.
  const [refreshState, setRefreshState] = useState<'idle' | 'checking' | 'pending'>('idle');
  const mountedRef = useRef(true);

  useEffect(() => {
    return () => {
      mountedRef.current = false;
    };
  }, []);

  /** Fetch /me once; returns the payload, or null when signed out (token cleared). */
  const fetchMe = useCallback(async (): Promise<MeResponse | null> => {
    const token = sessionStorage.getItem('oz_session');
    if (!token || !API) return null;
    const res = await fetch(`${API}/api/v1/web/me`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (res.status === 401) {
      // Expired/revoked session — clear the stored token AND the cached
      // email; caller shows anon.
      clearSession();
      return null;
    }
    if (!res.ok) throw new Error('me failed');
    return (await res.json()) as MeResponse;
  }, []);
  // Password state: the optional login credential managed via set-password.
  const [pw, setPw] = useState('');
  const [pwConfirm, setPwConfirm] = useState('');
  const [pwMsg, setPwMsg] = useState<'idle' | 'saved' | 'error'>('idle');
  const [pwSaving, setPwSaving] = useState(false);

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
    fetchMe()
      .then((data) => {
        if (!mountedRef.current) return;
        if (data) {
          setMe(data);
          setState('ready');
        } else {
          setState('anon');
        }
      })
      .catch(() => {
        if (mountedRef.current) setState('error');
      });
  }, [fetchMe]);

  const savePassword = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setPwMsg('idle');
    setPwSaving(true);
    try {
      const token = sessionStorage.getItem('oz_session');
      if (!token) throw new Error('no session');
      const res = await fetch(`${API}/api/v1/web/set-password`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ password: pw, password_confirm: pwConfirm }),
      });
      if (!res.ok) throw new Error('set-password failed');
      setPw('');
      setPwConfirm('');
      setPwMsg('saved');
    } catch {
      setPwMsg('error');
    } finally {
      setPwSaving(false);
    }
  };

  const subscribe = async (priceId: string, tierKey: string) => {
    setSubscribing(tierKey);
    setSubscribeError(false);
    try {
      const email = await getSessionEmail();
      if (!email) throw new Error('no session email');
      // After the overlay closes, refresh /me so a completed purchase shows
      // the subscription without a manual reload. The webhook provisions
      // asynchronously, so poll for it (up to ~20s) instead of a single fetch.
      await openPaddleCheckout(priceId, email, (completed) => {
        if (!completed) return;
        if (!mountedRef.current) return;
        setRefreshState('checking');
        void (async () => {
          let found = false;
          for (let i = 0; i < 8 && !found; i++) {
            await new Promise((r) => setTimeout(r, 2500));
            if (!mountedRef.current) return;
            try {
              const data = await fetchMe();
              if (data) {
                setMe(data);
                setState('ready');
                found = Boolean(data.subscription);
              }
            } catch {
              // Transient network error — keep polling.
            }
          }
          if (mountedRef.current) setRefreshState(found ? 'idle' : 'pending');
        })();
      });
    } catch (err) {
      console.error('checkout open failed', err);
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
              <dd className="capitalize">{statusLabel(locale, license?.status ?? tenant.status)}</dd>
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

      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6" aria-label={t(locale, 'account.password')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.password')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.passwordHelp')}</p>
          <form onSubmit={savePassword} className="mt-4 space-y-3">
            <PasswordField
              locale={locale}
              id="account-password"
              label={t(locale, 'account.passwordPlaceholder')}
              value={pw}
              onChange={(v) => {
                setPw(v);
                if (pwMsg !== 'idle') setPwMsg('idle');
              }}
              autoComplete="new-password"
              placeholder={t(locale, 'account.passwordPlaceholder')}
              showConfirm
              confirmValue={pwConfirm}
              onConfirmChange={(v) => {
                setPwConfirm(v);
                if (pwMsg !== 'idle') setPwMsg('idle');
              }}
            />
            {pwMsg === 'saved' && (
              <p className="text-sm text-link" role="status">{t(locale, 'account.passwordSaved')}</p>
            )}
            {pwMsg === 'error' && (
              <p className="text-sm text-link" role="alert">{t(locale, 'account.passwordError')}</p>
            )}
            <PasswordStrength locale={locale} password={pw} />
            <button
              type="submit"
              disabled={pwSaving || !isStrongPassword(pw) || !passwordsMatch(pw, pwConfirm)}
              className="rounded-md bg-accent px-4 py-2 text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
            >
              {pwSaving ? '…' : t(locale, 'account.passwordSave')}
            </button>
          </form>
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
              <dd className="capitalize">{statusLabel(locale, subscription.status)}</dd>
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
          {subscription.status !== 'active' && (
            <p className="mt-4 text-sm text-muted">
              {t(locale, 'account.renewHint')}{' '}
              <a href={`/${locale}/pricing`} className="text-link underline">
                {t(locale, 'account.renewLink')}
              </a>
            </p>
          )}
        </section>
      ) : (
        <section className="rounded-xl border border-accent/40 bg-surface/40 p-6" aria-label={t(locale, 'account.subscribe')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.subscribe')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.noSubscription')}</p>
          {isPaddleConfigured() ? (
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
          ) : (
            <p className="mt-4 text-sm text-muted" role="status">
              {t(locale, 'account.checkoutUnavailable')}
            </p>
          )}
          {subscribeError && (
            <p className="mt-3 text-sm text-link" role="alert">
              {t(locale, 'checkout.error')}
            </p>
          )}
          {refreshState === 'checking' && (
            <p className="mt-3 text-sm text-muted" role="status">
              {t(locale, 'account.checkingSubscription')}
            </p>
          )}
          {refreshState === 'pending' && (
            <p className="mt-3 text-sm text-muted" role="status">
              {t(locale, 'account.subscriptionPending')}
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
          clearSession();
          window.location.href = `/${locale}`;
        }}
        className="text-sm text-muted transition hover:text-ink"
      >
        {t(locale, 'account.logout')}
      </button>
    </div>
  );
}
