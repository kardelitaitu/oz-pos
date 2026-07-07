import { useState, useEffect } from 'react';

export interface GatewayStatus {
  name: string;
  configured: boolean;
  online: boolean;
}

export function useGatewayStatus(): GatewayStatus {
  const [status, setStatus] = useState<GatewayStatus>({ name: 'Stripe', configured: false, online: false });

  useEffect(() => {
    let cancelled = false;

    async function check() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const key: string | null = await invoke('get_setting', { key: 'stripe.api_key' });
        if (!cancelled) {
          const configured = key !== null && key !== '';
          setStatus({ name: 'Stripe', configured, online: configured });
        }
      } catch {
        if (!cancelled) {
          setStatus({ name: 'Stripe', configured: false, online: false });
        }
      }
    }

    check();
    const interval = setInterval(check, 60000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  return status;
}
