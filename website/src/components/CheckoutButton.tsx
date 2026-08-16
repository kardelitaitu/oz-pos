import { useState } from 'react';
import { t } from '../i18n';
import type { PricingTier } from '../content/pricing/types';

/**
 * Paddle.js (v2) checkout overlay. The script is loaded lazily on first
 * click; the price id comes from the tier content (placeholder `pri_…`
 * until real Paddle prices exist).
 *
 * The site has no account system, so the customer types the email their
 * license key should be delivered to. It is remembered in localStorage
 * (`oz_checkout_email`) to prefill repeat visits, prefilled into the
 * checkout overlay via `customer.email`, and passed as `customData.email`
 * — the Paddle webhook reads `data.custom_data.email` to upsert the tenant
 * (see apps/license-server/paddle_webhook.go), so no PADDLE_API_KEY is
 * needed on the server.
 *
 * When PADDLE_CLIENT_TOKEN is unset the button degrades to a mailto
 * fallback (see website-plan.md §7).
 */
declare global {
  interface Window {
    Paddle?: {
      Setup: (opts: { token: string; environment: 'sandbox' | 'production' }) => void;
      Checkout: (opts: {
        items: { priceId: string; quantity: number }[];
        customer?: { email: string };
        customData?: Record<string, string>;
      }) => void;
    };
  }
}

const EMAIL_STORAGE_KEY = 'oz_checkout_email';

function loadStoredEmail(): string {
  try {
    return window.localStorage.getItem(EMAIL_STORAGE_KEY) ?? '';
  } catch {
    return ''; // storage unavailable (private mode) — treat as empty
  }
}

function storeEmail(email: string): void {
  try {
    window.localStorage.setItem(EMAIL_STORAGE_KEY, email);
  } catch {
    // Non-fatal: persistence is a convenience, not a requirement.
  }
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

interface Props {
  tier: PricingTier;
  locale: string;
}

export default function CheckoutButton({ tier, locale }: Props) {
  const [loading, setLoading] = useState(false);
  const [email, setEmail] = useState(loadStoredEmail);
  const [emailError, setEmailError] = useState(false);
  const token = import.meta.env.PUBLIC_PADDLE_CLIENT_TOKEN as string | undefined;
  const environment = (import.meta.env.PUBLIC_PADDLE_ENVIRONMENT as string | undefined) === 'sandbox' ? 'sandbox' : 'production';
  const priceId = tier.priceId;

  if (!token || !priceId) {
    return (
      <a
        href={`mailto:sales@oz-pos.com?subject=${encodeURIComponent('OZ-POS plan: ' + tier.name)}`}
        className="block w-full rounded-md border border-ink/15 px-4 py-2.5 text-center text-sm font-semibold text-ink transition hover:bg-ink/5"
      >
        {tier.cta}
      </a>
    );
  }

  const openCheckout = () => {
    // The webhook provisions the tenant from custom_data.email — a valid
    // address is required, not optional.
    if (!EMAIL_RE.test(email)) {
      setEmailError(true);
      return;
    }
    setEmailError(false);
    storeEmail(email);
    setLoading(true);

    // If the Paddle script can't load (CDN blocked, offline) or the checkout
    // fails to start, fall back to the mailto so the user isn't stuck on "…".
    const mailtoFallback = () => {
      const subject = encodeURIComponent('OZ-POS plan: ' + tier.name);
      const body = encodeURIComponent(`Please send me the ${tier.name} license key.\n\nEmail: ${email}`);
      window.location.href = `mailto:sales@oz-pos.com?subject=${subject}&body=${body}`;
      setLoading(false);
    };

    const ensurePaddle = () =>
      new Promise<void>((resolve, reject) => {
        const existing = document.getElementById('paddle-js') as HTMLScriptElement | null;
        if (existing) {
          resolve();
          return;
        }
        const script = document.createElement('script');
        script.id = 'paddle-js';
        script.src = 'https://cdn.paddle.com/paddle/paddle.js';
        script.async = true;
        script.onload = () => resolve();
        script.onerror = () => reject(new Error('paddle failed to load'));
        document.head.appendChild(script);
      });

    const timer = window.setTimeout(() => {
      mailtoFallback();
    }, 8000);

    void ensurePaddle()
      .then(() => {
        window.clearTimeout(timer);
        if (!window.Paddle) {
          mailtoFallback();
          return;
        }
        window.Paddle.Setup({ token, environment });
        window.Paddle.Checkout({
          items: [{ priceId, quantity: 1 }],
          // Prefill the checkout's email field (customers can still change
          // it in the overlay). customData.email is what the webhook reads;
          // if the overlay email differs from the page email, the tenant is
          // created at the page email — which is where the receipt is sent.
          customer: { email },
          customData: { email },
        });
        setLoading(false);
      })
      .catch(() => {
        window.clearTimeout(timer);
        mailtoFallback();
      });
  };

  const inputClass =
    'w-full rounded-md border border-ink/10 bg-primary px-3 py-2 text-sm text-ink outline-none transition focus:border-accent';

  return (
    <div className="space-y-2">
      <label className="block">
        <span className="mb-1 block text-xs text-muted">{t(locale, 'checkout.email')}</span>
        <input
          type="email"
          required
          maxLength={200}
          autoComplete="email"
          value={email}
          onChange={(e) => {
            setEmail(e.target.value);
            if (emailError) setEmailError(false);
          }}
          placeholder={t(locale, 'checkout.emailPlaceholder')}
          aria-invalid={emailError}
          className={inputClass}
        />
      </label>
      {emailError && (
        <p className="text-xs text-link" role="alert">
          {t(locale, 'checkout.emailRequired')}
        </p>
      )}
      <button
        type="button"
        onClick={openCheckout}
        disabled={loading}
        className="block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
      >
        {loading ? '…' : tier.cta}
      </button>
    </div>
  );
}
