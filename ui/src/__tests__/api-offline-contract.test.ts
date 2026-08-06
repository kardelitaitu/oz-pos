// ── IPC contract tests for the offline/sync API layer ───────────────
//
// These tests verify the contract between `ui/src/api/offline.ts` and the
// Rust Tauri commands (`commands::offline`, `commands::sync`):
//
//   • SYNC-03 — `syncPull` must send `{ confirmDestructive: true }` as the
//     IPC payload; the backend command rejects any pull without explicit
//     destructive consent.
//   • SYNC-04 — `retryOfflineSync` invokes the real cloud-sync command
//     (no placeholder path) and returns the camelCase `SyncResult` DTO
//     (`syncedCount` / `failedCount` / `totalCount`).
//   • SYNC-11 — DTO shapes match the Rust `#[serde(rename_all = "camelCase")]`
//     serializers exactly.
//
// The `loggedInvoke` wrapper delegates to `@tauri-apps/api/core`'s
// `invoke`, so we mock that and assert the (command, args) pair.

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

// ── offline.ts ─────────────────────────────────────────────────────

import {
  syncPull,
  retryOfflineSync,
  syncRun,
  getSyncSettings,
  updateSyncSettings,
  testSyncConnection,
  requestSyncToken,
  pendingSyncCount,
  getOfflineQueueStatusSummary,
  enqueueOffline,
  listPendingOffline,
  listAllOffline,
  pendingOfflineCount,
  deleteOfflineItem,
  listRemoteFailures,
  requeueRemoteFailure,
  type SyncResult,
} from '@/api/offline';

describe('offline.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── SYNC-03: destructive pull consent contract ──────────────

  it('syncPull invokes "sync_pull" with confirmDestructive: true', async () => {
    mockInvoke.mockResolvedValue({ productsPulled: 3, taxRatesPulled: 1, usersPulled: 0, error: null });
    await syncPull({ confirmDestructive: true });
    expect(mockInvoke).toHaveBeenCalledWith('sync_pull', {
      args: { confirmDestructive: true },
    });
  });

  it('syncPull forwards a declined consent value verbatim (backend rejects it)', async () => {
    // The TS signature requires the flag, but the wire contract must still
    // forward the value as-is so the backend's consent gate is the source of
    // truth (false/missing is rejected server-side).
    mockInvoke.mockResolvedValue({ productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: 'no consent' });
    await syncPull({ confirmDestructive: false });
    expect(mockInvoke).toHaveBeenCalledWith('sync_pull', {
      args: { confirmDestructive: false },
    });
  });

  // ── SYNC-04: retry delegates to the real sync pipeline ──────

  it('retryOfflineSync invokes "retry_offline_sync" with no args', async () => {
    mockInvoke.mockResolvedValue({ syncedCount: 2, failedCount: 1, totalCount: 3 });
    await retryOfflineSync();
    expect(mockInvoke).toHaveBeenCalledWith('retry_offline_sync', undefined);
  });

  it('retryOfflineSync returns the camelCase SyncResult DTO shape', async () => {
    mockInvoke.mockResolvedValue({ syncedCount: 2, failedCount: 1, totalCount: 3 });
    const result: SyncResult = await retryOfflineSync();
    expect(result).toEqual({ syncedCount: 2, failedCount: 1, totalCount: 3 });
  });

  // ── sync_run / settings / connection helpers ─────────────────

  it('syncRun invokes "sync_run" with no args', async () => {
    mockInvoke.mockResolvedValue({ synced: 1, failed: 0, error: null });
    await syncRun();
    expect(mockInvoke).toHaveBeenCalledWith('sync_run', undefined);
  });

  it('getSyncSettings invokes "get_sync_settings" with no args', async () => {
    mockInvoke.mockResolvedValue({ serverUrl: null, hasApiKey: false, enabled: false });
    await getSyncSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_sync_settings', undefined);
  });

  it('updateSyncSettings invokes "update_sync_settings" with camelCase args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateSyncSettings({ serverUrl: 'https://sync.example.com', apiKey: 'sk-1', enabled: true });
    expect(mockInvoke).toHaveBeenCalledWith('update_sync_settings', {
      args: { serverUrl: 'https://sync.example.com', apiKey: 'sk-1', enabled: true },
    });
  });

  it('testSyncConnection invokes "test_sync_connection" with the candidate url', async () => {
    mockInvoke.mockResolvedValue({ ok: true, status: 'Connected', latencyMs: 12 });
    await testSyncConnection('https://sync.example.com');
    expect(mockInvoke).toHaveBeenCalledWith('test_sync_connection', { url: 'https://sync.example.com' });
  });

  it('testSyncConnection with no url passes null so the backend falls back to saved settings', async () => {
    mockInvoke.mockResolvedValue({ ok: false, status: 'nope', latencyMs: null });
    await testSyncConnection();
    expect(mockInvoke).toHaveBeenCalledWith('test_sync_connection', { url: null });
  });

  it('requestSyncToken invokes "request_sync_token" with the candidate url', async () => {
    mockInvoke.mockResolvedValue({ ok: true, token: 'jwt', status: 'issued', expiresAt: null });
    await requestSyncToken('https://sync.example.com');
    expect(mockInvoke).toHaveBeenCalledWith('request_sync_token', { url: 'https://sync.example.com' });
  });

  it('pendingSyncCount invokes "pending_sync_count" with no args', async () => {
    mockInvoke.mockResolvedValue(3);
    await pendingSyncCount();
    expect(mockInvoke).toHaveBeenCalledWith('pending_sync_count', undefined);
  });

  // ── offline queue CRUD ───────────────────────────────────────

  it('getOfflineQueueStatusSummary invokes "offline_queue_status_summary"', async () => {
    mockInvoke.mockResolvedValue({ pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0 });
    await getOfflineQueueStatusSummary();
    expect(mockInvoke).toHaveBeenCalledWith('offline_queue_status_summary', undefined);
  });

  it('enqueueOffline invokes "enqueue_offline" with action + payload args', async () => {
    mockInvoke.mockResolvedValue({ id: 'q1' });
    await enqueueOffline({ action: 'complete_sale', payload: '{}' });
    expect(mockInvoke).toHaveBeenCalledWith('enqueue_offline', {
      args: { action: 'complete_sale', payload: '{}' },
    });
  });

  it('listPendingOffline invokes "list_pending_offline" with no args', async () => {
    mockInvoke.mockResolvedValue([]);
    await listPendingOffline();
    expect(mockInvoke).toHaveBeenCalledWith('list_pending_offline', undefined);
  });

  it('OfflineQueueItemDto carries payload (SYNC-11 — matches the Rust serializer)', async () => {
    mockInvoke.mockResolvedValue([
      {
        id: 'oq-1',
        action: 'complete_sale',
        payload: '{"sale_id":"s-1"}',
        status: 'pending',
        retryCount: 0,
        lastError: null,
        createdAt: '2026-01-01T00:00:00Z',
        syncedAt: null,
        tenantId: 'store-a',
        priority: 'critical',
      },
    ]);
    const items = await listPendingOffline();
    expect(items).toHaveLength(1);
    // The payload field must survive the IPC round-trip — the Rust
    // OfflineQueueItemDto serializes it and the TS DTO must not drop it.
    const first = items[0]!;
    expect(first.payload).toBe('{"sale_id":"s-1"}');
    expect(first.action).toBe('complete_sale');
    // OFF-09: tenant + priority metadata must survive the IPC round-trip
    // too — the Rust DTO serializes camelCase tenantId/priority.
    expect(first.tenantId).toBe('store-a');
    expect(first.priority).toBe('critical');
  });

  it('enqueueOffline forwards optional tenantId + priority (OFF-09)', async () => {
    mockInvoke.mockResolvedValue({ id: 'q1' });
    await enqueueOffline({
      action: 'complete_sale',
      payload: '{}',
      tenantId: 'store-b',
      priority: 'critical',
    });
    expect(mockInvoke).toHaveBeenCalledWith('enqueue_offline', {
      args: {
        action: 'complete_sale',
        payload: '{}',
        tenantId: 'store-b',
        priority: 'critical',
      },
    });
  });

  it('listAllOffline invokes "list_all_offline" with no args', async () => {
    mockInvoke.mockResolvedValue([]);
    await listAllOffline();
    expect(mockInvoke).toHaveBeenCalledWith('list_all_offline', undefined);
  });

  it('pendingOfflineCount invokes "pending_offline_count" with no args', async () => {
    mockInvoke.mockResolvedValue(0);
    await pendingOfflineCount();
    expect(mockInvoke).toHaveBeenCalledWith('pending_offline_count', undefined);
  });

  it('deleteOfflineItem invokes "delete_offline_item" with id arg', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteOfflineItem('oq-1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_offline_item', { args: { id: 'oq-1' } });
  });

  // ── SYNC-11: remote dead-letter (quarantined pulls) ────────

  it('listRemoteFailures invokes "list_remote_failures" with no args', async () => {
    mockInvoke.mockResolvedValue([]);
    await listRemoteFailures();
    expect(mockInvoke).toHaveBeenCalledWith('list_remote_failures', undefined);
  });

  it('listRemoteFailures returns the camelCase RemoteSyncFailureDto shape', async () => {
    mockInvoke.mockResolvedValue([
      {
        itemId: 'remote-sale-1',
        action: 'upsert_sale',
        payload: '{"id":"remote-sale-1"}',
        attempts: 3,
        lastError: 'missing product sku-X',
        deadLettered: true,
      },
    ]);
    const failures = await listRemoteFailures();
    expect(failures).toHaveLength(1);
    const first = failures[0]!;
    expect(first.itemId).toBe('remote-sale-1');
    expect(first.deadLettered).toBe(true);
    expect(first.attempts).toBe(3);
  });

  it('requeueRemoteFailure invokes "requeue_remote_failure" with camelCase itemId arg', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await requeueRemoteFailure('remote-sale-1');
    expect(mockInvoke).toHaveBeenCalledWith('requeue_remote_failure', {
      args: { itemId: 'remote-sale-1' },
    });
  });

  it('propagates backend errors (does not swallow)', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('confirmDestructive must be true'));
    await expect(syncPull({ confirmDestructive: false })).rejects.toThrow(
      'confirmDestructive must be true',
    );
  });
});
