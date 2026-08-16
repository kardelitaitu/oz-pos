import type { FeatureRow, PricingTier } from './types';

// Prices are per locale; this file is the Indonesian pricing (id locale).
// NOTE: Paddle does not support IDR as a billing currency (their supported
// list has no IDR), so the checkout charges the USD price id below and the
// Rp figures on this page are the display price. Rp 299.000 ≈ $19,
// Rp 749.000 ≈ $49 — the checkout shows the USD amount. If true IDR billing
// is required, it needs a local provider (e.g. Midtrans/Xendit), not Paddle.
// Real Paddle sandbox prices: pro = pri_01m05gdnqp30xze6db73qcracp
// ($19/mo), premium = pri_01m05gdpk4hmnm0k8e6vxm8cec ($49/mo).
export const pricing: PricingTier[] = [
  {
    id: 'trial',
    tierKey: 'trial',
    name: 'Gratis',
    currency: 'IDR',
    price: 'Rp 0',
    period: 'uji coba 90 hari',
    description: 'Semua yang Anda butuhkan untuk satu toko — sepenuhnya offline.',
    cta: 'Mulai Uji Coba Gratis',
    features: [
      { label: '1 toko', included: true },
      { label: '1 register', included: true },
      { label: '1 gudang', included: true },
      { label: 'Pembayaran QRIS', included: false },
      { label: 'Sinkron cloud', included: false },
      { label: 'Skrip Lua', included: false },
    ],
  },
  {
    id: 'pro',
    tierKey: 'pro',
    name: 'Pro',
    currency: 'IDR',
    price: 'Rp 299.000',
    period: '/bulan',
    description: 'Untuk toko berkembang yang ingin sinkron cloud dan QRIS.',
    cta: 'Pilih Pro',
    highlight: true,
    priceId: 'pri_01m05gdnqp30xze6db73qcracp',
    features: [
      { label: '1 toko', included: true },
      { label: '2 register', included: true },
      { label: '1 gudang', included: true },
      { label: 'Pembayaran QRIS', included: true },
      { label: 'Sinkron cloud', included: true },
      { label: 'Skrip Lua', included: false },
    ],
  },
  {
    id: 'premium',
    tierKey: 'premium',
    name: 'Premium',
    currency: 'IDR',
    price: 'Rp 749.000',
    period: '/bulan',
    description: 'Toko, register, dan otomatisasi tanpa batas.',
    cta: 'Pilih Premium',
    priceId: 'pri_01m05gdpk4hmnm0k8e6vxm8cec',
    features: [
      { label: 'Toko tanpa batas', included: true },
      { label: 'Register tanpa batas', included: true },
      { label: 'Gudang tanpa batas', included: true },
      { label: 'Pembayaran QRIS', included: true },
      { label: 'Sinkron cloud', included: true },
      { label: 'Skrip Lua', included: true },
    ],
  },
  {
    id: 'enterprise',
    tierKey: 'enterprise',
    name: 'Enterprise',
    currency: 'IDR',
    price: 'Kustom',
    period: '',
    description: 'Batas yang disesuaikan, onboarding, dan dukungan prioritas.',
    cta: 'Hubungi kami',
    features: [
      { label: 'Toko tanpa batas', included: true },
      { label: 'Register tanpa batas', included: true },
      { label: 'Gudang tanpa batas', included: true },
      { label: 'Pembayaran QRIS', included: true },
      { label: 'Sinkron cloud', included: true },
      { label: 'Skrip Lua', included: true },
    ],
  },
];

export const featureRows: FeatureRow[] = [
  { label: 'Durasi', values: { trial: '90 hari', pro: 'Bulanan', premium: 'Bulanan', enterprise: 'Kustom' } },
  { label: 'Toko', values: { trial: 1, pro: 1, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Register', values: { trial: 1, pro: 2, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Gudang', values: { trial: 1, pro: 1, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Pembayaran QRIS', values: { trial: false, pro: true, premium: true, enterprise: true } },
  { label: 'Sinkron cloud', values: { trial: false, pro: true, premium: true, enterprise: true } },
  { label: 'Skrip Lua', values: { trial: false, pro: false, premium: true, enterprise: true } },
  { label: 'Dukungan prioritas', values: { trial: false, pro: false, premium: true, enterprise: true } },
];
