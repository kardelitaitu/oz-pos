use super::*;

// ── StoreProfileDto ─────────────────────────────────────────────────

#[test]
fn store_profile_dto_debug() {
    let dto = StoreProfileDto {
        id: "sp1".into(),
        name: "Main Store".into(),
        address: "123 Main St".into(),
        tax_id: "TAX-001".into(),
        currency: "USD".into(),
        timezone: "UTC".into(),
        is_primary: true,
        created_at: "2025-01-01".into(),
        updated_at: "2025-01-01".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("Main Store"));
    assert!(d.contains("USD"));
}

#[test]
fn store_profile_dto_serialize() {
    let dto = StoreProfileDto {
        id: "sp2".into(),
        name: "Branch".into(),
        address: String::new(),
        tax_id: String::new(),
        currency: "IDR".into(),
        timezone: "Asia/Jakarta".into(),
        is_primary: false,
        created_at: "2025-02-01".into(),
        updated_at: "2025-02-01".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["name"], "Branch");
    assert_eq!(json["is_primary"], false);
}

// ── CreateStoreProfileArgs ──────────────────────────────────────────

#[test]
fn create_store_profile_args_deserialize_minimal() {
    let json = r#"{"id":"sp-new","name":"New Store"}"#;
    let args: CreateStoreProfileArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "sp-new");
    assert_eq!(args.address, None);
    assert_eq!(args.currency, None);
}

#[test]
fn create_store_profile_args_deserialize_full() {
    let json = r##"{"id":"sp-full","name":"Full Store","address":"123 Rd","tax_id":"T1","currency":"EUR","timezone":"CET"}"##;
    let args: CreateStoreProfileArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.currency.as_deref(), Some("EUR"));
    assert_eq!(args.timezone.as_deref(), Some("CET"));
}

#[test]
fn create_store_profile_args_debug() {
    let args = CreateStoreProfileArgs {
        id: "x".into(),
        name: "Y".into(),
        address: None,
        tax_id: None,
        currency: None,
        timezone: None,
    };
    let d = format!("{args:?}");
    assert!(d.contains("Y"));
}

// ── UpdateStoreProfileArgs ──────────────────────────────────────────

#[test]
fn update_store_profile_args_deserialize() {
    let json = r##"{"id":"sp1","name":"Updated","address":"New Rd","tax_id":"T2","currency":"USD","timezone":"EST"}"##;
    let args: UpdateStoreProfileArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "Updated");
    assert_eq!(args.address, "New Rd");
}

#[test]
fn update_store_profile_args_debug() {
    let args = UpdateStoreProfileArgs {
        id: "x".into(),
        name: "Z".into(),
        address: "A".into(),
        tax_id: "T".into(),
        currency: "C".into(),
        timezone: "TZ".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("Z"));
}
