// ── Offline Queue & Cloud Sync ────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

// ── Offline Queue ────────────────────────────────────────────────

/** An item in the offline action queue. */
export interface OfflineQueueItemDto {
  id: string;
  action: string;
  /**
   * JSON-serialized action payload (SYNC-11: present in the Rust
   * `OfflineQueueItemDto` serializer — must not be dropped here).
   */
  payload: string;
  status: string;
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  syncedAt: string | null;
  /** Tenant / store ID for multi-store isolation (OFF-09). */
  tenantId: string;
  /** Sync priority tier: "critical" | "normal" | "low" (OFF-09). */
  priority: string;
}

/** Arguments for enqueuing an offline action. */
export interface EnqueueOfflineArgs {
  action: string;
  payload: string;
  /** Optional tenant / store ID (OFF-09). Defaults to "default". */
  tenantId?: string;
  /** Optional sync priority tier (OFF-09): "critical" | "normal" | "low". */
  priority?: 'critical' | 'normal' | 'low';
}

/**
 * Result of a manual retry of the pending offline queue.
 *
 * SYNC-04 / SYNC-11: matches the Rust `SyncResult` DTO exactly
 * (camelCase `syncedCount` / `failedCount` / `totalCount`).
 */
export interface SyncResult {
  /** Number of items successfully synced. */
  syncedCount: number;
  /** Number of items that failed to sync. */
  failedCount: number;
  /** Total number of items that were attempted. */
  totalCount: number;
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

/**
 * Retry syncing all pending offline actions through the real cloud sync
 * pipeline (SYNC-04 — no longer a placeholder).
 */
export const retryOfflineSync = (): Promise<SyncResult> =>
  loggedInvoke<SyncResult>('retry_offline_sync');

/** Delete an offline queue item by its identifier. */
export const deleteOfflineItem = (id: string): Promise<void> =>
  loggedInvoke('delete_offline_item', { args: { id } });

// ── Remote Dead-Letter (quarantined pulls) ────────────────────────

/**
 * A remote sync item that repeatedly failed to apply during a pull and
 * was quarantined in `sync_remote_failures` (SYNC-09 / SYNC-11).
 *
 * Matches the Rust `RemoteSyncFailureDto` serializer exactly
 * (camelCase `itemId` / `deadLettered`).
 */
export interface RemoteSyncFailureDto {
  /** Remote item identifier. */
  itemId: string;
  /** Remote action name. */
  action: string;
  /** Original payload retained for operator inspection. */
  payload: string;
  /** Number of failed application attempts. */
  attempts: number;
  /** Most recent application error. */
  lastError: string;
  /** Whether retry is exhausted and the item is quarantined. */
  deadLettered: boolean;
}

/**
 * List remote items quarantined after repeated pull-application failures.
 *
 * The listing is not session-scoped: the sync daemon runs server-side in
 * both clients, so an operator with backend access can inspect the dead
 * letter without an active POS session (SYNC-11).
 */
export const listRemoteFailures = (): Promise<RemoteSyncFailureDto[]> =>
  loggedInvoke<RemoteSyncFailureDto[]>('list_remote_failures');

/**
 * Requeue a dead-lettered remote item so the next sync cycle retries it.
 *
 * Operators call this after remediating the item's source (e.g. creating
 * the missing product a remote sale referenced). Returns `NotFound` for
 * ids that are not currently dead-lettered, so a mistyped id is never a
 * silent no-op.
 */
export const requeueRemoteFailure = (itemId: string): Promise<void> =>
  loggedInvoke('requeue_remote_failure', { args: { itemId } });

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
  /** The server rejected the attempt because this tenant is on the free
   *  plan (ADR sync-plan-gating) — show an upgrade prompt. */
  planRequired?: boolean;
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

// ── PostgreSQL sync settings & daemon ──────────────────────────────

/** PostgreSQL sync configuration (the PG transport's connection settings). */
export interface PgSyncSettingsDto {
  enabled: boolean;
  host: string | null;
  port: string | null;
  dbname: string | null;
  user: string | null;
  hasPassword: boolean;
}

/** Arguments for updating PostgreSQL sync settings. `password` is only
 *  written when provided — omit it to keep the stored secret untouched
 *  (the UI sends `undefined` for the unmasked field). */
export interface UpdatePgSyncSettingsArgs {
  enabled: boolean;
  host?: string | null;
  port?: string | null;
  dbname?: string | null;
  user?: string | null;
  password?: string | null;
}

/** Snapshot of the PG sync daemon's state (camelCase mirror of the Rust
 *  `PgDaemonStatus` serializer). */
export interface PgDaemonStatusDto {
  running: boolean;
  lastSyncAt: string | null;
  lastPushed: number;
  lastPulled: number;
  lastError: string | null;
  pendingCount: number;
}

/** Get the PostgreSQL sync settings. */
export const getPgSyncSettings = (): Promise<PgSyncSettingsDto> =>
  loggedInvoke<PgSyncSettingsDto>('get_pg_sync_settings');

/** Update the PostgreSQL sync settings. */
export const updatePgSyncSettings = (args: UpdatePgSyncSettingsArgs): Promise<void> =>
  loggedInvoke<void>('update_pg_sync_settings', { args });

/** Get the PG sync daemon's current status. */
export const pgSyncStatus = (): Promise<PgDaemonStatusDto> =>
  loggedInvoke<PgDaemonStatusDto>('pg_sync_status');

/** Start the background PG sync daemon (no-op when already running). */
export const pgSyncStart = (): Promise<void> => loggedInvoke<void>('pg_sync_start');

/** Stop the background PG sync daemon (no-op when not running). */
export const pgSyncStop = (): Promise<void> => loggedInvoke<void>('pg_sync_stop');

/**
 * Arguments for a destructive snapshot pull.
 *
 * SYNC-03: `confirmDestructive` must be `true` for the backend command
 * to proceed — it rejects any call without explicit user consent.
 */
export interface SyncPullArgs {
  confirmDestructive: boolean;
}

/**
 * Pull data (products, tax rates, users) from the cloud server.
 *
 * SYNC-03: the destructive consent is part of the IPC contract — the
 * caller must pass `{ confirmDestructive: true }` after showing a
 * confirmation dialog.
 */
export const syncPull = (args: SyncPullArgs): Promise<PullResult> =>
  loggedInvoke<PullResult>('sync_pull', { args });

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
