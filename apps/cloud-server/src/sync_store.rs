//! Sync data-store abstraction for the cloud server's sync function.
//!
//! This is the foundation of Phase 1.2 in `unify-auth-and-sync.md`: the
//! whole POS data layer ([`oz_core::Store`]) is a synchronous `rusqlite`
//! borrow-wrapper used by desktop, tablet, *and* cloud clients, so it cannot
//! be rewritten to Postgres. The cloud server therefore needs a **parallel
//! async data layer** covering only the surface the sync function touches:
//!
//! - `offline_queue` (push / pull / pending count)
//! - `tenant_plans` (plan gating)
//! - `products` / `tax_rates` / `users` (snapshot reference data)
//!
//! [`SyncStore`] is a small enum with one variant per backend so the HTTP
//! handlers stay backend-agnostic: SQLite (local dev / single-node) and
//! Postgres (Northflank cloud). Both variants implement the exact same
//! surface, so switching backend is a data-source decision, not a code-path
//! fork.
//!
//! # Type parity
//!
//! The Postgres port (`20260813_init.pg.sql`) maps SQLite `INTEGER` to
//! `BIGINT` **including boolean columns** (`track_serial`, `is_active`,
//! `is_default`, `is_inclusive`). The Postgres path therefore reads those
//! as `i64` and treats `0` as `false`, anything non-zero as `true` — the
//! same 0/1 convention SQLite uses.

use std::sync::Arc;

use deadpool_postgres::Pool;
use rusqlite::{Connection, params};
use tokio::sync::Mutex;
use tokio_postgres::error::SqlState;

use oz_core::TenantPlan;
use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus, SyncPriority};
use platform_sync::transport::PushOutcome;

/// Open a Postgres connection scoped to `tenant_id` for RLS enforcement.
///
/// Every Postgres branch below opens a transaction and sets the
/// `oz.tenant_id` GUC **locally** (`set_config(..., is_local := true)`), so
/// it auto-resets when the transaction ends — a leaked session-level
/// setting on a recycled pooled connection could expose the previous
/// borrower's tenant once RLS is FORCEd (see `scripts/rls-cutover.sql`).
/// While the app still connects as the table owner (which bypasses RLS)
/// this is a no-op; at cutover it becomes the per-request `SET LOCAL
/// oz.tenant_id` the `tenant_isolation` policy keys on.
///
/// Read paths rely on drop-to-roll-back; the write path (`push_item`)
/// commits explicitly.
///
/// The sync function's data backend.
///
/// [`SyncStore::Sqlite`] wraps the shared SQLite connection behind its
/// existing `Arc<Mutex<>>` (the same connection the REST API uses).
/// [`SyncStore::Postgres`] wraps a `deadpool_postgres::Pool`.
#[derive(Clone)]
pub enum SyncStore {
    /// Local SQLite backend (single-node dev / tests).
    Sqlite(Arc<Mutex<Connection>>),
    /// Postgres backend (Northflank cloud, Phase 1.2).
    Postgres(Pool),
}

impl SyncStore {
    /// Build a SQLite-backed store from the shared connection.
    pub fn sqlite(conn: Arc<Mutex<Connection>>) -> Self {
        Self::Sqlite(conn)
    }

    /// Build a Postgres-backed store from a connection pool.
    pub fn postgres(pool: Pool) -> Self {
        Self::Postgres(pool)
    }

    /// Read a tenant's sync plan, or `None` when the tenant has no row yet.
    ///
    /// Mirrors `oz_core::Store::get_tenant_plan` (missing row → `None`;
    /// unknown plan string degrades to `free`).
    pub async fn get_tenant_plan(&self, tenant_id: &str) -> Result<Option<TenantPlan>, String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                oz_core::Store::new(&conn)
                    .get_tenant_plan(tenant_id)
                    .map_err(|e| e.to_string())
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let mut tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                let row = tx
                    .query_opt(
                        "SELECT plan FROM tenant_plans WHERE tenant_id = $1",
                        &[&tenant_id],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                row.map(|r| {
                    let plan: String = r.try_get(0).map_err(|e| e.to_string())?;
                    Ok(TenantPlan::from_db(&plan))
                })
                .transpose()
            }
        }
    }

    /// Persist one offline queue item for the authenticated tenant.
    ///
    /// Returns the per-item outcome: `Accepted`, or `Rejected` for a
    /// duplicate id / database error. Only backend-connection failures
    /// (Postgres pool exhaustion) surface as `Err`, which the handler maps
    /// to a 500.
    pub async fn push_item(
        &self,
        item: &OfflineQueueItem,
        tenant_id: &str,
    ) -> Result<PushOutcome, String> {
        let status = OfflineQueueStatus::Pending.as_stored_str();
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                match conn.execute(
                    "INSERT INTO offline_queue (id, action, payload, status, retry_count, \
                     last_error, created_at, synced_at, tenant_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        item.id,
                        item.action,
                        item.payload,
                        status,
                        item.retry_count,
                        item.last_error,
                        item.created_at,
                        item.synced_at,
                        tenant_id,
                    ],
                ) {
                    Ok(_) => Ok(PushOutcome::Accepted),
                    Err(e) if e.to_string().contains("UNIQUE") => Ok(PushOutcome::Rejected {
                        reason: format!("duplicate id: {}", item.id),
                    }),
                    Err(e) => Ok(PushOutcome::Rejected {
                        reason: format!("database error: {e}"),
                    }),
                }
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[
                    &item.id,
                    &item.action,
                    &item.payload,
                    &status,
                    &item.retry_count,
                    &item.last_error,
                    &item.created_at,
                    &item.synced_at,
                    &tenant_id,
                ];
                let outcome = match tx
                    .execute(
                        "INSERT INTO offline_queue (id, action, payload, status, retry_count, \
                     last_error, created_at, synced_at, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                        params,
                    )
                    .await
                {
                    Ok(_) => Ok(PushOutcome::Accepted),
                    Err(e)
                        if e.as_db_error()
                            .map(|d| d.code() == &SqlState::UNIQUE_VIOLATION)
                            .unwrap_or(false) =>
                    {
                        Ok(PushOutcome::Rejected {
                            reason: format!("duplicate id: {}", item.id),
                        })
                    }
                    Err(e) => Ok(PushOutcome::Rejected {
                        reason: format!("database error: {e}"),
                    }),
                };
                // The write path must COMMIT (drop would roll back the insert).
                tx.commit().await.map_err(|e| e.to_string())?;
                outcome
            }
        }
    }

    /// Return the oldest retained `created_at` for a tenant, if any.
    ///
    /// Used for the P-1 anchor-expiry check. Errors are silently swallowed
    /// (returned as `None`) to match the historical SQLite behaviour — a
    /// failed min-scan degrades to "no anchor check" rather than failing
    /// the pull.
    pub async fn oldest_created_at(&self, tenant_id: &str) -> Option<String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                conn.query_row(
                    "SELECT MIN(created_at) FROM offline_queue WHERE tenant_id = ?1",
                    params![tenant_id],
                    |row| row.get(0),
                )
                .ok()
                .flatten()
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.ok()?;
                let tx = client.transaction().await.ok()?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .ok()?;
                tx.query_opt(
                    "SELECT MIN(created_at) FROM offline_queue WHERE tenant_id = $1",
                    &[&tenant_id],
                )
                .await
                .ok()
                .flatten()
                .and_then(|r| r.try_get::<_, Option<String>>(0).ok().flatten())
            }
        }
    }

    /// Fetch up to `limit` offline queue items for a tenant, ordered by
    /// `(created_at ASC, id ASC)`, respecting an optional `since` anchor and
    /// an optional `(cursor_ts, cursor_id)` pagination cursor.
    pub async fn pull_items(
        &self,
        tenant_id: &str,
        since: Option<&str>,
        cursor: Option<(&str, &str)>,
        limit: i64,
    ) -> Result<Vec<OfflineQueueItem>, String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                sqlite_pull_items(&conn, tenant_id, since, cursor, limit)
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let mut tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                pg_pull_items(&mut tx, tenant_id, since, cursor, limit).await
            }
        }
    }

    /// Number of `pending` items for a tenant (status endpoint).
    pub async fn pending_count(&self, tenant_id: &str) -> i64 {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                conn.query_row(
                    "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending' AND tenant_id = ?1",
                    params![tenant_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
            }
            Self::Postgres(pool) => {
                let mut client = match pool.get().await {
                    Ok(c) => c,
                    Err(_) => return 0,
                };
                let tx = match client.transaction().await {
                    Ok(t) => t,
                    Err(_) => return 0,
                };
                if tx
                    .execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .is_err()
                {
                    return 0;
                }
                tx.query_one(
                    "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending' AND tenant_id = $1",
                    &[&tenant_id],
                )
                .await
                .map(|r| r.get::<_, i64>(0))
                .unwrap_or(0)
            }
        }
    }

    /// Number of distinct tenants in the queue (status endpoint).
    ///
    /// Deliberately NOT tenant-scoped: this is a global operator-facing
    /// aggregate, so it intentionally runs without the `oz.tenant_id` GUC.
    /// Once RLS is FORCEd at cutover (`scripts/rls-cutover.sql`), the policy
    /// hides every row from the app role and this counter reads 0 — the
    /// status endpoint keeps working, the number is simply not visible to
    /// the restricted role.
    pub async fn distinct_tenant_count(&self) -> i64 {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                conn.query_row(
                    "SELECT COUNT(DISTINCT tenant_id) FROM offline_queue",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
            }
            Self::Postgres(pool) => {
                let Ok(client) = pool.get().await else {
                    return 0;
                };
                client
                    .query_one("SELECT COUNT(DISTINCT tenant_id) FROM offline_queue", &[])
                    .await
                    .map(|r| r.get::<_, i64>(0))
                    .unwrap_or(0)
            }
        }
    }

    /// Product rows for a tenant's snapshot (reference-data baseline).
    pub async fn snapshot_products(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                sqlite_snapshot_products(&conn, tenant_id)
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let mut tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                pg_snapshot_products(&mut tx, tenant_id).await
            }
        }
    }

    /// Tax-rate rows for a tenant's snapshot.
    pub async fn snapshot_tax_rates(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                sqlite_snapshot_tax_rates(&conn, tenant_id)
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let mut tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                pg_snapshot_tax_rates(&mut tx, tenant_id).await
            }
        }
    }

    /// User rows (without `pin_hash`) for a tenant's snapshot.
    pub async fn snapshot_users(&self, tenant_id: &str) -> Result<Vec<serde_json::Value>, String> {
        match self {
            Self::Sqlite(conn) => {
                let conn = conn.lock().await;
                sqlite_snapshot_users(&conn, tenant_id)
            }
            Self::Postgres(pool) => {
                let mut client = pool.get().await.map_err(|e| e.to_string())?;
                let mut tx = client.transaction().await.map_err(|e| e.to_string())?;
                tx.execute("SELECT set_config('oz.tenant_id', $1, true)", &[&tenant_id])
                    .await
                    .map_err(|e| e.to_string())?;
                pg_snapshot_users(&mut tx, tenant_id).await
            }
        }
    }
}

// ── SQLite implementations ────────────────────────────────────────────────
//
// These are faithful copies of the SQL that previously lived inline in the
// handlers (sync_api.rs). Behaviour — including SYNC-10's fail-loud row
// decode and the metric increment — is preserved exactly.

/// Pull rows via SQLite, preserving the three query shapes (cursor / since /
/// bare) and the fail-loud row decode (SYNC-10).
fn sqlite_pull_items(
    conn: &Connection,
    tenant_id: &str,
    since: Option<&str>,
    cursor: Option<(&str, &str)>,
    limit: i64,
) -> Result<Vec<OfflineQueueItem>, String> {
    const SELECT: &str = "SELECT id, action, payload, status, retry_count, last_error, \
                          created_at, synced_at, tenant_id, priority FROM offline_queue";

    let rows: Vec<rusqlite::Result<OfflineQueueItem>> = if let Some((ts, cid)) = cursor {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT} WHERE tenant_id = ?1 AND created_at >= ?2 \
                 AND (created_at > ?3 OR (created_at = ?3 AND id > ?4)) \
                 ORDER BY created_at ASC, id ASC LIMIT ?5"
            ))
            .map_err(|e| e.to_string())?;
        stmt.query_map(
            params![tenant_id, since.unwrap_or(""), ts, cid, limit],
            sqlite_row_to_item,
        )
        .map_err(|e| e.to_string())?
        .collect()
    } else if let Some(since) = since {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT} WHERE created_at >= ?1 AND tenant_id = ?2 \
                 ORDER BY created_at ASC, id ASC LIMIT ?3"
            ))
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![since, tenant_id, limit], sqlite_row_to_item)
            .map_err(|e| e.to_string())?
            .collect()
    } else {
        let mut stmt = conn
            .prepare(&format!(
                "{SELECT} WHERE tenant_id = ?1 ORDER BY created_at ASC, id ASC LIMIT ?2"
            ))
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![tenant_id, limit], sqlite_row_to_item)
            .map_err(|e| e.to_string())?
            .collect()
    };

    sqlite_collect_pull_rows(rows.into_iter(), tenant_id)
}

/// Collect SQLite pull rows, failing loudly on decode errors (SYNC-10).
fn sqlite_collect_pull_rows(
    rows: impl Iterator<Item = rusqlite::Result<OfflineQueueItem>>,
    tenant_id: &str,
) -> Result<Vec<OfflineQueueItem>, String> {
    let mut items = Vec::with_capacity(rows.size_hint().0);
    for row in rows {
        match row {
            Ok(item) => items.push(item),
            Err(e) => {
                crate::metrics::SYNC_PULL_ROW_DECODE_FAILURES_TOTAL.inc();
                tracing::error!(tenant_id, error = %e, "pull: row decode failed — returning 500");
                return Err(format!("offline_queue row decode failed: {e}"));
            }
        }
    }
    Ok(items)
}

/// Convert a SQLite row to an `OfflineQueueItem`.
fn sqlite_row_to_item(row: &rusqlite::Row) -> rusqlite::Result<OfflineQueueItem> {
    let status_str: String = row.get("status")?;
    Ok(OfflineQueueItem {
        id: row.get("id")?,
        action: row.get("action")?,
        payload: row.get("payload")?,
        status: OfflineQueueStatus::from_stored_str(&status_str)
            .unwrap_or(OfflineQueueStatus::Pending),
        retry_count: row.get("retry_count")?,
        last_error: row.get("last_error")?,
        created_at: row.get("created_at")?,
        synced_at: row.get("synced_at")?,
        tenant_id: row.get("tenant_id")?,
        priority: row
            .get::<_, i32>("priority")
            .map(SyncPriority::from)
            .unwrap_or(SyncPriority::Normal),
    })
}

fn sqlite_snapshot_products(
    conn: &Connection,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, \
             updated_at, price_updated_at, track_serial, store_id, brand, rack_location, notes, \
             unit, is_active FROM products WHERE tenant_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map(params![tenant_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>("id")?,
            "sku": row.get::<_, String>("sku")?,
            "name": row.get::<_, String>("name")?,
            "price_minor": row.get::<_, i64>("price_minor")?,
            "currency": row.get::<_, String>("currency")?,
            "category_id": row.get::<_, Option<String>>("category_id")?,
            "barcode": row.get::<_, Option<String>>("barcode")?,
            "created_at": row.get::<_, String>("created_at")?,
            "updated_at": row.get::<_, String>("updated_at")?,
            "price_updated_at": row.get::<_, String>("price_updated_at")?,
            "track_serial": row.get::<_, bool>("track_serial")?,
            "store_id": row.get::<_, Option<String>>("store_id")?,
            "brand": row.get::<_, Option<String>>("brand")?,
            "rack_location": row.get::<_, Option<String>>("rack_location")?,
            "notes": row.get::<_, Option<String>>("notes")?,
            "unit": row.get::<_, Option<String>>("unit")?,
            "is_active": row.get::<_, bool>("is_active")?
        }))
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("product row decode failed: {e}"))
}

fn sqlite_snapshot_tax_rates(
    conn: &Connection,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at \
             FROM tax_rates WHERE tenant_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map(params![tenant_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>("id")?,
            "name": row.get::<_, String>("name")?,
            "rate_bps": row.get::<_, i64>("rate_bps")?,
            "is_default": row.get::<_, bool>("is_default")?,
            "is_inclusive": row.get::<_, bool>("is_inclusive")?,
            "created_at": row.get::<_, Option<String>>("created_at")?,
            "updated_at": row.get::<_, Option<String>>("updated_at")?
        }))
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("tax rate row decode failed: {e}"))
}

fn sqlite_snapshot_users(
    conn: &Connection,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, username, display_name, role_id, is_active, created_at, updated_at \
             FROM users WHERE tenant_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    stmt.query_map(params![tenant_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>("id")?,
            "username": row.get::<_, String>("username")?,
            "display_name": row.get::<_, String>("display_name")?,
            "role_id": row.get::<_, String>("role_id")?,
            "is_active": row.get::<_, bool>("is_active")?,
            "created_at": row.get::<_, Option<String>>("created_at")?,
            "updated_at": row.get::<_, Option<String>>("updated_at")?
        }))
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("user row decode failed: {e}"))
}

// ── Postgres implementations ──────────────────────────────────────────────

/// Pull rows via Postgres, mirroring the three SQLite query shapes with
/// `$n` placeholders. Row decode failures fail the whole pull (SYNC-10).
///
/// Generic over `deadpool_postgres::GenericClient` so the same code runs on
/// the tenant-scoped transaction (the `oz.tenant_id` GUC from [`tenant_tx`])
/// or on a plain pooled client (tests).
async fn pg_pull_items(
    client: &mut impl deadpool_postgres::GenericClient,
    tenant_id: &str,
    since: Option<&str>,
    cursor: Option<(&str, &str)>,
    limit: i64,
) -> Result<Vec<OfflineQueueItem>, String> {
    const SELECT: &str = "SELECT id, action, payload, status, retry_count, last_error, \
                          created_at, synced_at, tenant_id, priority FROM offline_queue";

    let rows = if let Some((ts, cid)) = cursor {
        client
            .query(
                &format!(
                    "{SELECT} WHERE tenant_id = $1 AND created_at >= $2 \
                     AND (created_at > $3 OR (created_at = $3 AND id > $4)) \
                     ORDER BY created_at ASC, id ASC LIMIT $5"
                ),
                &[&tenant_id, &since.unwrap_or(""), &ts, &cid, &limit],
            )
            .await
            .map_err(|e| e.to_string())?
    } else if let Some(since) = since {
        client
            .query(
                &format!(
                    "{SELECT} WHERE created_at >= $1 AND tenant_id = $2 \
                     ORDER BY created_at ASC, id ASC LIMIT $3"
                ),
                &[&since, &tenant_id, &limit],
            )
            .await
            .map_err(|e| e.to_string())?
    } else {
        client
            .query(
                &format!("{SELECT} WHERE tenant_id = $1 ORDER BY created_at ASC, id ASC LIMIT $2"),
                &[&tenant_id, &limit],
            )
            .await
            .map_err(|e| e.to_string())?
    };

    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        match pg_row_to_item(row) {
            Ok(item) => items.push(item),
            Err(e) => {
                crate::metrics::SYNC_PULL_ROW_DECODE_FAILURES_TOTAL.inc();
                tracing::error!(tenant_id, error = %e, "pull: row decode failed — returning 500");
                return Err(format!("offline_queue row decode failed: {e}"));
            }
        }
    }
    Ok(items)
}

/// Convert a Postgres row to an `OfflineQueueItem`.
///
/// `priority` is `BIGINT` in Postgres (SQLite stored it as `INTEGER` read as
/// `i32`), so it is narrowed through [`SyncPriority::from`] the same way.
fn pg_row_to_item(row: &tokio_postgres::Row) -> Result<OfflineQueueItem, String> {
    let status_str: String = row.try_get("status").map_err(|e| e.to_string())?;
    let priority: i64 = row.try_get("priority").map_err(|e| e.to_string())?;
    Ok(OfflineQueueItem {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        action: row.try_get("action").map_err(|e| e.to_string())?,
        payload: row.try_get("payload").map_err(|e| e.to_string())?,
        status: OfflineQueueStatus::from_stored_str(&status_str)
            .unwrap_or(OfflineQueueStatus::Pending),
        retry_count: row.try_get("retry_count").map_err(|e| e.to_string())?,
        last_error: row.try_get("last_error").map_err(|e| e.to_string())?,
        created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
        synced_at: row.try_get("synced_at").map_err(|e| e.to_string())?,
        tenant_id: row.try_get("tenant_id").map_err(|e| e.to_string())?,
        priority: SyncPriority::from(priority as i32),
    })
}

/// Read a `BIGINT` boolean-ish column as `bool` (0 → false, else true).
fn pg_bool(row: &tokio_postgres::Row, column: &str) -> Result<bool, String> {
    let v: i64 = row.try_get(column).map_err(|e| e.to_string())?;
    Ok(v != 0)
}

async fn pg_snapshot_products(
    client: &mut impl deadpool_postgres::GenericClient,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = client
        .query(
            "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, \
             updated_at, price_updated_at, track_serial, store_id, brand, rack_location, notes, \
             unit, is_active FROM products WHERE tenant_id = $1",
            &[&tenant_id],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(
            serde_json::json!({
                "id": row.try_get::<_, String>("id").map_err(|e| e.to_string())?,
                "sku": row.try_get::<_, String>("sku").map_err(|e| e.to_string())?,
                "name": row.try_get::<_, String>("name").map_err(|e| e.to_string())?,
                "price_minor": row.try_get::<_, i64>("price_minor").map_err(|e| e.to_string())?,
                "currency": row.try_get::<_, String>("currency").map_err(|e| e.to_string())?,
                "category_id": row.try_get::<_, Option<String>>("category_id").map_err(|e| e.to_string())?,
                "barcode": row.try_get::<_, Option<String>>("barcode").map_err(|e| e.to_string())?,
                "created_at": row.try_get::<_, String>("created_at").map_err(|e| e.to_string())?,
                "updated_at": row.try_get::<_, String>("updated_at").map_err(|e| e.to_string())?,
                "price_updated_at": row.try_get::<_, Option<String>>("price_updated_at").map_err(|e| e.to_string())?.unwrap_or_default(),
                "track_serial": pg_bool(row, "track_serial")?,
                "store_id": row.try_get::<_, Option<String>>("store_id").map_err(|e| e.to_string())?,
                "brand": row.try_get::<_, Option<String>>("brand").map_err(|e| e.to_string())?,
                "rack_location": row.try_get::<_, Option<String>>("rack_location").map_err(|e| e.to_string())?,
                "notes": row.try_get::<_, Option<String>>("notes").map_err(|e| e.to_string())?,
                "unit": row.try_get::<_, Option<String>>("unit").map_err(|e| e.to_string())?,
                "is_active": pg_bool(row, "is_active")?,
            }),
        );
    }
    Ok(out)
}

async fn pg_snapshot_tax_rates(
    client: &mut impl deadpool_postgres::GenericClient,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = client
        .query(
            "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at \
             FROM tax_rates WHERE tenant_id = $1",
            &[&tenant_id],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(
            serde_json::json!({
                "id": row.try_get::<_, String>("id").map_err(|e| e.to_string())?,
                "name": row.try_get::<_, String>("name").map_err(|e| e.to_string())?,
                "rate_bps": row.try_get::<_, i64>("rate_bps").map_err(|e| e.to_string())?,
                "is_default": pg_bool(row, "is_default")?,
                "is_inclusive": pg_bool(row, "is_inclusive")?,
                "created_at": row.try_get::<_, Option<String>>("created_at").map_err(|e| e.to_string())?,
                "updated_at": row.try_get::<_, Option<String>>("updated_at").map_err(|e| e.to_string())?,
            }),
        );
    }
    Ok(out)
}

async fn pg_snapshot_users(
    client: &mut impl deadpool_postgres::GenericClient,
    tenant_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = client
        .query(
            "SELECT id, username, display_name, role_id, is_active, created_at, updated_at \
             FROM users WHERE tenant_id = $1",
            &[&tenant_id],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(
            serde_json::json!({
                "id": row.try_get::<_, String>("id").map_err(|e| e.to_string())?,
                "username": row.try_get::<_, String>("username").map_err(|e| e.to_string())?,
                "display_name": row.try_get::<_, String>("display_name").map_err(|e| e.to_string())?,
                "role_id": row.try_get::<_, String>("role_id").map_err(|e| e.to_string())?,
                "is_active": pg_bool(row, "is_active")?,
                "created_at": row.try_get::<_, Option<String>>("created_at").map_err(|e| e.to_string())?,
                "updated_at": row.try_get::<_, Option<String>>("updated_at").map_err(|e| e.to_string())?,
            }),
        );
    }
    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Arc<Mutex<Connection>> {
        Arc::new(Mutex::new(oz_core::migrations::fresh_db()))
    }

    fn sample_item(id: &str) -> OfflineQueueItem {
        OfflineQueueItem {
            id: id.to_owned(),
            action: "complete_sale".into(),
            payload: r#"{"total":100}"#.into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            tenant_id: "default".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            synced_at: None,
            priority: SyncPriority::Normal,
        }
    }

    /// The SQLite backend must round-trip a push → pull → snapshot through
    /// the same abstraction the handlers use, proving backend parity is
    /// exercised in unit tests (the full Postgres path is covered by the
    /// integration test below).
    #[tokio::test]
    async fn sqlite_backend_push_pull_plan_snapshot_roundtrip() {
        let conn = fresh_db();
        let store = SyncStore::sqlite(conn.clone());

        // Plan gating: no row → None, then a set row → Pro.
        assert_eq!(store.get_tenant_plan("tenant-a").await.unwrap(), None);
        {
            let conn = conn.lock().await;
            oz_core::Store::new(&conn)
                .set_tenant_plan("tenant-a", TenantPlan::Pro)
                .unwrap();
        }
        assert_eq!(
            store.get_tenant_plan("tenant-a").await.unwrap(),
            Some(TenantPlan::Pro)
        );

        // Push two items, one a duplicate.
        let item = sample_item("id-1");
        assert!(matches!(
            store.push_item(&item, "tenant-a").await.unwrap(),
            PushOutcome::Accepted
        ));
        assert!(matches!(
            store.push_item(&item, "tenant-a").await.unwrap(),
            PushOutcome::Rejected { .. }
        ));

        // Pull returns the one accepted item.
        let items = store
            .pull_items("tenant-a", Some("2026-01-01T00:00:00Z"), None, 501)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "id-1");

        // Status counts reflect the queue.
        assert_eq!(store.pending_count("tenant-a").await, 1);
        assert_eq!(store.distinct_tenant_count().await, 1);

        // Snapshot is empty but well-formed for a tenant with no products.
        assert_eq!(store.snapshot_products("tenant-a").await.unwrap().len(), 0);
        assert_eq!(store.snapshot_tax_rates("tenant-a").await.unwrap().len(), 0);
        assert_eq!(store.snapshot_users("tenant-a").await.unwrap().len(), 0);
    }

    /// Duplicate-id detection for the Postgres path keys on SQLSTATE 23505,
    /// not on the error message (unlike SQLite's "UNIQUE" substring).
    #[tokio::test]
    async fn sqlite_duplicate_rejection_uses_unique_substring() {
        let conn = fresh_db();
        let store = SyncStore::sqlite(conn);
        let item = sample_item("dup-id");
        store.push_item(&item, "default").await.unwrap();
        match store.push_item(&item, "default").await.unwrap() {
            PushOutcome::Rejected { reason } => {
                assert!(reason.contains("duplicate id: dup-id"), "got: {reason}");
            }
            other => panic!("expected Rejected, got: {other:?}"),
        }
    }

    /// Integration test against a live Postgres instance (the same Docker
    /// service `db.rs` uses, port 15432). Skips when unreachable, so the
    /// suite stays green on machines without a running Postgres.
    #[tokio::test]
    async fn pg_integration_push_pull_plan_snapshot_roundtrip() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

        let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
            Ok(crate::db::DbPool::Postgres(pool)) => pool,
            Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
            Err(e) => {
                eprintln!("PG sync-store integration test skipped: {e}");
                return;
            }
        };

        let tenant = format!("pg-sync-store-test-{}", uuid::Uuid::now_v7());
        let store = SyncStore::postgres(pool.clone());

        // Seed a plan and exercise every method end-to-end.
        {
            let client = pool.get().await.unwrap();
            client
                .execute(
                    "INSERT INTO tenant_plans (tenant_id, plan, updated_at) VALUES ($1, 'pro', now()::text)
                     ON CONFLICT (tenant_id) DO UPDATE SET plan = 'pro'",
                    &[&tenant],
                )
                .await
                .unwrap();
        }

        assert_eq!(
            store.get_tenant_plan(&tenant).await.unwrap(),
            Some(TenantPlan::Pro)
        );

        let mut item = sample_item("pg-item-1");
        item.tenant_id = tenant.clone();
        assert!(matches!(
            store.push_item(&item, &tenant).await.unwrap(),
            PushOutcome::Accepted
        ));
        assert!(matches!(
            store.push_item(&item, &tenant).await.unwrap(),
            PushOutcome::Rejected { .. }
        ));

        let items = store
            .pull_items(&tenant, Some("2026-01-01T00:00:00Z"), None, 501)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "pg-item-1");
        assert_eq!(items[0].tenant_id, tenant);

        assert_eq!(store.pending_count(&tenant).await, 1);
        assert!(store.distinct_tenant_count().await >= 1);

        // Seed reference data with boolean columns so the snapshot path —
        // including the Postgres BIGINT(0/1) → bool mapping — is exercised
        // against a live database, not just the empty-set fast path.
        {
            let client = pool.get().await.unwrap();
            let role_id = format!("role-{tenant}");
            client
                .execute(
                    "INSERT INTO roles (id, name, permissions) VALUES ($1, $2, '[]')",
                    &[&role_id, &role_id],
                )
                .await
                .unwrap();
            client
                .execute(
                    "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, tenant_id)
                     VALUES ($1, $2, 'hash', 'Tester', $3, 0, $4)",
                    &[
                        &format!("user-{tenant}"),
                        &format!("tester-{tenant}"),
                        &role_id,
                        &tenant,
                    ],
                )
                .await
                .unwrap();
            client
                .execute(
                    "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, tenant_id)
                     VALUES ($1, 'Tax', 800, 1, 0, $2)",
                    &[&format!("tax-{tenant}"), &tenant],
                )
                .await
                .unwrap();
            client
                .execute(
                    "INSERT INTO products (id, sku, name, price_minor, currency, track_serial, is_active, tenant_id)
                     VALUES ($1, $2, 'Widget', 100, 'USD', 1, 1, $3)",
                    &[&format!("prod-{tenant}"), &format!("SKU-{tenant}"), &tenant],
                )
                .await
                .unwrap();
        }

        // Products: track_serial=1 → true, is_active=1 → true.
        let products = store.snapshot_products(&tenant).await.unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(products[0]["track_serial"], true);
        assert_eq!(products[0]["is_active"], true);

        // Tax rates: is_default=1 → true, is_inclusive=0 → false.
        let tax_rates = store.snapshot_tax_rates(&tenant).await.unwrap();
        assert_eq!(tax_rates.len(), 1);
        assert_eq!(tax_rates[0]["is_default"], true);
        assert_eq!(tax_rates[0]["is_inclusive"], false);

        // Users: is_active=0 → false, and pin_hash must not leak (SYNC-06).
        let users = store.snapshot_users(&tenant).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["is_active"], false);
        assert!(users[0].get("pin_hash").is_none());

        // Clean up the rows this test created so a shared dev DB stays tidy.
        {
            let client = pool.get().await.unwrap();
            let role_id = format!("role-{tenant}");
            client
                .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM users WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM roles WHERE id = $1", &[&role_id])
                .await
                .unwrap();
            client
                .execute("DELETE FROM tax_rates WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM products WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute("DELETE FROM tenant_plans WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
        }
    }
}
