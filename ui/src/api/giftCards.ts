import { invoke } from '@tauri-apps/api/core';

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

export interface GiftCardWithTransactions {
  card: GiftCard;
  transactions: GiftCardTransaction[];
}

export interface GiftCardFilter {
  search?: string | null;
  status?: string | null;
  issued_to?: string | null;
  min_balance?: number | null;
}

export interface IssueGiftCardInput {
  card_number: string;
  pin?: string | null;
  initial_amount_minor: number;
  currency: string;
  issued_to?: string | null;
  created_by: string;
  expiry_date?: string | null;
}

export interface BalanceResult {
  balance_minor: number;
  currency: string;
  status: string;
}

export interface RedeemGiftCardResult {
  card: GiftCard;
  transaction: GiftCardTransaction;
}

export const issueGiftCard = (input: IssueGiftCardInput): Promise<GiftCardWithTransactions> =>
  invoke<GiftCardWithTransactions>('issue_gift_card', { input });

export const getGiftCard = (cardNumberOrId: string): Promise<GiftCardWithTransactions | null> =>
  invoke<GiftCardWithTransactions | null>('get_gift_card', { cardNumberOrId });

export const listGiftCards = (filter: GiftCardFilter): Promise<GiftCardWithTransactions[]> =>
  invoke<GiftCardWithTransactions[]>('list_gift_cards', { filter });

export const getGiftCardBalance = (cardNumberOrId: string): Promise<BalanceResult | null> =>
  invoke<BalanceResult | null>('get_gift_card_balance', { cardNumberOrId });

export const redeemGiftCard = (cardNumberOrId: string, amountMinor: number, saleId: string): Promise<RedeemGiftCardResult> =>
  invoke<RedeemGiftCardResult>('redeem_gift_card', { cardNumberOrId, amountMinor, saleId });

export const topUpGiftCard = (cardNumberOrId: string, amountMinor: number): Promise<GiftCardWithTransactions> =>
  invoke<GiftCardWithTransactions>('top_up_gift_card', { cardNumberOrId, amountMinor });

export const freezeGiftCard = (cardNumberOrId: string): Promise<GiftCard> =>
  invoke<GiftCard>('freeze_gift_card', { cardNumberOrId });

export const unfreezeGiftCard = (cardNumberOrId: string): Promise<GiftCard> =>
  invoke<GiftCard>('unfreeze_gift_card', { cardNumberOrId });
