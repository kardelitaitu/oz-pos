import type { FeatureRow, PricingTier } from './types';

// Prices are per locale; this file is the global/USD pricing (en locale).
//
// Tier lineup follows subscription-tiers.md (FINAL 2026-08-17): Free · Plus ·
// Pro ⭐ · Premium · Enterprise, with USD and IDR as independent market
// prices. Yearly = 2 months free (10 months paid, 12 granted) and is the
// DEFAULT selection on the pricing page — marketed as "2 months free", never
// as a percentage discount.
//
// Paddle: the six real prices (Plus/Pro/Premium × monthly/yearly) do not
// exist yet — PADDLE_PRICE_TIERS still maps only the two legacy sandbox
// prices (pro = pri_01m05gdnqp30xze6db73qcracp $19/mo, premium =
// pri_01m05gdpk4hmnm0k8e6vxm8cec $49/mo), which this lineup supersedes. Until
// the new catalog is live, paid tiers carry placeholder ids
// (pri_placeholder_…) so checkout degrades to the mailto fallback instead of
// charging the old (wrong) amounts. Swap in the six real price ids once the
// catalog lands (see apps/license-server PADDLE_PRICE_TIERS).
export const pricing: PricingTier[] = [
  {
    id: 'free',
    tierKey: 'free',
    name: 'Free',
    currency: 'USD',
    description: 'Free forever — run one store, fully offline.',
    cta: 'Download free',
    prices: {
      monthly: { price: '$0', period: 'free forever' },
      yearly: { price: '$0', period: 'free forever' },
    },
    features: [
      { label: '1 store', included: true },
      { label: '1 register', included: true },
      { label: '1 warehouse', included: true },
      { label: '30-day sales history', included: true },
      { label: 'QRIS payments', included: false },
      { label: 'Cloud sync', included: false },
    ],
  },
  {
    id: 'plus',
    tierKey: 'plus',
    name: 'Plus',
    currency: 'USD',
    description: 'The entry plan for single-store shops ready to grow.',
    cta: 'Choose Plus',
    prices: {
      monthly: { price: '$4.99', period: '/month', priceId: 'pri_placeholder_plus_monthly_usd' },
      yearly: { price: '$49.99', period: '/year', priceId: 'pri_placeholder_plus_yearly_usd' },
    },
    // Restaurant Starter bundle (C3.2, subscription-tiers.md §5): Plus +
    // KDS at 10% off à la carte. PLACEHOLDER prices (base Plus + KDS
    // add-on, 10% off) — swap for the catalog figures when the six real
    // prices land; the ids are placeholders that degrade to the mailto
    // fallback until then.
    bundle: {
      id: 'restaurant_starter',
      label: 'Restaurant Starter bundle',
      note: 'Plus + Kitchen Display (KDS) — 10% off à la carte',
      prices: {
        monthly: { price: '$7.49', period: '/month', priceId: 'pri_placeholder_plus_bundle_monthly' },
        yearly: { price: '$74.99', period: '/year', priceId: 'pri_placeholder_plus_bundle_yearly' },
      },
    },
    features: [
      { label: '1 store', included: true },
      { label: '2 registers', included: true },
      { label: '2 warehouses', included: true },
      { label: 'QRIS payments', included: true },
      { label: 'Daily Sales Dashboard', included: true },
      { label: 'Cloud sync', included: true },
    ],
  },
  {
    id: 'pro',
    tierKey: 'pro',
    name: 'Pro',
    currency: 'USD',
    description: 'For growing businesses — analytics, KDS, and multi-terminal.',
    cta: 'Choose Pro',
    highlight: true,
    prices: {
      monthly: { price: '$9.99', period: '/month', priceId: 'pri_placeholder_pro_monthly_usd' },
      yearly: { price: '$99.99', period: '/year', priceId: 'pri_placeholder_pro_yearly_usd' },
    },
    features: [
      { label: '2 stores', included: true },
      { label: '5 registers per store', included: true },
      { label: 'Kitchen display (KDS)', included: true },
      { label: 'Reports & analytics', included: true },
      { label: 'Stripe cards', included: true },
      { label: 'Cloud sync', included: true },
    ],
  },
  {
    id: 'premium',
    tierKey: 'premium',
    name: 'Premium',
    currency: 'USD',
    description: 'For multi-store chains — loyalty and automation.',
    cta: 'Choose Premium',
    prices: {
      monthly: { price: '$19.99', period: '/month', priceId: 'pri_placeholder_premium_monthly_usd' },
      yearly: { price: '$199.99', period: '/year', priceId: 'pri_placeholder_premium_yearly_usd' },
    },
    features: [
      { label: 'Unlimited stores', included: true },
      { label: 'Unlimited registers', included: true },
      { label: 'Loyalty program', included: true },
      { label: 'Scheduled report emails', included: true },
      { label: 'Lua scripting', included: true },
      { label: 'Priority support (1h)', included: true },
    ],
  },
  {
    id: 'enterprise',
    tierKey: 'enterprise',
    name: 'Enterprise',
    currency: 'USD',
    description: 'White-label, custom hardware, and a dedicated account manager.',
    cta: 'Contact us',
    prices: {
      monthly: { price: 'Custom', period: '' },
      yearly: { price: 'Custom', period: '' },
    },
    features: [
      { label: 'Unlimited stores', included: true },
      { label: 'Unlimited registers', included: true },
      { label: 'White-label branding', included: true },
      { label: 'Custom HAL drivers', included: true },
      { label: 'Dedicated account manager', included: true },
      { label: 'Custom support SLA', included: true },
    ],
  },
];

// Mirrors the quota & feature matrix in subscription-tiers.md §3.
export const featureRows: FeatureRow[] = [
  { label: 'Stores', values: { free: 1, plus: 1, pro: 2, premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'Terminals (registers) per store', values: { free: 1, plus: 2, pro: 5, premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'Warehouses', values: { free: 1, plus: 2, pro: 3, premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'KDS screens', values: { free: 0, plus: 0, pro: '1 per store', premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'Staff users', values: { free: 1, plus: 5, pro: 20, premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'Sales history', values: { free: '30 days', plus: 'Unlimited', pro: 'Unlimited', premium: 'Unlimited', enterprise: 'Unlimited' } },
  { label: 'QRIS payments', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Stripe cards', values: { free: false, plus: false, pro: true, premium: true, enterprise: true } },
  { label: 'Cloud sync', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Daily Sales Dashboard', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Reports & analytics', values: { free: false, plus: false, pro: true, premium: true, enterprise: true } },
  { label: 'Scheduled report emails', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Loyalty program', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Lua scripting', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Priority support', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Offline grace period', values: { free: '7 days', plus: '14 days', pro: '14 days', premium: '30 days', enterprise: 'Custom' } },
];
