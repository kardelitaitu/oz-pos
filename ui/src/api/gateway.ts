import { loggedInvoke } from '@/utils/logged-invoke';
import type { GatewayStatus } from '@/hooks/useGatewayStatus';

export type { GatewayStatus };

/**
 * Get the configured status of all payment gateways (Stripe, Square,
 * Midtrans).
 *
 * Propagates any backend error (DB failure, session expiry, missing
 * settings table) to the caller. The previous version caught all
 * errors and returned a synthetic `[{ name: 'Gateway', ... }]`
 * fallback, which masked real outages as "no gateways configured" and
 * returned an inconsistent array length (3 on success, 1 on failure).
 */
export async function getGatewayStatus(): Promise<GatewayStatus[]> {
  const stripeKey: string | null = await loggedInvoke('get_setting', { key: 'stripe.api_key' });
  const squareKey: string | null = await loggedInvoke('get_setting', { key: 'square.api_key' });
  const midtransKey: string | null = await loggedInvoke('get_setting', { key: 'midtrans.server_key' });
  // Always show all three gateways — configured state reflects whether a key is present
  return [
    { name: 'Stripe', configured: stripeKey !== null && stripeKey !== '', online: stripeKey !== null && stripeKey !== '' },
    { name: 'Square', configured: squareKey !== null && squareKey !== '', online: squareKey !== null && squareKey !== '' },
    { name: 'QRIS (Midtrans)', configured: midtransKey !== null && midtransKey !== '', online: midtransKey !== null && midtransKey !== '' },
  ];
}
