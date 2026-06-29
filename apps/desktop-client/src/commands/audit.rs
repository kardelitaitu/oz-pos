//! Audit log commands.
//!
//! `list_audit_log` exposes the append-only audit log entries
//! stored in SQLite via `oz_core::db::Store::list_audit_entries`.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

use crate::error::AppError;
use crate::state::AppState;

/// A single audit log entry sent to the front-end.
#[derive(Debug, Serialize)]
pub struct AuditEntryDto {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub details: String,
    pub outcome: String,
    pub created_at: String,
}

impl From<oz_core::AuditEntry> for AuditEntryDto {
    fn from(e: oz_core::AuditEntry) -> Self {
        Self {
            id: e.id,
            user_id: e.user_id,
            action: e.action,
            target_type: e.target_type,
            target_id: e.target_id,
            details: e.details,
            outcome: e.outcome,
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListAuditLogArgs {
    /// Maximum number of entries to return (default: 100).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Number of entries to skip for pagination (default: 0).
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Fetch audit log entries in reverse chronological order.
///
/// Supports pagination via `limit` and `offset`. Returns an array of
/// [`AuditEntryDto`] with action, target, outcome, and timestamp.
#[command]
pub async fn list_audit_log(
    args: ListAuditLogArgs,
    state: State<'_, AppState>,
) -> Result<Vec<AuditEntryDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let entries = store.list_audit_entries(args.limit, args.offset)?;
    drop(db);
    Ok(entries.into_iter().map(AuditEntryDto::from).collect())
}
