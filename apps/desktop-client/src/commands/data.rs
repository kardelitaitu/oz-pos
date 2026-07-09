//! Data management commands: backup, restore, export .ozpkg, import .ozpkg.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::Settings;
use oz_core::db::Store;
use oz_core::ozpkg::{export_ozpkg, import_ozpkg};

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BackupStatus {
    pub last_backup: Option<String>,
    pub last_backup_size: Option<String>,
    pub db_path: String,
}

#[derive(Debug, Serialize)]
pub struct BackupResult {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExportDataArgs {
    pub types: Vec<String>,
    pub password: String,
    pub output_path: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportDataResult {
    pub path: String,
    pub size_bytes: u64,
    pub types: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportPreviewArgs {
    pub file_path: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewResult {
    pub store_name: String,
    pub app_version: String,
    pub created_at: String,
    pub types: Vec<String>,
    pub product_count: usize,
    pub category_count: usize,
    pub sale_count: Option<usize>,
    pub customer_count: Option<usize>,
    pub user_count: Option<usize>,
    pub setting_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ImportDataArgs {
    pub file_path: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct ImportDataResult {
    pub products_imported: usize,
    pub categories_imported: usize,
    pub sales_imported: usize,
    pub customers_imported: usize,
    pub users_imported: usize,
    pub settings_imported: usize,
}

// ── Commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_backup_status(state: State<'_, AppState>) -> Result<BackupStatus, AppError> {
    let db_path = state.db_path.display().to_string();
    let backup_path = default_backup_path(&state);
    let (last_backup, last_backup_size) = match std::fs::metadata(&backup_path) {
        Ok(meta) => {
            let modified = meta.modified().ok().map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            });
            let size = Some(human_size(meta.len()));
            (modified, size)
        }
        Err(_) => (None, None),
    };
    Ok(BackupStatus {
        last_backup,
        last_backup_size,
        db_path,
    })
}

#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>) -> Result<BackupResult, AppError> {
    let output = default_backup_path(&state);
    let conn = state.db.lock().await;
    let store = Store::new(&conn);
    store.backup(&output)?;
    let size_bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
    Ok(BackupResult {
        path: output,
        size_bytes,
    })
}

#[tauri::command]
pub async fn export_data(
    args: ExportDataArgs,
    state: State<'_, AppState>,
) -> Result<ExportDataResult, AppError> {
    use oz_core::ozpkg::OzpkgPayload;

    let conn = state.db.lock().await;
    let store = Store::new(&conn);

    let all_types = args.types.is_empty() || args.types.iter().any(|t| t == "all");
    let wants = |name: &str| all_types || args.types.iter().any(|t| t == name);

    let products = if wants("products") {
        let prods = store.list_products()?;
        serde_json::to_value(&prods)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let categories = if wants("categories") {
        let cats = store.list_categories()?;
        serde_json::to_value(&cats)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let sales = if wants("sales") {
        let sales_list = store.list_sales()?;
        Some(
            serde_json::to_value(&sales_list)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let customers = if wants("customers") {
        let custs = store.list_customers()?;
        Some(
            serde_json::to_value(&custs)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let users = if wants("users") {
        let usrs = store.list_users()?;
        Some(
            serde_json::to_value(&usrs)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let settings = if wants("settings") {
        let rows = Settings::load_all(&conn)?;
        Some(
            rows.into_iter()
                .map(|(key, value)| serde_json::json!({ "key": key, "value": value }))
                .collect(),
        )
    } else {
        None
    };

    let mut data_types: Vec<String> = Vec::new();
    if wants("products") {
        data_types.push("products".into());
    }
    if wants("categories") {
        data_types.push("categories".into());
    }
    if wants("sales") {
        data_types.push("sales".into());
    }
    if wants("customers") {
        data_types.push("customers".into());
    }
    if wants("users") {
        data_types.push("users".into());
    }
    if wants("settings") {
        data_types.push("settings".into());
    }

    let payload = OzpkgPayload {
        products,
        categories,
        sales,
        customers,
        users,
        settings,
    };

    let store_name = store
        .get_store_name()?
        .unwrap_or_else(|| "OZ-POS Store".into());

    let features: HashMap<String, String> = store
        .load_features()
        .map(|reg| reg.to_settings_rows().into_iter().collect())
        .unwrap_or_default();

    let data_types_for_result = data_types.clone();
    let ozpkg_bytes = export_ozpkg(
        &args.password,
        &store_name,
        "0.0.1",
        data_types,
        features,
        &payload,
    )?;

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&args.output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("creating directory: {e}")))?;
    }

    std::fs::write(&args.output_path, &ozpkg_bytes)
        .map_err(|e| AppError::Internal(format!("writing export file: {e}")))?;

    let size_bytes = ozpkg_bytes.len() as u64;
    Ok(ExportDataResult {
        path: args.output_path,
        size_bytes,
        types: data_types_for_result,
    })
}

#[tauri::command]
pub async fn import_preview(args: ImportPreviewArgs) -> Result<ImportPreviewResult, AppError> {
    let data = std::fs::read(&args.file_path)
        .map_err(|e| AppError::Internal(format!("reading file: {e}")))?;
    let (header, payload) = import_ozpkg(&data, &args.password)?;

    Ok(ImportPreviewResult {
        store_name: header.store_name,
        app_version: header.app_version,
        created_at: header.created_at,
        types: header.data_types,
        product_count: payload.products.len(),
        category_count: payload.categories.len(),
        sale_count: payload.sales.as_ref().map(|s| s.len()),
        customer_count: payload.customers.as_ref().map(|c| c.len()),
        user_count: payload.users.as_ref().map(|u| u.len()),
        setting_count: payload.settings.as_ref().map(|s| s.len()),
    })
}

#[tauri::command]
pub async fn import_data(
    args: ImportDataArgs,
    state: State<'_, AppState>,
) -> Result<ImportDataResult, AppError> {
    let data = std::fs::read(&args.file_path)
        .map_err(|e| AppError::Internal(format!("reading file: {e}")))?;
    let (_header, payload) = import_ozpkg(&data, &args.password)?;

    let conn = state.db.lock().await;
    let store = Store::new(&conn);

    // Use a transaction for atomic import
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::Internal(format!("starting transaction: {e}")))?;

    let mut products_imported = 0;
    for val in &payload.products {
        if let Ok(product) = serde_json::from_value::<oz_core::Product>(val.clone()) {
            let exists = store
                .conn()
                .query_row(
                    "SELECT 1 FROM products WHERE sku = ?1",
                    rusqlite::params![product.sku.to_string()],
                    |_| Ok(()),
                )
                .is_ok();
            if exists {
                store.update_product(
                    &product.sku.to_string(),
                    &product.name,
                    product.price,
                    product.category_id.as_deref(),
                    product.barcode.as_ref().map(|b| b.as_str()),
                    Some(product.product_type.as_str()),
                )?;
            } else {
                store.create_product(
                    &product.sku.to_string(),
                    &product.name,
                    product.price,
                    product.category_id.as_deref(),
                    product.barcode.as_ref().map(|b| b.as_str()),
                    0,
                    Some(product.product_type.as_str()),
                )?;
            }
            products_imported += 1;
        }
    }

    let mut categories_imported = 0;
    for val in &payload.categories {
        if let Ok(cat) = serde_json::from_value::<oz_core::Category>(val.clone()) {
            let colour = if cat.colour.is_empty() {
                "#6366f1"
            } else {
                &cat.colour
            };
            let exists = store
                .conn()
                .query_row(
                    "SELECT 1 FROM categories WHERE id = ?1",
                    rusqlite::params![cat.id],
                    |_| Ok(()),
                )
                .is_ok();
            if !exists {
                let _ = tx.execute(
                    "INSERT INTO categories (id, name, colour, icon) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![cat.id, cat.name, colour, ""],
                );
            } else {
                let _ = tx.execute(
                    "UPDATE categories SET name = ?1, colour = ?2, icon = '' WHERE id = ?3",
                    rusqlite::params![cat.name, colour, cat.id],
                );
            }
            categories_imported += 1;
        }
    }

    let mut sales_imported = 0;
    if let Some(ref sales) = payload.sales {
        for val in sales {
            if let Ok(sale) = serde_json::from_value::<oz_core::Sale>(val.clone()) {
                let exists = store
                    .conn()
                    .query_row(
                        "SELECT 1 FROM sales WHERE id = ?1",
                        rusqlite::params![sale.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                if !exists {
                    let _ = store.create_sale(&sale);
                }
                sales_imported += 1;
            }
        }
    }

    let mut customers_imported = 0;
    if let Some(ref customers) = payload.customers {
        for val in customers {
            if let Ok(cust) = serde_json::from_value::<oz_core::Customer>(val.clone()) {
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let exists = store
                    .conn()
                    .query_row(
                        "SELECT 1 FROM customers WHERE id = ?1",
                        rusqlite::params![cust.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                let email_str = cust.email.map(|e| e.to_string());
                let phone_str = cust.phone.map(|p| p.to_string());
                if exists {
                    let _ = tx.execute(
                        "UPDATE customers SET name = ?1, email = ?2, phone = ?3, notes = ?4, updated_at = ?5 WHERE id = ?6",
                        rusqlite::params![cust.name, email_str, phone_str, cust.notes, now, cust.id],
                    );
                } else {
                    let _ = tx.execute(
                        "INSERT INTO customers (id, name, email, phone, notes, loyalty_points, total_spent_minor, currency, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![cust.id, cust.name, email_str, phone_str, cust.notes, 0i64, 0i64, "USD", now, now],
                    );
                }
                customers_imported += 1;
            }
        }
    }

    let mut users_imported = 0;
    if let Some(ref users) = payload.users {
        for val in users {
            if let Ok(user) = serde_json::from_value::<oz_core::User>(val.clone()) {
                let exists = store
                    .conn()
                    .query_row(
                        "SELECT 1 FROM users WHERE id = ?1",
                        rusqlite::params![user.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                if exists {
                    let _ = tx.execute(
                        "UPDATE users SET username = ?1, display_name = ?2, role_id = ?3, is_active = ?4, updated_at = ?5 WHERE id = ?6",
                        rusqlite::params![user.username, user.display_name, user.role_id, user.is_active, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), user.id],
                    );
                } else {
                    // Users from export have no PIN hash; mark as inactive so they must be re-invited
                    let _ = tx.execute(
                        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
                        rusqlite::params![user.id, user.username, "", user.display_name, user.role_id, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)],
                    );
                }
                users_imported += 1;
            }
        }
    }

    let mut settings_imported = 0;
    if let Some(ref settings) = payload.settings {
        for val in settings {
            if let Some(key) = val.get("key").and_then(|v| v.as_str())
                && let Some(value) = val.get("value").and_then(|v| v.as_str())
            {
                let _ = Settings::set(&tx, key, value);
                settings_imported += 1;
            }
        }
    }

    tx.commit()
        .map_err(|e| AppError::Internal(format!("committing import: {e}")))?;

    Ok(ImportDataResult {
        products_imported,
        categories_imported,
        sales_imported,
        customers_imported,
        users_imported,
        settings_imported,
    })
}

// ── Helpers ───────────────────────────────────────────────────────

fn default_backup_path(state: &AppState) -> String {
    let mut path = state.db_path.clone();
    path.set_extension("backup.db");
    path.display().to_string()
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
            app_version: "0.0.4".into(),
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
}
