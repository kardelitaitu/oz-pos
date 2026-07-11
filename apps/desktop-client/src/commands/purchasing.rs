use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::{Store, purchase_orders::CreatePoLineInput};
use oz_core::{PurchaseOrderLine, PurchaseOrderWithLines, Supplier};

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

// ── Supplier DTO ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SupplierDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub contact_person: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub tax_id: String,
    pub payment_terms: String,
    pub notes: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Supplier> for SupplierDto {
    fn from(s: Supplier) -> Self {
        Self {
            id: s.id,
            code: s.code,
            name: s.name,
            contact_person: s.contact_person,
            phone: s.phone,
            email: s.email,
            address: s.address,
            tax_id: s.tax_id,
            payment_terms: s.payment_terms,
            notes: s.notes,
            status: s.status,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

// ── Purchase Order DTOs ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PurchaseOrderLineDto {
    pub id: String,
    pub po_id: String,
    pub sku: String,
    pub product_name: String,
    pub qty: i64,
    pub unit_cost_minor: i64,
    pub line_total_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct PurchaseOrderDto {
    pub id: String,
    pub po_number: String,
    pub supplier_id: String,
    pub status: String,
    pub order_date: String,
    pub expected_date: String,
    pub received_date: Option<String>,
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub notes: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub lines: Vec<PurchaseOrderLineDto>,
    pub supplier_name: Option<String>,
}

impl From<PurchaseOrderLine> for PurchaseOrderLineDto {
    fn from(l: PurchaseOrderLine) -> Self {
        Self {
            id: l.id,
            po_id: l.po_id,
            sku: l.sku,
            product_name: l.product_name,
            qty: l.qty,
            unit_cost_minor: l.unit_cost_minor,
            line_total_minor: l.line_total_minor,
        }
    }
}

impl From<PurchaseOrderWithLines> for PurchaseOrderDto {
    fn from(po: PurchaseOrderWithLines) -> Self {
        Self {
            id: po.order.id,
            po_number: po.order.po_number,
            supplier_id: po.order.supplier_id,
            status: po.order.status,
            order_date: po.order.order_date,
            expected_date: po.order.expected_date,
            received_date: po.order.received_date,
            subtotal_minor: po.order.subtotal_minor,
            tax_minor: po.order.tax_minor,
            total_minor: po.order.total_minor,
            notes: po.order.notes,
            created_by: po.order.created_by,
            created_at: po.order.created_at,
            updated_at: po.order.updated_at,
            lines: po
                .lines
                .into_iter()
                .map(PurchaseOrderLineDto::from)
                .collect(),
            supplier_name: po.supplier_name,
        }
    }
}

// ── Input DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSupplierArgs {
    pub code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSupplierArgs {
    pub id: String,
    pub code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PoLineInput {
    pub sku: String,
    pub product_name: String,
    pub qty: i64,
    pub unit_cost_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseOrderArgs {
    pub po_number: String,
    pub supplier_id: String,
    pub expected_date: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<PoLineInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePoStatusArgs {
    pub id: String,
    pub status: String,
}

// ── Supplier commands ───────────────────────────────────────────────

#[command]
pub async fn list_suppliers(state: State<'_, AppState>) -> Result<Vec<SupplierDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let suppliers = store.list_suppliers()?;
    drop(db);
    Ok(suppliers.into_iter().map(SupplierDto::from).collect())
}

#[command]
pub async fn get_supplier(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SupplierDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let supplier = store.get_supplier(&id)?;
    drop(db);
    Ok(supplier.map(SupplierDto::from))
}

#[command]
pub async fn create_supplier(
    args: CreateSupplierArgs,
    state: State<'_, AppState>,
) -> Result<SupplierDto, AppError> {
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("code", &args.code).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let supplier = store.create_supplier(
        args.code.trim(),
        args.name.trim(),
        args.contact_person.as_deref().unwrap_or_default(),
        args.phone.as_deref().unwrap_or_default(),
        args.email.as_deref().unwrap_or_default(),
        args.address.as_deref().unwrap_or_default(),
        args.tax_id.as_deref().unwrap_or_default(),
        args.payment_terms.as_deref().unwrap_or_default(),
        args.notes.as_deref().unwrap_or_default(),
    )?;
    drop(db);
    Ok(SupplierDto::from(supplier))
}

#[command]
pub async fn update_supplier(
    args: UpdateSupplierArgs,
    state: State<'_, AppState>,
) -> Result<SupplierDto, AppError> {
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("code", &args.code).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let supplier = store.update_supplier(
        &args.id,
        args.code.trim(),
        args.name.trim(),
        args.contact_person.as_deref().unwrap_or_default(),
        args.phone.as_deref().unwrap_or_default(),
        args.email.as_deref().unwrap_or_default(),
        args.address.as_deref().unwrap_or_default(),
        args.tax_id.as_deref().unwrap_or_default(),
        args.payment_terms.as_deref().unwrap_or_default(),
        args.notes.as_deref().unwrap_or_default(),
        args.status.as_deref().unwrap_or("active"),
    )?;
    drop(db);
    Ok(SupplierDto::from(supplier))
}

// ── Purchase Order commands ─────────────────────────────────────────

#[command]
pub async fn list_purchase_orders(
    state: State<'_, AppState>,
) -> Result<Vec<PurchaseOrderDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let pos = store.list_purchase_orders()?;
    drop(db);
    Ok(pos.into_iter().map(PurchaseOrderDto::from).collect())
}

#[command]
pub async fn get_purchase_order(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<PurchaseOrderDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let po = store.get_purchase_order(&id)?;
    drop(db);
    Ok(po.map(PurchaseOrderDto::from))
}

#[command]
pub async fn create_purchase_order(
    args: CreatePurchaseOrderArgs,
    state: State<'_, AppState>,
) -> Result<PurchaseOrderDto, AppError> {
    validate_not_empty("po_number", &args.po_number)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let lines: Vec<CreatePoLineInput> = args
        .lines
        .into_iter()
        .map(|l| CreatePoLineInput {
            sku: l.sku,
            product_name: l.product_name,
            qty: l.qty,
            unit_cost_minor: l.unit_cost_minor,
        })
        .collect();
    let po = store.create_purchase_order(
        args.po_number.trim(),
        &args.supplier_id,
        args.expected_date.as_deref().unwrap_or_default(),
        args.notes.as_deref().unwrap_or_default(),
        None,
        &lines,
    )?;
    drop(db);
    Ok(PurchaseOrderDto::from(po))
}

#[command]
pub async fn update_po_status(
    args: UpdatePoStatusArgs,
    state: State<'_, AppState>,
) -> Result<PurchaseOrderDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let po = store.update_po_status(&args.id, &args.status)?;
    drop(db);
    Ok(PurchaseOrderDto::from(po))
}

#[command]
pub async fn receive_purchase_order(
    id: String,
    state: State<'_, AppState>,
) -> Result<PurchaseOrderDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let po = store.receive_purchase_order(&id)?;
    drop(db);
    Ok(PurchaseOrderDto::from(po))
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::{PurchaseOrder, PurchaseOrderLine, PurchaseOrderWithLines, Supplier};

    fn make_supplier() -> Supplier {
        Supplier {
            id: "sup-1".into(),
            code: "SUP001".into(),
            name: "Acme Corp".into(),
            contact_person: "John Doe".into(),
            phone: "+1234567890".into(),
            email: "john@acme.com".into(),
            address: "123 Main St".into(),
            tax_id: "TAX-001".into(),
            payment_terms: "NET30".into(),
            notes: String::new(),
            status: "active".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    fn make_po_line() -> PurchaseOrderLine {
        PurchaseOrderLine {
            id: "pol-1".into(),
            po_id: "po-1".into(),
            sku: "SKU-1".into(),
            product_name: "Widget".into(),
            qty: 10,
            unit_cost_minor: 5000,
            line_total_minor: 50000,
        }
    }

    // ── SupplierDto ─────────────────────────────────────────────────────

    #[test]
    fn supplier_dto_debug() {
        let dto = SupplierDto::from(make_supplier());
        let d = format!("{dto:?}");
        assert!(d.contains("Acme Corp"));
    }

    #[test]
    fn supplier_dto_serialize() {
        let dto = SupplierDto::from(make_supplier());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "Acme Corp");
        assert_eq!(json["status"], "active");
    }

    // ── PurchaseOrderLineDto ────────────────────────────────────────────

    #[test]
    fn purchase_order_line_dto_debug() {
        let dto = PurchaseOrderLineDto::from(make_po_line());
        let d = format!("{dto:?}");
        assert!(d.contains("SKU-1"));
    }

    #[test]
    fn purchase_order_line_dto_serialize() {
        let dto = PurchaseOrderLineDto::from(make_po_line());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["sku"], "SKU-1");
        assert_eq!(json["qty"], 10);
    }

    // ── PurchaseOrderDto ────────────────────────────────────────────────

    #[test]
    fn purchase_order_dto_debug() {
        let po_with_lines = PurchaseOrderWithLines {
            order: PurchaseOrder {
                id: "po-1".into(),
                po_number: "PO-2025-001".into(),
                supplier_id: "sup-1".into(),
                status: "draft".into(),
                order_date: "2025-01-01".into(),
                expected_date: "2025-01-15".into(),
                received_date: None,
                subtotal_minor: 50000,
                tax_minor: 5000,
                total_minor: 55000,
                notes: String::new(),
                created_by: Some("admin".into()),
                created_at: "2025-01-01T00:00:00.000Z".into(),
                updated_at: "2025-01-01T00:00:00.000Z".into(),
            },
            lines: vec![make_po_line()],
            supplier_name: Some("Acme Corp".into()),
        };
        let dto = PurchaseOrderDto::from(po_with_lines);
        let d = format!("{dto:?}");
        assert!(d.contains("PO-2025-001"));
    }

    #[test]
    fn purchase_order_dto_serialize() {
        let po_with_lines = PurchaseOrderWithLines {
            order: PurchaseOrder {
                id: "po-2".into(),
                po_number: "PO-2025-002".into(),
                supplier_id: "sup-2".into(),
                status: "pending".into(),
                order_date: "2025-02-01".into(),
                expected_date: "2025-02-15".into(),
                received_date: None,
                subtotal_minor: 100000,
                tax_minor: 10000,
                total_minor: 110000,
                notes: "Urgent".into(),
                created_by: None,
                created_at: "2025-02-01T00:00:00.000Z".into(),
                updated_at: "2025-02-01T00:00:00.000Z".into(),
            },
            lines: vec![],
            supplier_name: None,
        };
        let dto = PurchaseOrderDto::from(po_with_lines);
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["po_number"], "PO-2025-002");
        assert_eq!(json["total_minor"], 110000);
        assert!(json["lines"].as_array().unwrap().is_empty());
    }

    // ── CreateSupplierArgs ──────────────────────────────────────────────

    #[test]
    fn create_supplier_args_deserialize_minimal() {
        let json = r#"{"code":"SUP001","name":"Acme"}"#;
        let args: CreateSupplierArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.code, "SUP001");
        assert_eq!(args.contact_person, None);
    }

    #[test]
    fn create_supplier_args_debug() {
        let args = CreateSupplierArgs {
            code: "S1".into(),
            name: "Test".into(),
            contact_person: Some("Jane".into()),
            phone: None,
            email: None,
            address: None,
            tax_id: None,
            payment_terms: None,
            notes: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("Test"));
    }

    // ── UpdateSupplierArgs ──────────────────────────────────────────────

    #[test]
    fn update_supplier_args_deserialize() {
        let json = r##"{"id":"sup-1","code":"SUP001","name":"Acme","status":"active"}"##;
        let args: UpdateSupplierArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "sup-1");
        assert_eq!(args.status.as_deref(), Some("active"));
    }

    #[test]
    fn update_supplier_args_debug() {
        let args = UpdateSupplierArgs {
            id: "s1".into(),
            code: "C1".into(),
            name: "N1".into(),
            contact_person: None,
            phone: None,
            email: None,
            address: None,
            tax_id: None,
            payment_terms: None,
            notes: None,
            status: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("N1"));
    }

    // ── PoLineInput ─────────────────────────────────────────────────────

    #[test]
    fn po_line_input_deserialize() {
        let json = r#"{"sku":"SKU-1","product_name":"Widget","qty":10,"unit_cost_minor":5000}"#;
        let args: PoLineInput = serde_json::from_str(json).unwrap();
        assert_eq!(args.sku, "SKU-1");
        assert_eq!(args.unit_cost_minor, 5000);
    }

    #[test]
    fn po_line_input_debug() {
        let args = PoLineInput {
            sku: "S".into(),
            product_name: "P".into(),
            qty: 1,
            unit_cost_minor: 100,
        };
        let d = format!("{args:?}");
        assert!(d.contains("P"));
    }

    // ── CreatePurchaseOrderArgs ─────────────────────────────────────────

    #[test]
    fn create_purchase_order_args_deserialize_minimal() {
        let json = r#"{"po_number":"PO-001","supplier_id":"sup-1","lines":[]}"#;
        let args: CreatePurchaseOrderArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.po_number, "PO-001");
        assert_eq!(args.expected_date, None);
    }

    #[test]
    fn create_purchase_order_args_deserialize_full() {
        let json = r#"{"po_number":"PO-002","supplier_id":"sup-2","expected_date":"2025-06-01","notes":"Rush","lines":[{"sku":"SKU-A","product_name":"Widget","qty":5,"unit_cost_minor":1000}]}"#;
        let args: CreatePurchaseOrderArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.expected_date.as_deref(), Some("2025-06-01"));
        assert_eq!(args.lines.len(), 1);
    }

    #[test]
    fn create_purchase_order_args_debug() {
        let args = CreatePurchaseOrderArgs {
            po_number: "P1".into(),
            supplier_id: "S1".into(),
            expected_date: None,
            notes: None,
            lines: vec![],
        };
        let d = format!("{args:?}");
        assert!(d.contains("P1"));
    }

    // ── UpdatePoStatusArgs ──────────────────────────────────────────────

    #[test]
    fn update_po_status_args_deserialize() {
        let json = r#"{"id":"po-1","status":"approved"}"#;
        let args: UpdatePoStatusArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "po-1");
        assert_eq!(args.status, "approved");
    }

    #[test]
    fn update_po_status_args_debug() {
        let args = UpdatePoStatusArgs {
            id: "x".into(),
            status: "draft".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("draft"));
    }
}
