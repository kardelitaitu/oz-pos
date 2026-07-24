//! Shift management Tauri commands.
//!
//! Open/close cashier shifts with cash balance reconciliation.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::{ShiftPaymentBreakdown, ShiftReport, ShiftSalesByHour};
use oz_core::{CashPayout, Shift, Store};

use foundation::validate_not_empty;

use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Shift DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftDto {
    /// Unique identifier.
    pub id: String,
    /// ID of the associated user.
    pub user_id: String,
    /// ID of the associated terminal.
    pub terminal_id: Option<String>,
    /// Opened At.
    pub opened_at: String,
    /// Closed At.
    pub closed_at: Option<String>,
    /// Opening Balance Minor.
    pub opening_balance_minor: i64,
    /// Closing Balance Minor.
    pub closing_balance_minor: Option<i64>,
    /// Expected Cash Minor.
    pub expected_cash_minor: Option<i64>,
    /// Cash Difference Minor.
    pub cash_difference_minor: Option<i64>,
    /// Total Sales Minor.
    pub total_sales_minor: i64,
    /// Total Cash Minor.
    pub total_cash_minor: i64,
    /// Total Card Minor.
    pub total_card_minor: i64,
    /// Total Other Minor.
    pub total_other_minor: i64,
    /// Total Voids Minor.
    pub total_voids_minor: i64,
    /// Total Refunds Minor.
    pub total_refunds_minor: i64,
    /// Total Payouts Minor.
    pub total_payouts_minor: i64,
    /// Notes.
    pub notes: String,
    /// Current status.
    pub status: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl From<Shift> for ShiftDto {
    fn from(s: Shift) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            terminal_id: s.terminal_id,
            opened_at: s.opened_at,
            closed_at: s.closed_at,
            opening_balance_minor: s.opening_balance_minor,
            closing_balance_minor: s.closing_balance_minor,
            expected_cash_minor: s.expected_cash_minor,
            cash_difference_minor: s.cash_difference_minor,
            total_sales_minor: s.total_sales_minor,
            total_cash_minor: s.total_cash_minor,
            total_card_minor: s.total_card_minor,
            total_other_minor: s.total_other_minor,
            total_voids_minor: s.total_voids_minor,
            total_refunds_minor: s.total_refunds_minor,
            total_payouts_minor: s.total_payouts_minor,
            notes: s.notes,
            status: s.status,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Arguments for opening a new shift.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShiftArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// ID of the associated terminal.
    pub terminal_id: Option<String>,
    /// Opening Balance Minor.
    pub opening_balance_minor: i64,
}

/// Args for `open_shift_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShiftScopedArgs {
    /// ID of the associated terminal.
    pub terminal_id: Option<String>,
    /// Opening Balance Minor.
    pub opening_balance_minor: i64,
}

// ── Commands ──────────────────────────────────────────────────────────

/// Open a new shift for a user using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `open_shift_scoped`.
#[command]
pub async fn open_shift(
    args: OpenShiftArgs,
    state: State<'_, AppState>,
) -> Result<ShiftDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    require_permission_for_user(&store, &args.user_id, permissions::SHIFTS_OPEN)?;

    let shift = store.open_shift(
        &args.user_id,
        args.terminal_id.as_deref(),
        args.opening_balance_minor,
    )?;
    drop(db);

    tracing::info!(id = %shift.id, user_id = %shift.user_id, "shift opened");
    Ok(ShiftDto::from(shift))
}

/// Open a shift in the store resolved from a session token. ADR #7.
#[command]
pub async fn open_shift_scoped(
    session_token: String,
    args: OpenShiftScopedArgs,
    state: State<'_, AppState>,
) -> Result<ShiftDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    require_permission_for_user(&store, &session.user_id, permissions::SHIFTS_OPEN)?;

    let shift = store.open_shift(
        &session.user_id,
        args.terminal_id.as_deref(),
        args.opening_balance_minor,
    )?;
    drop(db);

    tracing::info!(id = %shift.id, user_id = %shift.user_id, "shift opened (scoped)");
    Ok(ShiftDto::from(shift))
}

/// Arguments for closing a shift.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseShiftArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Unique identifier.
    pub id: String,
    /// Closing Balance Minor.
    pub closing_balance_minor: i64,
    /// Notes.
    pub notes: Option<String>,
}

/// Args for `close_shift_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseShiftScopedArgs {
    /// Unique identifier.
    pub id: String,
    /// Closing Balance Minor.
    pub closing_balance_minor: i64,
    /// Notes.
    pub notes: Option<String>,
}

/// Close an active shift using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `close_shift_scoped`.
#[command]
pub async fn close_shift(
    args: CloseShiftArgs,
    state: State<'_, AppState>,
) -> Result<ShiftDto, AppError> {
    validate_not_empty("id", &args.id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    require_permission_for_user(&store, &args.user_id, permissions::SHIFTS_CLOSE)?;

    let shift = store.close_shift(&args.id, args.closing_balance_minor, args.notes.as_deref())?;
    drop(db);

    tracing::info!(id = %shift.id, "shift closed");
    Ok(ShiftDto::from(shift))
}

/// Close a shift in the store resolved from a session token. ADR #7.
#[command]
pub async fn close_shift_scoped(
    session_token: String,
    args: CloseShiftScopedArgs,
    state: State<'_, AppState>,
) -> Result<ShiftDto, AppError> {
    validate_not_empty("id", &args.id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    require_permission_for_user(&store, &session.user_id, permissions::SHIFTS_CLOSE)?;

    let shift = store.close_shift(&args.id, args.closing_balance_minor, args.notes.as_deref())?;
    drop(db);

    tracing::info!(id = %shift.id, "shift closed (scoped)");
    Ok(ShiftDto::from(shift))
}

/// Get the currently open shift for a user from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_active_shift_scoped`.
#[command]
pub async fn get_active_shift(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ShiftDto>, AppError> {
    validate_not_empty("user_id", &user_id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let shift = store.get_active_shift(&user_id)?;
    drop(db);

    Ok(shift.map(ShiftDto::from))
}

/// Get the active shift for the session user from the store-scoped DB. ADR #7.
#[command]
pub async fn get_active_shift_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Option<ShiftDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let shift = store.get_active_shift(&session.user_id)?;
    drop(db);

    Ok(shift.map(ShiftDto::from))
}

/// List all shifts, most recent first.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_shifts_scoped`.
#[command]
pub async fn list_shifts(state: State<'_, AppState>) -> Result<Vec<ShiftDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let shifts = store.list_shifts()?;
    drop(db);

    Ok(shifts.into_iter().map(ShiftDto::from).collect())
}

/// List shifts for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_shifts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<ShiftDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let shifts = store.list_shifts()?;
    drop(db);

    Ok(shifts.into_iter().map(ShiftDto::from).collect())
}

/// Get a single shift by id.
#[command]
pub async fn get_shift(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<ShiftDto>, AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let shift = store.get_shift(&id)?;
    drop(db);

    Ok(shift.map(ShiftDto::from))
}

// ── Shift Report DTOs ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Cashpayoutdto.
pub struct CashPayoutDto {
    /// Unique identifier.
    pub id: String,
    /// ID of the associated shift.
    pub shift_id: String,
    /// Amount Minor.
    pub amount_minor: i64,
    /// Reason.
    pub reason: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl From<CashPayout> for CashPayoutDto {
    fn from(p: CashPayout) -> Self {
        Self {
            id: p.id,
            shift_id: p.shift_id,
            amount_minor: p.amount_minor,
            reason: p.reason,
            created_at: p.created_at,
        }
    }
}

/// Shift report DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftReportDto {
    /// Shift.
    pub shift: ShiftDto,
    /// Payment Breakdown.
    pub payment_breakdown: Vec<ShiftPaymentBreakdownDto>,
    /// Hourly Breakdown.
    pub hourly_breakdown: Vec<ShiftSalesByHourDto>,
    /// Cash Payouts.
    pub cash_payouts: Vec<CashPayoutDto>,
    /// Sale Count.
    pub sale_count: i64,
    /// Void Count.
    pub void_count: i64,
    /// Refund Count.
    pub refund_count: i64,
}

impl From<ShiftReport> for ShiftReportDto {
    fn from(r: ShiftReport) -> Self {
        Self {
            shift: ShiftDto::from(r.shift),
            payment_breakdown: r.payment_breakdown.into_iter().map(Into::into).collect(),
            hourly_breakdown: r.hourly_breakdown.into_iter().map(Into::into).collect(),
            cash_payouts: r.cash_payouts.into_iter().map(Into::into).collect(),
            sale_count: r.sale_count,
            void_count: r.void_count,
            refund_count: r.refund_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Shiftpaymentbreakdowndto.
pub struct ShiftPaymentBreakdownDto {
    /// Method.
    pub method: String,
    /// Count.
    pub count: i64,
    /// Total amount in minor currency units.
    pub total_minor: i64,
}

impl From<ShiftPaymentBreakdown> for ShiftPaymentBreakdownDto {
    fn from(b: ShiftPaymentBreakdown) -> Self {
        Self {
            method: b.method,
            count: b.count,
            total_minor: b.total_minor,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Shiftsalesbyhourdto.
pub struct ShiftSalesByHourDto {
    /// Hour.
    pub hour: i64,
    /// Total amount in minor currency units.
    pub total_minor: i64,
    /// Sale Count.
    pub sale_count: i64,
}

impl From<ShiftSalesByHour> for ShiftSalesByHourDto {
    fn from(h: ShiftSalesByHour) -> Self {
        Self {
            hour: h.hour,
            total_minor: h.total_minor,
            sale_count: h.sale_count,
        }
    }
}

/// Arguments for creating a cash payout.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCashPayoutArgs {
    /// ID of the associated shift.
    pub shift_id: String,
    /// Amount Minor.
    pub amount_minor: i64,
    /// Reason.
    pub reason: String,
}

/// Record a cash payout (safe drop) against an open shift.
#[command]
pub async fn create_cash_payout(
    args: CreateCashPayoutArgs,
    state: State<'_, AppState>,
) -> Result<CashPayoutDto, AppError> {
    validate_not_empty("shift_id", &args.shift_id).map_err(|e| AppError::Invalid(e.to_string()))?;
    if args.amount_minor <= 0 {
        return Err(AppError::Invalid("amount_minor must be > 0".into()));
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let payout = store.create_cash_payout(&args.shift_id, args.amount_minor, &args.reason)?;
    drop(db);

    tracing::info!(id = %payout.id, shift_id = %args.shift_id, amount = %args.amount_minor, "cash payout recorded");
    Ok(CashPayoutDto::from(payout))
}

/// Generate a comprehensive report for a single shift.
#[command]
pub async fn get_shift_report(
    shift_id: String,
    state: State<'_, AppState>,
) -> Result<ShiftReportDto, AppError> {
    validate_not_empty("shift_id", &shift_id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let report = store.get_shift_report(&shift_id)?;
    drop(db);

    Ok(ShiftReportDto::from(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    fn seed_user(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-cashier', 'cashier', 'Cashier', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('user-1', 'alice', 'hash', 'Alice', 'role-cashier', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    #[test]
    fn open_shift_returns_dto() {
        let conn = fresh_conn();
        seed_user(&conn);
        let store = Store::new(&conn);

        let shift = store.open_shift("user-1", None, 500).unwrap();
        let dto = ShiftDto::from(shift);
        assert_eq!(dto.user_id, "user-1");
        assert_eq!(dto.opening_balance_minor, 500);
        assert_eq!(dto.status, "open");
        assert!(dto.closed_at.is_none());
    }

    #[test]
    fn close_shift_returns_closed_dto() {
        let conn = fresh_conn();
        seed_user(&conn);
        let store = Store::new(&conn);

        let shift = store.open_shift("user-1", None, 100).unwrap();
        let closed = store
            .close_shift(&shift.id, 200, Some("Good shift"))
            .unwrap();
        let dto = ShiftDto::from(closed);
        assert_eq!(dto.status, "closed");
        assert!(dto.closed_at.is_some());
        assert_eq!(dto.closing_balance_minor, Some(200));
        assert_eq!(dto.notes, "Good shift");
    }

    #[test]
    fn get_active_shift_returns_dto() {
        let conn = fresh_conn();
        seed_user(&conn);
        let store = Store::new(&conn);

        let shift = store.open_shift("user-1", None, 300).unwrap();
        let active = store.get_active_shift("user-1").unwrap().unwrap();
        let dto = ShiftDto::from(active);
        assert_eq!(dto.id, shift.id);
        assert_eq!(dto.opening_balance_minor, 300);
    }

    #[test]
    fn list_shifts_returns_dtos() {
        let conn = fresh_conn();
        seed_user(&conn);
        let store = Store::new(&conn);

        let s1 = store.open_shift("user-1", None, 100).unwrap();
        let s2 = store.open_shift("user-1", None, 200).unwrap();

        let shifts = store.list_shifts().unwrap();
        assert_eq!(shifts.len(), 2);
        let dtos: Vec<ShiftDto> = shifts.into_iter().map(ShiftDto::from).collect();
        assert_eq!(dtos[0].id, s2.id);
        assert_eq!(dtos[1].id, s1.id);
    }

    #[test]
    fn get_shift_returns_dto() {
        let conn = fresh_conn();
        seed_user(&conn);
        let store = Store::new(&conn);

        let shift = store.open_shift("user-1", None, 500).unwrap();
        let loaded = store.get_shift(&shift.id).unwrap().unwrap();
        let dto = ShiftDto::from(loaded);
        assert_eq!(dto.id, shift.id);
        assert_eq!(dto.opening_balance_minor, 500);
    }

    #[test]
    fn open_shift_invalid_args_rejected() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let err = store.open_shift("", None, 0).unwrap_err();
        assert!(matches!(err, oz_core::CoreError::Validation { field, .. } if field == "user_id"));

        let err = store.open_shift("user-1", None, -1).unwrap_err();
        assert!(
            matches!(err, oz_core::CoreError::Validation { field, .. } if field == "opening_balance_minor")
        );
    }

    #[test]
    fn close_shift_not_found() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let err = store.close_shift("nonexistent", 100, None).unwrap_err();
        assert!(matches!(err, oz_core::CoreError::NotFound { entity, .. } if entity == "shift"));
    }

    #[test]
    fn shifts_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn get_active_shift_nonexistent_user() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let active = store.get_active_shift("nobody").unwrap();
        assert!(active.is_none());
    }

    // -- DTO struct tests --

    #[test]
    fn shift_dto_debug() {
        let dto = ShiftDto {
            id: "s1".into(),
            user_id: "u1".into(),
            terminal_id: None,
            opened_at: "2025-01-01".into(),
            closed_at: None,
            opening_balance_minor: 500,
            closing_balance_minor: None,
            expected_cash_minor: None,
            cash_difference_minor: None,
            total_sales_minor: 0,
            total_cash_minor: 0,
            total_card_minor: 0,
            total_other_minor: 0,
            total_voids_minor: 0,
            total_refunds_minor: 0,
            total_payouts_minor: 0,
            notes: String::new(),
            status: "open".into(),
            created_at: "2025-01-01".into(),
            updated_at: "2025-01-01".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("s1"));
    }

    #[test]
    fn shift_dto_serialize() {
        let dto = ShiftDto {
            id: "s2".into(),
            user_id: "u2".into(),
            terminal_id: Some("t1".into()),
            opened_at: "2025-02-01".into(),
            closed_at: Some("2025-02-01".into()),
            opening_balance_minor: 1000,
            closing_balance_minor: Some(2000),
            expected_cash_minor: Some(1500),
            cash_difference_minor: Some(500),
            total_sales_minor: 5000,
            total_cash_minor: 3000,
            total_card_minor: 2000,
            total_other_minor: 0,
            total_voids_minor: 0,
            total_refunds_minor: 0,
            total_payouts_minor: 0,
            notes: "Good shift".into(),
            status: "closed".into(),
            created_at: "2025-02-01".into(),
            updated_at: "2025-02-01".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["status"], "closed");
        assert_eq!(json["totalSalesMinor"], 5000);
    }

    #[test]
    fn open_shift_args_deserialize() {
        let json = r##"{"userId":"u1","openingBalanceMinor":500}"##;
        let args: OpenShiftArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.user_id, "u1");
        assert_eq!(args.opening_balance_minor, 500);
        assert_eq!(args.terminal_id, None);
    }

    #[test]
    fn open_shift_args_debug() {
        let args = OpenShiftArgs {
            user_id: "u".into(),
            terminal_id: None,
            opening_balance_minor: 100,
        };
        let d = format!("{args:?}");
        assert!(d.contains("u"));
    }

    #[test]
    fn close_shift_args_deserialize() {
        let json = r##"{"userId":"u1","id":"s1","closingBalanceMinor":2000}"##;
        let args: CloseShiftArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "s1");
        assert_eq!(args.closing_balance_minor, 2000);
        assert_eq!(args.notes, None);
    }

    #[test]
    fn close_shift_args_debug() {
        let args = CloseShiftArgs {
            user_id: "u".into(),
            id: "s".into(),
            closing_balance_minor: 0,
            notes: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("s"));
    }

    #[test]
    fn cash_payout_dto_serialize() {
        let dto = CashPayoutDto {
            id: "cp1".into(),
            shift_id: "s1".into(),
            amount_minor: 1000,
            reason: "Safe drop".into(),
            created_at: "2025-01-01".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["amountMinor"], 1000);
        assert_eq!(json["reason"], "Safe drop");
    }

    #[test]
    fn cash_payout_dto_debug() {
        let dto = CashPayoutDto {
            id: "cp2".into(),
            shift_id: "s2".into(),
            amount_minor: 500,
            reason: "Test".into(),
            created_at: "2025-01-01".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("cp2"));
    }

    #[test]
    fn create_cash_payout_args_deserialize() {
        let json = r##"{"shiftId":"s1","amountMinor":1000,"reason":"Safe drop"}"##;
        let args: CreateCashPayoutArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.shift_id, "s1");
        assert_eq!(args.amount_minor, 1000);
    }

    #[test]
    fn create_cash_payout_args_debug() {
        let args = CreateCashPayoutArgs {
            shift_id: "s".into(),
            amount_minor: 100,
            reason: "R".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("R"));
    }
}
