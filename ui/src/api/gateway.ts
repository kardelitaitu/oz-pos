import { loggedInvoke } from '@/utils/logged-invoke';
import type { GatewayStatus } from '@/hooks/useGatewayStatus';

export type { GatewayStatus };

/** Get the configured status of all payment gateways (Stripe, Square, Midtrans). */
export async function getGatewayStatus(): Promise<GatewayStatus[]> {
  try {
    const stripeKey: string | null = await loggedInvoke('get_setting', { key: 'stripe.api_key' });
    const squareKey: string | null = await loggedInvoke('get_setting', { key: 'square.api_key' });
    const midtransKey: string | null = await loggedInvoke('get_setting', { key: 'midtrans.server_key' });
    // Always show all three gateways — configured state reflects whether a key is present
    return [
      { name: 'Stripe', configured: stripeKey !== null && stripeKey !== '', online: stripeKey !== null && stripeKey !== '' },
      { name: 'Square', configured: squareKey !== null && squareKey !== '', online: squareKey !== null && squareKey !== '' },
      { name: 'QRIS (Midtrans)', configured: midtransKey !== null && midtransKey !== '', online: midtransKey !== null && midtransKey !== '' },
    ];
  } catch {
    return [{ name: 'Gateway', configured: false, online: false }];
  }
}
