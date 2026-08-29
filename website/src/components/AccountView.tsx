import { useCallback, useEffect, useRef, useState } from 'react';
import { t } from '../i18n';
import { pricingFor } from '../content/pricing';
import { isStrongPassword, passwordsMatch } from '../lib/passwordPolicy';
import { clearSession, getSessionEmail, isPaddleConfigured, isPlaceholderPriceId, openPaddleCheckout } from './paddle';
import { openMidtransCheckout } from './midtrans';
import { type Region, getRegion, getExplicitRegion, setRegion } from '../lib/region';
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

/** A registered terminal/device from GET /api/v1/web/devices. */
interface Device {
  /** PocketBase record id — used as the revoke target. */
  id?: string;
  machine_id: string;
  created?: string;
  revoked_at?: string | null;
  status?: string;
}

/** Region options for the billing-region selector. */
const REGION_OPTIONS: { value: Region; labelKey: string }[] = [
  { value: 'global', labelKey: 'signup.regionGlobal' },
  { value: 'id', labelKey: 'signup.regionIndonesia' },
];

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

/** CSS classes for a status pill based on the raw server status value. */
function statusPillClass(status: string | undefined): string {
  switch (status) {
    case 'active':
      return 'bg-success/15 text-success';
    case 'grace_period':
      return 'bg-warning/15 text-warning';
    case 'expired':
    case 'revoked':
      return 'bg-danger/15 text-danger';
    default:
      return 'bg-ink/10 text-muted';
  }
}

/** Format an ISO date string to a locale-aware short date, or fallback. */
function fmtDate(dateStr: string | undefined, locale: string): string {
  if (!dateStr) return '—';
  try {
    // Use UTC timezone so the displayed calendar day is the same on every
    // machine — "2027-01-01" and "2027-01-01T00:00:00Z" both show "Jan 1,
    // 2027" regardless of whether the user is in Los Angeles or Jakarta.
    return new Date(dateStr).toLocaleDateString(locale === 'id' ? 'id-ID' : 'en-US', {
      timeZone: 'UTC', year: 'numeric', month: 'short', day: 'numeric',
    });
  } catch {
    return dateStr;
  }
}

/** Days until an ISO date string, or null when missing/parse fails. */
function daysUntil(dateStr: string | undefined): number | null {
  if (!dateStr) return null;
  try {
    // UTC-based calendar-day count, timezone- and clock-independent: the
    // difference between the expiry's UTC date and today's UTC date. A
    // subscription expiring "in 10 days" reports exactly 10 on any machine.
    const d = new Date(dateStr);
    // new Date('not-a-date') produces an Invalid Date (not a throw); its
    // UTC getters return NaN. Treat that as "no date" instead of leaking
    // NaN into the countdown label.
    if (Number.isNaN(d.getTime())) return null;
    const expiryUTC = Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
    const now = new Date();
    const todayUTC = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
    const days = Math.round((expiryUTC - todayUTC) / 86_400_000);
    return Number.isNaN(days) ? null : days;
  } catch {
    return null;
  }
}

/**
 * Localized "Renews in N days" label with correct singular/plural, or the
 * raw date fallback. `locale` picks the string; `days` drives the form.
 */
function renewsLabel(locale: string, days: number): string {
  const key = days === 1 ? 'account.renewsInDay' : 'account.renewsInDays';
  return t(locale, key).replace('{days}', String(days));
}

/** Renewal countdown pill for an active subscription, color-coded by urgency. */
function renderRenewBadge(locale: string, status: string | undefined, expiresAt: string | undefined) {
  if (status !== 'active' || !expiresAt) return null;
  const d = daysUntil(expiresAt);
  // A negative/past countdown is meaningless ("Renews in -3 days") — the
  // server can report status=active while the expiry has already lapsed
  // (clock skew, grace-period data). Hide the badge rather than show a
  // nonsensical countdown.
  if (d === null || d < 0) return null;
  const cls = d < 7 ? 'bg-danger/15 text-danger' : d < 30 ? 'bg-warning/15 text-warning' : 'bg-ink/10 text-muted';
  return <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${cls}`}>{renewsLabel(locale, d)}</span>;
}

export default function AccountView({ locale }: Props) {
  // Read API at component level so window.__OZ_CONFIG__ is available after hydration
  const API = licenseApiUrl();
  const [state, setState] = useState<'loading' | 'anon' | 'error' | 'ready'>('loading');
  const [me, setMe] = useState<MeResponse | null>(null);
  const [devices, setDevices] = useState<Device[] | null>(null);
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

  /** Fetch the tenant's registered devices (best-effort; null on any error). */
  const fetchDevices = useCallback(async (): Promise<Device[] | null> => {
    const token = sessionStorage.getItem('oz_session');
    if (!token || !API) return null;
    try {
      const res = await fetch(`${API}/api/v1/web/devices`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!res.ok) return null;
      const body = (await res.json()) as { devices?: Device[] };
      return body.devices ?? [];
    } catch {
      return null;
    }
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
  const [copiedKey, setCopiedKey] = useState(false);
  // Payment routing follows the saved region (ADR #39 D1) exactly like the
  // pricing-page CheckoutButton: Indonesia → Midtrans Snap, everything else
  // → Paddle. Falls back to the locale only when region is unset.
  const [useMidtrans, setUseMidtrans] = useState<boolean>(() => {
    const r = getExplicitRegion();
    return r === 'id' || (!r && locale === 'id');
  });
  useEffect(() => {
    const r = getExplicitRegion();
    setUseMidtrans(r === 'id' || (!r && locale === 'id'));
  }, [region, locale]);
  // Device revoke state: record id currently being revoked, plus the last
  // failure message (shown inline on the device row).
  const [revokingId, setRevokingId] = useState<string | null>(null);
  const [revokeError, setRevokeError] = useState<string | null>(null);

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
    // Best-effort device list — a failure here must not fail the dashboard.
    void fetchDevices()
      .then((list) => {
        if (mountedRef.current) setDevices(list);
      })
      .catch(() => {
        if (mountedRef.current) setDevices(null);
      });
  }, [fetchMe, fetchDevices]);

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
    // other region through Paddle. useMidtrans follows the saved region
    // preference (see state init), same as the pricing-page CheckoutButton.
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

  /** Revoke a device via POST /web/devices/{id}/revoke, then refresh the list. */
  const revokeDevice = async (device: Device) => {
    if (!device.id || !API) return;
    const token = sessionStorage.getItem('oz_session');
    if (!token) return;
    setRevokingId(device.id);
    setRevokeError(null);
    try {
      const res = await fetch(`${API}/api/v1/web/devices/${encodeURIComponent(device.id)}/revoke`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!res.ok) throw new Error(`revoke failed (${res.status})`);
      // Mark this device revoked in local state immediately; refresh the
      // full list so any server-side ordering is preserved. If the refresh
      // fails (null), keep the existing list and just stamp the revoked
      // device — the list must not collapse to the fallback hint.
      const fresh = await fetchDevices();
      if (mountedRef.current) {
        setDevices((prev) => {
          const list = fresh ?? prev ?? [];
          return list.map((d) => (d.id === device.id ? { ...d, revoked_at: d.revoked_at ?? new Date().toISOString() } : d));
        });
      }
    } catch (err) {
      if (mountedRef.current) setRevokeError(err instanceof Error ? err.message : String(err));
    } finally {
      if (mountedRef.current) setRevokingId(null);
    }
  };

  if (state === 'loading') {
    return (
      <div className="space-y-4 animate-pulse">
        <div className="rounded-xl border border-ink/10 bg-surface/40 p-6">
          <div className="flex items-center gap-3.5">
            <div className="w-11 h-11 rounded-full bg-ink/10" />
            <div className="space-y-2 flex-1">
              <div className="h-4 w-48 rounded bg-ink/10" />
              <div className="h-3 w-32 rounded bg-ink/10" />
            </div>
          </div>
        </div>
        <div className="rounded-xl border border-ink/10 bg-surface/40 p-6">
          <div className="h-4 w-24 rounded bg-ink/10 mb-4" />
          <div className="grid gap-3.5 sm:grid-cols-2">
            <div className="h-3 w-full rounded bg-ink/10" />
            <div className="h-3 w-full rounded bg-ink/10" />
            <div className="h-3 w-full rounded bg-ink/10" />
            <div className="h-3 w-full rounded bg-ink/10" />
          </div>
        </div>
        <div className="rounded-xl border border-ink/10 bg-surface/40 p-6">
          <div className="h-4 w-28 rounded bg-ink/10 mb-4" />
          <div className="grid grid-cols-3 gap-3">
            <div className="h-20 rounded-lg bg-ink/10" />
            <div className="h-20 rounded-lg bg-ink/10" />
            <div className="h-20 rounded-lg bg-ink/10" />
          </div>
        </div>
      </div>
    );
  }

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
  // WIP: all Paddle price ids are currently pri_placeholder_* (see
  // pricing/en.ts). For the id locale (Midtrans) the filter is bypassed,
  // so subscribe buttons render. For other locales all plans are filtered
  // out and the section shows "checkout unavailable".
  // The id market bills through Midtrans (fixed Rp from the server's
  // MIDTRANS_PRICE_TIERS map), so Paddle price ids don't gate it; other
  // markets need a real, non-placeholder Paddle price. useMidtrans is the
  // region-derived state (see state init) — do not redeclare it here.
  // The pricing source follows the payment provider: when useMidtrans is true
  // the checkout goes through Midtrans (IDR) so the displayed prices must
  // match — use the id pricing content, not the URL locale.
  const subscribable = (pricingFor(useMidtrans ? 'id' : locale) ?? [])
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
  // WIP: bundle checkout needs a real Paddle bundle price id (currently
  // pri_placeholder_plus_bundle_*) — the card stays hidden until then.
  // Pricing source follows the payment provider, same as the subscribe
  // section: Midtrans (useMidtrans) bills in IDR, so show the id pricing.
  const plusBundle = (pricingFor(useMidtrans ? 'id' : locale) ?? []).find((tier) => tier.tierKey === 'plus')?.bundle;
  const bundleYearly = plusBundle?.prices.yearly;
  const bundleCheckoutAvailable =
    Boolean(plusBundle) &&
    (useMidtrans
      ? Boolean(licenseApiUrl())
      : Boolean(bundleYearly?.priceId) && !isPlaceholderPriceId(bundleYearly?.priceId) && isPaddleConfigured());

  return (
    <div className="mx-auto max-w-2xl space-y-4">
      {tenant && (
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
                  <span className="text-success font-medium inline-flex items-center gap-1">
                    <span aria-hidden="true">✓</span> {t(locale, 'account.verified')}
                  </span>
                ) : (
                  <span className="text-muted inline-flex items-center gap-1">
                    <span aria-hidden="true">○</span> {t(locale, 'account.notVerified')}
                  </span>
                )}
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={async () => {
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
            className="self-start sm:self-auto rounded-lg border border-ink/15 bg-surface px-3 py-1.5 text-xs font-medium text-muted transition hover:text-ink hover:bg-ink/5"
          >
            {t(locale, 'account.logout')}
          </button>
        </div>
      )}

      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.license')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.license')}</h2>
          <dl className="mt-4 grid gap-3.5 text-sm sm:grid-cols-2">
            <div>
              <dt className="text-muted">{t(locale, 'account.licenseKey')}</dt>
              <dd className="mt-1 flex items-center gap-2">
                <span className="font-mono bg-ink/5 px-2.5 py-1 rounded text-xs select-all border border-ink/10">
                  {license?.key ?? '—'}
                </span>
                {license?.key && (
                  <button
                    type="button"
                    onClick={() => {
                      void navigator.clipboard?.writeText(license.key);
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
                )}
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.tier')}</dt>
              <dd className="mt-1 font-medium capitalize">{license?.tierKey ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.status')}</dt>
              <dd className="mt-1 capitalize">
                <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${statusPillClass(license?.status ?? tenant.status)}`}>
                  {statusLabel(locale, license?.status ?? tenant.status)}
                </span>
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.expires')}</dt>
              <dd className="mt-1">{fmtDate(license?.expiresAt, locale)}</dd>
            </div>
          </dl>
        </section>
      )}

      {/* Quick Action Navigation Grid */}
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.quickActions')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.quickActions')}</h2>
          <div className="mt-4 grid gap-3 sm:grid-cols-3">
            <a
              href={`/${locale}/download`}
              className="flex flex-col items-center justify-center gap-2 rounded-lg border border-ink/10 bg-surface p-4 text-center transition hover:border-accent hover:shadow-sm"
            >
              <svg className="w-5 h-5 text-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <span className="text-sm font-semibold text-ink">{t(locale, 'account.downloadApp')}</span>
            </a>
            <a
              href={`/${locale}/docs/activation`}
              className="flex flex-col items-center justify-center gap-2 rounded-lg border border-ink/10 bg-surface p-4 text-center transition hover:border-accent hover:shadow-sm"
            >
              <svg className="w-5 h-5 text-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <polyline points="12 6 12 12 16 14" />
              </svg>
              <span className="text-sm font-semibold text-ink">{t(locale, 'account.activationGuide')}</span>
            </a>
            <a
              href={`/${locale}/support`}
              className="flex flex-col items-center justify-center gap-2 rounded-lg border border-ink/10 bg-surface p-4 text-center transition hover:border-accent hover:shadow-sm"
            >
              <svg className="w-5 h-5 text-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
              </svg>
              <span className="text-sm font-semibold text-ink">{t(locale, 'account.contactSupport')}</span>
            </a>
          </div>
        </section>
      )}

      {/* Device / Terminal Management */}
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.devices')}>
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">{t(locale, 'account.devices')}</h2>
            <span className="rounded-full bg-accent/15 px-2.5 py-0.5 text-xs font-semibold text-link">
              {devices !== null
                ? t(locale, 'account.terminalCountLive').replace('{count}', String(devices.length))
                : license?.tierKey === 'pro' || license?.tierKey === 'enterprise' || license?.tierKey === 'premium'
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
                        onClick={() => void revokeDevice(d)}
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
      )}

      {/* Billing & Tax Invoices */}
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.billingInvoices')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.billingInvoices')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.billingInvoicesHint')}</p>
          <div className="mt-4 rounded-lg border border-ink/10 bg-surface p-4 space-y-2">
            <p className="text-xs text-muted leading-relaxed">{t(locale, 'account.invoiceNote')}</p>
            <div className="pt-2 flex items-center gap-3">
              <a
                href={`mailto:sales@ozpos.my.id?subject=${encodeURIComponent(t(locale, 'account.invoiceSubject').replace('{email}', tenant.email))}`}
                className="inline-flex items-center gap-1.5 text-xs font-semibold text-link hover:underline"
              >
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                  <polyline points="14 2 14 8 20 8" />
                  <line x1="16" y1="13" x2="8" y2="13" />
                  <line x1="16" y1="17" x2="8" y2="17" />
                  <polyline points="10 9 9 9 8 9" />
                </svg>
                {t(locale, 'account.viewReceipts')}
              </a>
            </div>
          </div>
        </section>
      )}

      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.password')}>
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
              <p className="text-sm text-success" role="status">{t(locale, 'account.passwordSaved')}</p>
            )}
            {pwMsg === 'error' && (
              <p className="text-sm text-danger" role="alert">{t(locale, 'account.passwordError')}</p>
            )}
            <PasswordStrength locale={locale} password={pw} />
            <button
              type="submit"
              disabled={pwSaving || !isStrongPassword(pw) || !passwordsMatch(pw, pwConfirm)}
              className="rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
            >
              {pwSaving ? '…' : t(locale, 'account.passwordSave')}
            </button>
          </form>
        </section>
      )}

      {/* Region selector */}
      {tenant && (
        <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.region')}>
          <h2 className="text-lg font-semibold">{t(locale, 'account.region')}</h2>
          <p className="mt-1 text-sm text-muted">{t(locale, 'account.regionHint')}</p>
          <div className="relative mt-3">
            <button
              type="button"
              onClick={() => setRegionOpen(!regionOpen)}
              onBlur={(e) => {
                // Only close when focus leaves the whole listbox. When the
                // user keyboard-navigates to an option, focus moves to a
                // button inside the listbox — that blur must NOT close it,
                // otherwise a keyboard user loses the dropdown mid-arrow.
                if (e.relatedTarget instanceof HTMLElement && e.relatedTarget.closest('[role="listbox"]')) {
                  return;
                }
                setTimeout(() => setRegionOpen(false), 150);
              }}
              onKeyDown={(e) => {
                // ArrowDown/ArrowUp open the listbox and move focus to the first option;
                // Escape closes it.
                if (!regionOpen && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
                  e.preventDefault();
                  setRegionOpen(true);
                  setTimeout(() => {
                    const first = document.querySelector<HTMLButtonElement>('[data-region-option]');
                    first?.focus();
                  }, 0);
                } else if (regionOpen && e.key === 'Escape') {
                  setRegionOpen(false);
                  e.currentTarget.focus();
                }
              }}
              aria-haspopup="listbox"
              aria-expanded={regionOpen}
              className="w-full rounded-md border border-ink/10 bg-surface px-3 py-2 text-sm text-left outline-none transition focus:border-accent flex items-center justify-between"
            >
              <span>{t(locale, region === 'id' ? 'signup.regionIndonesia' : 'signup.regionGlobal')}</span>
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
              <div
                className="absolute z-50 mt-1 w-full rounded-md border border-ink/10 bg-surface shadow-lg overflow-hidden"
                role="listbox"
                aria-label={t(locale, 'account.region')}
              >
                {REGION_OPTIONS.map((opt) => {
                  const selected = region === opt.value;
                  return (
                    <button
                      key={opt.value}
                      type="button"
                      role="option"
                      aria-selected={selected}
                      data-region-option
                      onClick={() => {
                        setRegionState(opt.value);
                        setRegion(opt.value);
                        setRegionOpen(false);
                        setRegionMsg(true);
                        setTimeout(() => setRegionMsg(false), 3000);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                          e.preventDefault();
                          const options = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-region-option]'));
                          const idx = options.indexOf(e.currentTarget);
                          const next = e.key === 'ArrowDown' ? options[idx + 1] : options[idx - 1];
                          next?.focus();
                        } else if (e.key === 'Escape') {
                          setRegionOpen(false);
                          const trigger = document.querySelector<HTMLButtonElement>('[aria-haspopup="listbox"]');
                          trigger?.focus();
                        } else if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          e.currentTarget.click();
                        }
                      }}
                      className={`w-full px-3 py-2 text-sm text-left flex items-center gap-2 transition-colors duration-150 ${
                        selected ? 'text-link font-medium' : 'text-ink hover:bg-ink/5'
                      }`}
                    >
                      <span>{t(locale, opt.labelKey)}</span>
                      {selected && (
                        <svg className="w-4 h-4 ml-auto text-success" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                          <polyline points="20 6 9 17 4 12" />
                        </svg>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
          {regionMsg && (
            <p className="mt-2 text-sm text-success" role="status">{t(locale, 'account.regionSaved')}</p>
          )}
        </section>
      )}

      {subscription ? (
        <section
          className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm"
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
              <dd className="capitalize">
                <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${statusPillClass(subscription.status)}`}>
                  {statusLabel(locale, subscription.status)}
                </span>
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.starts')}</dt>
              <dd>{fmtDate(subscription.startsAt, locale)}</dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.expires')}</dt>
              <dd className="flex items-center gap-2">
                <span>{fmtDate(subscription.expiresAt, locale)}</span>
                {renderRenewBadge(locale, subscription.status, subscription.expiresAt)}
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t(locale, 'account.grace')}</dt>
              <dd>{fmtDate(subscription.graceUntil, locale)}</dd>
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
        <section className="rounded-xl border border-accent/40 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.subscribe')}>
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
        <p className="text-sm text-danger" role="alert">
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
    </div>
  );
}
