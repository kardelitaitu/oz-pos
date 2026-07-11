import { invoke } from '@tauri-apps/api/core';

export interface LoyaltyTier {
  id: string;
  name: string;
  min_points: number;
  points_per_unit: number;
  earn_multiplier: number;
  colour: string;
  sort_order: number;
  created_at: string;
}

export interface LoyaltyAccount {
  id: string;
  customer_id: string;
  points: number;
  lifetime_points: number;
  tier_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface LoyaltyTransaction {
  id: string;
  account_id: string;
  sale_id: string | null;
  points: number;
  txn_type: string;
  description: string;
  created_at: string;
}

export interface LoyaltyAccountWithDetails {
  account: LoyaltyAccount;
  tier: LoyaltyTier | null;
  recent_transactions: LoyaltyTransaction[];
  next_tier: LoyaltyTier | null;
  points_to_next_tier: number;
}

export interface RedeemResult {
  transaction: LoyaltyTransaction;
  discount_minor: number;
}

export const getLoyaltyAccount = (customerId: string): Promise<LoyaltyAccountWithDetails | null> =>
  invoke<LoyaltyAccountWithDetails | null>('get_loyalty_account', { customerId });

export const listLoyaltyAccounts = (): Promise<LoyaltyAccountWithDetails[]> =>
  invoke<LoyaltyAccountWithDetails[]>('list_loyalty_accounts');

export const earnLoyaltyPoints = (customerId: string, saleId: string, totalMinor: number): Promise<LoyaltyTransaction> =>
  invoke<LoyaltyTransaction>('earn_loyalty_points', { customerId, saleId, totalMinor });

export const redeemLoyaltyPoints = (customerId: string, points: number, saleId: string): Promise<RedeemResult> =>
  invoke<RedeemResult>('redeem_loyalty_points', { customerId, points, saleId });

export const listLoyaltyTiers = (): Promise<LoyaltyTier[]> =>
  invoke<LoyaltyTier[]>('list_loyalty_tiers');

export const updateLoyaltyTier = (tier: LoyaltyTier): Promise<LoyaltyTier> =>
  invoke<LoyaltyTier>('update_loyalty_tier', { tier });

export const getPointsValue = (points: number): Promise<number> =>
  invoke<number>('get_points_value', { points });

export const getOrCreateLoyaltyAccount = (customerId: string): Promise<LoyaltyAccount> =>
  invoke<LoyaltyAccount>('get_or_create_loyalty_account', { customerId });
