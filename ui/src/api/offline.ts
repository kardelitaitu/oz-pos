// ── Offline Queue & Cloud Sync ────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

// ── Offline Queue ────────────────────────────────────────────────

/** An item in the offline action queue. */
export interface OfflineQueueItemDto {
  id: string;
  action: string;
  status: string;
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  syncedAt: string | null;
}

/** Arguments for enqueuing an offline action. */
export interface EnqueueOfflineArgs {
  action: string;
  payload: string;
}

/** Result of a sync attempt. */
export interface SyncResult {
  synced: number;
  failed: number;
}

/** Summary of offline queue status (P1-6 sync observability). */
export interface OfflineQueueSummaryDto {
  pendingCount: number;
  syncedCount: number;
  failedCount: number;
  /** Number of items resolved via conflict during sync (P1-3). */
  conflictCount: number;
  lastSyncedAt: string | null;
  oldestPendingAt: string | null;
}

/** Get a summary of the offline queue status. */
export const getOfflineQueueStatusSummary = (): Promise<OfflineQueueSummaryDto> =>
  loggedInvoke<OfflineQueueSummaryDto>('offline_queue_status_summary');

/** Enqueue an action to be performed when back online. */
export const enqueueOffline = (args: EnqueueOfflineArgs): Promise<OfflineQueueItemDto> =>
  loggedInvoke<OfflineQueueItemDto>('enqueue_offline', { args });

/** List pending (not yet synced) offline actions. */
export const listPendingOffline = (): Promise<OfflineQueueItemDto[]> =>
  loggedInvoke<OfflineQueueItemDto[]>('list_pending_offline');

/** List all offline actions (pending and synced). */
export const listAllOffline = (): Promise<OfflineQueueItemDto[]> =>
  loggedInvoke<OfflineQueueItemDto[]>('list_all_offline');

/** Get the count of pending offline actions. */
export const pendingOfflineCount = (): Promise<number> =>
  loggedInvoke<number>('pending_offline_count');

/** Retry syncing all pending offline actions. */
export const retryOfflineSync = (): Promise<SyncResult> =>
  loggedInvoke<SyncResult>('retry_offline_sync');

/** Delete an offline queue item by its identifier. */
export const deleteOfflineItem = (id: string): Promise<void> =>
  loggedInvoke('delete_offline_item', { args: { id } });

// ── Cloud Sync Settings ──────────────────────────────────────────

/** Cloud sync configuration. */
export interface SyncSettingsDto {
  serverUrl: string | null;
  hasApiKey: boolean;
  enabled: boolean;
}

/** Arguments for updating cloud sync settings. */
export interface UpdateSyncSettingsArgs {
  serverUrl?: string | null;
  apiKey?: string | null;
  enabled: boolean;
}

/** Result of a sync run, including per-batch success/failure counts. */
export interface SyncAttemptResult {
  synced: number;
  failed: number;
  error: string | null;
}

/** Result of pulling data from the cloud server. */
export interface PullResult {
  productsPulled: number;
  taxRatesPulled: number;
  usersPulled: number;
  error: string | null;
}

/** Get the current cloud sync settings. */
export const getSyncSettings = (): Promise<SyncSettingsDto> =>
  loggedInvoke<SyncSettingsDto>('get_sync_settings');

/** Get cloud sync settings resolved from a session token. ADR #7. */
export const getSyncSettingsScoped = (sessionToken: string): Promise<SyncSettingsDto> =>
  loggedInvoke<SyncSettingsDto>('get_sync_settings_scoped', { sessionToken });

/** Update the cloud sync settings. */
export const updateSyncSettings = (args: UpdateSyncSettingsArgs): Promise<void> =>
  loggedInvoke<void>('update_sync_settings', { args });

/** Run a sync cycle — push pending local changes to the cloud server. */
export const syncRun = (): Promise<SyncAttemptResult> =>
  loggedInvoke<SyncAttemptResult>('sync_run');

/** Get the number of actions pending cloud sync. */
export const pendingSyncCount = (): Promise<number> =>
  loggedInvoke<number>('pending_sync_count');

/** Pull data (products, tax rates, users) from the cloud server. */
export const syncPull = (): Promise<PullResult> =>
  loggedInvoke<PullResult>('sync_pull');

// ── Connection Test ──────────────────────────────────────────────

/** Result of pinging the cloud server's health endpoint. */
export interface PingResult {
  ok: boolean;
  status: string;
  latencyMs: number | null;
}

/** Test connectivity to the configured cloud server.
 *  Pass the in-progress URL from the text field so users can
 *  test before saving. Falls back to saved settings if empty. */
export const testSyncConnection = (url?: string): Promise<PingResult> =>
  loggedInvoke<PingResult>('test_sync_connection', { url: url || null });

// ── Token Request ────────────────────────────────────────────────

/** Result of requesting a new JWT API token from the cloud server. */
export interface TokenResult {
  ok: boolean;
  token: string | null;
  status: string;
  expiresAt: string | null;
}

/** Request a new JWT token from the cloud server's
 *  POST /api/v1/tokens endpoint. Pass the in-progress URL
 *  so users can request a token before saving. */
export const requestSyncToken = (url?: string): Promise<TokenResult> =>
  loggedInvoke<TokenResult>('request_sync_token', { url: url || null });
