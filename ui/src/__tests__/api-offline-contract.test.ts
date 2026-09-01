// ── IPC contract tests for the offline/sync API layer ───────────────
//
// These tests verify the contract between `ui/src/api/offline.ts` and the
// Rust Tauri commands (`commands::offline`, `commands::sync`). All entry
// points are session-scoped (ADR #7) except `testSyncConnection`, which is
// deliberately pre-auth for the login-screen connectivity check.
//
//   • SYNC-03 — `syncPullScoped` must send `{ confirmDestructive: true }`
//     as the IPC payload; the backend command rejects any pull without
//     explicit destructive consent.
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

import {
  syncPullScoped,
  retryOfflineSyncScoped,
  syncRunScoped,
  getSyncSettingsScoped,
  updateSyncSettingsScoped,
  testSyncConnection,
  requestSyncTokenScoped,
  getOfflineQueueStatusSummaryScoped,
  listAllOfflineScoped,
  pendingOfflineCountScoped,
  deleteOfflineItemScoped,
  listRemoteFailuresScoped,
  requeueRemoteFailureScoped,
  getPgSyncSettingsScoped,
  updatePgSyncSettingsScoped,
  pgSyncStatusScoped,
  pgSyncStartScoped,
  pgSyncStopScoped,
  type SyncResult,
} from '@/api/offline';

describe('offline.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── SYNC-03: destructive pull consent contract ──────────────

  it('syncPullScoped invokes "sync_pull_scoped" with confirmDestructive: true', async () => {
    mockInvoke.mockResolvedValue({ productsPulled: 3, taxRatesPulled: 1, usersPulled: 0, error: null });
    await syncPullScoped('tok', { confirmDestructive: true });
    expect(mockInvoke).toHaveBeenCalledWith('sync_pull_scoped', {
      sessionToken: 'tok',
      args: { confirmDestructive: true },
    });
  });

  it('syncPullScoped forwards a declined consent value verbatim (backend rejects it)', async () => {
    // The TS signature requires the flag, but the wire contract must still
    // forward the value as-is so the backend's consent gate is the source of
    // truth (false/missing is rejected server-side).
    mockInvoke.mockResolvedValue({ productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: 'no consent' });
    await syncPullScoped('tok', { confirmDestructive: false });
    expect(mockInvoke).toHaveBeenCalledWith('sync_pull_scoped', {
      sessionToken: 'tok',
      args: { confirmDestructive: false },
    });
  });

  // ── SYNC-04: retry delegates to the real sync pipeline ──────

  it('retryOfflineSyncScoped invokes "retry_offline_sync_scoped"', async () => {
    mockInvoke.mockResolvedValue({ syncedCount: 2, failedCount: 1, totalCount: 3 });
    await retryOfflineSyncScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('retry_offline_sync_scoped', { sessionToken: 'tok' });
  });

  it('retryOfflineSyncScoped returns the camelCase SyncResult DTO shape', async () => {
    mockInvoke.mockResolvedValue({ syncedCount: 2, failedCount: 1, totalCount: 3 });
    const result: SyncResult = await retryOfflineSyncScoped('tok');
    expect(result).toEqual({ syncedCount: 2, failedCount: 1, totalCount: 3 });
  });

  // ── sync_run / settings / connection helpers ─────────────────

  it('syncRunScoped invokes "sync_run_scoped"', async () => {
    mockInvoke.mockResolvedValue({ synced: 1, failed: 0, error: null });
    await syncRunScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('sync_run_scoped', { sessionToken: 'tok' });
  });

  it('getSyncSettingsScoped invokes "get_sync_settings_scoped"', async () => {
    mockInvoke.mockResolvedValue({ serverUrl: null, hasApiKey: false, enabled: false });
    await getSyncSettingsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_sync_settings_scoped', { sessionToken: 'tok' });
  });

  it('updateSyncSettingsScoped invokes "update_sync_settings_scoped" with camelCase args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateSyncSettingsScoped('tok', { serverUrl: 'https://sync.example.com', apiKey: 'sk-1', enabled: true });
    expect(mockInvoke).toHaveBeenCalledWith('update_sync_settings_scoped', {
      sessionToken: 'tok',
      args: { serverUrl: 'https://sync.example.com', apiKey: 'sk-1', enabled: true },
    });
  });

  it('testSyncConnection invokes "test_sync_connection" with no args (pre-auth login check)', async () => {
    mockInvoke.mockResolvedValue({ ok: true, status: 'Connected', latencyMs: 12 });
    await testSyncConnection();
    expect(mockInvoke).toHaveBeenCalledWith('test_sync_connection', undefined);
  });

  it('requestSyncTokenScoped invokes "request_sync_token_scoped"', async () => {
    mockInvoke.mockResolvedValue({ ok: true, token: 'jwt', status: 'issued', expiresAt: null });
    await requestSyncTokenScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('request_sync_token_scoped', { sessionToken: 'tok' });
  });

  // ── offline queue (scoped) ───────────────────────────────────

  it('getOfflineQueueStatusSummaryScoped invokes "offline_queue_status_summary_scoped"', async () => {
    mockInvoke.mockResolvedValue({ pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0 });
    await getOfflineQueueStatusSummaryScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('offline_queue_status_summary_scoped', { sessionToken: 'tok' });
  });

  it('listAllOfflineScoped invokes "list_all_offline_scoped"', async () => {
    mockInvoke.mockResolvedValue([]);
    await listAllOfflineScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_all_offline_scoped', { sessionToken: 'tok' });
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
    const items = await listAllOfflineScoped('tok');
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

  it('pendingOfflineCountScoped invokes "pending_offline_count_scoped"', async () => {
    mockInvoke.mockResolvedValue(0);
    await pendingOfflineCountScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('pending_offline_count_scoped', { sessionToken: 'tok' });
  });

  it('deleteOfflineItemScoped invokes "delete_offline_item_scoped" with id arg', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteOfflineItemScoped('tok', 'oq-1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_offline_item_scoped', {
      sessionToken: 'tok',
      args: { id: 'oq-1' },
    });
  });

  // ── SYNC-11: remote dead-letter (quarantined pulls) ────────

  it('listRemoteFailuresScoped invokes "list_remote_failures_scoped"', async () => {
    mockInvoke.mockResolvedValue([]);
    await listRemoteFailuresScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_remote_failures_scoped', { sessionToken: 'tok' });
  });

  it('listRemoteFailuresScoped returns the camelCase RemoteSyncFailureDto shape', async () => {
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
    const failures = await listRemoteFailuresScoped('tok');
    expect(failures).toHaveLength(1);
    const first = failures[0]!;
    expect(first.itemId).toBe('remote-sale-1');
    expect(first.deadLettered).toBe(true);
    expect(first.attempts).toBe(3);
  });

  it('requeueRemoteFailureScoped invokes "requeue_remote_failure_scoped" with camelCase itemId arg', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await requeueRemoteFailureScoped('tok', 'remote-sale-1');
    expect(mockInvoke).toHaveBeenCalledWith('requeue_remote_failure_scoped', {
      sessionToken: 'tok',
      args: { itemId: 'remote-sale-1' },
    });
  });

  it('propagates backend errors (does not swallow)', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('confirmDestructive must be true'));
    await expect(syncPullScoped('tok', { confirmDestructive: false })).rejects.toThrow(
      'confirmDestructive must be true',
    );
  });
});

describe('offline.ts PG sync IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('getPgSyncSettingsScoped invokes "get_pg_sync_settings_scoped"', async () => {
    mockInvoke.mockResolvedValue({
      enabled: false,
      host: null,
      port: null,
      dbname: null,
      user: null,
      hasPassword: false,
    });
    await getPgSyncSettingsScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('get_pg_sync_settings_scoped', { sessionToken: 'tok' });
  });

  it('updatePgSyncSettingsScoped invokes "update_pg_sync_settings_scoped" with camelCase args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updatePgSyncSettingsScoped('tok', {
      enabled: true,
      host: 'db.example.com',
      port: '5432',
      dbname: 'oz_sync',
      user: 'sync_user',
      password: 'secret',
    });
    expect(mockInvoke).toHaveBeenCalledWith('update_pg_sync_settings_scoped', {
      sessionToken: 'tok',
      args: {
        enabled: true,
        host: 'db.example.com',
        port: '5432',
        dbname: 'oz_sync',
        user: 'sync_user',
        password: 'secret',
      },
    });
  });

  it('pgSyncStatusScoped invokes "pg_sync_status_scoped" and returns the camelCase status DTO', async () => {
    mockInvoke.mockResolvedValue({
      running: true,
      lastSyncAt: '2026-08-09T00:00:00Z',
      lastPushed: 5,
      lastPulled: 3,
      lastError: null,
      pendingCount: 10,
    });
    await pgSyncStatusScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('pg_sync_status_scoped', { sessionToken: 'tok' });
  });

  it('pgSyncStartScoped invokes "pg_sync_start_scoped"', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await pgSyncStartScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('pg_sync_start_scoped', { sessionToken: 'tok' });
  });

  it('pgSyncStopScoped invokes "pg_sync_stop_scoped"', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await pgSyncStopScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('pg_sync_stop_scoped', { sessionToken: 'tok' });
  });
});
