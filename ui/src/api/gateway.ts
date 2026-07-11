import { invoke } from '@tauri-apps/api/core';
import type { GatewayStatus } from '@/hooks/useGatewayStatus';

export type { GatewayStatus };

export async function getGatewayStatus(): Promise<GatewayStatus[]> {
  try {
    const stripeKey: string | null = await invoke('get_setting', { key: 'stripe.api_key' });
    const squareKey: string | null = await invoke('get_setting', { key: 'square.api_key' });
    const midtransKey: string | null = await invoke('get_setting', { key: 'midtrans.server_key' });
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
