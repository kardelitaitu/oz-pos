//! Real PostgreSQL coverage for `PgTransport`.
//!
//! These tests intentionally use a disposable database rather than the
//! development Compose volume. Run them with:
//!
//! ```text
//! PG_SYNC_TEST_URL=postgresql://ozsync:ozsync@127.0.0.1:15432/ozsync \
//! cargo test -p platform-sync --test pg_integration -- --ignored --nocapture
//! ```

use platform_sync::SyncError;
use platform_sync::pg_transport::PgTransport;
use tokio::sync::{Mutex, OnceCell};
use tokio_postgres::{Client, NoTls};

const PG_URL: &str = "postgresql://ozsync:ozsync@127.0.0.1:15432/ozsync";
static PG_TEST_LOCK: OnceCell<Mutex<()>> = OnceCell::const_new();

async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    PG_TEST_LOCK
        .get_or_init(|| async { Mutex::new(()) })
        .await
        .lock()
        .await
}

fn test_transport() -> PgTransport {
    let url = std::env::var("PG_SYNC_TEST_URL").unwrap_or_else(|_| PG_URL.to_owned());
    let config: tokio_postgres::Config = url.parse().expect("valid PG_SYNC_TEST_URL");
    let host = match config.get_hosts().first() {
        Some(tokio_postgres::config::Host::Tcp(host)) => host.as_str(),
        _ => panic!("PG_SYNC_TEST_URL must use a TCP host"),
    };
    let port = config.get_ports().first().copied().unwrap_or(5432);
    let dbname = config.get_dbname().expect("PG_SYNC_TEST_URL database");
    let user = config.get_user().expect("PG_SYNC_TEST_URL user");
    let password = config
        .get_password()
        .map(|password| std::str::from_utf8(password).expect("UTF-8 PG password"))
        .unwrap_or("");
    PgTransport::new(host, port, dbname, user, password, "test-tenant")
        .expect("create PG transport")
}

async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    let url = std::env::var("PG_SYNC_TEST_URL").unwrap_or_else(|_| PG_URL.to_owned());
    let (client, connection) = tokio_postgres::connect(&url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("postgres test connection failed: {error}");
        }
    });
    Ok(client)
}

async fn reset_schema(client: &Client) -> Result<(), tokio_postgres::Error> {
    client
        .batch_execute(
            "DROP TABLE IF EXISTS offline_queue, products, tax_rates, users CASCADE;
             CREATE TABLE offline_queue (
                 id TEXT PRIMARY KEY,
                 tenant_id TEXT NOT NULL DEFAULT 'default',
                 action TEXT NOT NULL,
                 payload TEXT NOT NULL,
                 status TEXT NOT NULL DEFAULT 'pending',
                 retry_count INTEGER NOT NULL DEFAULT 0,
                 last_error TEXT,
                 created_at TIMESTAMPTZ NOT NULL,
                 synced_at TIMESTAMPTZ
             );
             CREATE TABLE products (
                 id TEXT PRIMARY KEY,
                 tenant_id TEXT NOT NULL DEFAULT 'default',
                 sku TEXT NOT NULL UNIQUE,
                 name TEXT NOT NULL,
                 price_minor BIGINT NOT NULL,
                 currency TEXT NOT NULL,
                 category_id TEXT,
                 barcode TEXT,
                 created_at TIMESTAMPTZ,
                 updated_at TIMESTAMPTZ,
                 price_updated_at TIMESTAMPTZ,
                 track_serial BOOLEAN NOT NULL DEFAULT FALSE,
                 store_id TEXT
             );
             CREATE TABLE tax_rates (
                 id TEXT PRIMARY KEY,
                 tenant_id TEXT NOT NULL DEFAULT 'default',
                 name TEXT NOT NULL,
                 rate_bps BIGINT NOT NULL,
                 is_default BOOLEAN NOT NULL DEFAULT FALSE,
                 is_inclusive BOOLEAN NOT NULL DEFAULT FALSE,
                 created_at TIMESTAMPTZ,
                 updated_at TIMESTAMPTZ
             );
             CREATE TABLE users (
                 id TEXT PRIMARY KEY,
                 tenant_id TEXT NOT NULL DEFAULT 'default',
                 username TEXT NOT NULL UNIQUE,
                 pin_hash TEXT NOT NULL,
                 display_name TEXT NOT NULL,
                 role_id TEXT NOT NULL,
                 is_active BOOLEAN NOT NULL DEFAULT TRUE,
                 created_at TIMESTAMPTZ,
                 updated_at TIMESTAMPTZ
             );",
        )
        .await
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL instance"]
async fn pull_updates_detects_expired_anchor_against_postgres_retention() {
    let _lock = test_lock().await;
    let client = connect().await.expect("connect to PG_SYNC_TEST_URL");
    reset_schema(&client)
        .await
        .expect("create isolated test schema");
    client
        .execute(
            "INSERT INTO offline_queue (id, action, payload, tenant_id, created_at)
             VALUES ('retained-1', 'settings.update', '{}', 'test-tenant', '2026-02-01T00:00:00Z')",
            &[],
        )
        .await
        .expect("seed retained queue row");

    let result = test_transport()
        .pull_updates(Some("2026-01-01T00:00:00Z"), None)
        .await;

    match result {
        Err(SyncError::AnchorExpired { oldest_available }) => {
            assert!(
                oldest_available
                    .as_deref()
                    .is_some_and(|value| value.starts_with("2026-02-01")),
                "expected the real PG MIN(created_at), got {oldest_available:?}"
            );
        }
        other => panic!("expected AnchorExpired from real PostgreSQL, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL instance"]
async fn fetch_snapshot_decodes_postgres_types_without_credentials() {
    let _lock = test_lock().await;
    let client = connect().await.expect("connect to PG_SYNC_TEST_URL");
    reset_schema(&client)
        .await
        .expect("create isolated test schema");
    client
        .batch_execute(
            "INSERT INTO products
                 (id, sku, name, price_minor, currency, track_serial, tenant_id, created_at, updated_at)
             VALUES
                 ('product-1', 'COFFEE', 'Coffee', 350, 'USD', TRUE, 'test-tenant',
                  '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z');
             INSERT INTO tax_rates
                 (id, name, rate_bps, tenant_id, is_default, is_inclusive)
             VALUES ('tax-1', 'VAT', 1100, 'test-tenant', TRUE, FALSE);
             INSERT INTO users
                 (id, username, pin_hash, display_name, role_id, tenant_id, is_active)
             VALUES ('user-1', 'cashier', 'must-not-leak', 'Cashier', 'role-staff', 'test-tenant', TRUE);",
        )
        .await
        .expect("seed snapshot rows");

    let snapshot = test_transport()
        .fetch_snapshot()
        .await
        .expect("fetch PG snapshot");

    assert_eq!(snapshot.products.len(), 1);
    assert_eq!(snapshot.products[0].sku, "COFFEE");
    assert!(snapshot.products[0].track_serial);
    assert_eq!(snapshot.tax_rates[0].rate_bps, 1100);
    assert!(snapshot.tax_rates[0].is_default);
    assert!(!snapshot.tax_rates[0].is_inclusive);
    assert_eq!(snapshot.users[0].username, "cashier");
    assert_eq!(snapshot.users[0].display_name, "Cashier");

    let serialized = serde_json::to_string(&snapshot).expect("serialize typed snapshot");
    assert!(!serialized.contains("must-not-leak"));
    assert!(!serialized.contains("pin_hash"));
}
