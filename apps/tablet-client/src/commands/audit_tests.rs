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

// ── Export (AUD-09) ────────────────────────────────────────────

#[test]
fn export_args_deserialize_camel_case() {
    let json = r#"{"outcome":"failure","query":"sale"}"#;
    let args: ExportAuditLogArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.outcome.as_deref(), Some("failure"));
    assert_eq!(args.query.as_deref(), Some("sale"));
}

#[test]
fn export_args_deserialize_empty() {
    let json = r#"{}"#;
    let args: ExportAuditLogArgs = serde_json::from_str(json).unwrap();
    assert!(args.outcome.is_none());
    assert!(args.query.is_none());
}

#[test]
fn csv_row_quotes_embedded_quotes_and_commas() {
    // RFC-4180: embedded quotes are doubled; every field is quoted.
    let row = csv_row(&["a\"b", "c,d", "plain"]);
    assert_eq!(row, "\"a\"\"b\",\"c,d\",\"plain\"");
}

#[test]
fn csv_row_empty_and_nullable_fields() {
    let row = csv_row(&["id-1", "", "user-1"]);
    assert_eq!(row, "\"id-1\",\"\",\"user-1\"");
}

#[test]
fn export_dto_serialize_has_all_fields() {
    let dto = AuditExportDto {
        csv: "\u{FEFF}id\n".into(),
        row_count: 1,
        generated_at: "2026-08-01T00:00:00.000Z".into(),
        requested_by: "user-1".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["row_count"], 1);
    assert_eq!(json["requested_by"], "user-1");
    assert!(json["csv"].as_str().unwrap().starts_with('\u{FEFF}'));
}
