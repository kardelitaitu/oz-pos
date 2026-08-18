import { useEffect, useState } from 'react';
import type { BillingPeriod, CheckoutTier, PricingTier } from '../content/pricing/types';
import { t } from '../i18n';
import CheckoutButton from './CheckoutButton';

/**
 * Pricing grid (pricing.astro). Renders the five tiers from
 * subscription-tiers.md (Free · Plus · Pro ⭐ · Premium · Enterprise) with a
 * monthly/yearly toggle.
 *
 * Annual is the DEFAULT selection (§2): yearly = 2 months free (pay 10
 * months, get 12), marketed as "2 months free" / "2 bulan gratis" — never as
 * a percentage discount — and users actively switch to monthly.
 *
 * Checkout is register-first (website-plan.md §5): paid cards go through
 * CheckoutButton (redirects to /login until a session exists); the Free
 * (free-forever) card links to the download page; Enterprise has no price id
 * and falls through to the mailto contact fallback inside CheckoutButton.
 */
interface Props {
  tiers: PricingTier[];
  locale: string;
  downloadHref: string;
  contactHref: string;
}

export default function PricingGrid({ tiers, locale, downloadHref }: Props) {
  const [billing, setBilling] = useState<BillingPeriod>('yearly');
  // Vertical landing pages deep-link `pricing?bundle=restaurant_starter#plus`
  // (C3.2): pre-enable the Plus card's bundle toggle from the URL. SSR-safe:
  // window only exists after hydration (the grid mounts with client:load).
  const [bundleOn, setBundleOn] = useState(false);
  // C4.1: A/B test — ?ab=pro_price activates the $7.99 variant for Pro monthly
  const [abVariant, setAbVariant] = useState<string | null>(null);
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    if (params.get('bundle') === 'restaurant_starter') setBundleOn(true);
    const ab = params.get('ab');
    if (ab) setAbVariant(ab);
  }, []);

  const buttonClass = (active: boolean) =>
    [
      'rounded-full px-4 py-1.5 font-semibold transition',
      active ? 'bg-accent text-black' : 'text-muted hover:text-ink',
    ].join(' ');

  return (
    <>
      <div className="mt-12 flex justify-center">
        <div
          role="group"
          aria-label={t(locale, 'pricingPage.billing.label')}
          className="inline-flex items-center rounded-full border border-ink/10 bg-surface/40 p-1 text-sm"
        >
          <button
            type="button"
            onClick={() => setBilling('monthly')}
            aria-pressed={billing === 'monthly'}
            className={buttonClass(billing === 'monthly')}
          >
            {t(locale, 'pricingPage.billing.monthly')}
          </button>
          <button
            type="button"
            onClick={() => setBilling('yearly')}
            aria-pressed={billing === 'yearly'}
            className={buttonClass(billing === 'yearly')}
          >
            {t(locale, 'pricingPage.billing.yearly')}
            <span
              className={[
                'ml-1.5 rounded-full px-2 py-0.5 text-xs font-semibold',
                billing === 'yearly' ? 'bg-black/15 text-black' : 'bg-accent/15 text-link',
              ].join(' ')}
            >
              {t(locale, 'pricingPage.billing.yearlyNote')}
            </span>
          </button>
        </div>
      </div>

      <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
        {tiers.map((tier) => {
          const isFree = tier.tierKey === 'free';
          const isEnterprise = tier.tierKey === 'enterprise';
          // The bundle toggle (C3.2) swaps the Plus card's price + checkout
          // to the Restaurant Starter bundle: the checkout then carries
          // bundle='restaurant_starter' and the webhook mints the
          // bundle-widened quota block (kds at Plus).
          const bundleActive = Boolean(tier.bundle && bundleOn);
          let price = bundleActive && tier.bundle ? tier.bundle.prices[billing] : tier.prices[billing];
          // C4.1: A/B test override — when ?ab=pro_price is set and this tier
          // has a variantPriceId, swap the price + priceId for the variant.
          let abActive = false;
          if (abVariant === 'pro_price' && price.variantPriceId && billing === 'monthly') {
            price = { ...price, priceId: price.variantPriceId, price: price.variantPrice ?? price.price };
            abActive = true;
          }
          const checkoutTier: CheckoutTier = {
            tierKey: tier.tierKey,
            name: tier.name,
            cta: tier.cta,
            period: billing,
            priceId: price.priceId,
            bundle: bundleActive && tier.bundle ? tier.bundle.id : undefined,
            abVariant: abActive ? abVariant : undefined,
          };
          return (
            <article
              key={tier.id}
              id={tier.id}
              className={[
                'flex scroll-mt-24 flex-col rounded-xl border p-6',
                tier.highlight ? 'border-accent bg-surface' : 'border-ink/10 bg-surface/40',
              ].join(' ')}
            >
              {tier.highlight && (
                <span className="mb-3 inline-flex w-fit items-center rounded-full bg-accent px-2.5 py-0.5 text-xs font-semibold text-black">
                  {t(locale, 'pricingPage.mostPopular')}
                </span>
              )}
              <h3 className="text-lg font-semibold">{tier.name}</h3>
              <p className="mt-3 text-3xl font-bold">
                {price.price}
                {price.period && <span className="text-sm font-normal text-muted"> {price.period}</span>}
              </p>
              {billing === 'yearly' && !isFree && !isEnterprise && (
                <p className="mt-1 text-xs text-muted">{t(locale, 'pricingPage.billing.billedYearly')}</p>
              )}
              <p className="mt-2 text-sm text-muted">{tier.description}</p>

              {tier.bundle && (
                <label className="mt-4 flex cursor-pointer items-start gap-2 rounded-md border border-ink/10 bg-surface/40 p-3 text-xs">
                  <input
                    type="checkbox"
                    checked={bundleOn}
                    onChange={(e) => setBundleOn(e.target.checked)}
                    className="mt-0.5 accent-accent"
                  />
                  <span>
                    <span className="font-semibold text-ink">{tier.bundle.label}</span>
                    <span className="mt-0.5 block text-muted">{tier.bundle.note}</span>
                  </span>
                </label>
              )}

              <div className="mt-6">
                {isFree ? (
                  <a
                    href={downloadHref}
                    className="block w-full rounded-md bg-accent px-4 py-2.5 text-center text-sm font-semibold text-black transition hover:opacity-90"
                  >
                    {tier.cta}
                  </a>
                ) : (
                  <CheckoutButton tier={checkoutTier} locale={locale} />
                )}
              </div>

              <ul className="mt-6 space-y-2 text-sm">
                {tier.features.map((f) => (
                  <li key={f.label} className={['flex items-center gap-2', f.included ? '' : 'text-muted'].join(' ')}>
                    <span aria-hidden="true" className={f.included ? 'text-link' : 'text-muted/50'}>
                      {f.included ? '✓' : '✗'}
                    </span>
                    {f.label}
                  </li>
                ))}
              </ul>
            </article>
          );
        })}
      </div>
    </>
  );
}
