use super::*;
use oz_core::migrations;
use rusqlite::Connection;

fn fresh_conn() -> Connection {
    migrations::fresh_db()
}

#[test]
fn terminals_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[test]
fn list_terminals_empty_db() {
    let conn = fresh_conn();
    let terminals = run_list_terminals(&conn).unwrap();
    assert!(terminals.is_empty());
}

#[test]
fn list_terminals_with_seeded_data() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t1 = Terminal::new("Front Counter", "host-01");
    store.create_terminal(&t1).unwrap();
    let t2 = Terminal::new("Drive-Thru", "host-02");
    store.create_terminal(&t2).unwrap();

    let terminals = run_list_terminals(&conn).unwrap();
    assert_eq!(terminals.len(), 2);
    assert_eq!(terminals[0].name, "Drive-Thru");
    assert_eq!(terminals[1].name, "Front Counter");
}

#[test]
fn register_and_get_terminal() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Back Office", "host-03")
        .with_secret("s3cr3t")
        .with_metadata(r#"{"os":"windows"}"#);
    store.create_terminal(&t).unwrap();

    let loaded = store.get_terminal(&t.id).unwrap().unwrap();
    assert_eq!(loaded.name, "Back Office");
    assert_eq!(loaded.device_id, "host-03");
    assert_eq!(loaded.terminal_secret, Some("s3cr3t".into()));
    assert!(loaded.is_active);
}

#[test]
fn get_terminal_by_device_id() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Counter", "host-04");
    store.create_terminal(&t).unwrap();

    let loaded = store.get_terminal_by_device_id("host-04").unwrap().unwrap();
    assert_eq!(loaded.id, t.id);
    assert_eq!(loaded.name, "Counter");
}

#[test]
fn get_terminal_not_found() {
    let conn = fresh_conn();
    let store = Store::new(&conn);
    let t = store.get_terminal("nonexistent").unwrap();
    assert!(t.is_none());
}

#[test]
fn update_terminal_fields() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Old Name", "host-05");
    store.create_terminal(&t).unwrap();

    let mut updated = t.clone();
    updated.name = "New Name".into();
    store.update_terminal(&updated).unwrap();

    let loaded = store.get_terminal(&t.id).unwrap().unwrap();
    assert_eq!(loaded.name, "New Name");
}

#[test]
fn update_terminal_not_found() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Ghost", "ghost");
    let err = store.update_terminal(&t).unwrap_err();
    assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
}

#[test]
fn ping_terminal_updates_timestamp() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Counter", "host-06");
    store.create_terminal(&t).unwrap();

    assert!(
        store
            .get_terminal(&t.id)
            .unwrap()
            .unwrap()
            .last_seen_at
            .is_none()
    );

    store.ping_terminal(&t.id).unwrap();
    let loaded = store.get_terminal(&t.id).unwrap().unwrap();
    assert!(
        loaded.last_seen_at.is_some(),
        "ping should set last_seen_at"
    );
}

#[test]
fn ping_terminal_not_found() {
    let conn = fresh_conn();
    let store = Store::new(&conn);
    let err = store.ping_terminal("nope").unwrap_err();
    assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
}

#[test]
fn delete_terminal_removes_row() {
    let conn = fresh_conn();
    let store = Store::new(&conn);

    let t = Terminal::new("Temp", "host-07");
    store.create_terminal(&t).unwrap();
    store.delete_terminal(&t.id).unwrap();

    let loaded = store.get_terminal(&t.id).unwrap();
    assert!(loaded.is_none());
}

#[test]
fn delete_terminal_not_found() {
    let conn = fresh_conn();
    let store = Store::new(&conn);
    let err = store.delete_terminal("nope").unwrap_err();
    assert!(matches!(err, oz_core::CoreError::NotFound { .. }));
}

// -- DTO struct tests --

#[test]
fn terminal_dto_debug() {
    let dto = TerminalDto {
        id: "t1".into(),
        name: "Front Counter".into(),
        device_id: "host-01".into(),
        is_active: true,
        last_seen_at: None,
        metadata: None,
        created_at: "2025-01-01".into(),
        updated_at: "2025-01-01".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("Front Counter"));
}

#[test]
fn terminal_dto_serialize() {
    let dto = TerminalDto {
        id: "t2".into(),
        name: "Drive-Thru".into(),
        device_id: "host-02".into(),
        is_active: false,
        last_seen_at: Some("2025-06-01".into()),
        metadata: Some(r#"{"os":"linux"}"#.into()),
        created_at: "2025-01-01".into(),
        updated_at: "2025-01-01".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["name"], "Drive-Thru");
    assert_eq!(json["isActive"], false);
}

#[test]
fn register_terminal_args_deserialize() {
    let json = r##"{"name":"POS-1","deviceId":"host-03"}"##;
    let args: RegisterTerminalArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "POS-1");
    assert_eq!(args.terminal_secret, None);
}

#[test]
fn register_terminal_args_debug() {
    let args = RegisterTerminalArgs {
        name: "N".into(),
        device_id: "D".into(),
        terminal_secret: None,
        metadata: None,
    };
    let d = format!("{args:?}");
    assert!(d.contains("N"));
}

#[test]
fn register_terminal_result_serialize() {
    let result = RegisterTerminalResult { id: "t99".into() };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["id"], "t99");
}

#[test]
fn register_terminal_result_debug() {
    let result = RegisterTerminalResult { id: "t42".into() };
    let d = format!("{result:?}");
    assert!(d.contains("t42"));
}

#[test]
fn update_terminal_args_deserialize_minimal() {
    let json = r##"{"id":"t1"}"##;
    let args: UpdateTerminalArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "t1");
    assert_eq!(args.name, None);
    assert_eq!(args.is_active, None);
}

#[test]
fn update_terminal_args_debug() {
    let args = UpdateTerminalArgs {
        id: "x".into(),
        name: None,
        device_id: None,
        terminal_secret: None,
        is_active: None,
        metadata: None,
    };
    let d = format!("{args:?}");
    assert!(d.contains("x"));
}

#[test]
fn update_terminal_result_serialize() {
    let result = UpdateTerminalResult { id: "t-up".into() };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["id"], "t-up");
}
