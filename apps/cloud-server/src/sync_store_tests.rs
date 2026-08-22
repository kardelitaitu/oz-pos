use super::*;

fn fresh_db() -> Arc<Mutex<Connection>> {
    Arc::new(Mutex::new(oz_core::migrations::fresh_db()))
}

fn sample_item(id: &str) -> OfflineQueueItem {
    OfflineQueueItem {
        id: id.to_owned(),
        action: "complete_sale".into(),
        payload: r#"{"total":100}"#.into(),
        status: OfflineQueueStatus::Pending,
        retry_count: 0,
        last_error: None,
        tenant_id: "default".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        synced_at: None,
        priority: SyncPriority::Normal,
    }
}

/// The SQLite backend must round-trip a push → pull → snapshot through
/// the same abstraction the handlers use, proving backend parity is
/// exercised in unit tests (the full Postgres path is covered by the
/// integration test below).
#[tokio::test]
async fn sqlite_backend_push_pull_plan_snapshot_roundtrip() {
    let conn = fresh_db();
    let store = SyncStore::sqlite(conn.clone());

    // Plan gating: no row → None, then a set row → Pro.
    assert_eq!(store.get_tenant_plan("tenant-a").await.unwrap(), None);
    {
        let conn = conn.lock().await;
        oz_core::Store::new(&conn)
            .set_tenant_plan("tenant-a", TenantPlan::Pro)
            .unwrap();
    }
    assert_eq!(
        store.get_tenant_plan("tenant-a").await.unwrap(),
        Some(TenantPlan::Pro)
    );

    // Push two items, one a duplicate.
    let item = sample_item("id-1");
    assert!(matches!(
        store.push_item(&item, "tenant-a").await.unwrap(),
        PushOutcome::Accepted
    ));
    assert!(matches!(
        store.push_item(&item, "tenant-a").await.unwrap(),
        PushOutcome::Rejected { .. }
    ));

    // Pull returns the one accepted item.
    let items = store
        .pull_items("tenant-a", Some("2026-01-01T00:00:00Z"), None, 501)
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "id-1");

    // Status counts reflect the queue.
    assert_eq!(store.pending_count("tenant-a").await, 1);
    assert_eq!(store.distinct_tenant_count().await, 1);

    // Snapshot is empty but well-formed for a tenant with no products.
    let (products, tax_rates, users) = store.snapshot_all("tenant-a").await.unwrap();
    assert_eq!(products.len(), 0);
    assert_eq!(tax_rates.len(), 0);
    assert_eq!(users.len(), 0);
}

/// Duplicate-id detection for the Postgres path keys on SQLSTATE 23505,
/// not on the error message (unlike SQLite's "UNIQUE" substring).
#[tokio::test]
async fn sqlite_duplicate_rejection_uses_unique_substring() {
    let conn = fresh_db();
    let store = SyncStore::sqlite(conn);
    let item = sample_item("dup-id");
    store.push_item(&item, "default").await.unwrap();
    match store.push_item(&item, "default").await.unwrap() {
        PushOutcome::Rejected { reason } => {
            assert!(reason.contains("duplicate id: dup-id"), "got: {reason}");
        }
        other => panic!("expected Rejected, got: {other:?}"),
    }
}

/// `push_batch` must persist every item in one lock acquisition (SQLite)
/// and return one outcome per item, in order, matching `push_item` on the
/// same inputs — proving the batched path is a drop-in for the loop.
#[tokio::test]
async fn sqlite_push_batch_matches_per_item_semantics() {
    let conn = fresh_db();
    let store = SyncStore::sqlite(conn.clone());

    let items = vec![
        sample_item("batch-1"),
        sample_item("batch-2"),
        sample_item("batch-3"),
    ];
    let outcomes = store.push_batch(&items, "tenant-b").await.unwrap();
    assert_eq!(outcomes.len(), 3);
    assert!(matches!(outcomes[0], PushOutcome::Accepted));
    assert!(matches!(outcomes[1], PushOutcome::Accepted));
    assert!(matches!(outcomes[2], PushOutcome::Accepted));

    // Duplicate within the same batch: second copy is Rejected, the
    // remaining items still Accepted (no batch-wide rollback).
    let dup_batch = vec![
        sample_item("batch-4"),
        sample_item("batch-4"),
        sample_item("batch-5"),
    ];
    let outcomes = store.push_batch(&dup_batch, "tenant-b").await.unwrap();
    assert_eq!(outcomes.len(), 3);
    assert!(matches!(outcomes[0], PushOutcome::Accepted));
    match &outcomes[1] {
        PushOutcome::Rejected { reason } => {
            assert!(reason.contains("duplicate id: batch-4"), "got: {reason}");
        }
        other => panic!("expected Rejected for dup, got: {other:?}"),
    }
    assert!(matches!(outcomes[2], PushOutcome::Accepted));

    // Pull confirms exactly the accepted rows landed.
    let pulled = store
        .pull_items("tenant-b", Some("2026-01-01T00:00:00Z"), None, 501)
        .await
        .unwrap();
    let mut ids: Vec<_> = pulled.iter().map(|i| i.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec!["batch-1", "batch-2", "batch-3", "batch-4", "batch-5"]
    );
}

/// An empty batch must return an empty outcome list without error on both
/// backends — no pointless transaction is opened or committed.
#[tokio::test]
async fn store_push_batch_empty_returns_empty() {
    let conn = fresh_db();
    let store = SyncStore::sqlite(conn.clone());

    let outcomes = store.push_batch(&[], "tenant-empty").await.unwrap();
    assert!(outcomes.is_empty(), "empty batch → no outcomes");
    assert_eq!(store.pending_count("tenant-empty").await, 0);
}

/// Integration test against a live Postgres instance (the same Docker
/// service `db.rs` uses, port 15432). Skips when unreachable, so the
/// suite stays green on machines without a running Postgres.
#[tokio::test]
async fn pg_integration_push_pull_plan_snapshot_roundtrip() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG sync-store integration test skipped: {e}");
            return;
        }
    };

    let tenant = format!("pg-sync-store-test-{}", uuid::Uuid::now_v7());
    let store = SyncStore::postgres(pool.clone());

    // Seed a plan and exercise every method end-to-end.
    {
        let client = pool.get().await.unwrap();
        client
            .execute(
                "INSERT INTO tenant_plans (tenant_id, plan, updated_at) VALUES ($1, 'pro', now()::text)
                 ON CONFLICT (tenant_id) DO UPDATE SET plan = 'pro'",
                &[&tenant],
            )
            .await
            .unwrap();
    }

    assert_eq!(
        store.get_tenant_plan(&tenant).await.unwrap(),
        Some(TenantPlan::Pro)
    );

    let mut item = sample_item(&format!("pg-item-{tenant}"));
    item.tenant_id = tenant.clone();
    assert!(matches!(
        store.push_item(&item, &tenant).await.unwrap(),
        PushOutcome::Accepted
    ));
    assert!(matches!(
        store.push_item(&item, &tenant).await.unwrap(),
        PushOutcome::Rejected { .. }
    ));

    let items = store
        .pull_items(&tenant, Some("2026-01-01T00:00:00Z"), None, 501)
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, format!("pg-item-{tenant}"));
    assert_eq!(items[0].tenant_id, tenant);

    assert_eq!(store.pending_count(&tenant).await, 1);
    assert!(store.distinct_tenant_count().await >= 1);

    // Seed reference data with boolean columns so the snapshot path —
    // including the Postgres BIGINT(0/1) → bool mapping — is exercised
    // against a live database, not just the empty-set fast path.
    {
        let client = pool.get().await.unwrap();
        let role_id = format!("role-{tenant}");
        client
            .execute(
                "INSERT INTO roles (id, name, permissions) VALUES ($1, $2, '[]')",
                &[&role_id, &role_id],
            )
            .await
            .unwrap();
        client
            .execute(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, tenant_id)
                 VALUES ($1, $2, 'hash', 'Tester', $3, 0, $4)",
                &[
                    &format!("user-{tenant}"),
                    &format!("tester-{tenant}"),
                    &role_id,
                    &tenant,
                ],
            )
            .await
            .unwrap();
        client
            .execute(
                // is_default=0: the idx_tax_rates_single_default partial
                // UNIQUE index is GLOBAL (one default across the whole DB),
                // so seeding is_default=1 here would collide with any
                // concurrent test that also seeds a default. The snapshot
                // mapping (BIGINT 0/1 -> bool) is exercised identically
                // with a non-default row.
                "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive, tenant_id)
                 VALUES ($1, 'Tax', 800, 0, 0, $2)",
                &[&format!("tax-{tenant}"), &tenant],
            )
            .await
            .unwrap();
        client
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, track_serial, is_active, tenant_id)
                 VALUES ($1, $2, 'Widget', 100, 'USD', 1, 1, $3)",
                &[&format!("prod-{tenant}"), &format!("SKU-{tenant}"), &tenant],
            )
            .await
            .unwrap();
    }

    // Products: track_serial=1 → true, is_active=1 → true.
    let (products, tax_rates, users) = store.snapshot_all(&tenant).await.unwrap();
    assert_eq!(products.len(), 1);
    assert_eq!(products[0]["track_serial"], true);
    assert_eq!(products[0]["is_active"], true);

    // Tax rates: is_default=0 → false, is_inclusive=0 → false.
    assert_eq!(tax_rates.len(), 1);
    assert_eq!(tax_rates[0]["is_default"], false);
    assert_eq!(tax_rates[0]["is_inclusive"], false);

    // Users: is_active=0 → false, and pin_hash must not leak (SYNC-06).
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["is_active"], false);
    assert!(users[0].get("pin_hash").is_none());

    // Clean up the rows this test created so a shared dev DB stays tidy.
    {
        let client = pool.get().await.unwrap();
        let role_id = format!("role-{tenant}");
        client
            .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
        client
            .execute("DELETE FROM users WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
        client
            .execute("DELETE FROM roles WHERE id = $1", &[&role_id])
            .await
            .unwrap();
        client
            .execute("DELETE FROM tax_rates WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
        client
            .execute("DELETE FROM products WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
        client
            .execute("DELETE FROM tenant_plans WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
    }
}

/// The critical batch regression test: a duplicate id in the MIDDLE of a
/// batch must NOT poison the transaction on PostgreSQL.
///
/// `push_batch` uses `INSERT … ON CONFLICT (id) DO NOTHING RETURNING id`
/// so a duplicate reports `Rejected` without aborting the transaction —
/// a naive plain `INSERT` would abort the whole batch on the first
/// UNIQUE violation, and every subsequent item would fail with "current
/// transaction is aborted".
#[tokio::test]
async fn pg_integration_push_batch_duplicate_in_middle_survives() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG push-batch integration test skipped: {e}");
            return;
        }
    };
    let tenant = format!("pg-batch-{}", uuid::Uuid::now_v7());
    let store = SyncStore::postgres(pool.clone());

    // Seed an existing row that the batch will duplicate.
    let existing = sample_item(&format!("pg-batch-dup-{}", uuid::Uuid::now_v7()));
    assert!(matches!(
        store.push_item(&existing, &tenant).await.unwrap(),
        PushOutcome::Accepted
    ));

    // Batch: [new-A, duplicate-of-existing, new-B].
    let new_a = sample_item(&format!("pg-batch-a-{}", uuid::Uuid::now_v7()));
    let new_b = sample_item(&format!("pg-batch-b-{}", uuid::Uuid::now_v7()));
    let mut dup = existing.clone();
    dup.id = existing.id.clone(); // same id → UNIQUE conflict
    let batch = vec![new_a.clone(), dup, new_b.clone()];

    let outcomes = store.push_batch(&batch, &tenant).await.unwrap();
    assert_eq!(outcomes.len(), 3);
    assert!(
        matches!(outcomes[0], PushOutcome::Accepted),
        "first (new) item must be Accepted, got: {:?}",
        outcomes[0]
    );
    match &outcomes[1] {
        PushOutcome::Rejected { reason } => {
            assert!(
                reason.contains("duplicate id:"),
                "middle item must be Rejected as duplicate, got: {reason}"
            );
        }
        other => panic!("expected Rejected for duplicate, got: {other:?}"),
    }
    assert!(
        matches!(outcomes[2], PushOutcome::Accepted),
        "third (new) item must STILL be Accepted — the duplicate must not abort the batch, got: {:?}",
        outcomes[2]
    );

    // Cleanup.
    pool.get()
        .await
        .unwrap()
        .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
        .await
        .unwrap();
}

/// A committed batch must be durable and visible to a FRESH connection —
/// a `drop(tx)` (rollback) regression would pass within the batch's own
/// transaction but fail here.
#[tokio::test]
async fn pg_integration_push_batch_commit_visible_to_new_connection() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG push-batch integration test skipped: {e}");
            return;
        }
    };
    let tenant = format!("pg-batch-commit-{}", uuid::Uuid::now_v7());
    let store = SyncStore::postgres(pool.clone());

    let items = vec![
        sample_item(&format!("pg-commit-1-{}", uuid::Uuid::now_v7())),
        sample_item(&format!("pg-commit-2-{}", uuid::Uuid::now_v7())),
        sample_item(&format!("pg-commit-3-{}", uuid::Uuid::now_v7())),
    ];
    let outcomes = store.push_batch(&items, &tenant).await.unwrap();
    assert_eq!(outcomes.len(), 3);
    assert!(outcomes.iter().all(|o| matches!(o, PushOutcome::Accepted)));

    // A brand-new pool connection (fresh checkout) must see all 3 rows —
    // proving the batch COMMIT was durable.
    let client = pool.get().await.unwrap();
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM offline_queue WHERE tenant_id = $1",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        count, 3,
        "committed batch must be visible on a fresh connection"
    );

    // Cleanup.
    client
        .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
        .await
        .unwrap();
}

/// RED (TDD): a per-item DATA error (not a UNIQUE conflict) in the middle
/// of a PG batch must NOT abort the whole transaction — the good items
/// must still land, and the batch must return per-item outcomes.
///
/// PostgreSQL aborts a transaction on ANY statement failure, so the
/// `Err` branch of the current `query_opt` loop leaves every subsequent
/// item failing with "current transaction is aborted" and `commit()`
/// fails — silently losing the valid items. This test installs a trigger
/// that raises on one specific payload to simulate a CHECK/trigger/NOT
/// NULL failure, exactly the class of error `ON CONFLICT DO NOTHING`
/// does NOT suppress.
#[tokio::test]
async fn pg_integration_push_batch_data_error_does_not_abort_batch() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG push-batch data-error integration test skipped: {e}");
            return;
        }
    };
    let tenant = format!("pg-batch-err-{}", uuid::Uuid::now_v7());
    let store = SyncStore::postgres(pool.clone());

    // Install a trigger that rejects inserts whose payload contains
    // "poison" — simulating a CHECK constraint / trigger / future NOT
    // NULL failure that ON CONFLICT DO NOTHING cannot suppress.
    let trigger_fn = format!("reject_poison_{}", uuid::Uuid::now_v7().simple());
    let trigger = format!("{trigger_fn}_trg");
    let client = pool.get().await.unwrap();
    client
        .batch_execute(&format!(
            "CREATE OR REPLACE FUNCTION {trigger_fn}() RETURNS trigger AS $$
             BEGIN
                 IF NEW.payload LIKE '%poison%' THEN
                     RAISE EXCEPTION 'poison payload rejected by test trigger';
                 END IF;
                 RETURN NEW;
             END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER {trigger}
                 BEFORE INSERT ON offline_queue
                 FOR EACH ROW EXECUTE FUNCTION {trigger_fn}();"
        ))
        .await
        .unwrap();
    drop(client);

    let ok_a = sample_item(&format!("pg-err-a-{}", uuid::Uuid::now_v7()));
    let mut poison = sample_item(&format!("pg-err-b-{}", uuid::Uuid::now_v7()));
    poison.payload = r#"{"poison":true}"#.to_owned();
    let ok_c = sample_item(&format!("pg-err-c-{}", uuid::Uuid::now_v7()));
    let batch = vec![ok_a.clone(), poison.clone(), ok_c.clone()];

    let result = store.push_batch(&batch, &tenant).await;
    let outcomes = match result {
        Ok(o) => o,
        Err(e) => panic!(
            "push_batch must return per-item outcomes, not Err: {e}\n\
             (a data error in one item must not abort the whole batch)"
        ),
    };

    assert_eq!(outcomes.len(), 3);
    assert!(
        matches!(outcomes[0], PushOutcome::Accepted),
        "item before the data error must be Accepted, got: {:?}",
        outcomes[0]
    );
    match &outcomes[1] {
        PushOutcome::Rejected { reason } => {
            assert!(
                reason.contains("poison payload rejected"),
                "poison item must be Rejected with its real error, got: {reason}"
            );
        }
        other => panic!("expected Rejected for poison item, got: {other:?}"),
    }
    assert!(
        matches!(outcomes[2], PushOutcome::Accepted),
        "item AFTER the data error must STILL be Accepted — the batch must survive, got: {:?}",
        outcomes[2]
    );

    // The two good items must actually have landed (the transaction
    // committed); the poison item must not.
    let client = pool.get().await.unwrap();
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM offline_queue WHERE tenant_id = $1",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        count, 2,
        "exactly the two good items must be persisted, poison item dropped"
    );

    // Cleanup: drop the trigger and the tenant's rows.
    client
        .batch_execute(&format!(
            "DROP TRIGGER IF EXISTS {trigger} ON offline_queue;
             DROP FUNCTION IF EXISTS {trigger_fn}();"
        ))
        .await
        .unwrap();
    client
        .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
        .await
        .unwrap();
}
