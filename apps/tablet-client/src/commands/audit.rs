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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_dto_debug() {
        let dto = AuditEntryDto {
            id: "e1".into(),
            user_id: "u1".into(),
            action: "sale.void".into(),
            target_type: Some("sale".into()),
            target_id: Some("s1".into()),
            details: "voided by manager".into(),
            outcome: "success".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let debug = format!("{:?}", dto);
        assert!(debug.contains("sale.void"));
        assert!(debug.contains("u1"));
    }

    #[test]
    fn audit_entry_dto_serialize() {
        let dto = AuditEntryDto {
            id: "e1".into(),
            user_id: "u1".into(),
            action: "login".into(),
            target_type: None,
            target_id: None,
            details: "staff login".into(),
            outcome: "success".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["id"], "e1");
        assert_eq!(json["action"], "login");
        assert!(json["target_type"].is_null());
        assert!(json["target_id"].is_null());
    }

    #[test]
    fn audit_entry_dto_from_core_entry() {
        let entry = oz_core::AuditEntry {
            id: "e2".into(),
            user_id: "u2".into(),
            action: "product.create".into(),
            target_type: Some("product".into()),
            target_id: Some("p1".into()),
            details: "created".into(),
            outcome: "success".into(),
            created_at: "2026-01-15T12:00:00Z".into(),
        };
        let dto = AuditEntryDto::from(entry);
        assert_eq!(dto.action, "product.create");
        assert_eq!(dto.target_type.unwrap(), "product");
    }

    #[test]
    fn list_audit_log_args_deserialize_minimal() {
        let json = r#"{}"#;
        let args: ListAuditLogArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.limit, 100);
        assert_eq!(args.offset, 0);
    }

    #[test]
    fn list_audit_log_args_deserialize_full() {
        let json = r#"{"limit": 50, "offset": 10}"#;
        let args: ListAuditLogArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.limit, 50);
        assert_eq!(args.offset, 10);
    }

    #[test]
    fn list_audit_log_args_debug() {
        let args = ListAuditLogArgs {
            limit: 50,
            offset: 0,
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("50"));
    }
}
