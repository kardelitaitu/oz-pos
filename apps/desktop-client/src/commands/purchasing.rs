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
