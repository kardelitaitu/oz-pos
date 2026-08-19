use super::*;
use rusqlite::params;
use serial_test::serial;

/// The prune DELETE must treat ids as data, never as SQL. The cloud
/// server accepts client-supplied ids verbatim in `push_handler` (no
/// UUID validation), so a hostile id sitting in an old synced row must
/// not execute arbitrary statements when the hourly prune runs — the
/// "IDs are UUIDv7 — safe" comment is an assumption, not an invariant.
#[serial]
#[test]
fn prune_delete_treats_hostile_id_as_data() {
    let conn = oz_core::migrations::fresh_db();
    // An old synced row whose id carries a statement terminator plus a
    // destructive CREATE. If the DELETE interpolates the id, `hacked`
    // appears in the schema.
    let hostile_id = "x'); CREATE TABLE hacked(id TEXT);--";
    conn.execute(
        "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority)
         VALUES (?1, 'act', '{}', 'synced', 0, NULL, '2025-01-01T00:00:00Z', '2025-01-02T00:00:00Z', 't1', 1)",
        params![hostile_id],
    )
    .unwrap();

    let db = Arc::new(Mutex::new(conn));
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_prune_cycle(&db));

    let conn = db.blocking_lock();
    let hacked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'hacked'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        hacked, 0,
        "hostile id must never execute SQL in the prune DELETE"
    );
}
/// P-1 retention must cover API-pushed rows. `push_handler` persists
/// every accepted item with status `pending` and nothing ever
/// transitions it server-side, so the old `status IN ('synced','failed')`
/// filter exempted the entire push path — the cloud queue grew without
/// bound. Retention applies to every status: rows older than the 90-day
/// horizon are pruned (the anchor_expired -> snapshot recovery path is
/// the designed guardrail for stragglers), recent rows survive.
#[serial]
#[test]
fn prune_ages_out_old_pending_rows_like_synced_ones() {
    let conn = oz_core::migrations::fresh_db();
    conn.execute_batch(
        "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority) VALUES
         ('old-pending', 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, 't1', 1),
         ('old-synced', 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', 't1', 1),
         ('recent-pending', 'act', '{}', 'pending', 0, NULL, '2026-08-09T00:00:00Z', NULL, 't1', 1)"
    )
    .unwrap();

    let db = Arc::new(Mutex::new(conn));
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_prune_cycle(&db));

    let conn = db.blocking_lock();
    let remaining: Vec<String> = conn
        .prepare("SELECT id FROM offline_queue ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(
        remaining,
        vec!["recent-pending".to_string()],
        "old pending and old synced rows must be pruned; the recent pending row survives"
    );
}
/// The prune must record every deleted row on the retention counter so
/// operators can observe that old queue rows are actually being aged
/// out (round-121 follow-up: retention observability).
#[serial]
#[test]
fn prune_records_deleted_rows_on_retention_counter() {
    let conn = oz_core::migrations::fresh_db();
    conn.execute_batch(
        "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority) VALUES
         ('old-1', 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, 't1', 1),
         ('old-2', 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', 't1', 1),
         ('fresh', 'act', '{}', 'pending', 0, NULL, '2026-08-09T00:00:00Z', NULL, 't1', 1)"
    )
    .unwrap();

    // Delta around the cycle: other prune tests (serialized via
    // #[serial]) may have incremented the shared counter earlier.
    let before = crate::metrics::PRUNE_QUEUE_DELETED_TOTAL.get();
    let db = Arc::new(Mutex::new(conn));
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_prune_cycle(&db));
    let after = crate::metrics::PRUNE_QUEUE_DELETED_TOTAL.get();

    assert_eq!(
        (after - before) as u64,
        2,
        "the prune must record the two deleted rows on the retention counter"
    );
}

/// Integration test: the Postgres prune cycle applies P-1 offline-queue
/// retention — old rows (any status) are deleted, recent rows survive.
/// Skips when no reachable Postgres is configured, so the suite stays
/// green on machines without one.
#[tokio::test]
async fn pg_integration_prune_ages_out_old_rows() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG prune integration test skipped: {e}");
            return;
        }
    };

    let tenant = format!("pg-prune-test-{}", uuid::Uuid::now_v7());
    let recent = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    {
        let client = pool.get().await.unwrap();
        client
            .execute(
                "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority)
                 VALUES
                 ($1, 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, $4, 1),
                 ($2, 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', $4, 1),
                 ($3, 'act', '{}', 'pending', 0, NULL, $5, NULL, $4, 1)",
                &[
                    &format!("old-pending-{tenant}"),
                    &format!("old-synced-{tenant}"),
                    &format!("recent-{tenant}"),
                    &tenant,
                    &recent,
                ],
            )
            .await
            .unwrap();
    }

    super::run_prune_cycle_pg(&pool).await;

    {
        let client = pool.get().await.unwrap();
        let rows = client
            .query(
                "SELECT id FROM offline_queue WHERE tenant_id = $1 ORDER BY id",
                &[&tenant],
            )
            .await
            .unwrap();
        let ids: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
        assert_eq!(
            ids,
            vec![format!("recent-{tenant}")],
            "old pending and old synced rows must be pruned; the recent row survives"
        );

        client
            .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
    }
}

/// The `sent_reports` dedup table must not grow forever: claims older
/// than the 90-day horizon are aged out by the same prune cycle that
/// handles `offline_queue`. Seed an old claim plus fresh claims for two
/// tenants and assert only the old one is swept (fresh claims survive
/// regardless of tenant — the sweep must not over-delete).
#[tokio::test]
async fn pg_integration_prune_ages_out_old_sent_reports() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG prune sent_reports integration test skipped: {e}");
            return;
        }
    };

    let ns = format!("pg-prune-sr-{}", uuid::Uuid::now_v7());
    let tenant_a = format!("{ns}-a");
    let tenant_b = format!("{ns}-b");
    let old = "2025-01-01T00:00:00Z"; // > 90 days before now
    let recent = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    {
        let client = pool.get().await.unwrap();
        client
            .execute(
                "INSERT INTO sent_reports (tenant_id, period, report_id, sent_at) VALUES
                 ($1, '2025-01-01', 'r-old', $3),
                 ($2, '2026-08-01', 'r-fresh-a', $4),
                 ($2, '2026-08-02', 'r-fresh-b', $4)",
                &[&tenant_a, &tenant_b, &old, &recent],
            )
            .await
            .unwrap();
    }

    super::run_prune_cycle_pg(&pool).await;

    {
        let client = pool.get().await.unwrap();
        let rows = client
            .query(
                "SELECT tenant_id, period FROM sent_reports WHERE tenant_id IN ($1, $2) ORDER BY tenant_id, period",
                &[&tenant_a, &tenant_b],
            )
            .await
            .unwrap();
        let remaining: Vec<(String, String)> = rows.iter().map(|r| (r.get(0), r.get(1))).collect();
        assert_eq!(
            remaining,
            vec![
                (tenant_b.clone(), "2026-08-01".to_string()),
                (tenant_b.clone(), "2026-08-02".to_string()),
            ],
            "only the old claim is swept; fresh claims for both tenants survive"
        );

        client
            .execute(
                "DELETE FROM sent_reports WHERE tenant_id LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
    }
}
