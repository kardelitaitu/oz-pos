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
    pub id: String,
    pub user_id: String,
    pub terminal_id: Option<String>,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub opening_balance_minor: i64,
    pub closing_balance_minor: Option<i64>,
    pub expected_cash_minor: Option<i64>,
    pub cash_difference_minor: Option<i64>,
    pub total_sales_minor: i64,
    pub total_cash_minor: i64,
    pub total_card_minor: i64,
    pub total_other_minor: i64,
    pub total_voids_minor: i64,
    pub total_refunds_minor: i64,
    pub total_payouts_minor: i64,
    pub notes: String,
    pub status: String,
    pub created_at: String,
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
    pub user_id: String,
    pub terminal_id: Option<String>,
    pub opening_balance_minor: i64,
}

// ── Commands ──────────────────────────────────────────────────────────

/// Open a new shift for a user.
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

/// Arguments for closing a shift.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseShiftArgs {
    pub user_id: String,
    pub id: String,
    pub closing_balance_minor: i64,
    pub notes: Option<String>,
}

/// Close an active shift with a counted closing balance.
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

/// Get the currently open shift for a user, if any.
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

/// List all shifts, most recent first.
#[command]
pub async fn list_shifts(state: State<'_, AppState>) -> Result<Vec<ShiftDto>, AppError> {
    let db = state.db.lock().await;
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
pub struct CashPayoutDto {
    pub id: String,
    pub shift_id: String,
    pub amount_minor: i64,
    pub reason: String,
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
    pub shift: ShiftDto,
    pub payment_breakdown: Vec<ShiftPaymentBreakdownDto>,
    pub hourly_breakdown: Vec<ShiftSalesByHourDto>,
    pub cash_payouts: Vec<CashPayoutDto>,
    pub sale_count: i64,
    pub void_count: i64,
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
pub struct ShiftPaymentBreakdownDto {
    pub method: String,
    pub count: i64,
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
pub struct ShiftSalesByHourDto {
    pub hour: i64,
    pub total_minor: i64,
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
    pub shift_id: String,
    pub amount_minor: i64,
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
    fn get_active_shift_nonexistent_user() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let active = store.get_active_shift("nobody").unwrap();
        assert!(active.is_none());
    }
}
