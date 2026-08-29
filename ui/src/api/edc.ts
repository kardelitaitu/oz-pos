// ── EDC card-present payment terminal ──────────────────────────────
//
// Card-present payments via the EDC terminal wired into the desktop
// client (currently a success-mode mock). These wrappers call the
// `edc_*` Tauri commands registered in lib.rs.

import { loggedInvoke } from '@/utils/logged-invoke';

/** Terminal status discriminator (mirrors TerminalStatus). */
export type EdcTerminalStatus =
  | 'ready'
  | 'busy'
  | 'offline'
  | 'paperError'
  | 'error';

/** Result of an EDC terminal status query. */
export interface EdcStatus {
  status: EdcTerminalStatus;
}

/** Result of a card-present sale / refund / void. */
export interface EdcResult {
  success: boolean;
  transactionId: string | null;
  authCode: string | null;
  cardScheme: string | null;
  cardLast4: string | null;
  message: string;
}

/** Query the EDC terminal's current status. */
export const edcTerminalStatus = (): Promise<EdcStatus> =>
  loggedInvoke<EdcStatus>('edc_terminal_status');

/**
 * Process a card-present sale (authorize + capture).
 *
 * `amountMinor` is in the currency's minor units (e.g. cents for USD,
 * rupiah for IDR).
 */
export const edcSale = (
  amountMinor: number,
  currency: string,
): Promise<EdcResult> =>
  loggedInvoke<EdcResult>('edc_sale', { amountMinor, currency });

/** Refund a previously captured card transaction. */
export const edcRefund = (
  transactionId: string,
  amountMinor: number,
  currency: string,
): Promise<EdcResult> =>
  loggedInvoke<EdcResult>('edc_refund', { transactionId, amountMinor, currency });

/** Void a pending authorisation before capture. */
export const edcVoid = (transactionId: string): Promise<EdcResult> =>
  loggedInvoke<EdcResult>('edc_void', { transactionId });
