// ── Local REST API server (loopback, Settings → Local API) ─────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** Status of the embedded local API server (camelCase IPC wire). */
export interface LocalApiStatusDto {
  /** Whether the setting is on (persisted intent). */
  enabled: boolean;
  /** Whether a server is currently listening on loopback. */
  running: boolean;
  /** The configured port (from settings, even when not running). */
  port: number;
  /** Base URL for scripts, e.g. `http://127.0.0.1:3099/api/v1`. */
  baseUrl: string | null;
  /** The store currently served (configured override or primary). */
  storeId: string;
}

/** A minted long-lived API token (snake_case — mirrors the REST contract). */
export interface LocalApiTokenDto {
  token: string;
  expires_at: string;
  token_id: string;
}

/** Report whether the local API is enabled/running and on which port. */
export const getLocalApiStatusScoped = (sessionToken: string): Promise<LocalApiStatusDto> =>
  loggedInvoke<LocalApiStatusDto>('local_api_status_scoped', { sessionToken });

/** Enable or disable the local API server (loopback bind happens here). */
export const setLocalApiEnabledScoped = (sessionToken: string, enabled: boolean): Promise<LocalApiStatusDto> =>
  loggedInvoke<LocalApiStatusDto>('local_api_set_enabled_scoped', { sessionToken, enabled });

/** Change the listen port (restarts the server when running). */
export const setLocalApiPortScoped = (sessionToken: string, port: number): Promise<LocalApiStatusDto> =>
  loggedInvoke<LocalApiStatusDto>('local_api_set_port_scoped', { sessionToken, port });

/**
 * Choose which store the API serves ('' = primary store). Restarts the
 * server against the new target when running.
 */
export const setLocalApiStoreScoped = (sessionToken: string, storeId: string): Promise<LocalApiStatusDto> =>
  loggedInvoke<LocalApiStatusDto>('local_api_set_store_scoped', { sessionToken, storeId });

/**
 * Rotate the per-install signing secret. Every previously minted token
 * stops working immediately and the operator X-Admin-Key changes —
 * confirm with the user before calling.
 */
export const rotateLocalApiSecretScoped = (sessionToken: string): Promise<LocalApiStatusDto> =>
  loggedInvoke<LocalApiStatusDto>('local_api_rotate_secret_scoped', { sessionToken });

/** Mint a long-lived API token signed with the per-install secret. */
export const mintLocalApiTokenScoped = (
  sessionToken: string,
  label: string,
  expiryHours?: number,
): Promise<LocalApiTokenDto> =>
  loggedInvoke<LocalApiTokenDto>('local_api_mint_token_scoped', {
    sessionToken,
    label,
    ...(expiryHours !== undefined ? { expiryHours } : {}),
  });
