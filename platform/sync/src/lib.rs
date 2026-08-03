#![warn(missing_docs)]

//! OZ-POS Sync Engine
//!
//! Offline-first sync with eventual consistency. Provides:
//!
//! - **Queue** — local change log backed by the `offline_queue` SQLite table
//! - **Transport** — async HTTP client for communicating with a remote sync server
//! - **Replication** — push pending changes / pull remote updates orchestration
//! - **Conflict** — last-write-wins (LWW) conflict resolution
//!
//! # Usage
//! ```ignore
//! # use platform_sync::{SyncEngine, SyncConfig};
//! # use oz_core::db::Store;
//! # use oz_core::migrations;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let conn = migrations::fresh_db();
//! # let store = Store::new(&conn);
//! let config = SyncConfig {
//!     server_url: "http://localhost:3099".into(),
//!     api_key: None,
//! };
//! let engine = SyncEngine::new(config);
//! let result = engine.run_sync_cycle(&store).await?;
//! # Ok(())
//! # }
//! ```

#![allow(clippy::items_after_test_module)]

pub mod conflict;
pub mod daemon;
pub mod pg_daemon;
pub mod pg_transport;
pub mod queue;
pub mod replication;
pub mod transport;

#[cfg(test)]
pub(crate) mod test_helpers;

use oz_core::db::Store;
use oz_core::sync_client::SyncConfig;

use crate::queue::SyncQueue;
use crate::replication::ReplicationResult;
use crate::transport::SyncTransport;

/// Convenience result type for sync operations.
pub type SyncResult<T> = Result<T, SyncError>;

/// Common sync error type.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Network or HTTP error communicating with the sync server.
    #[error("transport error: {0}")]
    Transport(String),

    /// Local queue operation failed (read/write/mark).
    #[error("queue error: {0}")]
    Queue(String),

    /// Replication logic error (push/pull cycle).
    #[error("replication error: {0}")]
    Replication(String),

    /// Conflict resolution failed.
    #[error("conflict error: {0}")]
    Conflict(String),

    /// Invalid or missing sync configuration.
    #[error("configuration error: {0}")]
    Config(String),

    /// The client's sync anchor (`since` timestamp) is older than the
    /// oldest retained row on the server. Data in that gap has been
    /// pruned (P-1 retention). The client should log a warning and
    /// retry on the next scheduled cycle.
    #[error("anchor expired: data older than {}", oldest_available.as_deref().unwrap_or("unknown"))]
    AnchorExpired {
        /// ISO-8601 timestamp of the oldest retained row on the server.
        oldest_available: Option<String>,
    },

    /// The sync server has been permanently migrated to a new URL
    /// (ADR #11). The client should update its local `sync_server_url`
    /// setting and reconnect on the next cycle.
    #[error("server migrated to {new_url}")]
    ServerMigrated {
        /// The new server URL to connect to.
        new_url: String,
    },

    /// Database error from the underlying oz-core store.
    #[error("database error: {0}")]
    Database(#[from] oz_core::error::CoreError),
}

impl From<reqwest::Error> for SyncError {
    fn from(e: reqwest::Error) -> Self {
        SyncError::Transport(e.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_literal_unwrap)]
mod tests {
    use super::*;
    use oz_core::offline::OfflineQueueItem;
    use oz_core::sync_client::SyncConfig;

    // ── build_batches ────────────────────────────────────────────

    #[test]
    fn build_batches_empty() {
        let batches = build_batches(&[], MAX_BATCH_BYTES);
        assert!(batches.is_empty());
    }

    #[test]
    fn build_batches_single_item() {
        let items = vec![OfflineQueueItem::new("test", "{}")];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
    }

    #[test]
    fn build_batches_multiple_items_one_batch() {
        let items: Vec<_> = (0..5)
            .map(|i| OfflineQueueItem::new("test", format!("{{\"n\":{i}}}")))
            .collect();
        // 5 tiny items should fit in one 64 KB batch.
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 5);
    }

    #[test]
    fn build_batches_respects_byte_limit() {
        // Create payloads that force splitting: each item serialises to
        // ~33 KB (payload + JSON envelope overhead). Two items exceed the
        // 64 KB budget, forcing a split after the first item.
        let big_payload = "x".repeat(33 * 1024);
        let small = "{}";
        let items = vec![
            OfflineQueueItem::new("a", &big_payload),
            OfflineQueueItem::new("b", &big_payload),
            OfflineQueueItem::new("c", small),
        ];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert!(
            batches.len() >= 2,
            "large items should cause splitting, got {} batches",
            batches.len()
        );
        // Each batch should have at least 1 item.
        for batch in &batches {
            assert!(!batch.is_empty(), "no empty batches allowed");
        }
    }

    #[test]
    fn build_batches_sorts_by_priority() {
        use oz_core::offline::SyncPriority;

        let critical = OfflineQueueItem::with_priority("a", "{}", SyncPriority::Critical);
        let normal = OfflineQueueItem::with_priority("b", "{}", SyncPriority::Normal);
        let low = OfflineQueueItem::with_priority("c", "{}", SyncPriority::Low);
        // Put them in reverse priority order to verify sorting.
        let items = vec![low.clone(), normal.clone(), critical.clone()];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        // All 3 small items should fit in one batch, but Critical must be first.
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch[0].priority, SyncPriority::Critical);
        assert_eq!(batch[1].priority, SyncPriority::Normal);
        assert_eq!(batch[2].priority, SyncPriority::Low);
    }

    #[test]
    fn build_batches_minimum_one_item_per_batch() {
        // An item larger than the byte limit still gets its own batch
        // (minimum 1 item per batch, no empty requests).
        let huge = "x".repeat(128 * 1024); // 128 KB payload
        let items = vec![OfflineQueueItem::new("huge", &huge)];
        let batches = build_batches(&items, MAX_BATCH_BYTES);
        assert_eq!(batches.len(), 1, "single huge item still gets a batch");
        assert_eq!(batches[0].len(), 1);
    }

    // ── SyncError ────────────────────────────────────────────────

    #[test]
    fn sync_error_transport_display() {
        let err = SyncError::Transport("connection timeout".into());
        assert_eq!(err.to_string(), "transport error: connection timeout");
    }

    #[test]
    fn sync_error_queue_display() {
        let err = SyncError::Queue("item not found".into());
        assert_eq!(err.to_string(), "queue error: item not found");
    }

    #[test]
    fn sync_error_replication_display() {
        let err = SyncError::Replication("push failed".into());
        assert_eq!(err.to_string(), "replication error: push failed");
    }

    #[test]
    fn sync_error_conflict_display() {
        let err = SyncError::Conflict("version mismatch".into());
        assert_eq!(err.to_string(), "conflict error: version mismatch");
    }

    #[test]
    fn sync_error_config_display() {
        let err = SyncError::Config("missing server URL".into());
        assert_eq!(err.to_string(), "configuration error: missing server URL");
    }

    #[test]
    fn sync_error_database_display() {
        let err = SyncError::Database(oz_core::CoreError::NotFound {
            entity: "item",
            id: "x".into(),
        });
        let msg = err.to_string();
        assert!(
            msg.contains("database error"),
            "expected database error, got: {msg}"
        );
        assert!(
            msg.contains("not found"),
            "expected 'not found' in message, got: {msg}"
        );
    }

    #[test]
    fn sync_error_server_migrated_display() {
        let err = SyncError::ServerMigrated {
            new_url: "https://new.example.com".into(),
        };
        assert_eq!(
            err.to_string(),
            "server migrated to https://new.example.com"
        );
    }

    #[test]
    fn sync_error_server_migrated_debug() {
        let err = SyncError::ServerMigrated {
            new_url: "https://new.example.com".into(),
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("ServerMigrated"));
        assert!(debug.contains("https://new.example.com"));
    }

    #[test]
    fn sync_error_debug() {
        let err = SyncError::Transport("e".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn sync_error_from_requwest_error() {
        // Verify the From<reqwest::Error> impl compiles by checking the
        // conversion function signature at compile time.
        fn assert_convert(_e: reqwest::Error) -> SyncError {
            SyncError::from(_e)
        }
        let _ = assert_convert;
    }

    // ── SyncEngine ───────────────────────────────────────────────

    #[test]
    fn sync_engine_new_creates_transport() {
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let engine = SyncEngine::new(config);
        assert_eq!(engine.config.server_url, "http://localhost:3099");
    }

    #[test]
    fn sync_engine_new_with_api_key() {
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: Some("sk-key".into()),
        };
        let engine = SyncEngine::new(config);
        assert_eq!(engine.config.api_key, Some("sk-key".into()));
    }

    // ── SyncResult ───────────────────────────────────────────────

    #[test]
    fn sync_result_ok() {
        let result: SyncResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn sync_result_err() {
        let result: SyncResult<i32> = Err(SyncError::Config("bad config".into()));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "configuration error: bad config"
        );
    }

    // ── ADR #11: run_sync_cycle snapshot redirect propagation ─

    #[tokio::test]
    async fn run_sync_cycle_propagates_snapshot_server_migrated() {
        use oz_core::db::Store;
        use oz_core::migrations;

        let new_url = "https://snapshot-propagated.example.com";
        // Server returns 410 on pull → triggers AnchorExpired → snapshot
        // path. Snapshot returns 421 → ServerMigrated should propagate.
        let server_url = crate::test_helpers::spawn_anchor_then_redirect_server(new_url).await;

        let db = migrations::fresh_db();
        let store = Store::new(&db);
        // Enqueue one item so push succeeds (server accepts everything),
        // then pull gets 410 → snapshot gets 421.
        store
            .enqueue_offline("test_action", r#"{"val":1}"#)
            .unwrap();

        let config = SyncConfig {
            server_url: server_url.clone(),
            api_key: None,
        };
        let engine = SyncEngine::new(config);

        let result = engine.run_sync_cycle(&store).await;

        match result {
            Err(SyncError::ServerMigrated { new_url: url }) => {
                assert_eq!(url, new_url, "ServerMigrated should carry the new_url");
            }
            other => panic!(
                "expected SyncError::ServerMigrated from snapshot path, got {:?}",
                other
            ),
        }
    }

    #[tokio::test]
    async fn run_sync_cycle_propagates_pull_server_migrated() {
        use oz_core::db::Store;
        use oz_core::migrations;

        let new_url = "https://pull-propagated.example.com";
        // Server returns 421 on all endpoints — pull gets it directly.
        let server_url = crate::test_helpers::spawn_redirect_server(new_url).await;

        let db = migrations::fresh_db();
        let store = Store::new(&db);

        let config = SyncConfig {
            server_url: server_url.clone(),
            api_key: None,
        };
        let engine = SyncEngine::new(config);

        let result = engine.run_sync_cycle(&store).await;

        match result {
            Err(SyncError::ServerMigrated { new_url: url }) => {
                assert_eq!(url, new_url, "ServerMigrated should carry the new_url");
            }
            other => panic!(
                "expected SyncError::ServerMigrated from pull path, got {:?}",
                other
            ),
        }
    }

    // ── P1-4: import_snapshot tests ───────────────────────────────

    /// Seed a role so user FK constraints are satisfied.
    fn seed_role(conn: &rusqlite::Connection, id: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, name) VALUES (?1, ?2)",
            rusqlite::params![id, format!("Role {id}")],
        )
        .unwrap();
    }

    fn verify_product_sku_exists(sku: &str, store: &Store<'_>) -> bool {
        store.product_id_by_sku(sku).ok().flatten().is_some()
    }

    /// Build a typed snapshot product (RUST-04) with valid defaults.
    fn product(sku: &str, name: &str, price_minor: i64) -> transport::SnapshotProduct {
        transport::SnapshotProduct {
            id: format!("id-{sku}"),
            sku: sku.to_owned(),
            name: name.to_owned(),
            price_minor,
            currency: "USD".to_owned(),
            category_id: None,
            barcode: None,
            created_at: None,
            updated_at: None,
            price_updated_at: None,
            track_serial: false,
            store_id: None,
        }
    }

    /// Build a typed snapshot tax rate (RUST-04) with valid defaults.
    fn tax_rate(id: &str, name: &str, rate_bps: i64) -> transport::SnapshotTaxRate {
        transport::SnapshotTaxRate {
            id: id.to_owned(),
            name: name.to_owned(),
            rate_bps,
            is_default: false,
            is_inclusive: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Build a typed snapshot user (RUST-04) with valid defaults.
    fn user(username: &str, display_name: &str, role_id: &str) -> transport::SnapshotUser {
        transport::SnapshotUser {
            id: format!("id-{username}"),
            username: username.to_owned(),
            display_name: display_name.to_owned(),
            role_id: role_id.to_owned(),
            is_active: true,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn import_snapshot_empty_returns_zero() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![],
            users: vec![],
        };
        let count = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(count, 0, "empty snapshot should import 0 rows");
    }

    #[test]
    fn import_snapshot_single_product() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-1".into(),
                sku: "COFFEE-001".into(),
                name: "Coffee Beans".into(),
                price_minor: 15000,
                currency: "IDR".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        let count = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(count, 1, "one product should import 1 row");

        // Verify the product was created.
        assert!(store.product_id_by_sku("COFFEE-001").unwrap().is_some());
    }

    #[test]
    fn import_snapshot_rejects_blank_sku() {
        // RUST-04: blank required fields must be rejected BEFORE the
        // transaction opens (previously they imported with defaults).
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-bad".into(),
                sku: "  ".into(),
                name: "No SKU Product".into(),
                price_minor: 100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "product with blank sku must be rejected (RUST-04)"
        );
        assert!(!verify_product_sku_exists("", &store));
    }

    #[test]
    fn import_snapshot_rejects_blank_name() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-bad".into(),
                sku: "NO-NAME".into(),
                name: String::new(),
                price_minor: 100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "product with blank name must be rejected (RUST-04)"
        );
        assert!(!verify_product_sku_exists("NO-NAME", &store));
    }

    #[test]
    fn import_snapshot_rejects_negative_price() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-bad".into(),
                sku: "NEG-PRICE".into(),
                name: "Negative Price".into(),
                price_minor: -100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "product with negative price_minor must be rejected (RUST-04)"
        );
    }

    #[test]
    fn import_snapshot_rejects_blank_tax_rate() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![transport::SnapshotTaxRate {
                id: String::new(),
                name: "Blank Tax".into(),
                rate_bps: 1000,
                is_default: false,
                is_inclusive: false,
                created_at: None,
                updated_at: None,
            }],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "tax rate with blank id must be rejected (RUST-04)"
        );
    }

    #[test]
    fn import_snapshot_rejects_blank_user_fields() {
        // RUST-04: users must carry username/display_name/role_id;
        // previously a missing role_id imported as the empty string
        // (masking a malformed snapshot).
        let conn = oz_core::migrations::fresh_db();
        seed_role(&conn, "role-real");
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![],
            users: vec![transport::SnapshotUser {
                id: "u-corrupt".into(),
                username: "corrupted-staff".into(),
                display_name: "Corrupted Staff".into(),
                role_id: String::new(),
                is_active: true,
                created_at: None,
                updated_at: None,
            }],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "user with blank role_id must be rejected (RUST-04)"
        );
        let users = store.list_users().unwrap();
        assert!(!users.iter().any(|u| u.username == "corrupted-staff"));
    }

    #[test]
    fn import_snapshot_rejects_newer_schema_version() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 999,
            products: vec![product("V-TOO-NEW", "Too New", 100)],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "snapshot with unsupported schema version must be rejected (RUST-04)"
        );
        assert!(!verify_product_sku_exists("V-TOO-NEW", &store));
    }

    #[test]
    fn import_snapshot_idempotent_second_call_same_count() {
        let conn = oz_core::migrations::fresh_db();
        seed_role(&conn, "role-1");
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![product("IDEMPOTENT-1", "Idempotent Product", 5000)],
            tax_rates: vec![tax_rate("tax-vat-10", "VAT 10%", 1000)],
            users: vec![user("admin", "Admin", "role-1")],
        };
        let first = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(first, 3, "first import: 3 rows");

        let second = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(
            second, 3,
            "second import should also return 3 (ON CONFLICT upserts)"
        );
    }

    #[test]
    fn import_snapshot_overwrites_existing_product() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot_v1 = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![product("UPDATABLE", "Old Name", 1000)],
            tax_rates: vec![],
            users: vec![],
        };
        import_snapshot(&store, &snapshot_v1).unwrap();

        let snapshot_v2 = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![product("UPDATABLE", "New Name", 2000)],
            tax_rates: vec![],
            users: vec![],
        };
        import_snapshot(&store, &snapshot_v2).unwrap();

        assert!(store.product_id_by_sku("UPDATABLE").unwrap().is_some());
    }

    #[test]
    fn import_snapshot_overwrites_existing_user() {
        let conn = oz_core::migrations::fresh_db();
        seed_role(&conn, "role-admin");
        let store = Store::new(&conn);
        let snapshot_v1 = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![],
            users: vec![transport::SnapshotUser {
                id: "u-staff".into(),
                username: "staff-1".into(),
                display_name: "Old Display".into(),
                role_id: "role-admin".into(),
                is_active: true,
                created_at: None,
                updated_at: None,
            }],
        };
        import_snapshot(&store, &snapshot_v1).unwrap();

        let snapshot_v2 = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![],
            users: vec![transport::SnapshotUser {
                id: "u-staff".into(),
                username: "staff-1".into(),
                display_name: "New Display".into(),
                role_id: "role-admin".into(),
                is_active: false,
                created_at: None,
                updated_at: None,
            }],
        };
        import_snapshot(&store, &snapshot_v2).unwrap();

        let users = store.list_users().unwrap();
        let user = users.into_iter().find(|u| u.username == "staff-1").unwrap();
        // SYNC-06: pin_hash is NEVER read from the snapshot. The first
        // import writes the non-verifiable placeholder, and the second
        // import preserves it (the UPDATE clause omits pin_hash) — even
        // though the snapshot carried "new-hash", it must not land in DB.
        assert_eq!(user.pin_hash, "!snapshot-no-credential!");
        assert_ne!(user.pin_hash, "new-hash");
        assert_eq!(user.display_name, "New Display");
        assert!(!user.is_active);
    }

    #[test]
    fn import_snapshot_rejects_corrupted_product() {
        // RUST-04: a corrupted row (missing required fields) is rejected at
        // deserialization; a blank name is rejected here before the
        // transaction opens — never imported with defaults.
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-corrupt".into(),
                sku: "CORRUPTED".into(),
                name: String::new(),
                price_minor: 100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "corrupted product must be rejected, not imported with defaults (RUST-04)"
        );
        assert!(!verify_product_sku_exists("CORRUPTED", &store));
    }

    #[test]
    fn import_snapshot_out_of_schema_fields_ignored() {
        let conn = oz_core::migrations::fresh_db();
        seed_role(&conn, "role-1");
        let store = Store::new(&conn);
        // RUST-04: unknown/extra fields stay wire-compatible — serde drops
        // them during deserialization (no deny_unknown_fields), so a server
        // that adds forward-compatible fields does not break the client.
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-extra".into(),
                sku: "EXTRA-FIELDS".into(),
                name: "Has Extra".into(),
                price_minor: 100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![tax_rate("tax-extra", "Extra Tax", 500)],
            users: vec![user("extra-user", "Extra User", "role-1")],
        };
        // Also assert the wire shape tolerates unknown keys at the serde
        // boundary (unknown fields are ignored, matching the DTO derives).
        let wire = serde_json::json!({
            "version": 1,
            "products": [{"id":"p-extra","sku":"EXTRA-FIELDS","name":"Has Extra","price_minor":100,"currency":"USD","future_field":"kept"}],
            "tax_rates": [{"id":"tax-extra","name":"Extra Tax","rate_bps":500,"future_flag":true}],
            "users": [{"id":"u-extra","username":"extra-user","display_name":"Extra User","role_id":"role-1","metadata":"ignored"}]
        });
        let _rt: transport::SyncSnapshotResponse =
            serde_json::from_value(wire).expect("unknown fields are tolerated");
        let count = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(count, 3, "all 3 entities with extra fields should import");
    }

    #[test]
    fn import_snapshot_all_types_multiple_entities() {
        let conn = oz_core::migrations::fresh_db();
        seed_role(&conn, "r1");
        seed_role(&conn, "r2");
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![
                product("A", "Product A", 100),
                product("B", "Product B", 200),
                product("C", "Product C", 300),
            ],
            tax_rates: vec![tax_rate("tax-ppn", "PPN", 1100)],
            users: vec![user("user-a", "A", "r1"), user("user-b", "B", "r2")],
        };
        let count = import_snapshot(&store, &snapshot).unwrap();
        assert_eq!(count, 6, "3 products + 1 tax rate + 2 users = 6 rows");

        // Verify all products exist.
        assert!(verify_product_sku_exists("A", &store));
        assert!(verify_product_sku_exists("B", &store));
        assert!(verify_product_sku_exists("C", &store));

        // Verify tax rate exists.
        let tax = store.get_tax_rate("tax-ppn").unwrap().unwrap();
        assert_eq!(tax.rate_bps, 1100);

        // Verify users exist.
        let users = store.list_users().unwrap();
        assert!(users.iter().any(|u| u.username == "user-a"));
        assert!(users.iter().any(|u| u.username == "user-b"));
    }

    #[test]
    fn import_snapshot_partial_rollback_on_error() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);

        // First import valid product data.
        let valid = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![product("VALID", "Valid", 100)],
            tax_rates: vec![],
            users: vec![],
        };
        import_snapshot(&store, &valid).unwrap();
        assert!(verify_product_sku_exists("VALID", &store));

        // Now try to import a user with a non-existent role_id (FK violation).
        let invalid = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![],
            tax_rates: vec![],
            users: vec![user("broken-user", "Broken", "nonexistent-role")],
        };
        let result = import_snapshot(&store, &invalid);
        assert!(result.is_err(), "FK violation should cause error");

        // The invalid user should NOT be in the DB (transaction rolled back).
        let users = store.list_users().unwrap();
        assert!(
            !users.iter().any(|u| u.username == "broken-user"),
            "broken user should not exist after rollback"
        );

        // Previously valid product should still exist (separate transaction).
        assert!(
            verify_product_sku_exists("VALID", &store),
            "previously imported product should survive"
        );
    }

    #[test]
    fn import_snapshot_null_barcode_stored_as_null() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![transport::SnapshotProduct {
                id: "p-nobc".into(),
                sku: "NO-BARCODE".into(),
                name: "No Barcode".into(),
                price_minor: 100,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
            }],
            tax_rates: vec![],
            users: vec![],
        };
        import_snapshot(&store, &snapshot).unwrap();

        let exists = verify_product_sku_exists("NO-BARCODE", &store);
        assert!(exists, "product with null barcode should be created");
    }

    #[test]
    fn import_snapshot_preserves_store_scoping() {
        // Phase B: the snapshot import must land store-tagged rows scoped.
        // A product tagged with store-a stays visible only to store-a (plus
        // the global catalog) — never store-b's — exercising the ?13
        // store_id write-through in the products upsert.
        let conn = oz_core::migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO store_profiles (id, name) VALUES \
             ('store-a', 'Store A'), ('store-b', 'Store B')",
        )
        .unwrap();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![
                transport::SnapshotProduct {
                    id: "p-a".into(),
                    sku: "SKU-A".into(),
                    name: "Prod A".into(),
                    price_minor: 100,
                    currency: "USD".into(),
                    category_id: None,
                    barcode: None,
                    created_at: None,
                    updated_at: None,
                    price_updated_at: None,
                    track_serial: false,
                    store_id: Some("store-a".into()),
                },
                transport::SnapshotProduct {
                    id: "p-b".into(),
                    sku: "SKU-B".into(),
                    name: "Prod B".into(),
                    price_minor: 200,
                    currency: "USD".into(),
                    category_id: None,
                    barcode: None,
                    created_at: None,
                    updated_at: None,
                    price_updated_at: None,
                    track_serial: false,
                    store_id: Some("store-b".into()),
                },
                transport::SnapshotProduct {
                    id: "p-g".into(),
                    sku: "SKU-G".into(),
                    name: "Prod Global".into(),
                    price_minor: 300,
                    currency: "USD".into(),
                    category_id: None,
                    barcode: None,
                    created_at: None,
                    updated_at: None,
                    price_updated_at: None,
                    track_serial: false,
                    store_id: None,
                },
            ],
            tax_rates: vec![],
            users: vec![],
        };
        import_snapshot(&store, &snapshot).unwrap();

        let a = store.list_products_for_store("store-a").unwrap();
        let mut a_ids: Vec<&str> = a.iter().map(|p| p.product.sku.as_str()).collect();
        a_ids.sort_unstable();
        assert_eq!(
            a_ids,
            vec!["SKU-A", "SKU-G"],
            "store-a must see its own imported row plus the global row"
        );

        let b = store.list_products_for_store("store-b").unwrap();
        let mut b_ids: Vec<&str> = b.iter().map(|p| p.product.sku.as_str()).collect();
        b_ids.sort_unstable();
        assert_eq!(
            b_ids,
            vec!["SKU-B", "SKU-G"],
            "store-b must see its own imported row plus the global row"
        );
    }

    #[test]
    fn import_snapshot_unknown_store_id_fails_closed_and_rolls_back() {
        // Phase B: a snapshot row tagged with a store the local DB does not
        // know must fail the FK and roll back the WHOLE import (no partial
        // products) — the same fail-closed contract as the oz-core path.
        let conn = oz_core::migrations::fresh_db();
        conn.execute(
            "INSERT INTO store_profiles (id, name) VALUES ('store-a', 'Store A')",
            [],
        )
        .unwrap();
        let store = Store::new(&conn);
        let snapshot = transport::SyncSnapshotResponse {
            version: 1,
            products: vec![
                transport::SnapshotProduct {
                    id: "p-ok".into(),
                    sku: "SKU-OK".into(),
                    name: "Valid".into(),
                    price_minor: 100,
                    currency: "USD".into(),
                    category_id: None,
                    barcode: None,
                    created_at: None,
                    updated_at: None,
                    price_updated_at: None,
                    track_serial: false,
                    store_id: Some("store-a".into()),
                },
                transport::SnapshotProduct {
                    id: "p-ghost".into(),
                    sku: "SKU-GHOST".into(),
                    name: "Ghost".into(),
                    price_minor: 200,
                    currency: "USD".into(),
                    category_id: None,
                    barcode: None,
                    created_at: None,
                    updated_at: None,
                    price_updated_at: None,
                    track_serial: false,
                    store_id: Some("ghost-store".into()),
                },
            ],
            tax_rates: vec![],
            users: vec![],
        };
        let result = import_snapshot(&store, &snapshot);
        assert!(
            result.is_err(),
            "snapshot row for an unknown store must fail the FK"
        );

        // No partial import — the whole transaction rolled back.
        let count: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 0,
            "failed import must leave no products behind (transaction rolled back)"
        );
    }
}

/// The top-level sync engine that orchestrates queue, transport, replication,
/// and conflict resolution for a single sync cycle.
pub struct SyncEngine {
    /// Sync configuration (server URL, API key).
    pub config: SyncConfig,
    /// HTTP transport for communicating with the remote sync server.
    pub transport: SyncTransport,
}

/// Maximum bytes per batch (64 KB). P-1 retention spec §Batching.
pub const MAX_BATCH_BYTES: usize = 64 * 1024;

/// Split pending items into batches that each serialise to ≤ `max_bytes`
/// bytes of JSON. Ensures at least one item per batch (no empty requests).
///
/// Items are sorted by priority (P-2) before chunking: all Critical items
/// transmit before any Normal item, which transmit before Low items.
/// Within each priority tier, original arrival order is preserved.
pub fn build_batches(
    items: &[oz_core::offline::OfflineQueueItem],
    max_bytes: usize,
) -> Vec<Vec<oz_core::offline::OfflineQueueItem>> {
    // Sort by priority (Critical=0, Normal=1, Low=2) — stable sort
    // preserves arrival order within each tier.
    let mut sorted: Vec<oz_core::offline::OfflineQueueItem> = items.to_vec();
    sorted.sort_by_key(|item| item.priority);

    let mut batches: Vec<Vec<oz_core::offline::OfflineQueueItem>> = Vec::new();
    let mut current: Vec<oz_core::offline::OfflineQueueItem> = Vec::new();
    let mut current_bytes = 0usize;

    for item in &sorted {
        // Estimate the JSON size of this item alone.
        let item_bytes = serde_json::to_vec(item).map(|v| v.len()).unwrap_or(0);

        // If adding this item would exceed the budget and we already have
        // items in the current batch, finalise and start a new batch.
        if !current.is_empty() && current_bytes + item_bytes > max_bytes {
            batches.push(std::mem::take(&mut current));
            current_bytes = 0;
        }

        current_bytes += item_bytes;
        current.push(item.clone());
    }

    // Don't drop the last partial batch.
    if !current.is_empty() {
        batches.push(current);
    }

    batches
}

/// Import a server snapshot into the local store (P-3 Step 5).
///
/// Upserts products (by SKU), tax rates (by ID), and users (by username)
/// inside a single transaction. Returns the total number of rows written.
fn import_snapshot(
    store: &Store<'_>,
    snapshot: &transport::SyncSnapshotResponse,
) -> SyncResult<usize> {
    // RUST-04: reject malformed reference data BEFORE opening the import
    // transaction. The typed DTOs already fail deserialization when required
    // fields are missing; here we reject blank values and invalid numeric
    // ranges that serde cannot catch (empty strings deserialize fine).
    if snapshot.version > transport::SNAPSHOT_SCHEMA_VERSION {
        return Err(SyncError::Replication(format!(
            "snapshot schema version {} is newer than supported version {}",
            snapshot.version,
            transport::SNAPSHOT_SCHEMA_VERSION
        )));
    }
    for p in &snapshot.products {
        if p.sku.trim().is_empty() || p.name.trim().is_empty() || p.currency.trim().is_empty() {
            return Err(SyncError::Replication(format!(
                "snapshot product has blank required field (sku='{}', name='{}', currency='{}')",
                p.sku, p.name, p.currency
            )));
        }
        if p.price_minor < 0 {
            return Err(SyncError::Replication(format!(
                "snapshot product '{}' has negative price_minor {}",
                p.sku, p.price_minor
            )));
        }
    }
    for r in &snapshot.tax_rates {
        if r.id.trim().is_empty() || r.name.trim().is_empty() {
            return Err(SyncError::Replication(
                "snapshot tax rate has blank id or name".to_owned(),
            ));
        }
        if r.rate_bps < 0 {
            return Err(SyncError::Replication(format!(
                "snapshot tax rate '{}' has negative rate_bps {}",
                r.id, r.rate_bps
            )));
        }
    }
    for u in &snapshot.users {
        if u.username.trim().is_empty()
            || u.display_name.trim().is_empty()
            || u.role_id.trim().is_empty()
        {
            return Err(SyncError::Replication(format!(
                "snapshot user '{}' has blank username/display_name/role_id",
                u.username
            )));
        }
    }

    let conn = store.conn();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| SyncError::Replication(format!("snapshot import tx: {e}")))?;

    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut count = 0usize;

    // Upsert products by SKU.
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO products (id, sku, name, price_minor, currency,
                                       category_id, barcode, created_at, updated_at,
                                       price_updated_at, track_serial, store_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                         COALESCE(?8, ?11), COALESCE(?9, ?11), COALESCE(?10, ?11), ?12, ?13)
                 ON CONFLICT(sku) DO UPDATE SET
                     name            = excluded.name,
                     price_minor     = excluded.price_minor,
                     currency        = excluded.currency,
                     category_id     = excluded.category_id,
                     barcode         = excluded.barcode,
                     updated_at      = COALESCE(excluded.updated_at, ?11),
                     price_updated_at = COALESCE(excluded.price_updated_at, ?11),
                     track_serial    = excluded.track_serial,
                     store_id        = excluded.store_id",
            )
            .map_err(|e| SyncError::Replication(format!("prepare products: {e}")))?;

        for p in &snapshot.products {
            stmt.execute(rusqlite::params![
                p.id,
                p.sku,
                p.name,
                p.price_minor,
                p.currency,
                p.category_id.as_deref(),
                p.barcode.as_deref(),
                p.created_at.as_deref(),
                p.updated_at.as_deref(),
                p.price_updated_at.as_deref(),
                now,
                p.track_serial as i64,
                p.store_id.as_deref(),
            ])
            .map_err(|e| SyncError::Replication(format!("upsert product: {e}")))?;
            count += 1;
        }
    }

    // Upsert tax rates by ID.
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive,
                                        created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, COALESCE(?6, ?8), COALESCE(?7, ?8))
                 ON CONFLICT(id) DO UPDATE SET
                     name         = excluded.name,
                     rate_bps     = excluded.rate_bps,
                     is_default   = excluded.is_default,
                     is_inclusive = excluded.is_inclusive,
                     updated_at   = COALESCE(excluded.updated_at, ?8)",
            )
            .map_err(|e| SyncError::Replication(format!("prepare tax_rates: {e}")))?;

        for r in &snapshot.tax_rates {
            stmt.execute(rusqlite::params![
                r.id,
                r.name,
                r.rate_bps,
                r.is_default as i64,
                r.is_inclusive as i64,
                r.created_at.as_deref(),
                r.updated_at.as_deref(),
                now,
            ])
            .map_err(|e| SyncError::Replication(format!("upsert tax_rate: {e}")))?;
            count += 1;
        }
    }

    // Upsert users by username.
    //
    // SYNC-06: `pin_hash` is deliberately NEVER read from the snapshot —
    // credential verifier material must not travel over the sync channel.
    // New rows get a non-verifiable placeholder, and on conflict the
    // EXISTING local hash is preserved (the UPDATE clause omits pin_hash),
    // so an import can neither replicate credentials nor lock out an
    // operator who already has a working PIN.
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                                    is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE(?7, ?9), COALESCE(?8, ?9))
                 ON CONFLICT(username) DO UPDATE SET
                     display_name = excluded.display_name,
                     role_id      = excluded.role_id,
                     is_active    = excluded.is_active,
                     updated_at   = COALESCE(excluded.updated_at, ?9)",
            )
            .map_err(|e| SyncError::Replication(format!("prepare users: {e}")))?;

        for u in &snapshot.users {
            stmt.execute(rusqlite::params![
                u.id,
                u.username,
                oz_core::sync_client::SNAPSHOT_PIN_HASH_PLACEHOLDER,
                u.display_name,
                u.role_id,
                u.is_active as i64,
                u.created_at.as_deref(),
                u.updated_at.as_deref(),
                now,
            ])
            .map_err(|e| SyncError::Replication(format!("upsert user: {e}")))?;
            count += 1;
        }
    }

    tx.commit()
        .map_err(|e| SyncError::Replication(format!("snapshot import commit: {e}")))?;

    Ok(count)
}

impl SyncEngine {
    /// Create a new sync engine from the given configuration.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            transport: SyncTransport::new(&config.server_url, config.api_key.as_deref()),
            config,
        }
    }

    /// Run a full sync cycle: push pending items in batches, then pull remote updates.
    ///
    /// Items are split into ≤ 64 KB batches (P-1 batching) and sent sequentially.
    /// Each batch commits independently — a failure in batch N does not roll back
    /// the results of batches 1..N-1.
    ///
    /// A pre-sync health check verifies the server is reachable before pushing
    /// any data. If the health check fails, the cycle is skipped with an info log
    /// rather than an error — this prevents noisy error logs when the server is
    /// intentionally offline.
    ///
    /// Returns a [`ReplicationResult`] with counts of pushed/pulled items.
    pub async fn run_sync_cycle(&self, store: &Store<'_>) -> SyncResult<ReplicationResult> {
        // Pre-sync health check — skip the full cycle if the server is unreachable.
        match self.transport.health_check().await {
            Ok(()) => {
                tracing::debug!(
                    url = %self.config.server_url,
                    "sync health check passed"
                );
            }
            Err(e) => {
                tracing::info!(
                    url = %self.config.server_url,
                    error = %e,
                    "sync health check failed — skipping sync cycle"
                );
                return Ok(ReplicationResult {
                    pushed: 0,
                    pulled: 0,
                });
            }
        }

        let cycle_start = std::time::Instant::now();
        let queue = SyncQueue::new();

        // Phase 1: Push pending local changes in batches.
        let pending = queue.list_pending(store)?;
        let pending_count = pending.len();
        let mut total_pushed = 0usize;
        let mut total_bytes_sent = 0usize;
        let batch_count;

        if !pending.is_empty() {
            let batches = build_batches(&pending, MAX_BATCH_BYTES);
            batch_count = batches.len();
            for (batch_idx, batch) in batches.iter().enumerate() {
                let batch_items = batch.len();
                let batch_bytes = serde_json::to_vec(batch).map(|v| v.len()).unwrap_or(0);
                total_bytes_sent += batch_bytes;

                tracing::debug!(
                    batch = batch_idx + 1,
                    total_batches = batch_count,
                    items = batch_items,
                    bytes = batch_bytes,
                    "pushing batch"
                );

                let results = self.transport.push_items(batch).await?;
                for (item, outcome) in batch.iter().zip(results.iter()) {
                    match outcome {
                        transport::PushOutcome::Accepted => {
                            queue.mark_synced(store, &item.id)?;
                        }
                        transport::PushOutcome::Conflict(server_item) => {
                            // SYNC-02: single shared conflict-application
                            // service — identical ADR #21 strategy whether the
                            // conflict is processed here or by the daemon.
                            queue.apply_push_conflict(store, item, server_item)?;
                        }
                        transport::PushOutcome::Rejected { reason } => {
                            queue.mark_failed(store, &item.id, reason)?;
                        }
                    }
                }
                total_pushed += results.len();
            }
        } else {
            batch_count = 0;
        }

        // Phase 2: Pull remote updates from the server.
        // P-3: Paginated pull — loop until next_cursor is null.
        let last_sync = queue.last_synced_at(store)?;
        let mut total_pulled = 0usize;
        let mut cursor: Option<String> = None;
        let mut pages = 0u32;

        loop {
            pages += 1;
            let pull_result = match self
                .transport
                .pull_updates(last_sync.as_deref(), cursor.as_deref())
                .await
            {
                Ok(result) => result,
                Err(SyncError::AnchorExpired { oldest_available }) => {
                    tracing::warn!(
                        oldest_available = oldest_available,
                        "sync anchor expired — fetching snapshot to recover"
                    );
                    // P-3 Step 5: fetch the server's snapshot and import it.
                    match self.transport.fetch_snapshot().await {
                        Ok(snapshot) => {
                            let snapshot_count = import_snapshot(store, &snapshot)?;
                            tracing::info!(
                                products = snapshot.products.len(),
                                tax_rates = snapshot.tax_rates.len(),
                                users = snapshot.users.len(),
                                imported = snapshot_count,
                                "snapshot imported successfully after anchor expiry"
                            );
                        }
                        Err(e) => {
                            // ADR #11: Propagate server migration redirect so
                            // the daemon can update the local sync_server_url.
                            if matches!(&e, SyncError::ServerMigrated { .. }) {
                                return Err(e);
                            }
                            tracing::error!(
                                error = %e,
                                "snapshot fetch failed after anchor expiry; will retry next cycle"
                            );
                        }
                    }
                    return Ok(ReplicationResult {
                        pushed: total_pushed,
                        pulled: total_pulled,
                    });
                }
                Err(e) => return Err(e),
            };

            let page_count = pull_result.items.len();
            total_pulled += page_count;
            let has_more = pull_result.next_cursor.is_some();

            tracing::debug!(
                page = pages,
                items = page_count,
                has_more = has_more,
                "pulled page"
            );

            for remote_item in &pull_result.items {
                queue.apply_remote(store, remote_item)?;
            }

            cursor = pull_result.next_cursor;
            if !has_more {
                break;
            }
        }

        let elapsed_ms = cycle_start.elapsed().as_millis() as u64;

        tracing::info!(
            pending = pending_count,
            pushed = total_pushed,
            pulled = total_pulled,
            batches = batch_count,
            pages = pages,
            bytes_sent = total_bytes_sent,
            elapsed_ms = elapsed_ms,
            "sync cycle complete"
        );

        Ok(ReplicationResult {
            pushed: total_pushed,
            pulled: total_pulled,
        })
    }
}
