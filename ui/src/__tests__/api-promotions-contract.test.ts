import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  createPromotion,
  createPromotionScoped,
  listPromotions,
  listPromotionsScoped,
  updatePromotion,
  deletePromotion,
  applyPromotion,
  applyPromotionScoped,
} from '@/api/promotions';

describe('promotions.ts API contract', () => {
  const TOKEN = 'tok_promo';
  const USER_ID = 'u1';

  const promo = {
    id: 'promo1', name: 'Summer Sale', description: 'Big sale', promo_type: 'percentage',
    value_minor: 1000, min_qty: null, trigger_sku: null, reward_sku: null, reward_qty: null,
    starts_at: null, ends_at: null, min_order_minor: 0, category_id: null,
    active: true, created_at: '2026-01-01', updated_at: '2026-01-01',
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('createPromotion calls correct command', async () => {
    const args = { name: 'Summer Sale', promo_type: 'percentage', value_minor: 1000 };
    mockInvoke.mockResolvedValue(promo);
    const result = await createPromotion(USER_ID, args);
    expect(mockInvoke).toHaveBeenCalledWith('create_promotion', { userId: USER_ID, args });
    expect(result.id).toBe('promo1');
  });

  it('createPromotionScoped calls correct command', async () => {
    const args = { name: 'Scoped Promo', promo_type: 'fixed', value_minor: 5000 };
    mockInvoke.mockResolvedValue({ ...promo, name: 'Scoped Promo' });
    await createPromotionScoped(TOKEN, args);
    expect(mockInvoke).toHaveBeenCalledWith('create_promotion_scoped', {
      sessionToken: TOKEN,
      args,
    });
  });

  it('listPromotions calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPromotions();
    expect(mockInvoke).toHaveBeenCalledWith('list_promotions');
  });

  it('listPromotionsScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPromotionsScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('list_promotions_scoped', {
      sessionToken: TOKEN,
    });
  });

  it('updatePromotion calls correct command', async () => {
    mockInvoke.mockResolvedValue(promo);
    await updatePromotion(USER_ID, promo);
    expect(mockInvoke).toHaveBeenCalledWith('update_promotion', { userId: USER_ID, promotion: promo });
  });

  it('deletePromotion calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deletePromotion(USER_ID, 'promo1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_promotion', { userId: USER_ID, id: 'promo1' });
  });

  it('applyPromotion calls correct command', async () => {
    mockInvoke.mockResolvedValue({ discount: 5000 });
    await applyPromotion(USER_ID, 'sale-1', 'promo1');
    expect(mockInvoke).toHaveBeenCalledWith('apply_promotion', {
      userId: USER_ID,
      saleId: 'sale-1',
      promotionId: 'promo1',
    });
  });

  it('applyPromotionScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue({ discount: 5000 });
    await applyPromotionScoped(TOKEN, 'sale-1', 'promo1');
    expect(mockInvoke).toHaveBeenCalledWith('apply_promotion_scoped', {
      sessionToken: TOKEN,
      saleId: 'sale-1',
      promotionId: 'promo1',
    });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('promotion expired'));
    await expect(listPromotions()).rejects.toThrow('promotion expired');
  });
});
