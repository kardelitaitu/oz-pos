use super::*;
use oz_core::migrations;
use rusqlite::Connection;

fn fresh_conn() -> Connection {
    migrations::fresh_db()
}

fn seed_user(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-staff', 'staff', 'Staff', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
            ('user-1', 'alice', 'hash', 'Alice', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
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
    store.close_shift(&s1.id, 150, None).unwrap();
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
