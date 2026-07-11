import { invoke } from '@tauri-apps/api/core';

/** A gift card record with balance and status. */
export interface GiftCard {
  id: string;
  card_number: string;
  pin: string;
  initial_balance_minor: number;
  current_balance_minor: number;
  currency: string;
  status: string;
  issued_to: string;
  issue_date: string;
  expiry_date: string | null;
  created_by: string | null;
  updated_at: string;
}

/** A transaction against a gift card (issue, redeem, top-up, freeze). */
export interface GiftCardTransaction {
  id: string;
  gift_card_id: string;
  sale_id: string | null;
  txn_type: string;
  amount_minor: number;
  balance_after_minor: number;
  notes: string;
  created_at: string;
}

/** A gift card with its full transaction history. */
export interface GiftCardWithTransactions {
  card: GiftCard;
  transactions: GiftCardTransaction[];
}

/** Filter parameters for listing gift cards. */
export interface GiftCardFilter {
  search?: string | null;
  status?: string | null;
  issued_to?: string | null;
  min_balance?: number | null;
}

/** Input for issuing a new gift card. */
export interface IssueGiftCardInput {
  card_number: string;
  pin?: string | null;
  initial_amount_minor: number;
  currency: string;
  issued_to?: string | null;
  created_by: string;
  expiry_date?: string | null;
}

/** Gift card balance check result. */
export interface BalanceResult {
  balance_minor: number;
  currency: string;
  status: string;
}

/** Result of redeeming a gift card against a sale. */
export interface RedeemGiftCardResult {
  card: GiftCard;
  transaction: GiftCardTransaction;
}

/** Issue a new gift card with an initial balance. */
export const issueGiftCard = (input: IssueGiftCardInput): Promise<GiftCardWithTransactions> =>
  invoke<GiftCardWithTransactions>('issue_gift_card', { input });

/** Get a gift card by card number or ID, including transactions. */
export const getGiftCard = (cardNumberOrId: string): Promise<GiftCardWithTransactions | null> =>
  invoke<GiftCardWithTransactions | null>('get_gift_card', { cardNumberOrId });

/** List gift cards with optional filtering. */
export const listGiftCards = (filter: GiftCardFilter): Promise<GiftCardWithTransactions[]> =>
  invoke<GiftCardWithTransactions[]>('list_gift_cards', { filter });

/** Check a gift card's current balance and status. */
export const getGiftCardBalance = (cardNumberOrId: string): Promise<BalanceResult | null> =>
  invoke<BalanceResult | null>('get_gift_card_balance', { cardNumberOrId });

/** Redeem a gift card for a given amount against a sale. */
export const redeemGiftCard = (cardNumberOrId: string, amountMinor: number, saleId: string): Promise<RedeemGiftCardResult> =>
  invoke<RedeemGiftCardResult>('redeem_gift_card', { cardNumberOrId, amountMinor, saleId });

/** Add funds to an existing gift card. */
export const topUpGiftCard = (cardNumberOrId: string, amountMinor: number): Promise<GiftCardWithTransactions> =>
  invoke<GiftCardWithTransactions>('top_up_gift_card', { cardNumberOrId, amountMinor });

/** Freeze a gift card to prevent further use. */
export const freezeGiftCard = (cardNumberOrId: string): Promise<GiftCard> =>
  invoke<GiftCard>('freeze_gift_card', { cardNumberOrId });

/** Unfreeze a previously frozen gift card. */
export const unfreezeGiftCard = (cardNumberOrId: string): Promise<GiftCard> =>
  invoke<GiftCard>('unfreeze_gift_card', { cardNumberOrId });
