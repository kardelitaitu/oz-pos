// ── Offline Queue & Cloud Sync ────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

// ── Offline Queue ────────────────────────────────────────────────

export interface OfflineQueueItemDto {
  id: string;
  action: string;
  status: string;
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  syncedAt: string | null;
}

export interface EnqueueOfflineArgs {
  action: string;
  payload: string;
}

export interface SyncResult {
  synced: number;
  failed: number;
}

export const enqueueOffline = (args: EnqueueOfflineArgs): Promise<OfflineQueueItemDto> =>
  invoke<OfflineQueueItemDto>('enqueue_offline', { args });

export const listPendingOffline = (): Promise<OfflineQueueItemDto[]> =>
  invoke<OfflineQueueItemDto[]>('list_pending_offline');

export const listAllOffline = (): Promise<OfflineQueueItemDto[]> =>
  invoke<OfflineQueueItemDto[]>('list_all_offline');

export const pendingOfflineCount = (): Promise<number> =>
  invoke<number>('pending_offline_count');

export const retryOfflineSync = (): Promise<SyncResult> =>
  invoke<SyncResult>('retry_offline_sync');

export const deleteOfflineItem = (id: string): Promise<void> =>
  invoke('delete_offline_item', { args: { id } });

// ── Cloud Sync Settings ──────────────────────────────────────────

export interface SyncSettingsDto {
  serverUrl: string | null;
  hasApiKey: boolean;
  enabled: boolean;
}

export interface UpdateSyncSettingsArgs {
  serverUrl?: string | null;
  apiKey?: string | null;
  enabled: boolean;
}

export interface SyncAttemptResult {
  synced: number;
  failed: number;
  error: string | null;
}

export const getSyncSettings = (): Promise<SyncSettingsDto> =>
  invoke<SyncSettingsDto>('get_sync_settings');

export const updateSyncSettings = (args: UpdateSyncSettingsArgs): Promise<void> =>
  invoke<void>('update_sync_settings', { args });

export const triggerSync = (): Promise<SyncAttemptResult> =>
  invoke<SyncAttemptResult>('trigger_sync');

export const pendingSyncCount = (): Promise<number> =>
  invoke<number>('pending_sync_count');
