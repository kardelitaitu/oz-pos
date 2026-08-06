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

/** Get a loyalty account from the store resolved by the active session. */
export const getLoyaltyAccount = (
  sessionToken: string,
  customerId: string,
): Promise<LoyaltyAccountWithDetails | null> =>
  loggedInvoke<LoyaltyAccountWithDetails | null>('get_loyalty_account_scoped', {
    sessionToken,
    customerId,
  });

/** List loyalty accounts from the store resolved by the active session. */
export const listLoyaltyAccounts = (
  sessionToken: string,
): Promise<LoyaltyAccountWithDetails[]> =>
  loggedInvoke<LoyaltyAccountWithDetails[]>('list_loyalty_accounts_scoped', {
    sessionToken,
  });

/** Earn loyalty points in the store resolved by the active session. */
export const earnLoyaltyPoints = (
  sessionToken: string,
  customerId: string,
  saleId: string,
  totalMinor: number,
): Promise<LoyaltyTransaction> =>
  loggedInvoke<LoyaltyTransaction>('earn_loyalty_points_scoped', {
    sessionToken,
    customerId,
    saleId,
    totalMinor,
  });

/** Redeem loyalty points in the store resolved by the active session. */
export const redeemLoyaltyPoints = (
  sessionToken: string,
  customerId: string,
  points: number,
  saleId: string,
): Promise<RedeemResult> =>
  loggedInvoke<RedeemResult>('redeem_loyalty_points_scoped', {
    sessionToken,
    customerId,
    points,
    saleId,
  });

/** List loyalty tiers from the store resolved by the active session. */
export const listLoyaltyTiers = (sessionToken: string): Promise<LoyaltyTier[]> =>
  loggedInvoke<LoyaltyTier[]>('list_loyalty_tiers_scoped', { sessionToken });

/** Update a loyalty tier in the store resolved by the active session. */
export const updateLoyaltyTier = (
  sessionToken: string,
  tier: LoyaltyTier,
): Promise<LoyaltyTier> =>
  loggedInvoke<LoyaltyTier>('update_loyalty_tier_scoped', { sessionToken, tier });

/** Convert loyalty points into minor currency units in the active store. */
export const getPointsValue = (sessionToken: string, points: number): Promise<number> =>
  loggedInvoke<number>('get_points_value_scoped', { sessionToken, points });

/** Get or create a loyalty account in the store resolved by the active session. */
export const getOrCreateLoyaltyAccount = (
  sessionToken: string,
  customerId: string,
): Promise<LoyaltyAccount> =>
  loggedInvoke<LoyaltyAccount>('get_or_create_loyalty_account_scoped', {
    sessionToken,
    customerId,
  });
