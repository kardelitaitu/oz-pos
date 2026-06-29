import { invoke } from '@tauri-apps/api/core';

export interface GatewayStatus {
  name: string;
  configured: boolean;
  online: boolean;
}

export async function getGatewayStatus(): Promise<GatewayStatus[]> {
  try {
    const stripeKey: string | null = await invoke('get_setting', { key: 'stripe.api_key' });
    const squareKey: string | null = await invoke('get_setting', { key: 'square.api_key' });
    const statuses: GatewayStatus[] = [];
    if (stripeKey) statuses.push({ name: 'Stripe', configured: true, online: true });
    if (squareKey) statuses.push({ name: 'Square', configured: true, online: true });
    if (statuses.length === 0) {
      statuses.push({ name: 'Mock', configured: true, online: true });
    }
    return statuses;
  } catch {
    return [{ name: 'Gateway', configured: false, online: false }];
  }
}
