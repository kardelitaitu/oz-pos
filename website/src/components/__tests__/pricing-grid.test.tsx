// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import PricingGrid from '../PricingGrid';
import type { PricingTier } from '../../content/pricing/types';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

const mockTiersUSD: PricingTier[] = [
  {
    id: 'free',
    tierKey: 'free',
    name: 'Free',
    description: 'Basic POS for small shops.',
    highlight: false,
    cta: 'Download free',
    prices: {
      monthly: { price: '$0', period: '/month' },
      yearly: { price: '$0', period: '/year' },
    },
    features: [{ label: '1 Terminal', included: true }],
  },
  {
    id: 'pro',
    tierKey: 'pro',
    name: 'Pro',
    description: 'For growing businesses.',
    highlight: true,
    cta: 'Get Pro',
    prices: {
      monthly: { price: '$9.99', period: '/month', priceId: 'pri_pro_monthly', variantPrice: '$7.99', variantPriceId: 'pri_pro_ab' },
      yearly: { price: '$99', period: '/year', priceId: 'pri_pro_yearly' },
    },
    features: [{ label: 'Unlimited Terminals', included: true }],
  },
];

const mockTiersIDR: PricingTier[] = [
  {
    id: 'free',
    tierKey: 'free',
    name: 'Gratis',
    description: 'POS dasar untuk toko kecil.',
    highlight: false,
    cta: 'Unduh gratis',
    prices: {
      monthly: { price: 'Rp 0', period: '/bulan' },
      yearly: { price: 'Rp 0', period: '/tahun' },
    },
    features: [{ label: '1 Terminal', included: true }],
  },
  {
    id: 'pro',
    tierKey: 'pro',
    name: 'Pro',
    description: 'Untuk bisnis berkembang.',
    highlight: true,
    cta: 'Pilih Pro',
    prices: {
      monthly: { price: 'Rp 99.000', period: '/bulan' },
      yearly: { price: 'Rp 990.000', period: '/tahun' },
    },
    features: [{ label: 'Terminal Tanpa Batas', included: true }],
  },
];

describe('PricingGrid Component', () => {
  beforeEach(() => {
    sessionStorage.clear();
    localStorage.clear();
  });

  async function renderGrid(
    locale = 'en',
    tiers = locale === 'id' ? mockTiersIDR : mockTiersUSD,
    tiersAlt = locale === 'id' ? mockTiersUSD : mockTiersIDR,
  ) {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(
        <PricingGrid
          locale={locale}
          tiers={tiers}
          tiersAlt={tiersAlt}
          downloadHref={`/${locale}/download`}
          contactHref={`/${locale}/support`}
        />,
      );
    });

    return {
      container,
      unmount: async () => {
        await act(async () => {
          root.unmount();
        });
        container.remove();
      },
    };
  }

  it('renders annual pricing by default ($99 for Pro)', async () => {
    const { container, unmount } = await renderGrid('en');
    expect(container.textContent).toContain('Pro');
    expect(container.textContent).toContain('$99');
    expect(container.textContent).toContain('Download free');
    await unmount();
  });

  it('switches to monthly pricing when monthly toggle is clicked', async () => {
    const { container, unmount } = await renderGrid('en');
    const monthlyBtn = Array.from(container.querySelectorAll('button')).find((b) => b.textContent?.includes('Monthly'));
    expect(monthlyBtn).toBeDefined();

    await act(async () => {
      monthlyBtn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    expect(container.textContent).toContain('$9.99');
    await unmount();
  });

  it('renders IDR pricing when locale is id', async () => {
    const { container, unmount } = await renderGrid('id', mockTiersIDR, mockTiersUSD);
    expect(container.textContent).toContain('Rp 990.000');
    expect(container.textContent).toContain('Unduh gratis');
    await unmount();
  });

  it('renders free tier download link correctly', async () => {
    const { container, unmount } = await renderGrid('en');
    const downloadLink = container.querySelector('a[href="/en/download"]');
    expect(downloadLink).not.toBeNull();
    expect(downloadLink?.textContent).toContain('Download free');
    await unmount();
  });
});

