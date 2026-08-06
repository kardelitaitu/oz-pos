//! PostgreSQL Transport — writes offline queue items directly to a remote
//! PostgreSQL database via `tokio-postgres`.
//!
//! This transport bypasses the HTTP sync server and writes directly to a
//! cloud PostgreSQL database (AWS RDS, Azure Database for PostgreSQL, etc.).

use deadpool_postgres::{Config, Pool, Runtime};
use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus};
use tokio_postgres::{NoTls, types::ToSql};

use crate::SyncError;

/// Transport that writes offline queue items to a remote PostgreSQL database.
pub struct PgTransport {
    pool: Pool,
}

/// Maximum rows returned per pull page (mirrors the HTTP server's 500).
/// The transport fetches one extra row (501) to detect whether more pages
/// exist (P-3 pagination).
const PG_PULL_PAGE_SIZE: usize = 500;
const PG_PULL_FETCH_LIMIT: i64 = 501;

/// Decode a `"created_at|id"` composite pull cursor into its parts.
///
/// A malformed or missing cursor yields `(None, None)` — the caller then
/// starts from `since` (or the beginning on first sync), mirroring the
/// HTTP server's `splitn(2, '|')` decoding.
fn decode_pull_cursor(cursor: Option<&str>) -> (Option<String>, Option<String>) {
    match cursor {
        Some(c) => {
            let parts: Vec<&str> = c.splitn(2, '|').collect();
            if parts.len() == 2 {
                (Some(parts[0].to_owned()), Some(parts[1].to_owned()))
            } else {
                (None, None)
            }
        }
        None => (None, None),
    }
}

/// Build the SQL for a pull page.
///
/// Anchoring on `created_at` (not `synced_at`) means rows whose remote
/// `synced_at` is never stamped still advance the durable anchor. The
/// cursor branch carries the composite `(created_at, id)` tiebreak — so
/// rows sharing the anchor's exact timestamp are never skipped — matching
/// the HTTP server's paginated pull semantics.
pub fn build_pull_sql(since: Option<&str>, cursor: Option<&str>) -> &'static str {
    if cursor.is_some() {
        "SELECT id, action, payload, status, retry_count, last_error,\n\n                tenant_id, created_at::TEXT, synced_at::TEXT\n\n         FROM offline_queue\n\n         WHERE created_at >= $1 AND (created_at > $2 OR (created_at = $2 AND id > $3))\n\n         ORDER BY created_at ASC, id ASC\n\n         LIMIT $4"
    } else if since.is_some() {
        "SELECT id, action, payload, status, retry_count, last_error,\n\n                tenant_id, created_at::TEXT, synced_at::TEXT\n\n         FROM offline_queue\n\n         WHERE created_at >= $1\n\n         ORDER BY created_at ASC, id ASC\n\n         LIMIT $2"
    } else {
        "SELECT id, action, payload, status, retry_count, last_error,\n\n                tenant_id, created_at::TEXT, synced_at::TEXT\n\n         FROM offline_queue\n\n         ORDER BY created_at ASC, id ASC\n\n         LIMIT $1"
    }
}

/// Truncate a fetched page to [`PG_PULL_PAGE_SIZE`] rows and derive the
/// composite `"created_at|id"` next cursor from the last KEPT row.
///
/// Returns `None` when the page was not full (no more pages). The cursor
/// must come from the last kept row — never the dropped overflow row — so
/// a follow-up page resumes exactly after the kept boundary (RUST-07).
fn derive_next_cursor(items: &mut Vec<OfflineQueueItem>) -> Option<String> {
    if items.len() > PG_PULL_PAGE_SIZE {
        items.truncate(PG_PULL_PAGE_SIZE);
        items
            .last()
            .map(|last| format!("{}|{}", last.created_at, last.id))
    } else {
        None
    }
}

impl std::fmt::Debug for PgTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgTransport").finish_non_exhaustive()
    }
}

impl PgTransport {
    /// Create a new PostgreSQL transport from connection parameters.
    pub fn new(
        host: &str,
        port: u16,
        dbname: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, SyncError> {
        let mut cfg = Config::new();
        cfg.host = Some(host.to_owned());
        cfg.port = Some(port);
        cfg.dbname = Some(dbname.to_owned());
        cfg.user = Some(user.to_owned());
        cfg.password = Some(password.to_owned());

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| SyncError::Transport(format!("failed to create pg pool: {e}")))?;

        Ok(Self { pool })
    }

    /// Push pending items to the remote PostgreSQL database.
    ///
    /// Writes each item to an `offline_queue` table in the remote PG database.
    pub async fn push_items(
        &self,
        items: &[OfflineQueueItem],
    ) -> Result<Vec<super::transport::PushOutcome>, SyncError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| SyncError::Transport(format!("pg connection failed: {e}")))?;

        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS offline_queue (
                    id TEXT PRIMARY KEY,
                    action TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    retry_count INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    tenant_id TEXT NOT NULL DEFAULT 'default',
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    synced_at TIMESTAMPTZ
                )",
            )
            .await
            .map_err(|e| SyncError::Transport(format!("pg create table failed: {e}")))?;

        let mut outcomes = Vec::with_capacity(items.len());

        for item in items {
            let params: &[&(dyn ToSql + Sync)] = &[
                &item.id,
                &item.action,
                &item.payload,
                &item.retry_count,
                &item.last_error,
                &item.tenant_id,
            ];
            let result = client
                .execute(
                    "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, tenant_id)
                     VALUES ($1, $2, $3, 'pending', $4, $5, $6)
                     ON CONFLICT (id) DO NOTHING",
                    params,
                )
                .await;

            match result {
                Ok(_) => outcomes.push(super::transport::PushOutcome::Accepted),
                Err(e) => outcomes.push(super::transport::PushOutcome::Rejected {
                    reason: format!("pg insert failed: {e}"),
                }),
            }
        }

        Ok(outcomes)
    }

    /// Pull updates from the remote PostgreSQL database.
    ///
    /// Returns up to [`PG_PULL_PAGE_SIZE`] items ordered by
    /// `(created_at, id)` — the composite cursor key — so rows sharing an
    /// exact `created_at` timestamp are never skipped. The `since` anchor
    /// filters on `created_at` (not `synced_at`), so rows whose remote
    /// `synced_at` is never stamped still advance the durable anchor.
    ///
    /// When more pages exist, `next_cursor` carries a `"created_at|id"`
    /// composite cursor that the caller passes back on the next call.
    pub async fn pull_updates(
        &self,
        since: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<super::transport::PullResponse, SyncError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| SyncError::Transport(format!("pg connection failed: {e}")))?;

        let (cursor_ts, cursor_id) = decode_pull_cursor(cursor);
        let limit = PG_PULL_FETCH_LIMIT;
        // `since` for the cursor/since branches; PG binds an empty string
        // as the lower bound when no since is given (matches the HTTP
        // server passing `req.since.unwrap_or("")` to the cursor query).
        let since_param = since.unwrap_or("");

        let rows = if let (Some(ts), Some(cid)) = (&cursor_ts, &cursor_id) {
            client
                .query(
                    build_pull_sql(since, cursor),
                    &[&since_param, ts, cid, &limit],
                )
                .await
                .map_err(|e| SyncError::Transport(format!("pg query failed: {e}")))?
        } else if since.is_some() {
            client
                .query(build_pull_sql(since, cursor), &[&since_param, &limit])
                .await
                .map_err(|e| SyncError::Transport(format!("pg query failed: {e}")))?
        } else {
            client
                .query(build_pull_sql(None, None), &[&limit])
                .await
                .map_err(|e| SyncError::Transport(format!("pg query failed: {e}")))?
        };

        let mut items: Vec<OfflineQueueItem> = rows
            .iter()
            .map(|row| {
                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "synced" => OfflineQueueStatus::Synced,
                    "failed" => OfflineQueueStatus::Failed,
                    _ => OfflineQueueStatus::Pending,
                };
                OfflineQueueItem {
                    id: row.get("id"),
                    action: row.get("action"),
                    payload: row.get("payload"),
                    status,
                    retry_count: row.get("retry_count"),
                    last_error: row.get("last_error"),
                    created_at: row.get::<_, String>("created_at"),
                    // A remote row this terminal pushed as `pending` has a
                    // NULL synced_at until the remote stamps it; decoding as
                    // Option (not String) avoids a panic on that first row.
                    synced_at: row.get::<_, Option<String>>("synced_at"),
                    tenant_id: row.get("tenant_id"),
                    priority: oz_core::offline::SyncPriority::Normal,
                }
            })
            .collect();

        let next_cursor = derive_next_cursor(&mut items);

        Ok(super::transport::PullResponse { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PgTransport::new() ────────────────────────────────────────────

    #[test]
    fn new_succeeds_with_valid_params() {
        let transport = PgTransport::new("localhost", 5432, "testdb", "user", "pass");
        assert!(transport.is_ok(), "pool creation should succeed");
    }

    #[test]
    fn new_succeeds_with_ip_address_host() {
        let transport = PgTransport::new("192.168.1.100", 5432, "mydb", "admin", "s3cret");
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_fqdn_host() {
        let transport = PgTransport::new(
            "db.internal.example.com",
            5432,
            "production",
            "app_user",
            "p@ssw0rd!",
        );
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_custom_port() {
        let transport = PgTransport::new("localhost", 5433, "db", "u", "p");
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_max_port() {
        let transport = PgTransport::new("localhost", 65535, "db", "u", "p");
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_min_port() {
        let transport = PgTransport::new("localhost", 1, "db", "u", "p");
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_special_chars_in_password() {
        let transport = PgTransport::new(
            "localhost",
            5432,
            "testdb",
            "user",
            "p@ss!w0rd#with%special&chars",
        );
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_long_strings() {
        let long = "a".repeat(255);
        let transport = PgTransport::new(&long, 5432, &long, &long, &long);
        assert!(transport.is_ok());
    }

    #[test]
    fn new_succeeds_with_unicode_dbname() {
        let transport = PgTransport::new("localhost", 5432, "café_db", "user", "pass");
        assert!(transport.is_ok());
    }

    #[test]
    fn new_handles_empty_string_params_gracefully() {
        // deadpool-postgres may accept or reject empty params at pool
        // creation time — either outcome is acceptable as long as it
        // doesn't panic.
        let result = PgTransport::new("", 5432, "", "", "");
        match result {
            Ok(_) => {} // pool created lazily, will fail on first use
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("pool") || msg.contains("transport"),
                    "expected pool or transport error, got: {msg}"
                );
            }
        }
    }

    // ── Debug ─────────────────────────────────────────────────────────

    #[test]
    fn pg_transport_debug_output() {
        let transport = PgTransport::new("localhost", 5432, "db", "u", "p")
            .expect("pool creation should succeed");
        let debug = format!("{transport:?}");
        assert!(debug.contains("PgTransport"));
        // Debug should not expose connection details.
        assert!(!debug.contains("localhost"));
        assert!(!debug.contains("5432"));
    }

    // ── Send + Sync ───────────────────────────────────────────────────

    #[test]
    fn pg_transport_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PgTransport>();
    }

    // ── push_items edge cases ─────────────────────────────────────────

    #[tokio::test]
    async fn push_items_empty_list_handles_missing_server() {
        // Even with an empty items list, push_items calls pool.get() for
        // the CREATE TABLE IF NOT EXISTS statement. If PG is running
        // locally, the empty list produces an empty outcomes vec; if not,
        // we get a transport error. Either outcome is acceptable.
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let transport = PgTransport::new("localhost", 5432, "nonexistent", "u", "p")?;
            transport.push_items(&[]).await
        })
        .await;
        match result {
            Ok(Ok(outcomes)) => assert!(outcomes.is_empty()),
            Ok(Err(e)) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("transport") || msg.contains("connection"),
                    "expected transport or connection error, got: {msg}"
                );
            }
            Err(_elapsed) => {
                // Timed out — no PG server reachable, which is expected.
            }
        }
    }

    // ── Composite (created_at, id) cursor ──────────────────────────────

    #[test]
    fn decode_pull_cursor_splits_on_pipe() {
        let (ts, id) = decode_pull_cursor(Some("2026-01-01T00:00:00Z|item-42"));
        assert_eq!(ts.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(id.as_deref(), Some("item-42"));
    }

    #[test]
    fn decode_pull_cursor_missing_or_malformed() {
        assert_eq!(decode_pull_cursor(None), (None, None));
        assert_eq!(decode_pull_cursor(Some("no-pipe")), (None, None));
    }

    #[test]
    fn build_pull_sql_filters_on_created_at_not_synced_at() {
        // The strict `synced_at > anchor` filter skipped any row sharing the
        // anchor's exact timestamp. Anchoring on created_at (the composite
        // cursor's first key) never skips an equal-timestamp row.
        let sql = build_pull_sql(Some("2026-01-01"), None);
        assert!(
            sql.contains("created_at >= $1"),
            "since filter must compare created_at, got: {sql}"
        );
        assert!(
            !sql.contains("synced_at >"),
            "strict synced_at filter must be gone, got: {sql}"
        );
        assert!(sql.contains("ORDER BY created_at ASC, id ASC"));
    }

    #[test]
    fn build_pull_sql_with_cursor_has_composite_tiebreak() {
        // Equal-timestamp rows are handled by the (created_at, id) tiebreak
        // — mirroring the HTTP server's cursor semantics.
        let sql = build_pull_sql(Some("2026-01-01"), Some("2026-01-02|item-42"));
        assert!(
            sql.contains("created_at > $2 OR (created_at = $2 AND id > $3)"),
            "cursor branch must carry the composite tiebreak, got: {sql}"
        );
    }

    #[test]
    fn build_pull_sql_without_since_or_cursor_returns_everything() {
        let sql = build_pull_sql(None, None);
        assert!(
            !sql.contains("WHERE"),
            "initial sync must not filter, got: {sql}"
        );
    }

    #[test]
    fn derive_next_cursor_from_last_kept_row_when_full_page() {
        let mut items: Vec<OfflineQueueItem> = (0..501)
            .map(|i| {
                let mut it = OfflineQueueItem::new("test", "{}");
                it.id = format!("id-{i}");
                it.created_at = format!("2026-01-01T00:00:00.{:03}Z", i % 1000);
                it
            })
            .collect();
        let next = derive_next_cursor(&mut items);
        assert_eq!(items.len(), 500, "page must truncate to 500 rows");
        // Cursor derives from the last KEPT row (index 499), not the 501st.
        let kept = &items[499];
        assert_eq!(
            next.as_deref(),
            Some(format!("{}|{}", kept.created_at, kept.id).as_str())
        );
    }

    #[test]
    fn derive_next_cursor_none_when_page_not_full() {
        let mut items: Vec<OfflineQueueItem> = (0..10)
            .map(|i| {
                let mut it = OfflineQueueItem::new("test", "{}");
                it.id = format!("id-{i}");
                it.created_at = "2026-01-01T00:00:00.000Z".into();
                it
            })
            .collect();
        let next = derive_next_cursor(&mut items);
        assert_eq!(items.len(), 10, "short pages are not truncated");
        assert_eq!(next, None, "no next cursor when the page is not full");
    }

    #[test]
    fn derive_next_cursor_roundtrips_via_decode() {
        let mut items: Vec<OfflineQueueItem> = (0..501)
            .map(|i| {
                let mut it = OfflineQueueItem::new("test", "{}");
                it.id = format!("id-{i}");
                it.created_at = "2026-01-01T00:00:00.000Z".into();
                it
            })
            .collect();
        let next = derive_next_cursor(&mut items).unwrap();
        let (ts, id) = decode_pull_cursor(Some(&next));
        assert_eq!(ts.as_deref(), Some("2026-01-01T00:00:00.000Z"));
        assert_eq!(id.as_deref(), Some("id-499"));
    }

    // ── pull_updates edge cases ───────────────────────────────────────

    #[tokio::test]
    async fn pull_updates_both_with_and_without_since() {
        let transport = PgTransport::new("localhost", 5432, "nonexistent", "u", "p")
            .expect("pool creation should succeed");

        // pull_updates with since = None, cursor = None
        let result1 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            transport.pull_updates(None, None),
        )
        .await;
        match result1 {
            Ok(Ok(_resp)) => {} // PG running locally
            Ok(Err(e)) => {
                assert!(
                    e.to_string().contains("transport") || e.to_string().contains("connection")
                );
            }
            Err(_elapsed) => {} // timed out — expected without PG
        }

        // pull_updates with since = Some, cursor = None
        let result2 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            transport.pull_updates(Some("2026-01-01T00:00:00Z"), None),
        )
        .await;
        match result2 {
            Ok(Ok(_resp)) => {}
            Ok(Err(e)) => {
                assert!(
                    e.to_string().contains("transport") || e.to_string().contains("connection")
                );
            }
            Err(_elapsed) => {}
        }

        // pull_updates with since = Some, cursor = Some
        let result3 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            transport.pull_updates(
                Some("2026-01-01T00:00:00Z"),
                Some("2026-01-02T00:00:00Z|item-42"),
            ),
        )
        .await;
        match result3 {
            Ok(Ok(_resp)) => {}
            Ok(Err(e)) => {
                assert!(
                    e.to_string().contains("transport") || e.to_string().contains("connection")
                );
            }
            Err(_elapsed) => {}
        }
    }
}
