// ── Offline Queue & Cloud Sync ────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

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

/** Enqueue an action to be performed when back online. */
export const enqueueOffline = (args: EnqueueOfflineArgs): Promise<OfflineQueueItemDto> =>
  invoke<OfflineQueueItemDto>('enqueue_offline', { args });

/** List pending (not yet synced) offline actions. */
export const listPendingOffline = (): Promise<OfflineQueueItemDto[]> =>
  invoke<OfflineQueueItemDto[]>('list_pending_offline');

/** List all offline actions (pending and synced). */
export const listAllOffline = (): Promise<OfflineQueueItemDto[]> =>
  invoke<OfflineQueueItemDto[]>('list_all_offline');

/** Get the count of pending offline actions. */
export const pendingOfflineCount = (): Promise<number> =>
  invoke<number>('pending_offline_count');

/** Retry syncing all pending offline actions. */
export const retryOfflineSync = (): Promise<SyncResult> =>
  invoke<SyncResult>('retry_offline_sync');

/** Delete an offline queue item by its identifier. */
export const deleteOfflineItem = (id: string): Promise<void> =>
  invoke('delete_offline_item', { args: { id } });

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
  invoke<SyncSettingsDto>('get_sync_settings');

/** Update the cloud sync settings. */
export const updateSyncSettings = (args: UpdateSyncSettingsArgs): Promise<void> =>
  invoke<void>('update_sync_settings', { args });

/** Run a sync cycle — push pending local changes to the cloud server. */
export const syncRun = (): Promise<SyncAttemptResult> =>
  invoke<SyncAttemptResult>('sync_run');

/** Get the number of actions pending cloud sync. */
export const pendingSyncCount = (): Promise<number> =>
  invoke<number>('pending_sync_count');

/** Pull data (products, tax rates, users) from the cloud server. */
export const syncPull = (): Promise<PullResult> =>
  invoke<PullResult>('sync_pull');
