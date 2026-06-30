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

export const getPromotion = (id: string): Promise<Promotion | null> =>
  invoke<Promotion | null>('get_promotion', { id });

export const createPromotion = (args: CreatePromotionArgs): Promise<Promotion> =>
  invoke<Promotion>('create_promotion', { args });

export const updatePromotion = (promotion: Promotion): Promise<Promotion> =>
  invoke<Promotion>('update_promotion', { promotion });

export const deletePromotion = (id: string): Promise<void> =>
  invoke<void>('delete_promotion', { id });

export const applyPromotion = (saleId: string, promotionId: string): Promise<PromotionApplication> =>
  invoke<PromotionApplication>('apply_promotion', { saleId, promotionId });

export const getSalePromotions = (saleId: string): Promise<PromotionApplication[]> =>
  invoke<PromotionApplication[]>('get_sale_promotions', { saleId });
