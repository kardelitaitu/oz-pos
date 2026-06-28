//! Audit log — immutable, append-only record of sensitive actions.
//!
//! # PCI-DSS Compliance
//!
//! - **10.2.1**: Audit log captures user ID, event type, date/time,
//!   and success/failure.
//! - **10.3.1**: Audit logs cannot be modified (no UPDATE/DELETE).
//! - **10.3.2**: Audit logs are retained for at least 12 months
//!   (enforced by log rotation policy in `oz-logging`).

use serde::{Deserialize, Serialize};

/// A single immutable audit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// UUID v4 identifier.
    pub id: String,
    /// FK to `users.id`. Empty string for system-initiated actions.
    pub user_id: String,
    /// Action type (kebab-case, e.g. "sale.void", "login").
    pub action: String,
    /// Type of entity affected (e.g. "sale", "user", "setting").
    pub target_type: Option<String>,
    /// Identifier of the affected entity.
    pub target_id: Option<String>,
    /// JSON blob with action-specific metadata.
    pub details: String,
    /// Outcome: "success" or "failure".
    pub outcome: String,
    /// ISO-8601 timestamp.
    pub created_at: String,
}

impl AuditEntry {
    /// Create a new audit entry with a generated UUID and current timestamp.
    pub fn new(
        user_id: impl Into<String>,
        action: impl Into<String>,
        target_type: Option<impl Into<String>>,
        target_id: Option<impl Into<String>>,
        details: Option<impl Into<String>>,
        outcome: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            action: action.into(),
            target_type: target_type.map(|s| s.into()),
            target_id: target_id.map(|s| s.into()),
            details: details.map(|s| s.into()).unwrap_or_else(|| "{}".into()),
            outcome: outcome.into(),
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_new_generates_id_and_timestamp() {
        let entry = AuditEntry::new(
            "user-1",
            "sale.void",
            Some("sale"),
            Some("sale-abc"),
            Some(r#"{"reason": "customer request"}"#),
            "success",
        );
        assert!(!entry.id.is_empty());
        assert_eq!(entry.user_id, "user-1");
        assert_eq!(entry.action, "sale.void");
        assert_eq!(entry.target_type.as_deref(), Some("sale"));
        assert_eq!(entry.target_id.as_deref(), Some("sale-abc"));
        assert_eq!(entry.details, r#"{"reason": "customer request"}"#);
        assert_eq!(entry.outcome, "success");
        assert!(!entry.created_at.is_empty());
    }

    #[test]
    fn audit_entry_system_action() {
        let entry = AuditEntry::new(
            "",
            "system.backup",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        assert_eq!(entry.user_id, "");
        assert!(entry.target_type.is_none());
        assert!(entry.target_id.is_none());
        assert_eq!(entry.details, "{}");
    }

    #[test]
    fn audit_entry_failure_outcome() {
        let entry = AuditEntry::new(
            "user-2",
            "login",
            None::<String>,
            None::<String>,
            Some(r#"{"reason": "invalid PIN"}"#),
            "failure",
        );
        assert_eq!(entry.action, "login");
        assert_eq!(entry.outcome, "failure");
    }
}
