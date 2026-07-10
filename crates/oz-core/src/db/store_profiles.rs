//! Store-profile CRUD — list, get, create, update, set-primary.
//!
//! Every deployment has exactly one primary store, created on first
//! startup by the `platform-startup` crate. Additional stores can be
//! added / removed via these methods.

use rusqlite::params;

use super::Store;
use crate::{CoreError, StoreProfile};

impl Store<'_> {
    /// List all store profiles ordered by `created_at`.
    pub fn list_store_profiles(&self) -> Result<Vec<StoreProfile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at
             FROM store_profiles ORDER BY is_primary DESC, created_at ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_store_profile)?;
        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row?);
        }
        Ok(profiles)
    }

    /// Get a single store profile by id.
    pub fn get_store_profile(&self, id: &str) -> Result<Option<StoreProfile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at
             FROM store_profiles WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::row_to_store_profile)?;
        match rows.next() {
            Some(Ok(profile)) => Ok(Some(profile)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Get the primary store profile.
    pub fn get_primary_store(&self) -> Result<Option<StoreProfile>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at
             FROM store_profiles WHERE is_primary = 1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], Self::row_to_store_profile)?;
        match rows.next() {
            Some(Ok(profile)) => Ok(Some(profile)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Create a new store profile.
    ///
    /// The new store will be **non-primary** by default. Use
    /// [`set_primary_store`](Self::set_primary_store) to promote it after
    /// creation.
    pub fn create_store_profile(&self, profile: &StoreProfile) -> Result<StoreProfile, CoreError> {
        self.conn.execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                profile.id,
                profile.name,
                profile.address,
                profile.tax_id,
                profile.currency,
                profile.timezone,
                profile.is_primary as i32,
                profile.created_at,
                profile.updated_at,
            ],
        )?;
        Ok(profile.clone())
    }

    /// Update a store profile's mutable fields (name, address, tax_id, currency, timezone).
    ///
    /// Returns `NotFound` if the id does not exist.
    pub fn update_store_profile(
        &self,
        id: &str,
        name: &str,
        address: &str,
        tax_id: &str,
        currency: &str,
        timezone: &str,
    ) -> Result<StoreProfile, CoreError> {
        let affected = self.conn.execute(
            "UPDATE store_profiles SET name = ?1, address = ?2, tax_id = ?3,
             currency = ?4, timezone = ?5, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?6",
            params![name, address, tax_id, currency, timezone, id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "store_profile",
                id: id.to_owned(),
            });
        }
        self.get_store_profile(id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "store_profile",
                id: id.to_owned(),
            })
    }

    /// Promote a store to primary, demoting the current primary.
    ///
    /// Uses an explicit transaction so the `is_primary` invariant
    /// (exactly one row with `is_primary = 1`) is never violated.
    pub fn set_primary_store(&self, id: &str) -> Result<StoreProfile, CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        // Demote the current primary.
        tx.execute(
            "UPDATE store_profiles SET is_primary = 0 WHERE is_primary = 1",
            [],
        )?;
        // Promote the target.
        let affected = tx.execute(
            "UPDATE store_profiles SET is_primary = 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            tx.rollback()?;
            return Err(CoreError::NotFound {
                entity: "store_profile",
                id: id.to_owned(),
            });
        }
        tx.commit()?;
        self.get_store_profile(id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "store_profile",
                id: id.to_owned(),
            })
    }

    /// Delete a store profile. The primary store cannot be deleted.
    pub fn delete_store_profile(&self, id: &str) -> Result<(), CoreError> {
        // Prevent deleting the primary store.
        if let Some(profile) = self.get_store_profile(id)? {
            if profile.is_primary {
                return Err(CoreError::Validation {
                    field: "id",
                    message: "cannot delete the primary store".into(),
                });
            }
        } else {
            return Err(CoreError::NotFound {
                entity: "store_profile",
                id: id.to_owned(),
            });
        }
        self.conn
            .execute("DELETE FROM store_profiles WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Row mapper ───────────────────────────────────────────────

    fn row_to_store_profile(row: &rusqlite::Row) -> rusqlite::Result<StoreProfile> {
        let is_primary_int: i32 = row.get("is_primary")?;
        Ok(StoreProfile {
            id: row.get("id")?,
            name: row.get("name")?,
            address: row.get("address")?,
            tax_id: row.get("tax_id")?,
            currency: row.get("currency")?,
            timezone: row.get("timezone")?,
            is_primary: is_primary_int != 0,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn setup() -> (Store<'static>, String) {
        let conn = migrations::fresh_db();
        let conn: &'static rusqlite::Connection = Box::leak(Box::new(conn));
        let store = Store::new(conn);

        // Migration 025 seeds a default store_profiles row (id='default',
        // is_primary=0). Update it to is_primary=1 with full test data.
        // We use UPDATE rather than INSERT OR REPLACE because the latter
        // triggers a DELETE (blocked by ON DELETE RESTRICT from
        // workspace_instances referencing store_profiles).
        conn.execute(
            "UPDATE store_profiles SET
                name = ?1,
                address = ?2,
                tax_id = ?3,
                currency = ?4,
                timezone = ?5,
                is_primary = 1,
                created_at = ?6,
                updated_at = ?7
             WHERE id = 'default'",
            rusqlite::params![
                "Main Store",
                "123 Main St",
                "TAX-001",
                "USD",
                "America/New_York",
                "2026-06-30T12:00:00Z",
                "2026-06-30T12:00:00Z",
            ],
        )
        .unwrap();
        (store, "default".into())
    }

    #[test]
    fn list_returns_seeded_primary() {
        let (store, _) = setup();
        let profiles = store.list_store_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].is_primary);
    }

    #[test]
    fn get_returns_seeded_primary() {
        let (store, id) = setup();
        let profile = store.get_store_profile(&id).unwrap().unwrap();
        assert_eq!(profile.name, "Main Store");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let (store, _) = setup();
        let profile = store.get_store_profile("nonexistent").unwrap();
        assert!(profile.is_none());
    }

    #[test]
    fn get_primary_returns_seeded() {
        let (store, _) = setup();
        let profile = store.get_primary_store().unwrap().unwrap();
        assert_eq!(profile.id, "default");
        assert!(profile.is_primary);
    }

    #[test]
    fn create_second_store() {
        let (store, _) = setup();
        let second = StoreProfile {
            id: uuid::Uuid::now_v7().to_string(),
            name: "Branch 2".into(),
            address: "456 Oak Ave".into(),
            tax_id: "TAX-002".into(),
            currency: "USD".into(),
            timezone: "America/Chicago".into(),
            is_primary: false,
            created_at: "2026-06-30T13:00:00Z".into(),
            updated_at: "2026-06-30T13:00:00Z".into(),
        };
        store.create_store_profile(&second).unwrap();
        let profiles = store.list_store_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn update_store_profile() {
        let (store, id) = setup();
        let updated = store
            .update_store_profile(&id, "Updated Store", "456 New St", "TAX-999", "USD", "UTC")
            .unwrap();
        assert_eq!(updated.name, "Updated Store");
        assert_eq!(updated.address, "456 New St");
    }

    #[test]
    fn update_nonexistent_returns_not_found() {
        let (store, _) = setup();
        let err = store
            .update_store_profile("nonexistent", "X", "", "", "USD", "UTC")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn set_primary_store_promotes_and_demotes() {
        let (store, primary_id) = setup();
        let second = StoreProfile {
            id: uuid::Uuid::now_v7().to_string(),
            name: "Branch 2".into(),
            address: "".into(),
            tax_id: "".into(),
            currency: "USD".into(),
            timezone: "UTC".into(),
            is_primary: false,
            created_at: "2026-06-30T13:00:00Z".into(),
            updated_at: "2026-06-30T13:00:00Z".into(),
        };
        store.create_store_profile(&second).unwrap();

        // Promote branch 2 to primary.
        let promoted = store.set_primary_store(&second.id).unwrap();
        assert!(promoted.is_primary);

        // Original primary should now be non-primary.
        let original = store.get_store_profile(&primary_id).unwrap().unwrap();
        assert!(!original.is_primary);

        // Only one primary.
        let primaries: Vec<_> = store
            .list_store_profiles()
            .unwrap()
            .into_iter()
            .filter(|p| p.is_primary)
            .collect();
        assert_eq!(primaries.len(), 1);
    }

    #[test]
    fn set_primary_nonexistent_returns_not_found() {
        let (store, _) = setup();
        let err = store.set_primary_store("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn delete_second_store() {
        let (store, _) = setup();
        let second = StoreProfile {
            id: uuid::Uuid::now_v7().to_string(),
            name: "Branch 2".into(),
            address: "".into(),
            tax_id: "".into(),
            currency: "USD".into(),
            timezone: "UTC".into(),
            is_primary: false,
            created_at: "2026-06-30T13:00:00Z".into(),
            updated_at: "2026-06-30T13:00:00Z".into(),
        };
        store.create_store_profile(&second).unwrap();
        store.delete_store_profile(&second.id).unwrap();
        let profiles = store.list_store_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
    }

    #[test]
    fn delete_primary_store_rejected() {
        let (store, id) = setup();
        let err = store.delete_store_profile(&id).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "id", .. }));
    }

    #[test]
    fn delete_nonexistent_returns_not_found() {
        let (store, _) = setup();
        let err = store.delete_store_profile("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    /// ADR #6: Deleting a store that has workspace instances must be rejected
    /// by the ON DELETE RESTRICT foreign key constraint.
    #[test]
    fn delete_store_with_workspace_instances_rejected() {
        let (store, _) = setup();
        let second = StoreProfile {
            id: "store-branch".into(),
            name: "Branch".into(),
            address: "".into(),
            tax_id: "".into(),
            currency: "USD".into(),
            timezone: "UTC".into(),
            is_primary: false,
            created_at: "2026-06-30T13:00:00Z".into(),
            updated_at: "2026-06-30T13:00:00Z".into(),
        };
        store.create_store_profile(&second).unwrap();

        // Create a workspace instance referencing this store.
        store.conn.execute(
            "INSERT INTO workspace_instances (id, type_key, store_id, name) VALUES (?1, 'store-pos', ?2, 'Branch POS')",
            rusqlite::params!["wi-branch-pos", "store-branch"],
        ).unwrap();

        // Attempt to delete the store — must fail due to FK RESTRICT.
        let err = store.delete_store_profile("store-branch").unwrap_err();
        assert!(
            matches!(err, CoreError::Db(_)),
            "expected DB error from FK constraint, got: {err:?}"
        );

        // Clean up the workspace instance first, then deletion works.
        store
            .conn
            .execute(
                "DELETE FROM workspace_instances WHERE id = ?1",
                rusqlite::params!["wi-branch-pos"],
            )
            .unwrap();
        store.delete_store_profile("store-branch").unwrap();
    }

    /// ADR #6: user_store_access FK also enforces ON DELETE RESTRICT.
    #[test]
    fn delete_store_with_user_access_rejected() {
        let (store, _) = setup();
        let second = StoreProfile {
            id: "store-b2".into(),
            name: "Branch 2".into(),
            address: "".into(),
            tax_id: "".into(),
            currency: "USD".into(),
            timezone: "UTC".into(),
            is_primary: false,
            created_at: "2026-06-30T14:00:00Z".into(),
            updated_at: "2026-06-30T14:00:00Z".into(),
        };
        store.create_store_profile(&second).unwrap();

        // Seed a user and assign store access.
        store.conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-cashier', 'Cashier', '', '[]');
             INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('u-cashier', 'cash', 'hash', 'Cash', 'r-cashier');
             INSERT INTO user_store_access (user_id, store_id, access_level) VALUES ('u-cashier', 'store-b2', 'operator');"
        ).unwrap();

        // Attempt to delete the store — must fail due to FK RESTRICT.
        let err = store.delete_store_profile("store-b2").unwrap_err();
        assert!(
            matches!(err, CoreError::Db(_)),
            "expected DB error from FK constraint, got: {err:?}"
        );

        // Clean up the user access, then deletion works.
        store
            .conn
            .execute(
                "DELETE FROM user_store_access WHERE store_id = ?1",
                rusqlite::params!["store-b2"],
            )
            .unwrap();
        store.delete_store_profile("store-b2").unwrap();
    }
}
