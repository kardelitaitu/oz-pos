import { loggedInvoke } from '@/utils/logged-invoke';
import type { GatewayStatus } from '@/hooks/useGatewayStatus';

export type { GatewayStatus };

/**
 * Get the configured status of all payment gateways (Stripe, Square,
 * Midtrans).
 *
 * UI-1 fix: the backend computes the configured/online booleans
 * server-side (see `gateway_status` in the desktop/tablet clients) so
 * the raw credential values never reach the renderer. The previous
 * implementation fetched `stripe.api_key`, `square.api_key`, and
 * `midtrans.server_key` via `get_setting`, pulling secrets into the
 * webview just to compute booleans.
 *
 * Propagates any backend error (DB failure, session expiry, missing
 * settings table) to the caller. The previous version caught all
 * errors and returned a synthetic `[{ name: 'Gateway', ... }]`
 * fallback, which masked real outages as "no gateways configured" and
 * returned an inconsistent array length (3 on success, 1 on failure).
 */
export async function getGatewayStatus(): Promise<GatewayStatus[]> {
  return loggedInvoke('gateway_status');
}