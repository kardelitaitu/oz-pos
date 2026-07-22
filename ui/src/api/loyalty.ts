import { loggedInvoke } from '@/utils/logged-invoke';

/** A loyalty tier defining points thresholds and earn rates. */
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

/** A customer's loyalty account with current points balance. */
export interface LoyaltyAccount {
  id: string;
  customer_id: string;
  points: number;
  lifetime_points: number;
  tier_id: string | null;
  updated_at: string;
  created_at: string;
}

/** A points earn or redeem transaction on a loyalty account. */
export interface LoyaltyTransaction {
  id: string;
  account_id: string;
  sale_id: string | null;
  points: number;
  txn_type: string;
  description: string;
  created_at: string;
}

/** A loyalty account with tier info, recent transactions, and next tier progress. */
export interface LoyaltyAccountWithDetails {
  account: LoyaltyAccount;
  tier: LoyaltyTier | null;
  recent_transactions: LoyaltyTransaction[];
  next_tier: LoyaltyTier | null;
  points_to_next_tier: number;
}

/** Result of redeeming loyalty points, with the generated transaction and discount amount. */
export interface RedeemResult {
  transaction: LoyaltyTransaction;
  discount_minor: number;
}

/** Get a loyalty account for a customer with tier and transaction details. */
export const getLoyaltyAccount = (customerId: string): Promise<LoyaltyAccountWithDetails | null> =>
  loggedInvoke<LoyaltyAccountWithDetails | null>('get_loyalty_account', { customerId });

/** List all loyalty accounts with tier and transaction details. */
export const listLoyaltyAccounts = (): Promise<LoyaltyAccountWithDetails[]> =>
  loggedInvoke<LoyaltyAccountWithDetails[]>('list_loyalty_accounts');

/** Earn loyalty points for a customer on a completed sale. */
export const earnLoyaltyPoints = (customerId: string, saleId: string, totalMinor: number): Promise<LoyaltyTransaction> =>
  loggedInvoke<LoyaltyTransaction>('earn_loyalty_points', { customerId, saleId, totalMinor });

/** Redeem loyalty points for a discount on a sale. */
export const redeemLoyaltyPoints = (customerId: string, points: number, saleId: string): Promise<RedeemResult> =>
  loggedInvoke<RedeemResult>('redeem_loyalty_points', { customerId, points, saleId });

/** List all loyalty tiers. */
export const listLoyaltyTiers = (): Promise<LoyaltyTier[]> =>
  loggedInvoke<LoyaltyTier[]>('list_loyalty_tiers');

/** Update an existing loyalty tier. */
export const updateLoyaltyTier = (tier: LoyaltyTier): Promise<LoyaltyTier> =>
  loggedInvoke<LoyaltyTier>('update_loyalty_tier', { tier });

/** Get the monetary value (in minor units) for a given number of loyalty points. */
export const getPointsValue = (points: number): Promise<number> =>
  loggedInvoke<number>('get_points_value', { points });

/** Get or create a loyalty account for a customer. */
export const getOrCreateLoyaltyAccount = (customerId: string): Promise<LoyaltyAccount> =>
  loggedInvoke<LoyaltyAccount>('get_or_create_loyalty_account', { customerId });
