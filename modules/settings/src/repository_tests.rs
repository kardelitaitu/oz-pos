use super::*;
use rusqlite::Connection;

fn fresh() -> Connection {
    oz_core::migrations::fresh_db()
}

#[test]
fn get_returns_none_for_missing_key() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    assert!(repo.get("nonexistent").unwrap().is_none());
}

#[test]
fn set_and_get_roundtrip() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    repo.set("theme", "dark").unwrap();
    let val = repo.get("theme").unwrap().unwrap();
    assert_eq!(val, "dark");
}

#[test]
fn set_overwrites_existing() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    repo.set("key", "old").unwrap();
    repo.set("key", "new").unwrap();
    let val = repo.get("key").unwrap().unwrap();
    assert_eq!(val, "new");
}

#[test]
fn set_empty_value() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    repo.set("empty", "").unwrap();
    let val = repo.get("empty").unwrap().unwrap();
    assert_eq!(val, "");
}

#[test]
fn multiple_keys_independent() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    repo.set("k1", "v1").unwrap();
    repo.set("k2", "v2").unwrap();
    assert_eq!(repo.get("k1").unwrap().unwrap(), "v1");
    assert_eq!(repo.get("k2").unwrap().unwrap(), "v2");
}

#[test]
fn set_updates_timestamp() {
    let conn = fresh();
    let repo = SettingsRepository::new(&conn);
    repo.set("ts-key", "first").unwrap();
    // Setting the same key again should not error (upsert)
    repo.set("ts-key", "second").unwrap();
    assert_eq!(repo.get("ts-key").unwrap().unwrap(), "second");
}
