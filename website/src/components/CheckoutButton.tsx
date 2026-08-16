import { useState } from 'react';
import { t } from '../i18n';
import type { PricingTier } from '../content/pricing/types';
import { hasSession, isPlaceholderPriceId, openPaddleCheckout, getSessionEmail } from './paddle';

/**
 * Pricing-page checkout button (website-plan.md §7). Payment is
 * register-first: without a session the button redirects to the login
 * page (which self-signs a new account on first OTP verify) instead of
 * opening checkout, so the webhook always finds a tenant for
 * customData.email. With a session it opens the Paddle checkout overlay
 * prefilled with the account email.
 *
 * When PADDLE_CLIENT_TOKEN is unset (or the price id is a placeholder)
 * the button degrades to a mailto fallback (see website-plan.md §7).
 */
interface Props {
  tier: PricingTier;
  locale: string;
}

export default function CheckoutButton({ tier, locale }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const priceId = tier.priceId;

  if (!priceId || isPlaceholderPriceId(priceId)) {
    return (
      <a
        href={`mailto:sales@oz-pos.com?subject=${encodeURIComponent('OZ-POS plan: ' + tier.name)}`}
        className="block w-full rounded-md border border-ink/15 px-4 py-2.5 text-center text-sm font-semibold text-ink transition hover:bg-ink/5"
      >
        {tier.cta}
      </a>
    );
  }

  // Round-trip back to this page after login; tier hints the next step.
  const loginHref = `/${locale}/login?next=/${locale}/pricing&tier=${encodeURIComponent(tier.tierKey)}`;

  const handleClick = async () => {
    if (!hasSession()) {
      // register-first: payment requires an account (website-plan.md §5)
      window.location.href = loginHref;
      return;
    }
    setLoading(true);
    setError(false);
    try {
      const email = await getSessionEmail();
      if (!email) {
        // Session token present but no email resolvable — back to login.
        window.location.href = loginHref;
        return;
      }
      await openPaddleCheckout(priceId, email);
    } catch {
      setError(true);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-2">
      <button
        type="button"
        onClick={() => void handleClick()}
        disabled={loading}
        className="block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
      >
        {loading ? '…' : hasSession() ? tier.cta : t(locale, 'checkout.signInToSubscribe')}
      </button>
      {error && (
        <p className="text-xs text-link" role="alert">
          {t(locale, 'checkout.error')}
        </p>
      )}
    </div>
  );
}
