import type { FeatureRow, PricingTier } from './types';

// Prices are per locale; this file is the Indonesian pricing (id locale).
//
// Tier lineup follows subscription-tiers.md (FINAL 2026-08-17): Gratis ·
// Plus · Pro ⭐ · Premium · Enterprise, with USD and IDR as independent
// market prices (IDR is the lower local-market rate). Yearly = 2 bulan
// gratis (bayar 10 bulan, dapat 12) and is the DEFAULT selection on the
// pricing page — marketed as "2 bulan gratis", never as a percentage.
//
// NOTE: Paddle does not support IDR as a billing currency (their supported
// list has no IDR), so today the checkout charges the USD price id and the
// Rp figures on this page are the display price — the checkout shows the
// USD amount. True fixed-Rp billing needs the local provider phase
// (Midtrans — see subscription-tiers.md §2 Payment routing), not Paddle.
// Until the six new Paddle prices are catalogued, paid tiers carry
// placeholder ids (pri_placeholder_…) so checkout degrades to the mailto
// fallback instead of charging the old (wrong) amounts.
export const pricing: PricingTier[] = [
  {
    id: 'free',
    tierKey: 'free',
    name: 'Gratis',
    currency: 'IDR',
    description: 'Gratis selamanya — jalankan satu toko, sepenuhnya offline.',
    cta: 'Unduh Gratis',
    prices: {
      monthly: { price: 'Rp 0', period: 'gratis selamanya' },
      yearly: { price: 'Rp 0', period: 'gratis selamanya' },
    },
    features: [
      { label: '1 toko', included: true },
      { label: '1 register', included: true },
      { label: '1 gudang', included: true },
      { label: 'Riwayat penjualan 3 bulan', included: true },
      { label: 'Pembayaran QRIS', included: false },
      { label: 'Sinkron cloud', included: false },
    ],
  },
  {
    id: 'plus',
    tierKey: 'plus',
    name: 'Plus',
    currency: 'IDR',
    description: 'Paket awal untuk toko tunggal yang siap berkembang.',
    cta: 'Pilih Plus',
    prices: {
      monthly: { price: 'Rp 49.000', period: '/bulan', priceId: 'pri_placeholder_plus_monthly_usd' },
      yearly: { price: 'Rp 500.000', period: '/tahun', priceId: 'pri_placeholder_plus_yearly_usd' },
    },
    features: [
      { label: '1 toko', included: true },
      { label: '2 register', included: true },
      { label: '2 gudang', included: true },
      { label: 'Pembayaran QRIS', included: true },
      { label: 'Dasbor Penjualan Harian', included: true },
      { label: 'Sinkron cloud', included: true },
    ],
  },
  {
    id: 'pro',
    tierKey: 'pro',
    name: 'Pro',
    currency: 'IDR',
    description: 'Untuk bisnis berkembang — analitik, Display Dapur, dan multi-terminal.',
    cta: 'Pilih Pro',
    highlight: true,
    prices: {
      monthly: {
        price: 'Rp 99.000', period: '/bulan',
        priceId: 'pri_placeholder_pro_monthly_usd',
        // C4.1: A/B variant — Rp 79.000 vs Rp 99.000 (controlled by ?ab=pro_price)
        variantPriceId: 'pri_pro_monthly_usd_variant_799',
        variantPrice: 'Rp 79.000',
      },
      yearly: { price: 'Rp 1.000.000', period: '/tahun', priceId: 'pri_placeholder_pro_yearly_usd' },
    },
    features: [
      { label: '2 toko', included: true },
      { label: '5 register per toko', included: true },
      { label: '2 Display Dapur', included: true },
      { label: 'Laporan & analitik', included: true },
      { label: 'Kartu Stripe', included: true },
      { label: 'Sinkron cloud', included: true },
    ],
  },
  {
    id: 'premium',
    tierKey: 'premium',
    name: 'Premium',
    currency: 'IDR',
    description: 'Untuk jaringan multi-toko — loyalitas dan otomatisasi.',
    cta: 'Pilih Premium',
    prices: {
      monthly: { price: 'Rp 399.000', period: '/bulan', priceId: 'pri_placeholder_premium_monthly_usd' },
      yearly: { price: 'Rp 3.999.000', period: '/tahun', priceId: 'pri_placeholder_premium_yearly_usd' },
    },
    features: [
      { label: '5 toko', included: true },
      { label: 'Register tanpa batas', included: true },
      { label: 'Program loyalitas', included: true },
      { label: 'Email laporan terjadwal', included: true },
      { label: 'Skrip Lua', included: true },
      { label: 'Dukungan prioritas (1 jam)', included: true },
    ],
  },
  {
    id: 'enterprise',
    tierKey: 'enterprise',
    name: 'Enterprise',
    currency: 'IDR',
    description: 'White-label, perangkat keras khusus, dan account manager khusus.',
    cta: 'Hubungi kami',
    prices: {
      monthly: { price: 'Kustom', period: '' },
      yearly: { price: 'Kustom', period: '' },
    },
    features: [
      { label: 'Toko tanpa batas', included: true },
      { label: 'Register tanpa batas', included: true },
      { label: 'Branding white-label', included: true },
      { label: 'Driver HAL khusus', included: true },
      { label: 'Account manager khusus', included: true },
      { label: 'SLA dukungan khusus', included: true },
    ],
  },
];

// Mirrors the quota & feature matrix in subscription-tiers.md §3.
export const featureRows: FeatureRow[] = [
  { label: 'Toko', values: { free: 1, plus: 1, pro: 2, premium: 5, enterprise: 'Tanpa batas' } },
  { label: 'Terminal (register) per toko', values: { free: 1, plus: 2, pro: 5, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Gudang', values: { free: 1, plus: 2, pro: 3, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Layar Display Dapur', values: { free: 0, plus: 0, pro: 2, premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Max produk/menu', values: { free: 200, plus: 500, pro: 1000, premium: 10000, enterprise: 'Tanpa batas' } },
  { label: 'Staf pengguna', values: { free: 1, plus: 5, pro: 20, premium: 50, enterprise: 'Tanpa batas' } },
  { label: 'Riwayat penjualan', values: { free: '3 bulan', plus: '1 tahun', pro: '5 tahun', premium: 'Tanpa batas', enterprise: 'Tanpa batas' } },
  { label: 'Pembayaran QRIS', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Kartu Stripe', values: { free: false, plus: false, pro: true, premium: true, enterprise: true } },
  { label: 'Sinkron cloud', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Dasbor Penjualan Harian', values: { free: false, plus: true, pro: true, premium: true, enterprise: true } },
  { label: 'Laporan & analitik', values: { free: false, plus: false, pro: true, premium: true, enterprise: true } },
  { label: 'Email laporan terjadwal', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Program loyalitas', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Skrip Lua', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Dukungan prioritas', values: { free: false, plus: false, pro: false, premium: true, enterprise: true } },
  { label: 'Masa tenggang offline', values: { free: '7 hari', plus: '14 hari', pro: '14 hari', premium: '30 hari', enterprise: 'Kustom' } },
];
