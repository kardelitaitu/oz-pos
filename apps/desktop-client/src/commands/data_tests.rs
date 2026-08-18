
use super::*;

// ── BackupStatus ────────────────────────────────────────────────────

#[test]
fn backup_status_debug() {
    let bs = BackupStatus {
        last_backup: Some("2025-01-01 12:00:00".into()),
        last_backup_size: Some("1.5 MB".into()),
        db_path: "/data/oz-pos.db".into(),
    };
    let d = format!("{bs:?}");
    assert!(d.contains("oz-pos.db"));
}

#[test]
fn backup_status_serialize() {
    let bs = BackupStatus {
        last_backup: None,
        last_backup_size: None,
        db_path: "/tmp/test.db".into(),
    };
    let json = serde_json::to_value(&bs).unwrap();
    assert_eq!(json["db_path"], "/tmp/test.db");
    assert!(json["last_backup"].is_null());
}

// ── BackupResult ────────────────────────────────────────────────────

#[test]
fn backup_result_debug() {
    let br = BackupResult {
        path: "/backups/oz.backup.db".into(),
        size_bytes: 1024,
    };
    let d = format!("{br:?}");
    assert!(d.contains("1024"));
}

#[test]
fn backup_result_serialize() {
    let br = BackupResult {
        path: "/b/test.bak".into(),
        size_bytes: 2048,
    };
    let json = serde_json::to_value(&br).unwrap();
    assert_eq!(json["size_bytes"], 2048);
}

// ── ExportDataArgs ──────────────────────────────────────────────────

#[test]
fn export_data_args_deserialize() {
    let json = r#"{"types":["products","categories"],"password":"secret","output_path":"/out/export.ozpkg"}"#;
    let args: ExportDataArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.types, vec!["products", "categories"]);
    assert_eq!(args.password, "secret");
    assert_eq!(args.date_from, None);
    assert_eq!(args.date_to, None);
}

#[test]
fn export_data_args_debug() {
    let args = ExportDataArgs {
        types: vec!["all".into()],
        password: "pw".into(),
        output_path: "/o".into(),
        date_from: None,
        date_to: None,
    };
    let d = format!("{args:?}");
    assert!(d.contains("all"));
}

// ── ExportDataResult ────────────────────────────────────────────────

#[test]
fn export_data_result_debug() {
    let result = ExportDataResult {
        path: "/out/export.ozpkg".into(),
        size_bytes: 512,
        types: vec!["products".into(), "sales".into()],
    };
    let d = format!("{result:?}");
    assert!(d.contains("sales"));
}

#[test]
fn export_data_result_serialize() {
    let result = ExportDataResult {
        path: "/o/e.ozpkg".into(),
        size_bytes: 256,
        types: vec![],
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["size_bytes"], 256);
    assert!(json["types"].as_array().unwrap().is_empty());
}

// ── ImportPreviewArgs ───────────────────────────────────────────────

#[test]
fn import_preview_args_deserialize() {
    let json = r#"{"file_path":"/data/import.ozpkg","password":"pw123"}"#;
    let args: ImportPreviewArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.file_path, "/data/import.ozpkg");
    assert_eq!(args.password, "pw123");
}

#[test]
fn import_preview_args_debug() {
    let args = ImportPreviewArgs {
        file_path: "/f".into(),
        password: "p".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("/f"));
}

// ── ImportPreviewResult ─────────────────────────────────────────────

#[test]
fn import_preview_result_debug() {
    let result = ImportPreviewResult {
        store_name: "My Store".into(),
        app_version: "0.0.28".into(),
        created_at: "2025-01-01".into(),
        types: vec!["products".into()],
        product_count: 50,
        category_count: 5,
        sale_count: Some(100),
        customer_count: Some(20),
        user_count: Some(3),
        setting_count: None,
    };
    let d = format!("{result:?}");
    assert!(d.contains("My Store"));
    assert!(d.contains("50"));
}

#[test]
fn import_preview_result_serialize() {
    let result = ImportPreviewResult {
        store_name: "S".into(),
        app_version: "1.0".into(),
        created_at: "2025-01-01".into(),
        types: vec![],
        product_count: 0,
        category_count: 0,
        sale_count: None,
        customer_count: None,
        user_count: None,
        setting_count: None,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["store_name"], "S");
    assert!(json["sale_count"].is_null());
}

// ── ImportDataArgs ──────────────────────────────────────────────────

#[test]
fn import_data_args_deserialize() {
    let json = r#"{"file_path":"/data/import.ozpkg","password":"pw"}"#;
    let args: ImportDataArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.file_path, "/data/import.ozpkg");
}

#[test]
fn import_data_args_debug() {
    let args = ImportDataArgs {
        file_path: "/f".into(),
        password: "x".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("/f"));
}

// ── ImportDataResult ────────────────────────────────────────────────

#[test]
fn import_data_result_debug() {
    let result = ImportDataResult {
        products_imported: 10,
        categories_imported: 3,
        sales_imported: 50,
        customers_imported: 5,
        users_imported: 2,
        settings_imported: 1,
    };
    let d = format!("{result:?}");
    assert!(d.contains("50"));
}

#[test]
fn import_data_result_serialize() {
    let result = ImportDataResult {
        products_imported: 0,
        categories_imported: 0,
        sales_imported: 0,
        customers_imported: 0,
        users_imported: 0,
        settings_imported: 0,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["products_imported"], 0);
    assert_eq!(json["settings_imported"], 0);
}
