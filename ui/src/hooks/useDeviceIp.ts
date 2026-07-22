//! `useDeviceIp` — device IP address indicator hook.
//!
//! Tries to fetch the **public IP** from ipify.org first (fast, free, no API key).
//! Falls back to the **local IP** via the Tauri IPC command `get_local_ip`.
//! If both fail, returns `null` for both `ip` and `source`.

import { useState, useEffect, useRef } from 'react';
import { getLocalIp } from '@/api/system';

/** Source of the IP address. */
export type IpSource = 'public' | 'local';

/** Return type of the `useDeviceIp` hook. */
export interface DeviceIpStatus {
  /** The detected IP address, or null if unavailable. */
  ip: string | null;
  /** Whether the IP is public (internet-facing) or local (LAN). */
  source: IpSource | null;
}

/**
 * Detect the device's IP address — public if reachable, otherwise local.
 *
 * Resolution order:
 * 1. Fetch `https://api.ipify.org?format=json` (public IP via DNS)
 * 2. If that fails, invoke `get_local_ip` Tauri IPC (local IP)
 * 3. If both fail, returns `{ ip: null, source: null }`
 */
export function useDeviceIp(): DeviceIpStatus {
  const [ip, setIp] = useState<string | null>(null);
  const [source, setSource] = useState<IpSource | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    async function resolve() {
      // 1. Try public IP via ipify.org
      try {
        const response = await fetch('https://api.ipify.org?format=json', {
          signal: AbortSignal.timeout(5_000),
        });
        if (!mountedRef.current) return;
        if (response.ok) {
          const data: { ip: string } = await response.json();
          if (mountedRef.current && data.ip) {
            setIp(data.ip);
            setSource('public');
            return;
          }
        }
      } catch {
        // Public IP unavailable — fall through to local
      }

      if (!mountedRef.current) return;

      // 2. Fallback to local IP via Tauri IPC
      try {
        const localIp = await getLocalIp();
        if (mountedRef.current && localIp) {
          setIp(localIp);
          setSource('local');
          return;
        }
      } catch {
        // Local IP also unavailable
      }

      if (mountedRef.current) {
        setIp(null);
        setSource(null);
      }
    }

    resolve();

    return () => {
      mountedRef.current = false;
    };
  }, []);

  return { ip, source };
}
