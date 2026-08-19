// ── IPC contract tests for promotions.ts ───────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listPromotions,
  listPromotionsScoped,
  getPromotion,
  getPromotionScoped,
  createPromotion,
  createPromotionScoped,
  updatePromotion,
  updatePromotionScoped,
  deletePromotion,
  deletePromotionScoped,
  applyPromotion,
} from '@/api/promotions';

describe('promotions.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listPromotions → list_promotions (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPromotions();
    expect(mockInvoke).toHaveBeenCalledWith('list_promotions', undefined);
  });

  it('listPromotionsScoped → list_promotions_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPromotionsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_promotions_scoped', { sessionToken: 'tok' });
  });

  it('getPromotion → get_promotion with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getPromotion('p1');
    expect(mockInvoke).toHaveBeenCalledWith('get_promotion', { id: 'p1' });
  });

  it('getPromotionScoped → get_promotion_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getPromotionScoped('tok', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('get_promotion_scoped', { sessionToken: 'tok', id: 'p1' });
  });

  it('createPromotion → create_promotion with userId + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'p1' });
    await createPromotion('u1', { name: 'Sale', type: 'percentage', value: 10, conditions: [], isActive: true });
    expect(mockInvoke).toHaveBeenCalledWith('create_promotion', { userId: 'u1', args: expect.objectContaining({ name: 'Sale' }) });
  });

  it('createPromotionScoped → create_promotion_scoped with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'p1' });
    await createPromotionScoped('tok', { name: 'BOGO', type: 'bogo', value: 1, conditions: [], isActive: true });
    expect(mockInvoke).toHaveBeenCalledWith('create_promotion_scoped', { sessionToken: 'tok', args: expect.objectContaining({ name: 'BOGO' }) });
  });

  it('updatePromotion → update_promotion with userId + promotion', async () => {
    mockInvoke.mockResolvedValue({ id: 'p1' });
    await updatePromotion('u1', { id: 'p1', name: 'Updated', type: 'percentage', value: 15, conditions: [], isActive: true });
    expect(mockInvoke).toHaveBeenCalledWith('update_promotion', { userId: 'u1', promotion: expect.objectContaining({ id: 'p1' }) });
  });

  it('updatePromotionScoped → update_promotion_scoped with sessionToken + promotion', async () => {
    mockInvoke.mockResolvedValue({ id: 'p1' });
    await updatePromotionScoped('tok', { id: 'p1', name: 'Updated', type: 'fixed', value: 5000, conditions: [], isActive: false });
    expect(mockInvoke).toHaveBeenCalledWith('update_promotion_scoped', { sessionToken: 'tok', promotion: expect.objectContaining({ id: 'p1' }) });
  });

  it('deletePromotion → delete_promotion with userId + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deletePromotion('u1', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_promotion', { userId: 'u1', id: 'p1' });
  });

  it('deletePromotionScoped → delete_promotion_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deletePromotionScoped('tok', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_promotion_scoped', { sessionToken: 'tok', id: 'p1' });
  });

  it('applyPromotion → apply_promotion with userId + saleId + promotionId', async () => {
    mockInvoke.mockResolvedValue({ discount: 5000 });
    await applyPromotion('u1', 's1', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('apply_promotion', { userId: 'u1', saleId: 's1', promotionId: 'p1' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('promotion not found'));
    await expect(getPromotion('missing')).rejects.toThrow('promotion not found');
  });
});
