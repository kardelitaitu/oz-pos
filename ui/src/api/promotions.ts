import { invoke } from '@tauri-apps/api/core';

export interface Promotion {
  id: string;
  name: string;
  description: string;
  promo_type: string;
  value_minor: number;
  min_qty: number | null;
  trigger_sku: string | null;
  reward_sku: string | null;
  reward_qty: number | null;
  starts_at: string | null;
  ends_at: string | null;
  min_order_minor: number;
  category_id: string | null;
  active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PromotionApplication {
  id: string;
  promotion_id: string;
  sale_id: string;
  discount_minor: number;
  description: string;
  created_at: string;
}

export interface CreatePromotionArgs {
  name: string;
  description?: string;
  promo_type: string;
  value_minor: number;
  min_qty?: number | null;
  trigger_sku?: string | null;
  reward_sku?: string | null;
  reward_qty?: number | null;
  starts_at?: string | null;
  ends_at?: string | null;
  min_order_minor?: number;
  category_id?: string | null;
}

export const listPromotions = (): Promise<Promotion[]> =>
  invoke<Promotion[]>('list_promotions');

/** List promotions (scoped — ADR #7). */
export const listPromotionsScoped = (sessionToken: string): Promise<Promotion[]> =>
  invoke<Promotion[]>('list_promotions_scoped', { sessionToken });

export const getPromotion = (id: string): Promise<Promotion | null> =>
  invoke<Promotion | null>('get_promotion', { id });

/** Get a promotion (scoped — ADR #7). */
export const getPromotionScoped = (sessionToken: string, id: string): Promise<Promotion | null> =>
  invoke<Promotion | null>('get_promotion_scoped', { sessionToken, id });

export const createPromotion = (userId: string, args: CreatePromotionArgs): Promise<Promotion> =>
  invoke<Promotion>('create_promotion', { userId, args });

/** Create a promotion (scoped — ADR #7). */
export const createPromotionScoped = (sessionToken: string, args: CreatePromotionArgs): Promise<Promotion> =>
  invoke<Promotion>('create_promotion_scoped', { sessionToken, args });

export const updatePromotion = (userId: string, promotion: Promotion): Promise<Promotion> =>
  invoke<Promotion>('update_promotion', { userId, promotion });

/** Update a promotion (scoped — ADR #7). */
export const updatePromotionScoped = (sessionToken: string, promotion: Promotion): Promise<Promotion> =>
  invoke<Promotion>('update_promotion_scoped', { sessionToken, promotion });

export const deletePromotion = (userId: string, id: string): Promise<void> =>
  invoke<void>('delete_promotion', { userId, id });

/** Delete a promotion (scoped — ADR #7). */
export const deletePromotionScoped = (sessionToken: string, id: string): Promise<void> =>
  invoke<void>('delete_promotion_scoped', { sessionToken, id });

export const applyPromotion = (userId: string, saleId: string, promotionId: string): Promise<PromotionApplication> =>
  invoke<PromotionApplication>('apply_promotion', { userId, saleId, promotionId });

/** Apply a promotion to a sale (scoped — ADR #7). */
export const applyPromotionScoped = (sessionToken: string, saleId: string, promotionId: string): Promise<PromotionApplication> =>
  invoke<PromotionApplication>('apply_promotion_scoped', { sessionToken, saleId, promotionId });

export const getSalePromotions = (saleId: string): Promise<PromotionApplication[]> =>
  invoke<PromotionApplication[]>('get_sale_promotions', { saleId });

/** Get sale promotions (scoped — ADR #7). */
export const getSalePromotionsScoped = (sessionToken: string, saleId: string): Promise<PromotionApplication[]> =>
  invoke<PromotionApplication[]>('get_sale_promotions_scoped', { sessionToken, saleId });
