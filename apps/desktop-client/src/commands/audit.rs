//! Audit log commands.
//!
//! `list_audit_log` exposes the append-only audit log entries
//! stored in SQLite via `oz_core::db::Store::list_audit_entries`.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::db::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
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

/// Default `limit` value for [`ListAuditLogArgs`].
fn default_limit() -> i64 {
    100
}

/// Default `limit` for the store-scoped args (unsigned page size).
fn default_limit_u64() -> u64 {
    100
}

/// Fetch audit log entries in reverse chronological order.
///
/// Supports pagination via `limit` and `offset`. Returns an array of
/// [`AuditEntryDto`] with action, target, outcome, and timestamp.
///
/// **Deprecated for multi-store UI paths (ADR #7):** Use
/// [`list_audit_log_scoped`] so the session selects the store and user.
#[tauri::command]
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

// ── Store-scoped audit log (AUD-01/AUD-02/AUD-03) ───────────────

/// Server-filtered, keyset-paginated page of audit entries (AUD-02/AUD-03).
#[derive(Debug, Serialize)]
pub struct AuditLogPageDto {
    /// Entries on this page (most recent first).
    pub items: Vec<AuditEntryDto>,
    /// Total matching rows across all pages.
    pub total: u64,
    /// Whether another page follows the cursor.
    pub has_more: bool,
}

/// Arguments for the store-scoped audit log query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogScopedArgs {
    /// Maximum entries per page (clamped to `[1, 200]` server-side).
    #[serde(default = "default_limit_u64")]
    pub limit: u64,
    /// Optional outcome filter (`success` | `failure` | anything).
    pub outcome: Option<String>,
    /// Optional free-text query over action/target/user.
    pub query: Option<String>,
    /// Keyset cursor: fetch entries strictly older than `(created_at, id)`.
    pub before_created_at: Option<String>,
    pub before_id: Option<String>,
}

/// Fetch audit log entries scoped to the session's store (AUD-01).
///
/// Resolves the store and authenticated user from the session token,
/// enforces `audit:view`, and reads the session store's audit table — so a
/// multi-store deployment cannot disclose another store's events. Filtering
/// and pagination run server-side with a stable `(created_at, id)` cursor
/// (AUD-02/AUD-03).
#[tauri::command]
pub async fn list_audit_log_scoped(
    session_token: String,
    args: ListAuditLogScopedArgs,
    state: State<'_, AppState>,
) -> Result<AuditLogPageDto, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_audit_permission(&state, &session.user_id, permissions::AUDIT_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let (items, total, has_more) = store.list_audit_entries_filtered(
        args.outcome.as_deref(),
        args.query.as_deref(),
        args.before_created_at.as_deref(),
        args.before_id.as_deref(),
        args.limit,
    )?;
    Ok(AuditLogPageDto {
        items: items.into_iter().map(AuditEntryDto::from).collect(),
        total,
        has_more,
    })
}

/// Users and roles are global authentication records (ADR #4 / ADR #7);
/// audit events are read from the store-scoped connection after this check
/// succeeds. Mirror of `require_customer_permission` in customers.rs.
async fn require_audit_permission(
    state: &AppState,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, permission)
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
