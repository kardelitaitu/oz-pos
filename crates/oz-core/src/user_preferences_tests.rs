use super::*;
use crate::migrations;
use rusqlite::Connection;

fn setup_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&mut conn).unwrap();
    conn
}

#[test]
fn get_all_returns_empty_for_new_user() {
    let conn = setup_db();
    let prefs = UserPreferences::get_all(&conn, "user-1").unwrap();
    assert!(prefs.is_empty());
}

#[test]
fn set_batch_stores_and_retrieves_preferences() {
    let conn = setup_db();
    let user_id = "user-1";

    UserPreferences::set_batch(
        &conn,
        user_id,
        &[
            ("card_size".into(), "large".into()),
            ("font_size".into(), "14".into()),
        ],
    )
    .unwrap();

    let prefs = UserPreferences::get_all(&conn, user_id).unwrap();
    assert_eq!(prefs.len(), 2);
    assert_eq!(prefs.get("card_size").unwrap(), "large");
    assert_eq!(prefs.get("font_size").unwrap(), "14");
}

#[test]
fn set_batch_upserts_existing_keys() {
    let conn = setup_db();
    let user_id = "user-1";

    // Insert initial preferences.
    UserPreferences::set_batch(&conn, user_id, &[("theme".into(), "dark".into())]).unwrap();

    // Update the same key.
    UserPreferences::set_batch(&conn, user_id, &[("theme".into(), "light".into())]).unwrap();

    let prefs = UserPreferences::get_all(&conn, user_id).unwrap();
    assert_eq!(prefs.len(), 1);
    assert_eq!(prefs.get("theme").unwrap(), "light");
}

#[test]
fn preferences_are_scoped_per_user() {
    let conn = setup_db();

    UserPreferences::set_batch(&conn, "user-a", &[("lang".into(), "en".into())]).unwrap();

    UserPreferences::set_batch(&conn, "user-b", &[("lang".into(), "id".into())]).unwrap();

    let prefs_a = UserPreferences::get_all(&conn, "user-a").unwrap();
    let prefs_b = UserPreferences::get_all(&conn, "user-b").unwrap();
    assert_eq!(prefs_a.get("lang").unwrap(), "en");
    assert_eq!(prefs_b.get("lang").unwrap(), "id");
}

#[test]
fn set_batch_empty_is_noop() {
    let conn = setup_db();
    let result = UserPreferences::set_batch(&conn, "user-1", &[]);
    assert!(result.is_ok(), "empty batch should not error");
    let prefs = UserPreferences::get_all(&conn, "user-1").unwrap();
    assert!(prefs.is_empty());
}
