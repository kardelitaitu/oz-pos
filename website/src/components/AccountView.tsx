import { useCallback, useEffect, useRef, useState } from 'react';
import { t } from '../i18n';
import { pricingFor } from '../content/pricing';
import { isStrongPassword, passwordsMatch } from '../lib/passwordPolicy';
import { clearSession, getSessionEmail, isPaddleConfigured, isPlaceholderPriceId, openPaddleCheckout } from './paddle';
import { openMidtransCheckout } from './midtrans';
import { type Region, getRegion, setRegion } from '../lib/region';
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
 *
 * ## Register-first custom_data contract (ADR #23 Deviation 2)
 *
 * Both the subscribe section and the bundle upgrade card open the Paddle
 * checkout via `openPaddleCheckout(priceId, email, onClosed, bundle?)`.
 * The checkout embeds `custom_data` so the webhook can attach the
 * subscription to the correct tenant:
 *
 * - `custom_data.email` — the buyer's account email. **Required.** The
 *   webhook upserts the tenant by this value (`resolvePaddleEmail`).
 * - `custom_data.bundle` — optional C3.2 vertical bundle id
 *   (e.g. `"restaurant_starter"`). Cross-checked against the price map;
 *   never trusted alone.
 * - `custom_data.phone` — may ride along when Paddle collects it;
 *   backfilled onto the tenant when non-empty.
 *
 * The signup vertical is **not** carried — trial segmentation is a
 * desktop-activation concern, not a billing one.
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
    /** Vertical-bundle id (C3.2) this subscription was purchased with. */
    bundleId?: string;
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
  // Region state
  const [region, setRegionState] = useState<Region>(() => getRegion());
  const [regionMsg, setRegionMsg] = useState(false);
  const [regionOpen, setRegionOpen] = useState(false);

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

  /**
   * Open the checkout overlay for the given tier/bundle.
   *
   * Register-first custom_data contract (ADR #23 Deviation 2):
   * - `custom_data.email` — buyer's account email (required; webhook upserts
   *   the tenant by it)
   * - `custom_data.bundle` — optional C3.2 bundle id (cross-checked against
   *   the price map; never trusted alone)
   * - `custom_data.phone` — may ride along; backfilled onto tenant
   *
   * For Midtrans (id-locale), the equivalent fields are custom_field1–4
   * in the Snap request (see midtrans.ts).
   */
  const subscribe = async (priceId: string, tierKey: string, bundle?: string) => {
    setSubscribing(tierKey);
    setSubscribeError(false);
    // Indonesian market bills through Midtrans Snap (ADR #39 D1); every
    // other locale through Paddle.
    const useMidtrans = locale === 'id';
    try {
      if (useMidtrans) {
        // The bundle (C3.2) rides the snap request (custom_field4) so the
        // webhook mints the bundle-widened quota block.
        await openMidtransCheckout(tierKey, 'yearly', (completed) => pollAfterCheckout(completed), bundle);
        return;
      }
      const email = await getSessionEmail();
      if (!email) throw new Error('no session email');
      // After the overlay closes, refresh /me so a completed purchase shows
      // the subscription without a manual reload. The webhook provisions
      // asynchronously, so poll for it (up to ~20s) instead of a single fetch.
      await openPaddleCheckout(priceId, email, (completed) => pollAfterCheckout(completed), bundle);
    } catch (err) {
      console.error('checkout open failed', err);
      setSubscribeError(true);
    } finally {
      setSubscribing(null);
    }
  };

  /** Poll /me after a completed checkout until the webhook provisions. */
  const pollAfterCheckout = (completed: boolean) => {
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
  };

  if (state === 'loading') return <p className="text-muted">{t(locale, 'account.loading')}</p>;

  if (state === 'anon') {
    return (
      <div className="rounded-xl border border-ink/10 bg-surface/40 p-6 text-center">
        <p className="text-muted">{t(locale, 'account.notSignedIn')}</p>
        <a
          href={`/${locale}/login`}
          className="mt-4 inline-block rounded-md bg-accent px-5 py-2.5 text-sm font-semibold text-white transition hover:opacity-90"
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
  // Subscribe options from the locale's pricing content: the three paid tiers
  // (plus/pro/premium), billed at the yearly (default) rate. Tiers whose
  // Paddle price id is still a placeholder (subscription-tiers.md — six real
  // prices not yet catalogued) are excluded so the button never opens a
  // dead checkout; free/enterprise have no price id at all.
  // The id market bills through Midtrans (fixed Rp from the server's
  // MIDTRANS_PRICE_TIERS map), so Paddle price ids don't gate it; other
  // locales need a real, non-placeholder Paddle price.
  const useMidtrans = locale === 'id';
  const subscribable = (pricingFor(locale) ?? [])
    .filter((tier) => tier.tierKey === 'plus' || tier.tierKey === 'pro' || tier.tierKey === 'premium')
    .map((tier) => {
      const yearly = tier.prices.yearly;
      return { tierKey: tier.tierKey, name: tier.name, price: yearly.price, period: yearly.period, priceId: yearly.priceId ?? '' };
    })
    .filter((plan) => (useMidtrans ? true : plan.priceId && !isPlaceholderPriceId(plan.priceId)));

  // Restaurant Starter bundle (C3.2, subscription-tiers.md §5): the Plus
  // add-on, offered as an in-app upgrade to existing Plus subscribers who
  // don't own it yet. Gated on the same checkout availability as the
  // subscribe section — Midtrans needs the license API (the server's
  // MIDTRANS_PRICE_TIERS carries the bundle amount); Paddle needs a real,
  // non-placeholder bundle price id plus the client token. Until the real
  // catalog lands the placeholder ids keep the card hidden, exactly like
  // the subscribe section hides placeholder plans.
  const plusBundle = (pricingFor(locale) ?? []).find((tier) => tier.tierKey === 'plus')?.bundle;
  const bundleYearly = plusBundle?.prices.yearly;
  const bundleCheckoutAvailable =
    Boolean(plusBundle) &&
    (useMidtrans
      ? Boolean(licenseApiUrl())
      : Boolean(bundleYearly?.priceId) && !isPlaceholderPriceId(bundleYearly?.priceId) && isPaddleConfigured());

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
              className="rounded-md bg-accent px-4 py-2 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
            >
              {pwSaving ? '…' : t(locale, 'account.passwordSave')}
            </button>
          </form>
        </section>
      )}

      {/* Region selector */}
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6" aria-label={t(locale, 'account.region')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.region')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.regionHint')}</p>
          <div className="relative mt-3">
            <button
              type="button"
              onClick={() => setRegionOpen(!regionOpen)}
              onBlur={() => setTimeout(() => setRegionOpen(false), 150)}
              className="w-full rounded-md border border-ink/10 bg-primary px-3 py-2 text-sm text-left outline-none transition focus:border-accent flex items-center justify-between"
            >
              <span className="flex items-center gap-2">
                <span>{region === 'id' ? '🇮🇩' : '🌍'}</span>
                <span>{t(locale, region === 'id' ? 'signup.regionIndonesia' : 'signup.regionGlobal')}</span>
              </span>
              <svg
                className={`w-4 h-4 text-muted transition-transform duration-200 ${regionOpen ? 'rotate-180' : ''}`}
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="4 6 8 10 12 6" />
              </svg>
            </button>
            {regionOpen && (
              <div className="absolute z-50 mt-1 w-full rounded-md border border-ink/10 bg-primary shadow-lg overflow-hidden">
                {([
                  { value: 'global' as Region, flag: '🌍', label: t(locale, 'signup.regionGlobal') },
                  { value: 'id' as Region, flag: '🇮🇩', label: t(locale, 'signup.regionIndonesia') },
                ]).map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => {
                      setRegionState(opt.value);
                      setRegion(opt.value);
                      setRegionOpen(false);
                      setRegionMsg(true);
                      setTimeout(() => setRegionMsg(false), 3000);
                    }}
                    className={`w-full px-3 py-2 text-sm text-left flex items-center gap-2 transition-colors duration-150 ${
                      region === opt.value ? 'bg-accent/10 text-link font-medium' : 'text-ink hover:bg-ink/5'
                    }`}
                  >
                    <span>{opt.flag}</span>
                    <span>{opt.label}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
          {regionMsg && (
            <p className="mt-2 text-sm text-link" role="status">{t(locale, 'account.regionSaved')}</p>
          )}
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
          {/* In-app bundle upgrade (C3.2): existing Plus subscribers without
              the bundle get the Restaurant Starter add-on right here. The
              checkout carries bundle=restaurant_starter so the webhook
              mints the kds-widened quota block (Midtrans custom_field4 /
              Paddle custom_data.bundle). Hidden once bundleId is set. */}
          {subscription.tierKey === 'plus' && !subscription.bundleId && plusBundle && bundleCheckoutAvailable && (
            <div className="mt-5 rounded-lg border border-accent/40 p-4" data-testid="account-bundle-upgrade">
              <div className="flex items-baseline justify-between gap-2">
                <p className="font-semibold">{plusBundle.label}</p>
                <p className="text-sm text-muted">
                  {bundleYearly?.price}
                  {bundleYearly?.period && <span> {bundleYearly.period}</span>}
                </p>
              </div>
              <p className="mt-1 text-sm text-muted">{plusBundle.note}</p>
              <p className="mt-2 text-sm text-muted">{t(locale, 'account.bundleUpgradeHint')}</p>
              <button
                type="button"
                onClick={() => void subscribe(bundleYearly?.priceId ?? '', 'plus', plusBundle.id)}
                disabled={subscribing !== null}
                className="mt-3 block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
              >
                {subscribing === 'plus' ? '…' : t(locale, 'account.bundleUpgrade')}
              </button>
            </div>
          )}
        </section>
      ) : (
        <section className="rounded-xl border border-accent/40 bg-surface/40 p-6" aria-label={t(locale, 'account.subscribe')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.subscribe')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.noSubscription')}</p>
          {(useMidtrans ? Boolean(licenseApiUrl()) : isPaddleConfigured()) && subscribable.length > 0 ? (
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
                    className="mt-3 block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
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
        </section>
      )}

      {/* Checkout feedback shared by the subscribe section AND the bundle
          upgrade card (a Plus subscriber's bundle purchase also polls /me). */}
      {subscribeError && (
        <p className="text-sm text-link" role="alert">
          {t(locale, 'checkout.error')}
        </p>
      )}
      {refreshState === 'checking' && (
        <p className="text-sm text-muted" role="status">
          {t(locale, 'account.checkingSubscription')}
        </p>
      )}
      {refreshState === 'pending' && (
        <p className="text-sm text-muted" role="status">
          {t(locale, 'account.subscriptionPending')}
        </p>
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
