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
    /// Unique identifier.
    pub id: String,
    /// ID of the user who performed the action.
    pub user_id: String,
    /// Action.
    pub action: String,
    /// Target Type.
    pub target_type: Option<String>,
    /// ID of the entity acted upon (sale, product, shift, etc.), if any.
    pub target_id: Option<String>,
    /// Free-form context or metadata describing the action (e.g., void
    /// reason, adjustment amount, error summary).
    pub details: String,
    /// Result of the action — typically `"success"` or `"failure"`
    /// followed by an error summary when relevant.
    pub outcome: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl From<oz_core::AuditEntry> for AuditEntryDto {
    /// Converts a core [`oz_core::AuditEntry`] into a front-end [`AuditEntryDto`].
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

/// Arguments for paginating the audit log query.
#[derive(Debug, Deserialize)]
pub struct ListAuditLogArgs {
    /// Maximum number of entries to return (default: 100).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Number of entries to skip for pagination (default: 0).
    #[serde(default)]
    pub offset: i64,
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::AuditEntry;

    // ── AuditEntryDto ───────────────────────────────────────────────────

    #[test]
    fn audit_entry_dto_debug() {
        let dto = AuditEntryDto {
            id: "a1".into(),
            user_id: "u1".into(),
            action: "void_sale".into(),
            target_type: Some("sale".into()),
            target_id: Some("s1".into()),
            details: "Voided by manager".into(),
            outcome: "success".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("void_sale"));
        assert!(d.contains("sale"));
    }

    #[test]
    fn audit_entry_dto_serialize() {
        let dto = AuditEntryDto {
            id: "a2".into(),
            user_id: "u2".into(),
            action: "login".into(),
            target_type: None,
            target_id: None,
            details: String::new(),
            outcome: "success".into(),
            created_at: "2025-02-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["action"], "login");
        assert!(json["target_type"].is_null());
    }

    #[test]
    fn audit_entry_dto_from_entry() {
        let entry = AuditEntry {
            id: "a3".into(),
            user_id: "u3".into(),
            action: "create_product".into(),
            target_type: Some("product".into()),
            target_id: Some("p1".into()),
            details: "Created new product".into(),
            outcome: "success".into(),
            created_at: "2025-03-01T00:00:00.000Z".into(),
        };
        let dto = AuditEntryDto::from(entry);
        assert_eq!(dto.action, "create_product");
        assert_eq!(dto.target_type.as_deref(), Some("product"));
    }

    // ── ListAuditLogArgs ────────────────────────────────────────────────

    #[test]
    fn list_audit_log_args_deserialize_minimal() {
        let json = r#"{}"#;
        let args: ListAuditLogArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.limit, 100);
        assert_eq!(args.offset, 0);
    }

    #[test]
    fn list_audit_log_args_deserialize_full() {
        let json = r#"{"limit":50,"offset":10}"#;
        let args: ListAuditLogArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.limit, 50);
        assert_eq!(args.offset, 10);
    }

    #[test]
    fn list_audit_log_args_debug() {
        let args = ListAuditLogArgs {
            limit: 25,
            offset: 0,
        };
        let d = format!("{args:?}");
        assert!(d.contains("25"));
    }
}
