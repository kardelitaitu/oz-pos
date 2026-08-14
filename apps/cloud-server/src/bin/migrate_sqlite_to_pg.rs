//! Phase 1.4 migration tool — copy the cloud server's live data from the
//! SQLite database to Postgres, then verify row counts + checksums.
//!
//! The old single-node cloud server kept its data in a SQLite file
//! (`OZ_DB_PATH`, default `oz-pos.db`). Phase 1 moved the cloud branch onto
//! Postgres; this binary performs the cutover copy for the surface the cloud
//! server actually reads/writes:
//!
//! - sync function: `offline_queue`, `tenant_plans`
//! - REST / snapshots: `products`, `categories`, `tax_rates`, `users`,
//!   `roles`, `assignments`, `sales`, `sale_lines`, `payments`,
//!   `sync_terminals`, `settings`
//! - webhooks: `processed_webhooks`, `stripe_customers`
//!
//! # Usage
//!
//! ```text
//! cargo run -p oz-cloud-server --bin migrate_sqlite_to_pg \
//!     --sqlite oz-pos.db --pg postgres://postgres:postgres@localhost:5432/postgres
//! ```
//!
//! Environment fallbacks: `OZ_DB_PATH` for `--sqlite`, `DATABASE_URL` for
//! `--pg`. The destination schema is applied first (idempotent `PG_INIT`),
//! rows are copied with `ON CONFLICT DO NOTHING` (so re-runs are safe and
//! never clobber rows already synced to Postgres), and every table is
//! verified by row count plus a content checksum computed identically on
//! both sides.
//!
//! # FK-safe copy order
//!
//! The copy order is derived at runtime from Postgres's `pg_constraint`
//! metadata (topological sort of the FK graph), so custom `--tables` lists
//! are ordered correctly too. Tables with no FK edges keep their configured
//! relative order.

use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use bytes::BytesMut;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use rusqlite::Connection;
use tokio_postgres::types::{IsNull, ToSql, Type};

/// Default copy surface — the tables the cloud server reads/writes on the
/// Postgres branch (superset so custom `--tables` is only ever a
/// restriction). Ordered for readability; the real order comes from the FK
/// topological sort.
const DEFAULT_TABLES: &[&str] = &[
    "offline_queue",
    "tenant_plans",
    "products",
    "categories",
    "tax_rates",
    "users",
    "roles",
    "assignments",
    "sales",
    "sale_lines",
    "payments",
    "sync_terminals",
    "processed_webhooks",
    "stripe_customers",
    "settings",
];

/// Normalized value for cross-database checksumming.
#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// One row of normalized cells plus the raw bytes used for the PG insert.
struct Row {
    cells: Vec<Cell>,
}

impl Row {
    /// Checksum fragment: a stable textual encoding of every cell, folded
    /// into the table checksum via FNV-1a (identical on both sides).
    fn checksum_fragment(&self) -> String {
        let mut out = String::new();
        for cell in &self.cells {
            match cell {
                Cell::Null => out.push_str("\\N;"),
                Cell::Int(i) => {
                    out.push_str("i:");
                    out.push_str(&i.to_string());
                    out.push(';');
                }
                Cell::Real(f) => {
                    out.push_str("r:");
                    // Canonical formatting: always emit a decimal point so
                    // the string is identical across drivers.
                    out.push_str(&format!("{f:.6}"));
                    out.push(';');
                }
                Cell::Text(t) => {
                    out.push_str("t:");
                    out.push_str(t);
                    out.push(';');
                }
                Cell::Blob(b) => {
                    out.push_str("b:");
                    for byte in b {
                        out.push_str(&format!("{byte:02x}"));
                    }
                    out.push(';');
                }
            }
        }
        out
    }
}

/// A NULL that binds to any Postgres parameter type.
///
/// `Option<i64>::accepts` is false for a TEXT-typed parameter, so the
/// previous typed-null binding made any row with a NULL in a TEXT column
/// (e.g. a NULL `category_id` on `products`) fail with "error serializing
/// parameter N" before reaching the server.
#[derive(Debug)]
struct WildcardNull;

impl ToSql for WildcardNull {
    fn to_sql(
        &self,
        _ty: &Type,
        _out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        Ok(IsNull::Yes)
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    fn to_sql_checked(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.to_sql(ty, out)
    }
}

/// FNV-1a 64-bit hash over a string.
fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in s.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Column list of a SQLite table (in table order).
fn sqlite_columns(conn: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{table}\")"))
        .map_err(|e| format!("PRAGMA table_info({table}): {e}"))?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| format!("read {table} columns: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("read {table} column: {e}"))?);
    }
    Ok(out)
}

/// Read every row of a SQLite table as normalized cells.
fn read_sqlite_rows(conn: &Connection, table: &str) -> Result<Vec<Row>, String> {
    let sql = format!("SELECT * FROM \"{table}\"");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare {table}: {e}"))?;
    let col_count = stmt.column_count();
    let rows = stmt
        .query_map([], |r| {
            let mut cells = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let v = r
                    .get_ref(i)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
                cells.push(match v {
                    rusqlite::types::ValueRef::Null => Cell::Null,
                    rusqlite::types::ValueRef::Integer(i) => Cell::Int(i),
                    rusqlite::types::ValueRef::Real(f) => Cell::Real(f),
                    rusqlite::types::ValueRef::Text(t) => {
                        Cell::Text(String::from_utf8_lossy(t).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(b) => Cell::Blob(b.to_vec()),
                });
            }
            Ok(Row { cells })
        })
        .map_err(|e| format!("query {table}: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("read {table} row: {e}"))?);
    }
    Ok(out)
}

/// Insert a batch of rows into Postgres with `ON CONFLICT DO NOTHING`.
async fn insert_pg_batch(
    client: &tokio_postgres::Client,
    table: &str,
    columns: &[String],
    rows: &[Row],
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let col_list = columns
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO \"{table}\" ({col_list}) VALUES ({}) ON CONFLICT DO NOTHING",
        (1..=columns.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    for row in rows {
        // Bind every cell: NULLs as a wildcard null (a typed int8 null fails
        // client-side with "error serializing parameter N" when Postgres
        // infers a TEXT parameter, e.g. a NULL `category_id` on products),
        // values with their natural type.
        let params: Vec<Box<dyn ToSql + Sync>> = row
            .cells
            .iter()
            .map(|cell| -> Box<dyn ToSql + Sync> {
                match cell {
                    Cell::Null => Box::new(WildcardNull),
                    Cell::Int(i) => Box::new(*i),
                    Cell::Real(f) => Box::new(*f),
                    Cell::Text(t) => Box::new(t.clone()),
                    Cell::Blob(b) => Box::new(b.clone()),
                }
            })
            .collect();
        let param_refs: Vec<&(dyn ToSql + Sync)> = params.iter().map(|p| p.as_ref()).collect();
        client
            .execute(&sql, &param_refs)
            .await
            .map_err(|e| format!("insert into {table}: {e}"))?;
    }
    Ok(())
}

/// Read every row of a Postgres table (same column order) as normalized
/// cells — mirrors [`read_sqlite_rows`] so checksums compare directly.
async fn read_pg_rows(pool: &Pool, table: &str, columns: &[String]) -> Result<Vec<Row>, String> {
    let col_list = columns
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {col_list} FROM \"{table}\" ORDER BY 1");
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client
        .query(&sql, &[])
        .await
        .map_err(|e| format!("read {table}: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut cells = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            let v = row.try_get::<_, Option<i64>>(i);
            match v {
                Ok(Some(i)) => cells.push(Cell::Int(i)),
                Ok(None) => cells.push(Cell::Null),
                Err(_) => {
                    // Not an integer column — try f64, then text, then blob.
                    let vf = row.try_get::<_, Option<f64>>(i);
                    match vf {
                        Ok(Some(f)) => cells.push(Cell::Real(f)),
                        Ok(None) => cells.push(Cell::Null),
                        Err(_) => {
                            let vt = row.try_get::<_, Option<String>>(i);
                            match vt {
                                Ok(Some(t)) => cells.push(Cell::Text(t)),
                                Ok(None) => cells.push(Cell::Null),
                                Err(_) => {
                                    let vb = row.try_get::<_, Option<Vec<u8>>>(i);
                                    match vb {
                                        Ok(Some(b)) => cells.push(Cell::Blob(b)),
                                        Ok(None) => cells.push(Cell::Null),
                                        Err(e) => {
                                            let col = &columns[i];
                                            return Err(format!("decode {table}.{col}: {e}"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out.push(Row { cells });
    }
    Ok(out)
}

/// Connect to Postgres and apply the full schema (mirrors
/// `cloud_server::db::DbPool::connect_postgres` — the bin target is a
/// separate crate so it cannot reach the library module).
async fn connect_postgres(url: &str) -> Result<Pool, String> {
    let config =
        tokio_postgres::Config::from_str(url).map_err(|e| format!("invalid DATABASE_URL: {e}"))?;
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        roots
            .add(cert)
            .map_err(|e| format!("failed to add root certificate: {e}"))?;
    }
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
    let manager = Manager::from_config(config, tls, mgr_config);
    let pool = Pool::builder(manager)
        .max_size(20)
        .build()
        .map_err(|e| format!("build pool: {e}"))?;
    let client = pool.get().await.map_err(|e| format!("connect: {e}"))?;
    client
        .execute("SELECT 1", &[])
        .await
        .map_err(|e| format!("connect: {e}"))?;
    client
        .batch_execute(oz_core::migrations::PG_INIT)
        .await
        .map_err(|e| format!("apply schema: {e}"))?;
    Ok(pool)
}

/// Build the FK dependency edges from Postgres metadata: (table → tables it
/// references). Used to topologically order the copy.
async fn pg_fk_edges(pool: &Pool) -> Result<Vec<(String, String)>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client
        .query(
            "SELECT tc.table_name AS src, ccu.table_name AS target
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
             JOIN information_schema.constraint_column_usage ccu
               ON ccu.constraint_name = tc.constraint_name
             WHERE tc.constraint_type = 'FOREIGN KEY'",
            &[],
        )
        .await
        .map_err(|e| format!("read FK metadata: {e}"))?;
    let mut edges = Vec::new();
    for row in rows {
        let src: String = row.get(0);
        let target: String = row.get(1);
        if src != target {
            edges.push((src, target));
        }
    }
    Ok(edges)
}

/// Topologically sort `tables` so every table comes after the tables it
/// references (Kahn's algorithm). Tables without edges keep input order.
fn topo_sort(tables: &[String], edges: &[(String, String)]) -> Vec<String> {
    let set: HashSet<&str> = tables.iter().map(|s| s.as_str()).collect();
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut indegree: HashMap<&str, usize> = tables.iter().map(|t| (t.as_str(), 0usize)).collect();
    for (src, target) in edges {
        if set.contains(src.as_str()) && set.contains(target.as_str()) {
            deps.entry(target.as_str()).or_default().push(src.as_str());
            *indegree.entry(src.as_str()).or_default() += 1;
        }
    }
    // Seed with zero-indegree tables in their configured order.
    let mut ready: Vec<&str> = tables
        .iter()
        .map(|t| t.as_str())
        .filter(|t| indegree[t] == 0)
        .collect();
    let mut out: Vec<String> = Vec::with_capacity(tables.len());
    let mut seen: HashSet<&str> = HashSet::new();
    while let Some(t) = ready.pop() {
        if !seen.insert(t) {
            continue;
        }
        out.push(t.to_string());
        if let Some(children) = deps.get(t) {
            for child in children {
                let d = indegree.get_mut(child).unwrap();
                *d -= 1;
                if *d == 0 {
                    ready.push(child);
                }
            }
        }
    }
    if out.len() != tables.len() {
        // Cycle — fall back to the configured order (PG constraints will
        // reject bad orders loudly rather than silently corrupting).
        eprintln!(
            "warning: FK graph cycle detected among {} tables; using configured order",
            tables.len()
        );
        return tables.to_vec();
    }
    out
}

struct Args {
    sqlite: PathBuf,
    pg: String,
    tables: Vec<String>,
    batch: usize,
    dry_run: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut sqlite = env::var("OZ_DB_PATH").unwrap_or_else(|_| "oz-pos.db".into());
    let mut pg: Option<String> = env::var("DATABASE_URL").ok();
    let mut tables: Vec<String> = DEFAULT_TABLES.iter().map(|s| s.to_string()).collect();
    let mut batch = 500usize;
    let mut dry_run = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sqlite" => sqlite = args.next().ok_or("--sqlite needs a path")?,
            "--pg" => pg = Some(args.next().ok_or("--pg needs a URL")?),
            "--tables" => {
                let list = args.next().ok_or("--tables needs a comma list")?;
                tables = list.split(',').map(|s| s.trim().to_string()).collect();
            }
            "--batch" => {
                batch = args
                    .next()
                    .ok_or("--batch needs a number")?
                    .parse()
                    .map_err(|_| "invalid --batch")?;
            }
            "--dry-run" => dry_run = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let pg = pg.ok_or("DATABASE_URL must be set or --pg provided")?;
    Ok(Args {
        sqlite: PathBuf::from(sqlite),
        pg,
        tables,
        batch,
        dry_run,
    })
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("migration failed: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = parse_args()?;

    // 1. Open the SQLite source (the old cloud DB).
    if !args.sqlite.exists() {
        return Err(format!("SQLite DB not found: {}", args.sqlite.display()));
    }
    let conn = Connection::open(&args.sqlite).map_err(|e| format!("open sqlite: {e}"))?;

    // 2. Connect to Postgres; applying PG_INIT is idempotent.
    let pool = connect_postgres(&args.pg).await?;

    // 3. Resolve the copy order (FK-safe) and per-table columns.
    println!("source: {}", args.sqlite.display());
    println!("target: postgres (schema applied via PG_INIT)");
    if args.dry_run {
        println!("dry-run: verifying only (no writes)\n");
    }

    let (total_copied, order_len, failures) =
        copy_and_verify(&pool, &conn, &args.tables, args.batch, args.dry_run).await?;

    println!(
        "\n{} rows copied across {} tables ({} failures)",
        total_copied, order_len, failures
    );
    if failures > 0 {
        return Err(format!("{failures} table(s) failed verification"));
    }
    Ok(())
}

/// Copy the given tables from SQLite to Postgres (FK-topological order) and
/// verify every table by row count + content checksum. Returns
/// `(rows copied, tables processed, failed tables)`.
async fn copy_and_verify(
    pool: &Pool,
    conn: &Connection,
    tables: &[String],
    batch: usize,
    dry_run: bool,
) -> Result<(usize, usize, usize), String> {
    let edges = pg_fk_edges(pool).await?;
    let order = topo_sort(tables, &edges);
    if !dry_run {
        println!("tables: {} (FK-ordered)", order.len());
    }

    let mut total_copied = 0usize;
    let mut failures = 0usize;

    for table in &order {
        // Skip tables absent on either side.
        let sqlite_cols = match sqlite_columns(conn, table) {
            Ok(c) if !c.is_empty() => c,
            Ok(_) => {
                println!("  {table:<24} skipped (empty on source)");
                continue;
            }
            Err(_) => {
                println!("  {table:<24} skipped (missing on source)");
                continue;
            }
        };
        let pg_cols: Vec<String> = {
            let client = pool.get().await.map_err(|e| e.to_string())?;
            let col_list = sqlite_cols
                .iter()
                .map(|c| format!("'{c}'"))
                .collect::<Vec<_>>()
                .join(", ");
            match client
                .query(
                    &format!(
                        "SELECT column_name FROM information_schema.columns \
                         WHERE table_name = $1 AND column_name IN ({col_list}) ORDER BY ordinal_position"
                    ),
                    &[&table.as_str()],
                )
                .await
            {
                Ok(rows) => rows.iter().map(|r| r.get::<_, String>(0)).collect(),
                Err(_) => {
                    println!("  {table:<24} skipped (missing on target)");
                    continue;
                }
            }
        };
        if pg_cols.is_empty() {
            println!("  {table:<24} skipped (no shared columns)");
            continue;
        }

        // 4. Read source rows.
        let src_rows = read_sqlite_rows(conn, table)?;

        // 5. Write (unless dry-run), in batches.
        if !dry_run {
            let client = pool.get().await.map_err(|e| e.to_string())?;
            for chunk in src_rows.chunks(batch) {
                insert_pg_batch(&client, table, &pg_cols, chunk).await?;
            }
        }
        total_copied += src_rows.len();

        // 6. Verify: row count + checksum on both sides.
        let pg_rows = read_pg_rows(pool, table, &pg_cols).await?;
        let src_checksum: u64 = src_rows
            .iter()
            .map(|r| fnv1a(&r.checksum_fragment()))
            .fold(0u64, |acc, h| acc ^ h);
        let pg_checksum: u64 = pg_rows
            .iter()
            .map(|r| fnv1a(&r.checksum_fragment()))
            .fold(0u64, |acc, h| acc ^ h);

        let count_ok = pg_rows.len() >= src_rows.len();
        let checksum_ok = src_checksum == pg_checksum;
        let status = if count_ok && checksum_ok {
            "OK"
        } else if count_ok {
            "CHECKSUM-DIFF"
        } else {
            "COUNT-DIFF"
        };
        if status != "OK" {
            failures += 1;
        }
        println!(
            "  {table:<24} src={:<6} pg={:<6} checksum={:<20} {status}",
            src_rows.len(),
            pg_rows.len(),
            format!("{src_checksum:016x}"),
        );
    }

    Ok((total_copied, order.len(), failures))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sqlite_with_data() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oz-pos.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE tenant_plans (
                 tenant_id TEXT PRIMARY KEY,
                 plan      TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE settings (
                 key        TEXT PRIMARY KEY,
                 value      TEXT NOT NULL DEFAULT '',
                 updated_at TEXT NOT NULL
             );
             INSERT INTO tenant_plans VALUES ('tenant-a', 'pro', '2026-01-01T00:00:00Z');
             INSERT INTO tenant_plans VALUES ('tenant-b', 'free', '2026-01-02T00:00:00Z');
             INSERT INTO settings VALUES ('store.name', 'Migrate Store', '2026-01-03T00:00:00Z');",
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn fnv1a_is_stable() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
        // XOR of two fragment hashes is order-independent.
        let a = fnv1a("x") ^ fnv1a("y");
        let b = fnv1a("y") ^ fnv1a("x");
        assert_eq!(a, b);
    }

    #[test]
    fn sqlite_rows_checksum_matches_readback() {
        let (_dir, path) = sqlite_with_data();
        let conn = Connection::open(&path).unwrap();
        let rows = read_sqlite_rows(&conn, "tenant_plans").unwrap();
        assert_eq!(rows.len(), 2);
        let checksum: u64 = rows
            .iter()
            .map(|r| fnv1a(&r.checksum_fragment()))
            .fold(0u64, |acc, h| acc ^ h);
        assert_ne!(checksum, 0);
        // The checksum is deterministic across reads.
        let rows2 = read_sqlite_rows(&conn, "tenant_plans").unwrap();
        let checksum2: u64 = rows2
            .iter()
            .map(|r| fnv1a(&r.checksum_fragment()))
            .fold(0u64, |acc, h| acc ^ h);
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn topo_sort_orders_fk_children_after_parents() {
        let tables = vec![
            "sale_lines".to_string(),
            "sales".to_string(),
            "products".to_string(),
            "users".to_string(),
        ];
        let edges = vec![
            ("sale_lines".to_string(), "sales".to_string()),
            ("sale_lines".to_string(), "products".to_string()),
            ("sales".to_string(), "users".to_string()),
        ];
        let order = topo_sort(&tables, &edges);
        let pos = |t: &str| order.iter().position(|x| x == t).unwrap();
        assert!(
            pos("sales") < pos("sale_lines"),
            "sale_lines after sales: {order:?}"
        );
        assert!(
            pos("products") < pos("sale_lines"),
            "sale_lines after products: {order:?}"
        );
        assert!(pos("users") < pos("sales"), "sales after users: {order:?}");
        // Cycle fallback preserves the configured order.
        let cyclic = vec!["a".to_string(), "b".to_string()];
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "a".to_string()),
        ];
        assert_eq!(topo_sort(&cyclic, &edges), cyclic);
    }

    /// Integration test: migrate a SQLite DB into a live Postgres and verify
    /// row counts + checksums. Skips when Postgres is unreachable.
    #[tokio::test]
    async fn pg_integration_migrate_and_verify() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let pool = match connect_postgres(&url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("PG migration integration test skipped: {e}");
                return;
            }
        };

        let ns = format!("pg-migrate-test-{}", uuid::Uuid::now_v7());
        let tenant_a = format!("{ns}-a");
        let tenant_b = format!("{ns}-b");
        let store_key = format!("{ns}-store");

        // Seed a second SQLite DB whose tenant/settings rows are namespaced.
        let dir2 = tempfile::tempdir().unwrap();
        let path2 = dir2.path().join("oz-pos.db");
        {
            let conn = Connection::open(&path2).unwrap();
            conn.execute_batch(&format!(
                "CREATE TABLE tenant_plans (
                     tenant_id TEXT PRIMARY KEY,
                     plan      TEXT NOT NULL,
                     updated_at TEXT NOT NULL
                 );
                 CREATE TABLE settings (
                     key        TEXT PRIMARY KEY,
                     value      TEXT NOT NULL DEFAULT '',
                     updated_at TEXT NOT NULL
                 );
                 INSERT INTO tenant_plans VALUES ('{tenant_a}', 'pro', '2026-02-01T00:00:00Z');
                 INSERT INTO tenant_plans VALUES ('{tenant_b}', 'free', '2026-02-02T00:00:00Z');
                 INSERT INTO settings VALUES ('{store_key}', 'Migrated Store', '2026-02-03T00:00:00Z');",
            ))
            .unwrap();
        }

        // Clean any rows left by previous (possibly crashed) runs of this
        // test, then run the migration.
        {
            let client = pool.get().await.unwrap();
            client
                .execute(
                    "DELETE FROM tenant_plans WHERE tenant_id LIKE 'pg-migrate-test-%'",
                    &[],
                )
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM settings WHERE key LIKE 'pg-migrate-test-%'",
                    &[],
                )
                .await
                .unwrap();
        }

        // Direct call: connect + copy (bypasses env parsing). The built-in
        // whole-table verification is not asserted — parallel test binaries
        // share this DB — so the definitive check below is namespaced.
        let conn = Connection::open(&path2).unwrap();
        let (_copied, _tables, _failures) = copy_and_verify(
            &pool,
            &conn,
            &["tenant_plans".to_string(), "settings".to_string()],
            500,
            false,
        )
        .await
        .unwrap();

        // Namespaced verification: every seeded row made it with an
        // identical checksum (tenant_id / key is the first column).
        let ns_prefix = format!("{ns}-");
        for table in ["tenant_plans", "settings"] {
            let columns = sqlite_columns(&conn, table).unwrap();
            let src = read_sqlite_rows(&conn, table).unwrap();
            let mut pg = read_pg_rows(&pool, table, &columns).await.unwrap();
            pg.retain(|r| matches!(&r.cells[0], Cell::Text(t) if t.starts_with(&ns_prefix)));
            assert_eq!(pg.len(), src.len(), "{table} row count");
            let src_sum: u64 = src
                .iter()
                .map(|r| fnv1a(&r.checksum_fragment()))
                .fold(0u64, |acc, h| acc ^ h);
            let pg_sum: u64 = pg
                .iter()
                .map(|r| fnv1a(&r.checksum_fragment()))
                .fold(0u64, |acc, h| acc ^ h);
            assert_eq!(pg_sum, src_sum, "{table} checksum");
        }

        // Verify the migrated values round-trip.
        let client = pool.get().await.unwrap();
        let plan: String = client
            .query_one(
                "SELECT plan FROM tenant_plans WHERE tenant_id = $1",
                &[&tenant_a],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(plan, "pro");
        let store: String = client
            .query_one("SELECT value FROM settings WHERE key = $1", &[&store_key])
            .await
            .unwrap()
            .get(0);
        assert_eq!(store, "Migrated Store");

        // Cleanup.
        client
            .execute(
                "DELETE FROM tenant_plans WHERE tenant_id LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM settings WHERE key LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
    }

    /// Volume test: migrate a SQLite DB seeded with the **real** schema
    /// (`oz_core::migrations::run`) and 10k+ rows across several tables, then
    /// assert every row made it with an identical checksum. Exercises the
    /// copy batching and the checksum path at the volume the cutover will
    /// actually see. Skips when Postgres is unreachable.
    #[tokio::test]
    async fn pg_integration_migrate_large_db() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let pool = match connect_postgres(&url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("PG migration volume test skipped: {e}");
                return;
            }
        };

        let ns = format!("pg-migrate-vol-{}", uuid::Uuid::now_v7());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oz-pos.db");

        // Clean any rows left by previous runs of this test (a crashed run
        // keeps its rows, and `copy_and_verify` compares whole tables), then
        // seed the source DB.
        {
            let client = pool.get().await.unwrap();
            client
                .execute(
                    "DELETE FROM offline_queue WHERE tenant_id LIKE 'pg-migrate-vol-%'",
                    &[],
                )
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM products WHERE tenant_id LIKE 'pg-migrate-vol-%'",
                    &[],
                )
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM settings WHERE key LIKE 'pg-migrate-vol-%'",
                    &[],
                )
                .await
                .unwrap();
        }

        {
            let mut conn = Connection::open(&path).unwrap();
            oz_core::migrations::run(&mut conn).unwrap();

            let now = "2026-03-01T00:00:00Z";
            let tenant = format!("{ns}-default");
            let tx = conn.transaction().unwrap();
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO offline_queue \
                         (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority) \
                         VALUES (?1, 'complete_sale', ?2, 'pending', 0, NULL, ?3, NULL, ?4, 1)",
                    )
                    .unwrap();
                for i in 0..10_000 {
                    stmt.execute(rusqlite::params![
                        format!("{ns}-q{i:05}"),
                        format!(r#"{{"total":{i}}}"#),
                        now,
                        tenant,
                    ])
                    .unwrap();
                }
            }
            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO products \
                         (id, sku, name, price_minor, currency, created_at, updated_at, price_updated_at, \
                          track_serial, product_type, version, cost_minor, store_id, tenant_id, \
                          brand, rack_location, notes, unit, is_active, default_supplier_id, popularity_score) \
                         VALUES (?1, ?2, ?3, ?4, 'USD', ?5, ?5, '', 0, 'retail', 1, 0, NULL, ?6, \
                                 NULL, NULL, NULL, NULL, 1, NULL, 0.0)",
                    )
                    .unwrap();
                for i in 0..300 {
                    stmt.execute(rusqlite::params![
                        format!("{ns}-p{i:04}"),
                        format!("{ns}-SKU-{i:04}"),
                        format!("Volume Product {i}"),
                        100 + i as i64,
                        now,
                        tenant,
                    ])
                    .unwrap();
                }
            }
            {
                let mut stmt = tx
                    .prepare("INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)")
                    .unwrap();
                for i in 0..100 {
                    stmt.execute(rusqlite::params![
                        format!("{ns}-key-{i}"),
                        format!("value-{i}"),
                        now
                    ])
                    .unwrap();
                }
            }
            tx.commit().unwrap();
        }

        // Migrate the three tables at volume. `copy_and_verify`'s built-in
        // whole-table verification is not asserted here: the shared dev DB
        // is written by parallel test binaries (webhooks, email), so a
        // concurrent row can legitimately appear mid-verify. The definitive
        // check below compares only this run's namespaced rows.
        let conn = Connection::open(&path).unwrap();
        let (copied, tables, _failures) = copy_and_verify(
            &pool,
            &conn,
            &[
                "offline_queue".to_string(),
                "products".to_string(),
                "settings".to_string(),
            ],
            500,
            false,
        )
        .await
        .unwrap();
        assert_eq!(tables, 3);
        assert_eq!(copied, 10_400, "10k queue + 300 products + 100 settings");

        // Namespaced verification: every row this test seeded made it to
        // Postgres with an identical checksum (ids/keys all carry the ns
        // prefix as their first column).
        let ns_prefix = format!("{ns}-");
        for table in ["offline_queue", "products", "settings"] {
            let columns = sqlite_columns(&conn, table).unwrap();
            let src = read_sqlite_rows(&conn, table).unwrap();
            let mut pg = read_pg_rows(&pool, table, &columns).await.unwrap();
            pg.retain(|r| matches!(&r.cells[0], Cell::Text(t) if t.starts_with(&ns_prefix)));
            assert_eq!(pg.len(), src.len(), "{table} row count");
            let src_sum: u64 = src
                .iter()
                .map(|r| fnv1a(&r.checksum_fragment()))
                .fold(0u64, |acc, h| acc ^ h);
            let pg_sum: u64 = pg
                .iter()
                .map(|r| fnv1a(&r.checksum_fragment()))
                .fold(0u64, |acc, h| acc ^ h);
            assert_eq!(pg_sum, src_sum, "{table} checksum");
        }

        // Exact row counts on the destination, then spot-check values.
        let client = pool.get().await.unwrap();
        let queue_rows: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM offline_queue WHERE tenant_id = $1",
                &[&format!("{ns}-default")],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(queue_rows, 10_000);
        let product_rows: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM products WHERE tenant_id = $1",
                &[&format!("{ns}-default")],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(product_rows, 300);
        let payload: String = client
            .query_one(
                "SELECT payload FROM offline_queue WHERE id = $1",
                &[&format!("{ns}-q09999")],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(payload, r#"{"total":9999}"#);
        let price: i64 = client
            .query_one(
                "SELECT price_minor FROM products WHERE sku = $1",
                &[&format!("{ns}-SKU-0299")],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(price, 100 + 299);

        // Cleanup.
        client
            .execute(
                "DELETE FROM offline_queue WHERE tenant_id LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM products WHERE tenant_id LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
        client
            .execute(
                "DELETE FROM settings WHERE key LIKE $1",
                &[&format!("{ns}-%")],
            )
            .await
            .unwrap();
    }
}
