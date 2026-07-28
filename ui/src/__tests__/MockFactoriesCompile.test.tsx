// ── Mock-factory compilation smoke tests ──────────────────────────
//
// Verifies that each mock factory in `mocks/api.ts` can be used with
// `vi.mock()` and returns sensible default values matching the real
// API module's types.  Each test imports the factory, wires it into
// `vi.mock()`, calls the real API function, and checks the default
// shape.
//
// These are NOT behaviour tests — they are compile-time + basic-shape
// smoke tests.  Real feature tests live in the per-screen test files
// (e.g. KdsScreen.test.tsx, GiftCardsScreen.test.tsx).

import { describe, it, expect, vi } from 'vitest';
import { createKdsApiMock, createGiftCardsApiMock, createLoyaltyApiMock, createReportsApiMock } from '@/__tests__/test-utils/mocks/api';

// ── KDS ─────────────────────────────────────────────────────────

vi.mock('@/api/kds', () => createKdsApiMock());

import * as kdsApi from '@/api/kds';

describe('createKdsApiMock', () => {
  it('returns a KdsOrder array from listKdsOrders', async () => {
    const orders = await kdsApi.listKdsOrders('user-1');
    expect(Array.isArray(orders)).toBe(true);
    expect(orders[0]?.id).toBe('kds-1');
    expect(orders[0]?.status).toBe('pending');
    expect(orders[0]?.item_count).toBe(2);
  });

  it('returns a KdsOrder from updateKdsStatus', async () => {
    const order = await kdsApi.updateKdsStatus('user-1', 'kds-1', 'preparing');
    expect(order.status).toBe('preparing');
    expect(order.display_number).toBe(101);
  });

  it('returns a KdsOrder from getKdsOrder', async () => {
    const order = await kdsApi.getKdsOrder('user-1', 'kds-1');
    expect(order).not.toBeNull();
    expect(order!.id).toBe('kds-1');
  });

});

// ── Gift Cards ──────────────────────────────────────────────────

vi.mock('@/api/giftCards', () => createGiftCardsApiMock());

import * as gcApi from '@/api/giftCards';

describe('createGiftCardsApiMock', () => {
  it('returns GiftCardWithTransactions from issueGiftCard', async () => {
    const result = await gcApi.issueGiftCard({
      card_number: '1111-2222',
      initial_amount_minor: 50000,
      currency: 'IDR',
      created_by: 'user-1',
    });
    expect(result.card.id).toBe('gc-1');
    expect(result.card.status).toBe('active');
    expect(Array.isArray(result.transactions)).toBe(true);
  });

  it('returns GiftCardWithTransactions from getGiftCard', async () => {
    const result = await gcApi.getGiftCard('1111-2222');
    expect(result).not.toBeNull();
    expect(result!.card.card_number).toBe('1234-5678-9012-3456');
  });

  it('returns a GiftCard array from listGiftCards', async () => {
    const cards = await gcApi.listGiftCards({});
    expect(Array.isArray(cards)).toBe(true);
  });

  it('returns BalanceResult from getGiftCardBalance', async () => {
    const balance = await gcApi.getGiftCardBalance('1111-2222');
    expect(balance).not.toBeNull();
    expect(balance!.balance_minor).toBe(75000);
    expect(balance!.currency).toBe('IDR');
  });

  it('returns RedeemGiftCardResult from redeemGiftCard', async () => {
    const result = await gcApi.redeemGiftCard('1111-2222', 25000, 'sale-1');
    expect(result.card.id).toBe('gc-1');
    expect(result.transaction.txn_type).toBe('redeem');
  });
});

// ── Loyalty ─────────────────────────────────────────────────────

vi.mock('@/api/loyalty', () => createLoyaltyApiMock());

import * as loyApi from '@/api/loyalty';

describe('createLoyaltyApiMock', () => {
  it('returns LoyaltyAccountWithDetails from getLoyaltyAccount', async () => {
    const account = await loyApi.getLoyaltyAccount('cust-1');
    expect(account).not.toBeNull();
    expect(account!.account.customer_id).toBe('cust-1');
    expect(account!.account.points).toBe(500);
    expect(account!.tier).not.toBeNull();
    expect(account!.tier!.name).toBe('Silver');
  });

  it('returns loyalty accounts list from listLoyaltyAccounts', async () => {
    const accounts = await loyApi.listLoyaltyAccounts();
    expect(Array.isArray(accounts)).toBe(true);
    expect(accounts[0]?.account.id).toBe('loyalty-1');
  });

  it('returns a LoyaltyTransaction from earnLoyaltyPoints', async () => {
    const txn = await loyApi.earnLoyaltyPoints('cust-1', 'sale-1', 50000);
    expect(txn.txn_type).toBe('earn');
    expect(txn.points).toBe(100);
  });

  it('returns RedeemResult from redeemLoyaltyPoints', async () => {
    const result = await loyApi.redeemLoyaltyPoints('cust-1', 200, 'sale-1');
    expect(result.transaction.txn_type).toBe('redeem');
    expect(result.discount_minor).toBe(50000);
  });

  it('returns tiers from listLoyaltyTiers', async () => {
    const tiers = await loyApi.listLoyaltyTiers();
    expect(Array.isArray(tiers)).toBe(true);
    expect(tiers[0]?.name).toBe('Silver');
  });

  it('returns points value from getPointsValue', async () => {
    const value = await loyApi.getPointsValue(100);
    expect(value).toBe(25000);
  });
});

// ── Reports ─────────────────────────────────────────────────────

vi.mock('@/api/reports', () => createReportsApiMock());

import * as rptApi from '@/api/reports';

describe('createReportsApiMock', () => {
  it('returns DailyRevenueRow[] from getDailyRevenue', async () => {
    const rows = await rptApi.getDailyRevenue('2026-07-01', '2026-07-31');
    expect(Array.isArray(rows)).toBe(true);
    expect(rows[0]?.total_minor).toBe(1250000);
    expect(rows[0]?.currency).toBe('IDR');
  });

  it('returns WeeklyRevenueRow[] from getWeeklyRevenue', async () => {
    const rows = await rptApi.getWeeklyRevenue('2026-07-01', '2026-07-31');
    expect(Array.isArray(rows)).toBe(true);
    expect(rows[0]?.total_minor).toBe(8500000);
  });

  it('returns TopProductRow[] from getTopProducts', async () => {
    const rows = await rptApi.getTopProducts('2026-07-01', '2026-07-31', 10);
    expect(Array.isArray(rows)).toBe(true);
    expect(rows[0]?.sku).toBe('SKU-001');
  });

  it('returns HourlyHeatmapRow[] from getHourlyHeatmap', async () => {
    const rows = await rptApi.getHourlyHeatmap('2026-07-01', '2026-07-31');
    expect(Array.isArray(rows)).toBe(true);
    expect(rows[0]?.hour).toBe(10);
  });

  it('returns LowStockAlert[] from getLowStockAlerts', async () => {
    const alerts = await rptApi.getLowStockAlerts(5);
    expect(Array.isArray(alerts)).toBe(true);
    expect(alerts).toEqual([]);
  });

  it('returns MenuEngineeringResult from getMenuEngineering', async () => {
    const result = await rptApi.getMenuEngineering('2026-07-01', '2026-07-31');
    expect(result.rows).toBeDefined();
    expect(result.median_volume).toBe(25);
    expect(result.median_margin).toBe(5000);
    expect(result.rows[0]?.sku).toBe('SKU-001');
  });

  it('returns CustomReportResponse from buildCustomReport', async () => {
    const result = await rptApi.buildCustomReport({
      dataset: 'sales',
      columns: ['SKU', 'Name'],
      start_date: null,
      end_date: null,
    });
    expect(result.columns).toEqual(['SKU', 'Name']);
    expect(result.rows[0]).toEqual(['SKU-001', 'Test']);
  });
});

// ── Existing factories import check (compile guard) ─────────────

describe('existing factory imports compile', () => {
  // Dynamic import to prevent hoisting conflicts with the vi.mock() calls above.
  it('imports all factories without error', async () => {
    const mod = await import('@/__tests__/test-utils/mocks/api');
    expect(typeof mod.createSalesApiMock).toBe('function');
    expect(typeof mod.createSettingsApiMock).toBe('function');
    expect(typeof mod.createShiftsApiMock).toBe('function');
    expect(typeof mod.createHardwareApiMock).toBe('function');
    expect(typeof mod.createProductsApiMock).toBe('function');
    expect(typeof mod.createKdsApiMock).toBe('function');
    expect(typeof mod.createGiftCardsApiMock).toBe('function');
    expect(typeof mod.createLoyaltyApiMock).toBe('function');
    expect(typeof mod.createReportsApiMock).toBe('function');
  });
});
