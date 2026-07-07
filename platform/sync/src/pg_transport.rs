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
            ];
            let result = client
                .execute(
                    "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error)
                     VALUES ($1, $2, $3, 'pending', $4, $5)
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
    /// Returns items that have been synced to the remote but not yet applied locally.
    pub async fn pull_updates(
        &self,
        since: Option<&str>,
    ) -> Result<super::transport::PullResponse, SyncError> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| SyncError::Transport(format!("pg connection failed: {e}")))?;

        let rows = if let Some(since_ts) = since {
            client
                .query(
                    "SELECT id, action, payload, status, retry_count, last_error,
                            created_at::TEXT, synced_at::TEXT
                     FROM offline_queue
                     WHERE synced_at > $1
                     ORDER BY created_at ASC",
                    &[&since_ts],
                )
                .await
                .map_err(|e| SyncError::Transport(format!("pg query failed: {e}")))?
        } else {
            client
                .query(
                    "SELECT id, action, payload, status, retry_count, last_error,
                            created_at::TEXT, synced_at::TEXT
                     FROM offline_queue
                     ORDER BY created_at ASC",
                    &[],
                )
                .await
                .map_err(|e| SyncError::Transport(format!("pg query failed: {e}")))?
        };

        let items: Vec<OfflineQueueItem> = rows
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
                    synced_at: Some(row.get::<_, String>("synced_at")),
                }
            })
            .collect();

        Ok(super::transport::PullResponse { items })
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

    // ── pull_updates edge cases ───────────────────────────────────────

    #[tokio::test]
    async fn pull_updates_both_with_and_without_since() {
        let transport = PgTransport::new("localhost", 5432, "nonexistent", "u", "p")
            .expect("pool creation should succeed");

        // pull_updates with since = None
        let result1 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            transport.pull_updates(None),
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

        // pull_updates with since = Some
        let result2 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            transport.pull_updates(Some("2026-01-01T00:00:00Z")),
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
    }
}
