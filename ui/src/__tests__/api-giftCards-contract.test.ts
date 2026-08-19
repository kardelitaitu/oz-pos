// ── IPC contract tests for giftCards.ts ────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  issueGiftCard,
  getGiftCard,
  listGiftCards,
  getGiftCardBalance,
  redeemGiftCard,
  topUpGiftCard,
  freezeGiftCard,
  unfreezeGiftCard,
} from '@/api/giftCards';

describe('giftCards.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('issueGiftCard → issue_gift_card with input', async () => {
    mockInvoke.mockResolvedValue({ id: 'gc1', cardNumber: 'GC-001', balance: 50000 });
    await issueGiftCard({ initialBalanceMinor: 50000, currency: 'USD', note: 'Gift' });
    expect(mockInvoke).toHaveBeenCalledWith('issue_gift_card', { input: expect.objectContaining({ initialBalanceMinor: 50000 }) });
  });

  it('getGiftCard → get_gift_card with cardNumberOrId', async () => {
    mockInvoke.mockResolvedValue(null);
    await getGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('get_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('listGiftCards → list_gift_cards with filter', async () => {
    mockInvoke.mockResolvedValue([]);
    await listGiftCards({ status: 'active' });
    expect(mockInvoke).toHaveBeenCalledWith('list_gift_cards', { filter: { status: 'active' } });
  });

  it('getGiftCardBalance → get_gift_card_balance with cardNumberOrId', async () => {
    mockInvoke.mockResolvedValue({ balance: 50000, currency: 'USD' });
    await getGiftCardBalance('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('get_gift_card_balance', { cardNumberOrId: 'GC-001' });
  });

  it('redeemGiftCard → redeem_gift_card with cardNumberOrId + amountMinor + saleId', async () => {
    mockInvoke.mockResolvedValue({ success: true, remainingBalance: 40000 });
    await redeemGiftCard('GC-001', 10000, 'sale-1');
    expect(mockInvoke).toHaveBeenCalledWith('redeem_gift_card', { cardNumberOrId: 'GC-001', amountMinor: 10000, saleId: 'sale-1' });
  });

  it('topUpGiftCard → top_up_gift_card with cardNumberOrId + amountMinor', async () => {
    mockInvoke.mockResolvedValue({ balance: 60000 });
    await topUpGiftCard('GC-001', 10000);
    expect(mockInvoke).toHaveBeenCalledWith('top_up_gift_card', { cardNumberOrId: 'GC-001', amountMinor: 10000 });
  });

  it('freezeGiftCard → freeze_gift_card with cardNumberOrId', async () => {
    mockInvoke.mockResolvedValue({ id: 'gc1', status: 'frozen' });
    await freezeGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('freeze_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('unfreezeGiftCard → unfreeze_gift_card with cardNumberOrId', async () => {
    mockInvoke.mockResolvedValue({ id: 'gc1', status: 'active' });
    await unfreezeGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('unfreeze_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('card not found'));
    await expect(getGiftCard('MISSING')).rejects.toThrow('card not found');
  });
});
