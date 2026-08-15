import { useState } from 'react';
import type { PricingTier } from '../content/pricing/types';

/**
 * Paddle.js (v2) checkout overlay. The script is loaded lazily on first
 * click; the price id comes from the tier content (placeholder `pri_…`
 * until real Paddle prices exist). When PADDLE_CLIENT_TOKEN is unset the
 * button degrades to a mailto fallback (see website-plan.md §7).
 */
declare global {
  interface Window {
    Paddle?: {
      Setup: (opts: { token: string; environment: 'sandbox' | 'production' }) => void;
      Checkout: (opts: { items: { priceId: string; quantity: number }[] }) => void;
    };
  }
}

export default function CheckoutButton({ tier }: { tier: PricingTier }) {
  const [loading, setLoading] = useState(false);
  const token = import.meta.env.PUBLIC_PADDLE_CLIENT_TOKEN as string | undefined;
  const environment = (import.meta.env.PUBLIC_PADDLE_ENVIRONMENT as string | undefined) === 'sandbox' ? 'sandbox' : 'production';
  const priceId = tier.priceId;

  if (!token || !priceId) {
    return (
      <a
        href="mailto:sales@oz-pos.com"
        className="block w-full rounded-md border border-ink/15 px-4 py-2.5 text-center text-sm font-semibold text-ink transition hover:bg-ink/5"
      >
        {tier.cta}
      </a>
    );
  }

  const openCheckout = () => {
    setLoading(true);
    // If the Paddle script can't load (CDN blocked, offline) or the checkout
    // fails to start, fall back to the mailto so the user isn't stuck on "…".
    const mailtoFallback = () => {
      window.location.href = `mailto:sales@oz-pos.com?subject=${encodeURIComponent('OZ-POS plan: ' + tier.name)}`;
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
        window.Paddle.Checkout({ items: [{ priceId, quantity: 1 }] });
        setLoading(false);
      })
      .catch(() => {
        window.clearTimeout(timer);
        mailtoFallback();
      });
  };

  return (
    <button
      type="button"
      onClick={openCheckout}
      disabled={loading}
      className="block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
    >
      {loading ? '…' : tier.cta}
    </button>
  );
}
