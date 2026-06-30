import { useState, useEffect } from 'react';

interface GatewayStatus {
  configured: boolean;
  online: boolean;
}

export function useGatewayStatus(): GatewayStatus {
  const [status, setStatus] = useState<GatewayStatus>({ configured: false, online: false });

  useEffect(() => {
    let cancelled = false;

    async function check() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const key: string | null = await invoke('get_setting', { key: 'stripe.api_key' });
        if (!cancelled) {
          const configured = key !== null && key !== '';
          setStatus({ configured, online: configured });
        }
      } catch {
        if (!cancelled) {
          setStatus({ configured: false, online: false });
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
