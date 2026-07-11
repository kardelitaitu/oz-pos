//! Per-user display preferences — card size, font size, theme, locale, etc.
//!
//! Preferences are stored as key-value pairs in the `user_preferences` table,
//! scoped by `user_id`. Each staff member can have their own settings that
//! persist across sessions and terminals.

use std::collections::HashMap;

use rusqlite::Connection;

use crate::error::CoreError;

/// Key-value user preferences stored in the `user_preferences` table.
///
/// Each row is scoped by `user_id` so every staff member can have their
/// own display preferences (card size, font size, etc.) that persist
/// across sessions and terminals.
pub struct UserPreferences;

impl UserPreferences {
    /// Load every preference for a user as a `(key, value)` map.
    pub fn get_all(conn: &Connection, user_id: &str) -> Result<HashMap<String, String>, CoreError> {
        let mut stmt = conn.prepare_cached(
            "SELECT pref_key, pref_value FROM user_preferences WHERE user_id = ?1",
        )?;
        let rows = stmt.query_map([user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    /// Upsert multiple preferences for a user inside a single transaction.
    pub fn set_batch(
        conn: &Connection,
        user_id: &str,
        prefs: &[(String, String)],
    ) -> Result<(), CoreError> {
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO user_preferences (user_id, pref_key, pref_value, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                 ON CONFLICT(user_id, pref_key)
                 DO UPDATE SET pref_value = excluded.pref_value,
                               updated_at  = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            )?;
            for (key, value) in prefs {
                stmt.execute(rusqlite::params![user_id, key, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
