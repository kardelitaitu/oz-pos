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
