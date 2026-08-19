import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
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

describe('giftCards.ts API contract', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('issueGiftCard calls correct command', async () => {
    const input = {
      card_number: 'GC-001',
      initial_amount_minor: 100000,
      currency: 'IDR',
      created_by: 'u1',
    };
    mockInvoke.mockResolvedValue({ card_number: 'GC-001', balance: 100000, transactions: [] });
    const result = await issueGiftCard(input);
    expect(mockInvoke).toHaveBeenCalledWith('issue_gift_card', { input });
    expect(result.card_number).toBe('GC-001');
  });

  it('getGiftCard calls correct command', async () => {
    mockInvoke.mockResolvedValue(null);
    await getGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('get_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('listGiftCards calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listGiftCards({});
    expect(mockInvoke).toHaveBeenCalledWith('list_gift_cards', { filter: {} });
  });

  it('getGiftCardBalance calls correct command', async () => {
    mockInvoke.mockResolvedValue({ balance: 50000 });
    const result = await getGiftCardBalance('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('get_gift_card_balance', { cardNumberOrId: 'GC-001' });
    expect(result?.balance).toBe(50000);
  });

  it('redeemGiftCard calls correct command', async () => {
    mockInvoke.mockResolvedValue({ remaining: 30000 });
    await redeemGiftCard('GC-001', 20000, 'sale-1');
    expect(mockInvoke).toHaveBeenCalledWith('redeem_gift_card', {
      cardNumberOrId: 'GC-001',
      amountMinor: 20000,
      saleId: 'sale-1',
    });
  });

  it('topUpGiftCard calls correct command', async () => {
    mockInvoke.mockResolvedValue({ balance: 150000 });
    await topUpGiftCard('GC-001', 50000);
    expect(mockInvoke).toHaveBeenCalledWith('top_up_gift_card', {
      cardNumberOrId: 'GC-001',
      amountMinor: 50000,
    });
  });

  it('freezeGiftCard calls correct command', async () => {
    mockInvoke.mockResolvedValue({ card_number: 'GC-001', frozen: true });
    await freezeGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('freeze_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('unfreezeGiftCard calls correct command', async () => {
    mockInvoke.mockResolvedValue({ card_number: 'GC-001', frozen: false });
    await unfreezeGiftCard('GC-001');
    expect(mockInvoke).toHaveBeenCalledWith('unfreeze_gift_card', { cardNumberOrId: 'GC-001' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('card not found'));
    await expect(getGiftCard('bad')).rejects.toThrow('card not found');
  });
});
