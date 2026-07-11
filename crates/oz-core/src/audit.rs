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
    /// Create a new audit entry with a generated UUID v7 and current UTC timestamp.
    pub fn new(
        user_id: impl Into<String>,
        action: impl Into<String>,
        target_type: Option<impl Into<String>>,
        target_id: Option<impl Into<String>>,
        details: Option<impl Into<String>>,
        outcome: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
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

    #[test]
    fn audit_entry_generates_uuid() {
        let entry = AuditEntry::new(
            "u1",
            "test",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        assert!(!entry.id.is_empty());
        assert_eq!(entry.id.len(), 36); // UUID v4 format
        assert_eq!(entry.id.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn audit_entry_generates_iso8601_timestamp() {
        let entry = AuditEntry::new(
            "u1",
            "test",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        assert!(!entry.created_at.is_empty());
        assert!(entry.created_at.contains('T'), "expected ISO-8601 format");
        assert!(entry.created_at.ends_with('Z'), "expected UTC timezone");
    }

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = AuditEntry::new(
            "user-1",
            "sale.void",
            Some("sale"),
            Some("sale-abc"),
            Some(r#"{"reason": "customer request"}"#),
            "success",
        );
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, entry.id);
        assert_eq!(back.user_id, entry.user_id);
        assert_eq!(back.action, entry.action);
        assert_eq!(back.target_type, entry.target_type);
        assert_eq!(back.target_id, entry.target_id);
        assert_eq!(back.details, entry.details);
        assert_eq!(back.outcome, entry.outcome);
        assert_eq!(back.created_at, entry.created_at);
    }

    #[test]
    fn audit_entry_serde_json_field_names() {
        let entry = AuditEntry::new(
            "u1",
            "login",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["user_id"], "u1");
        assert_eq!(json["action"], "login");
        assert_eq!(json["outcome"], "success");
        assert!(json.get("target_type").unwrap().is_null());
        assert!(json.get("target_id").unwrap().is_null());
    }

    #[test]
    fn audit_entry_details_defaults_to_empty_json() {
        let entry = AuditEntry::new(
            "u1",
            "action",
            None::<String>,
            None::<String>,
            None::<String>,
            "ok",
        );
        assert_eq!(entry.details, "{}");
    }

    #[test]
    fn audit_entry_details_custom_value() {
        let entry = AuditEntry::new(
            "u1",
            "action",
            None::<String>,
            None::<String>,
            Some("{\"key\":\"val\"}"),
            "ok",
        );
        assert_eq!(entry.details, "{\"key\":\"val\"}");
    }

    #[test]
    fn audit_entry_empty_user_id() {
        let entry = AuditEntry::new(
            "",
            "system.backup",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        assert_eq!(entry.user_id, "");
    }

    #[test]
    fn audit_entry_clone_eq() {
        let a = AuditEntry::new(
            "u1",
            "login",
            None::<String>,
            None::<String>,
            None::<String>,
            "success",
        );
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn audit_entry_debug_output() {
        let entry = AuditEntry::new(
            "u1",
            "test",
            None::<String>,
            None::<String>,
            None::<String>,
            "ok",
        );
        let debug = format!("{:?}", entry);
        assert!(debug.contains("u1"));
        assert!(debug.contains("test"));
    }
}
