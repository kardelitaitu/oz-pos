/*
last audited 25-07-26 by RSA-Agent (oz-cli slice A: commands deep read; CLI-1 + CLI-2 FIXED 25-07-26)
crate: oz-cli | status: SAFE | lint: CLEAN
findings: CLI-1 FIXED — run_import_ozpkg sale imports now use the new tx-aware Store::create_sale_in_tx (the previous store.create_sale opened a nested transaction inside the import transaction and failed with cannot-start-a-transaction-within-a-transaction, rolling back sale imports). CLI-2 FIXED — init-db seeds the admin user with a real argon2 hash of the documented default PIN 1234 (never-verifying hashed_pin_placeholder locked the first-run admin out) and prints a change-it-now warning. CLI-3 FIXED — run_user_create validates the --pin-hash argument as an argon2 PHC string (argon2 PHC parse + algorithm check, new argon2 workspace dep; placeholder/garbage/foreign-algorithm values are rejected up front); CLI-4 INFO unchanged (restore torn-restore risk); CLI-5 INFO unchanged (file length). Otherwise clean: parameterized SQL, single-tx import for other types, recoverable currency UTF-8 handling per RUST-07, Argon2id + AES-256-GCM export path, dry-run support
next: CLI-3/4/5 (LOW/INFO) | perf: N/A
*/
//! Command implementations for the `oz` CLI.

#![allow(clippy::items_after_test_module)]

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use rusqlite::Connection;

use oz_core::db::Store;
use oz_core::{CoreError, Currency, FeatureRegistry, Money, SaleStatus, Settings, format_minor};

use crate::cli::*;

use crate::seed_demo::run_seed_demo;

// ── DB helpers ────────────────────────────────────────────────────────

pub(crate) fn open_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path).with_context(|| format!("opening database at {path}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enabling foreign_keys")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enabling WAL")?;
    Ok(conn)
}

// ── Entry point ───────────────────────────────────────────────────────

/// Parse CLI arguments and dispatch to the matching subcommand.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let conn = open_db(&cli.db)?;

    match cli.command {
        Some(Command::Migrate) => run_migrate(conn),
        Some(Command::InitDb(args)) => run_init_db(&conn, &args),
        Some(Command::Product(args)) => run_product(&conn, args),
        Some(Command::Backup { output }) => run_backup(&conn, &output),
        Some(Command::Export { kind }) => run_export(&conn, &kind),
        Some(Command::Category(args)) => run_category(&conn, args),
        Some(Command::Inventory(args)) => run_inventory(&conn, args),
        Some(Command::Sale(args)) => run_sale(&conn, args),
        Some(Command::Customer(args)) => run_customer(&conn, args),
        Some(Command::User(args)) => run_user(&conn, args),
        Some(Command::Restore { input }) => run_restore(conn, &input),
        Some(Command::ExportOzpkg {
            output,
            types,
            password,
        }) => run_export_ozpkg(&conn, &output, &types, &password),
        Some(Command::ImportOzpkg {
            input,
            password,
            dry_run,
        }) => run_import_ozpkg(&conn, &input, &password, dry_run),
        Some(Command::SeedDemo(args)) => run_seed_demo(&conn, &args),
        None => {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            Ok(())
        }
    }
}

// ── Migrate ──────────────────────────────────────────────────────────

pub(crate) fn run_migrate(conn: Connection) -> Result<()> {
    eprintln!("applying migrations...");
    let mut conn = conn;
    oz_core::migrations::run(&mut conn).context("applying migrations")?;
    eprintln!("migrations up to date");
    Ok(())
}

// ── Init-DB ──────────────────────────────────────────────────────────

/// Seed the database with default settings and a feature preset.
pub(crate) fn run_init_db(conn: &Connection, args: &InitDbArgs) -> Result<()> {
    eprintln!("seeding database with preset: {}", args.preset);

    let store = Store::new(conn);

    // --- Default settings ---
    Settings::set_store_name(conn, "My Store").context("setting store name")?;
    Settings::set_default_currency(conn, "USD").context("setting default currency")?;
    Settings::set(conn, oz_core::settings::keys::SETUP_COMPLETE, "true")
        .context("marking setup complete")?;

    // --- Feature preset ---
    let registry = match args.preset.as_str() {
        "simple-retail" => FeatureRegistry::simple_retail(),
        "restaurant" => FeatureRegistry::restaurant(),
        "full-store" => FeatureRegistry::full_store(),
        "custom" => FeatureRegistry::custom(),
        other => {
            eprintln!("unknown preset '{other}'; using custom (no features enabled)");
            FeatureRegistry::custom()
        }
    };

    let feature_count = registry.count();
    store
        .save_features(&registry)
        .context("saving feature flags")?;
    eprintln!("  enabled {feature_count} feature(s)");

    // --- ISO-4217 Currencies ---
    eprintln!("  seeding currencies...");
    conn.execute_batch(
        "INSERT OR IGNORE INTO currencies (code, numeric_code, name, minor_exponent, symbol) VALUES
            ('USD', '840', 'US Dollar',               2, '$'),
            ('EUR', '978', 'Euro',                    2, '\u{20ac}'),
            ('GBP', '826', 'British Pound',           2, '\u{a3}'),
            ('JPY', '392', 'Japanese Yen',            0, '\u{a5}'),
            ('CAD', '124', 'Canadian Dollar',         2, 'CA$'),
            ('AUD', '036', 'Australian Dollar',       2, 'A$'),
            ('CHF', '756', 'Swiss Franc',             2, 'Fr'),
            ('CNY', '156', 'Chinese Yuan',            2, '\u{5143}'),
            ('INR', '356', 'Indian Rupee',            2, '\u{20b9}'),
            ('BRL', '986', 'Brazilian Real',          2, 'R$'),
            ('MXN', '484', 'Mexican Peso',            2, 'Mex$'),
            ('KRW', '410', 'South Korean Won',        0, '\u{20a9}'),
            ('SEK', '752', 'Swedish Krona',           2, 'kr'),
            ('NOK', '578', 'Norwegian Krone',         2, 'kr'),
            ('DKK', '208', 'Danish Krone',            2, 'kr'),
            ('NZD', '554', 'New Zealand Dollar',      2, 'NZ$'),
            ('SGD', '702', 'Singapore Dollar',        2, 'S$'),
            ('HKD', '344', 'Hong Kong Dollar',        2, 'HK$'),
            ('MYR', '458', 'Malaysian Ringgit',       2, 'RM'),
            ('THB', '764', 'Thai Baht',               2, '\u{e3f}'),
            ('PHP', '608', 'Philippine Peso',         2, '\u{20b1}'),
            ('IDR', '360', 'Indonesian Rupiah',       0, 'Rp'),
            ('VND', '704', 'Vietnamese Dong',         0, '\u{20ab}'),
            ('ZAR', '710', 'South African Rand',      2, 'R'),
            ('RUB', '643', 'Russian Ruble',           2, '\u{20bd}'),
            ('TRY', '949', 'Turkish Lira',            2, '\u{20ba}'),
            ('SAR', '682', 'Saudi Riyal',             2, '\u{fdfc}'),
            ('AED', '784', 'UAE Dirham',              2, '\u{62f}.\u{625}'),
            ('ILS', '376', 'Israeli Shekel',          2, '\u{20aa}'),
            ('PLN', '985', 'Polish Zloty',            2, 'z\u{142}'),
            ('CZK', '203', 'Czech Koruna',            2, 'K\u{10d}'),
            ('HUF', '348', 'Hungarian Forint',        0, 'Ft'),
            ('CLP', '152', 'Chilean Peso',            0, 'CLP$'),
            ('COP', '170', 'Colombian Peso',          2, 'COL$'),
            ('PEN', '604', 'Peruvian Sol',            2, 'S/'),
            ('ARS', '032', 'Argentine Peso',          2, 'AR$'),
            ('NGN', '566', 'Nigerian Naira',          2, '\u{20a6}'),
            ('KES', '404', 'Kenyan Shilling',         2, 'KSh'),
            ('EGP', '818', 'Egyptian Pound',          2, '\u{a3}');",
    )
    .context("seeding currencies")?;

    // --- Default Roles ---
    eprintln!("  seeding roles...");
    conn.execute_batch(
        "INSERT OR IGNORE INTO roles (id, name, description, permissions) VALUES
            ('role-owner',   'owner',   'Full access to all features and settings',
             '[\"*\"]'),
            ('role-manager', 'manager', 'Can manage products, categories, and view reports',
             '[\"products:crud\",\"categories:manage\",\"sales:void\",\"reports:view\"]'),
            ('role-staff', 'staff', 'Operational role with Manager-level access minus settings',
             '[\"sales:process\",\"sales:view\",\"customers:view\"]');",
    )
    .context("seeding roles")?;

    // --- Admin User ---
    eprintln!("  seeding admin user...");
    // CLI-2 fix: seed a REAL argon2 hash of a documented default PIN
    // ("1234") instead of the never-verifying "hashed_pin_placeholder"
    // string that locked the first-run admin out of PIN-gated flows. The
    // operator is told to change the PIN immediately (hashing takes ~100ms
    // — acceptable one-time init cost).
    let default_admin_pin = "1234";
    let admin_pin_hash = oz_core::auth::hash_pin(default_admin_pin)
        .map_err(|e| anyhow::anyhow!("hashing default admin PIN: {e}"))?;
    conn.execute(
        "INSERT OR IGNORE INTO users (id, username, pin_hash, display_name, role_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["user-admin", "admin", admin_pin_hash, "Admin", "role-owner"],
    )
    .context("seeding admin user")?;
    eprintln!("  NOTE: default admin PIN is '1234' — change it immediately after first login.");

    eprintln!("database initialised successfully");
    Ok(())
}

// ── Backup / Export ────────────────────────────────────────────────────

/// Create an online SQLite snapshot of the database.
pub(crate) fn run_backup(conn: &Connection, output: &str) -> Result<()> {
    let store = Store::new(conn);
    eprintln!("creating backup -> {output}...");
    store
        .backup(output)
        .with_context(|| format!("backup to {output}"))?;
    eprintln!("backup complete");
    Ok(())
}

/// Write a CSV report to stdout for the given kind.
pub(crate) fn run_export(conn: &Connection, kind: &str) -> Result<()> {
    let store = Store::new(conn);

    match kind {
        "daily-summary" => {
            let rows = store.export_daily_summary()?;
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            for r in &rows {
                wtr.serialize(r)?;
            }
            wtr.flush()?;
        }
        "sales-by-hour" => {
            let rows = store.export_sales_by_hour()?;
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            for r in &rows {
                wtr.serialize(r)?;
            }
            wtr.flush()?;
        }
        other => {
            eprintln!("unknown export kind '{other}'");
            eprintln!("available kinds: daily-summary, sales-by-hour");
            return Err(anyhow::anyhow!("unknown export kind '{other}'"));
        }
    }

    Ok(())
}

// ── Category commands ────────────────────────────────────────────────────

pub(crate) fn run_category(conn: &Connection, args: CategoryArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        CategoryAction::List => run_category_list(&store),
        CategoryAction::Get { id } => run_category_get(&store, &id),
        CategoryAction::Create { id, name, colour } => {
            run_category_create(&store, &id, &name, &colour)
        }
        CategoryAction::Delete { id } => run_category_delete(&store, &id),
    }
}

pub(crate) fn run_category_list(store: &Store<'_>) -> Result<()> {
    let categories = store.list_categories().context("listing categories")?;

    if categories.is_empty() {
        println!("No categories found.");
        return Ok(());
    }

    println!("{:<24} {:<24}  Colour", "ID", "Name");
    println!("{:-<24} {:-<24}  {:-}", "", "", "");

    for c in &categories {
        println!("{:<24} {:<24}  {}", c.id, c.name, c.colour);
    }

    Ok(())
}

pub(crate) fn run_category_get(store: &Store<'_>, id: &str) -> Result<()> {
    match store.get_category(id).context("looking up category")? {
        Some(c) => {
            println!("ID:     {}", c.id);
            println!("Name:   {}", c.name);
            println!("Colour: {}", c.colour);
        }
        None => {
            println!("Category not found: {id}");
        }
    }
    Ok(())
}

pub(crate) fn run_category_create(
    store: &Store<'_>,
    id: &str,
    name: &str,
    colour: &str,
) -> Result<()> {
    let cat = store
        .create_category(id, name, colour, "")
        .map_err(|e| match &e {
            CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
            CoreError::Conflict { entity, field } => {
                anyhow::anyhow!("Conflict: {entity} already exists ({field})")
            }
            _ => anyhow::anyhow!("Error: {e}"),
        })?;

    println!("Created category: {} ({})", cat.name, cat.id);
    Ok(())
}

pub(crate) fn run_category_delete(store: &Store<'_>, id: &str) -> Result<()> {
    store.delete_category(id).map_err(|e| match &e {
        CoreError::NotFound { .. } => anyhow::anyhow!("Category not found: {id}"),
        _ => anyhow::anyhow!("Error: {e}"),
    })?;

    println!("Deleted category: {id}");
    Ok(())
}

// ── Inventory commands ──────────────────────────────────────────────────

pub(crate) fn run_inventory(conn: &Connection, args: InventoryArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        InventoryAction::Get { sku } => run_inventory_get(&store, &sku),
        InventoryAction::Adjust { sku, delta } => run_inventory_adjust(&store, &sku, delta),
    }
}

pub(crate) fn run_inventory_get(store: &Store<'_>, sku: &str) -> Result<()> {
    let product = store.get_product(sku).context("looking up product")?;

    match product {
        Some(p) => {
            let qty = p.stock_qty.unwrap_or(0);
            println!("SKU:    {}", p.product.sku.as_str());
            println!("Name:   {}", p.product.name);
            println!("Stock:  {qty}");
        }
        None => {
            println!("Product not found: {sku}");
        }
    }

    Ok(())
}

#[allow(deprecated)]
pub(crate) fn run_inventory_adjust(store: &Store<'_>, sku: &str, delta: i64) -> Result<()> {
    let new_qty = store.adjust_stock(sku, delta).map_err(|e| match &e {
        CoreError::NotFound { .. } => anyhow::anyhow!("Product not found: {sku}"),
        CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
        _ => anyhow::anyhow!("Error: {e}"),
    })?;

    let verb = if delta >= 0 { "restocked" } else { "sold" };
    println!("Stock {verb} for {sku} (delta: {delta:+}) — new qty: {new_qty}");
    Ok(())
}

// ── Sale commands ──────────────────────────────────────────────────────

pub(crate) fn run_sale(conn: &Connection, args: SaleArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        SaleAction::List => run_sale_list(&store),
        SaleAction::Get { id, format } => run_sale_get(&store, &id, &format),
        SaleAction::UpdateStatus { id, status } => run_sale_update_status(&store, &id, &status),
    }
}

pub(crate) fn run_sale_list(store: &Store<'_>) -> Result<()> {
    let sales = store.list_sales().context("listing sales")?;

    if sales.is_empty() {
        println!("No sales found.");
        return Ok(());
    }

    println!(
        "{:<40} {:>10} {:>6}  {:>10}  Date",
        "ID", "Total", "Items", "Status"
    );
    println!(
        "{:-<40} {:->10} {:->6}  {:->10}  {:-<4}",
        "", "", "", "", ""
    );

    for s in &sales {
        let total_str = format_minor(s.total.minor_units, s.total.currency);
        let status_str = match s.status {
            SaleStatus::Pending => "pending",
            SaleStatus::Active => "active",
            SaleStatus::Completed => "done",
            SaleStatus::Voided => "voided",
        };
        let date_str = s.created_at.as_str();
        let date_str = if date_str.len() > 10 {
            &date_str[..10]
        } else {
            date_str
        };
        println!(
            "{:<40} {:>10} {:>6}  {:>10}  {}",
            s.id, total_str, s.line_count, status_str, date_str
        );
    }

    Ok(())
}

pub(crate) fn run_sale_get(store: &Store<'_>, id: &str, format: &str) -> Result<()> {
    match store.get_sale(id).context("looking up sale")? {
        Some(sale) => {
            if format == "json" {
                let json =
                    serde_json::to_string_pretty(&sale).context("serializing sale to JSON")?;
                println!("{json}");
            } else {
                let total_str = format!(
                    "{} {}",
                    format_minor(sale.total.minor_units, sale.currency),
                    std::str::from_utf8(&sale.currency.0).unwrap_or("???"),
                );
                println!("ID:           {}", sale.id);
                println!("Status:       {:?}", sale.status);
                println!("Total:        {}", total_str);
                println!("Line count:   {}", sale.line_count);
                println!(
                    "Currency:     {}",
                    std::str::from_utf8(&sale.currency.0).unwrap_or("???")
                );
                println!("Created:      {}", sale.created_at);
                println!("Updated:      {}", sale.updated_at);

                if !sale.lines.is_empty() {
                    println!();
                    println!("{:<4} {:<24} {:>6} {:>10}", "#", "SKU", "Qty", "Unit");
                    println!("{:-<4} {:-<24} {:->6} {:->10}", "", "", "", "");
                    for line in &sale.lines {
                        let unit_str = format_minor(line.unit_price.minor_units, sale.currency);
                        println!(
                            "{:<4} {:<24} {:>6} {:>10}",
                            line.line_position, line.sku, line.qty, unit_str
                        );
                    }
                }
            }
        }
        None => {
            if format == "json" {
                println!("null");
            } else {
                println!("Sale not found: {id}");
            }
        }
    }

    Ok(())
}

pub(crate) fn run_sale_update_status(store: &Store<'_>, id: &str, status_str: &str) -> Result<()> {
    let to = SaleStatus::from_stored_str(status_str).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid status '{status_str}'; expected one of: pending, active, completed, voided"
        )
    })?;

    let sale = store.update_sale_status(id, to).map_err(|e| match &e {
        CoreError::NotFound { .. } => anyhow::anyhow!("Sale not found: {id}"),
        CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
        _ => anyhow::anyhow!("Error: {e}"),
    })?;

    println!("Sale {id} status updated to {:?}", sale.status);
    Ok(())
}

// ── Customer commands ──────────────────────────────────────────────────

pub(crate) fn run_customer(conn: &Connection, args: CustomerArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        CustomerAction::List => run_customer_list(&store),
        CustomerAction::Get { id } => run_customer_get(&store, &id),
        CustomerAction::Create {
            name,
            email,
            phone,
            notes,
        } => run_customer_create(
            &store,
            &name,
            email.as_deref(),
            phone.as_deref(),
            notes.as_deref(),
        ),
    }
}

pub(crate) fn run_customer_list(store: &Store<'_>) -> Result<()> {
    let customers = store.list_customers().context("listing customers")?;

    if customers.is_empty() {
        println!("No customers found.");
        return Ok(());
    }

    println!(
        "{:<40} {:<24} {:<30} {:<16}",
        "ID", "Name", "Email", "Phone"
    );
    println!("{:-<40} {:-<24} {:-<30} {:-<16}", "", "", "", "");

    for c in &customers {
        let email = c.email.as_ref().map(|e| e.as_str()).unwrap_or("-");
        let phone = c.phone.as_ref().map(|p| p.as_str()).unwrap_or("-");
        println!("{:<40} {:<24} {:<30} {:<16}", c.id, c.name, email, phone);
    }

    Ok(())
}

pub(crate) fn run_customer_get(store: &Store<'_>, id: &str) -> Result<()> {
    match store.get_customer(id).context("looking up customer")? {
        Some(c) => {
            println!("ID:      {}", c.id);
            println!("Name:    {}", c.name);
            println!(
                "Email:   {}",
                c.email.as_ref().map(|e| e.as_str()).unwrap_or("(none)")
            );
            println!(
                "Phone:   {}",
                c.phone.as_ref().map(|p| p.as_str()).unwrap_or("(none)")
            );
            println!("Points:  {}", c.loyalty_points);
            println!("Spent:   {} {}", c.total_spent_minor, c.currency);
            println!("Notes:   {}", c.notes);
            println!("Created: {}", c.created_at);
            println!("Updated: {}", c.updated_at);
        }
        None => {
            println!("Customer not found: {id}");
        }
    }
    Ok(())
}

pub(crate) fn run_customer_create(
    store: &Store<'_>,
    name: &str,
    email: Option<&str>,
    phone: Option<&str>,
    notes: Option<&str>,
) -> Result<()> {
    if let Some(e) = email {
        foundation::Email::new(e).map_err(|e| anyhow::anyhow!("{}", e.message))?;
    }
    if let Some(p) = phone {
        foundation::Phone::new(p).map_err(|e| anyhow::anyhow!("{}", e.message))?;
    }

    let c = store
        .create_customer(name, email, phone, notes)
        .map_err(|e| match &e {
            CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
            _ => anyhow::anyhow!("Error: {e}"),
        })?;

    println!("Created customer: {} ({})", c.name, c.id);
    Ok(())
}

// ── User commands ──────────────────────────────────────────────────────

pub(crate) fn run_user(conn: &Connection, args: UserArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        UserAction::List => run_user_list(&store),
        UserAction::Get { id } => run_user_get(&store, &id),
        UserAction::Create {
            username,
            pin_hash,
            display_name,
            role_id,
        } => run_user_create(&store, &username, &pin_hash, &display_name, &role_id),
    }
}

pub(crate) fn run_user_list(store: &Store<'_>) -> Result<()> {
    let users = store.list_users().context("listing users")?;

    if users.is_empty() {
        println!("No users found.");
        return Ok(());
    }

    println!(
        "{:<40} {:<16} {:<24} {:<12} Active",
        "ID", "Username", "Display Name", "Role"
    );
    println!("{:-<40} {:-<16} {:-<24} {:-<12} {:-}", "", "", "", "", "");

    for u in &users {
        let active = if u.is_active { "yes" } else { "no" };
        println!(
            "{:<40} {:<16} {:<24} {:<12} {}",
            u.id, u.username, u.display_name, u.role_id, active
        );
    }

    Ok(())
}

pub(crate) fn run_user_get(store: &Store<'_>, id: &str) -> Result<()> {
    match store.get_user(id).context("looking up user")? {
        Some(u) => {
            println!("ID:       {}", u.id);
            println!("Username: {}", u.username);
            println!("Name:     {}", u.display_name);
            println!("Role:     {}", u.role_id);
            println!("Active:   {}", if u.is_active { "yes" } else { "no" });
            println!("Created:  {}", u.created_at);
            println!("Updated:  {}", u.updated_at);
        }
        None => {
            println!("User not found: {id}");
        }
    }
    Ok(())
}

/// Validate that a `--pin-hash` argument is a well-formed argon2 PHC
/// string (CLI-3 fix).
///
/// A typo'd or non-PHC value would otherwise be stored verbatim and can
/// never verify — `verify_pin` treats unparseable hashes as a clean
/// mismatch — silently locking the new user out. Fail closed at the CLI
/// boundary instead.
fn validate_phc_pin_hash(pin_hash: &str) -> Result<()> {
    use argon2::password_hash::PasswordHash;
    let parsed = PasswordHash::new(pin_hash)
        .map_err(|e| anyhow::anyhow!("--pin-hash is not a valid PHC string: {e}"))?;
    let alg = parsed.algorithm.as_str();
    if !alg.starts_with("argon2") {
        anyhow::bail!("--pin-hash must be an argon2 hash (got algorithm '{alg}')");
    }
    Ok(())
}

pub(crate) fn run_user_create(
    store: &Store<'_>,
    username: &str,
    pin_hash: &str,
    display_name: &str,
    role_id: &str,
) -> Result<()> {
    // CLI-3 fix: a typo'd or non-PHC `--pin-hash` value would be stored
    // verbatim and can never verify (verify_pin treats unparseable hashes
    // as a clean mismatch), silently locking the new user out. Validate the
    // argon2 PHC envelope up front with a fail-closed check.
    validate_phc_pin_hash(pin_hash)?;
    let u = store
        .create_user(username, pin_hash, display_name, role_id)
        .map_err(|e| match &e {
            CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
            CoreError::Conflict { entity, field } => {
                anyhow::anyhow!("Conflict: {entity} already exists ({field})")
            }
            _ => anyhow::anyhow!("Error: {e}"),
        })?;

    println!("Created user: {} ({})", u.display_name, u.username);
    Ok(())
}

// ── Product CRUD ─────────────────────────────────────────────────────

pub(crate) fn run_product(conn: &Connection, args: ProductArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        ProductAction::List => run_product_list(&store),
        ProductAction::Get { sku } => run_product_get(&store, &sku),
        ProductAction::Create {
            sku,
            name,
            price,
            currency,
        } => run_product_create(&store, &sku, &name, price, &currency),
        ProductAction::Update {
            sku,
            name,
            price,
            currency,
            category_id,
            barcode,
        } => run_product_update(
            &store,
            &sku,
            &name,
            price,
            &currency,
            category_id.as_deref(),
            barcode.as_deref(),
        ),
        ProductAction::Delete { sku } => run_product_delete(&store, &sku),
    }
}

pub(crate) fn run_product_list(store: &Store<'_>) -> Result<()> {
    let products = store.list_products().context("listing products")?;

    if products.is_empty() {
        println!("No products found.");
        return Ok(());
    }

    println!("{:<12} {:<24} {:>10}  Stock", "SKU", "Name", "Price");
    println!("{:-<12} {:-<24} {:->10}  {:-}", "", "", "", "");

    for p in &products {
        let price_str = format_minor(p.product.price.minor_units, p.product.price.currency);
        let stock_str = match p.stock_qty {
            Some(q) => q.to_string(),
            None => "-".into(),
        };
        println!(
            "{:<12} {:<24} {:>10}  {}",
            p.product.sku.as_str(),
            p.product.name,
            price_str,
            stock_str,
        );
    }

    Ok(())
}

pub(crate) fn run_product_get(store: &Store<'_>, sku: &str) -> Result<()> {
    match store.get_product(sku).context("looking up product")? {
        Some(p) => {
            let price_str = format!(
                "{} {}",
                format_minor(p.product.price.minor_units, p.product.price.currency),
                std::str::from_utf8(&p.product.price.currency.0).unwrap_or("???"),
            );
            println!("SKU:          {}", p.product.sku.as_str());
            println!("Name:         {}", p.product.name);
            println!("Price:        {}", price_str);
            println!(
                "Category:     {}",
                p.category_name.as_deref().unwrap_or("(none)")
            );
            println!(
                "Barcode:      {}",
                p.product
                    .barcode
                    .as_ref()
                    .map(|b| b.as_str())
                    .unwrap_or("(none)")
            );
            match p.stock_qty {
                Some(q) => println!("Stock:        {q}"),
                None => println!("Stock:        (no inventory)"),
            }
            println!("ID:           {}", p.product.id);
            println!("Created:      {}", p.product.created_at);
            println!("Updated:      {}", p.product.updated_at);
        }
        None => {
            println!("Product not found: {sku}");
        }
    }
    Ok(())
}

pub(crate) fn run_product_create(
    store: &Store<'_>,
    sku: &str,
    name: &str,
    price_minor: i64,
    currency_code: &str,
) -> Result<()> {
    let currency = Currency::from_str(currency_code)
        .with_context(|| format!("invalid currency code: {currency_code}"))?;
    let money = Money {
        minor_units: price_minor,
        currency,
    };

    let product = store
        .create_product(sku, name, money, None, None, 0, None)
        .map_err(|e| match &e {
            CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
            CoreError::Conflict { entity, field } => {
                anyhow::anyhow!("Conflict: {entity} already exists ({field})")
            }
            _ => anyhow::anyhow!("Error: {e}"),
        })?;

    println!(
        "Created product: {} ({})",
        product.name,
        product.sku.as_str()
    );
    Ok(())
}

pub(crate) fn run_product_update(
    store: &Store<'_>,
    sku: &str,
    name: &str,
    price_minor: i64,
    currency_code: &str,
    category_id: Option<&str>,
    barcode: Option<&str>,
) -> Result<()> {
    let currency = Currency::from_str(currency_code)
        .with_context(|| format!("invalid currency code: {currency_code}"))?;
    let money = Money {
        minor_units: price_minor,
        currency,
    };

    // Treat empty strings passed via --category-id or --barcode as None
    // so the caller can clear a previously-set value.
    let cat = category_id.filter(|s| !s.is_empty());
    let bar = barcode.filter(|s| !s.is_empty());

    let product = store
        .update_product(sku, name, money, cat, bar, None, None)
        .map_err(|e| match &e {
            CoreError::NotFound { .. } => anyhow::anyhow!("Product not found: {sku}"),
            CoreError::Validation { message, .. } => anyhow::anyhow!("Validation error: {message}"),
            _ => anyhow::anyhow!("Error: {e}"),
        })?;

    println!(
        "Updated product: {} ({})",
        product.name,
        product.sku.as_str()
    );
    Ok(())
}

pub(crate) fn run_product_delete(store: &Store<'_>, sku: &str) -> Result<()> {
    store.delete_product(sku).map_err(|e| match &e {
        CoreError::NotFound { .. } => anyhow::anyhow!("Product not found: {sku}"),
        _ => anyhow::anyhow!("Error: {e}"),
    })?;

    println!("Deleted product: {sku}");
    Ok(())
}

// ── Restore ────────────────────────────────────────────────────────────

/// Restore the database from a backup file.
pub(crate) fn run_restore(conn: Connection, input: &str) -> Result<()> {
    eprintln!("restoring from {input}...");

    // Close the existing connection, then copy the backup over.
    let db_path = conn
        .path()
        .map(|p| p.to_owned())
        .unwrap_or_else(|| "oz-pos.db".into());
    drop(conn);

    std::fs::copy(input, &db_path)
        .with_context(|| format!("copying backup {input} to {db_path}"))?;

    eprintln!("restore complete — database replaced with backup");
    Ok(())
}

// ── Export .ozpkg ─────────────────────────────────────────────────────

/// Export store data to an encrypted .ozpkg file.
pub(crate) fn run_export_ozpkg(
    conn: &Connection,
    output: &str,
    types_str: &str,
    password: &str,
) -> Result<()> {
    use oz_core::ozpkg::{OzpkgPayload, export_ozpkg};

    let store = Store::new(conn);

    // Parse which data types to include.
    let all_types = types_str == "all";
    let requested: Vec<String> = if all_types {
        vec![]
    } else {
        types_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect()
    };

    let wants = |name: &str| all_types || requested.iter().any(|r| r == name);

    eprintln!("exporting data...");

    // Collect data from the database.
    let products = if wants("products") {
        let prods = store.list_products()?;
        serde_json::to_value(&prods)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let categories = if wants("categories") {
        let cats = store.list_categories()?;
        serde_json::to_value(&cats)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let sales = if wants("sales") {
        let sales_list = store.list_sales()?;
        Some(
            serde_json::to_value(&sales_list)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let customers = if wants("customers") {
        let custs = store.list_customers()?;
        Some(
            serde_json::to_value(&custs)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let users = if wants("users") {
        let usrs = store.list_users()?;
        Some(
            serde_json::to_value(&usrs)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let settings = if wants("settings") {
        let rows = oz_core::Settings::load_all(conn)?;
        Some(
            rows.into_iter()
                .map(|(key, value)| serde_json::json!({ "key": key, "value": value }))
                .collect(),
        )
    } else {
        None
    };

    // Collect feature flags for header metadata.
    let reg = store.load_features()?;
    let features: HashMap<String, String> = reg.to_settings_rows().into_iter().collect();

    // Build data_types list.
    let mut data_types: Vec<String> = Vec::new();
    if wants("products") {
        data_types.push("products".into());
    }
    if wants("categories") {
        data_types.push("categories".into());
    }
    if wants("sales") {
        data_types.push("sales".into());
    }
    if wants("customers") {
        data_types.push("customers".into());
    }
    if wants("users") {
        data_types.push("users".into());
    }
    if wants("settings") {
        data_types.push("settings".into());
    }

    let payload = OzpkgPayload {
        products,
        categories,
        sales,
        customers,
        users,
        settings,
    };

    let store_name = store
        .get_store_name()?
        .unwrap_or_else(|| "OZ-POS Store".into());

    eprintln!("  encrypting with Argon2id + AES-256-GCM...");
    let ozpkg_bytes = export_ozpkg(
        password,
        &store_name,
        "0.0.1",
        data_types,
        features,
        &payload,
    )
    .context("encrypting export")?;

    std::fs::write(output, &ozpkg_bytes).with_context(|| format!("writing {output}"))?;

    eprintln!("exported to {output} ({} bytes)", ozpkg_bytes.len());
    Ok(())
}

// ── Import .ozpkg ─────────────────────────────────────────────────────

/// Import data from an encrypted .ozpkg file.
/// Decode a product's raw currency bytes as UTF-8, returning a recoverable
/// error instead of panicking when an imported `.ozpkg` carries non-UTF-8
/// currency bytes (RUST-07: recoverable user-supplied input).
fn currency_to_utf8(product: &oz_core::Product) -> Result<String> {
    std::str::from_utf8(&product.price.currency.0)
        .map(|s| s.to_owned())
        .map_err(|e| {
            anyhow::anyhow!(
                "product {} has invalid (non-UTF-8) currency: {e}",
                product.sku
            )
        })
}

pub(crate) fn run_import_ozpkg(
    conn: &Connection,
    input: &str,
    password: &str,
    dry_run: bool,
) -> Result<()> {
    use oz_core::ozpkg::import_ozpkg;

    eprintln!("reading {input}...");
    let data = std::fs::read(input).with_context(|| format!("reading {input}"))?;

    eprintln!("  decrypting...");
    let (header, payload) = import_ozpkg(&data, password).context("decrypting import file")?;

    // Show metadata.
    println!();
    println!("Store:      {}", header.store_name);
    println!("Version:    {}", header.app_version);
    println!("Created:    {}", header.created_at);
    println!("Types:      {}", header.data_types.join(", "));
    println!("Products:   {}", payload.products.len());
    println!("Categories: {}", payload.categories.len());
    if let Some(sales) = &payload.sales {
        println!("Sales:      {}", sales.len());
    }
    if let Some(customers) = &payload.customers {
        println!("Customers:  {}", customers.len());
    }
    if let Some(users) = &payload.users {
        println!("Users:      {}", users.len());
    }
    if let Some(settings) = &payload.settings {
        println!("Settings:   {}", settings.len());
    }
    println!();

    if dry_run {
        println!("Dry-run mode — no data written.");
        return Ok(());
    }

    // Write data to the database inside a single transaction.
    let store = Store::new(conn);
    let tx = conn
        .unchecked_transaction()
        .context("starting import transaction")?;

    let mut total = 0usize;

    // ── Categories ──────────────────────────────────────────────
    for val in &payload.categories {
        if let Ok(cat) = serde_json::from_value::<oz_core::Category>(val.clone()) {
            let colour = if cat.colour.is_empty() {
                "#6366f1"
            } else {
                &cat.colour
            };
            let exists = tx
                .query_row(
                    "SELECT 1 FROM categories WHERE id = ?1",
                    rusqlite::params![cat.id],
                    |_| Ok(()),
                )
                .is_ok();
            if exists {
                tx.execute(
                    "UPDATE categories SET name = ?1, colour = ?2 WHERE id = ?3",
                    rusqlite::params![cat.name, colour, cat.id],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO categories (id, name, colour) VALUES (?1, ?2, ?3)",
                    rusqlite::params![cat.id, cat.name, colour],
                )?;
            }
            total += 1;
        }
    }

    // ── Products ────────────────────────────────────────────────
    for val in &payload.products {
        if let Ok(product) = serde_json::from_value::<oz_core::Product>(val.clone()) {
            let exists = tx
                .query_row(
                    "SELECT 1 FROM products WHERE sku = ?1",
                    rusqlite::params![product.sku.to_string()],
                    |_| Ok(()),
                )
                .is_ok();
            if exists {
                let cur_str = currency_to_utf8(&product)?;
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                tx.execute(
                    "UPDATE products SET name = ?1, price_minor = ?2, currency = ?3, category_id = ?4, barcode = ?5, updated_at = ?6 WHERE sku = ?7",
                    rusqlite::params![product.name, product.price.minor_units, cur_str, product.category_id, product.barcode.as_ref().map(|b| b.as_str()), now, product.sku.to_string()],
                )?;
            } else {
                let cur_str = currency_to_utf8(&product)?;
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                tx.execute(
                    "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![product.id, product.sku.to_string(), product.name, product.price.minor_units, cur_str, product.category_id, product.barcode.as_ref().map(|b| b.as_str()), now, now],
                )?;
            }
            total += 1;
        }
    }

    // ── Sales ───────────────────────────────────────────────────
    if let Some(ref sales) = payload.sales {
        for val in sales {
            if let Ok(sale) = serde_json::from_value::<oz_core::Sale>(val.clone()) {
                let exists = tx
                    .query_row(
                        "SELECT 1 FROM sales WHERE id = ?1",
                        rusqlite::params![sale.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                if !exists {
                    // CLI-1 fix: use the tx-aware variant — the previous
                    // `store.create_sale` opened a nested transaction on the
                    // same connection ("cannot start a transaction within a
                    // transaction") and rolled the whole import back.
                    store.create_sale_in_tx(&tx, &sale)?;
                }
                total += 1;
            }
        }
    }

    // ── Customers ───────────────────────────────────────────────
    if let Some(ref customers) = payload.customers {
        for val in customers {
            if let Ok(cust) = serde_json::from_value::<oz_core::Customer>(val.clone()) {
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let exists = tx
                    .query_row(
                        "SELECT 1 FROM customers WHERE id = ?1",
                        rusqlite::params![cust.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                let email_str = cust.email.map(|e| e.to_string());
                let phone_str = cust.phone.map(|p| p.to_string());
                if exists {
                    tx.execute(
                        "UPDATE customers SET name = ?1, email = ?2, phone = ?3, notes = ?4, updated_at = ?5 WHERE id = ?6",
                        rusqlite::params![cust.name, email_str, phone_str, cust.notes, now, cust.id],
                    )?;
                } else {
                    tx.execute(
                        "INSERT INTO customers (id, name, email, phone, notes, loyalty_points, total_spent_minor, currency, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 'USD', ?6, ?7)",
                        rusqlite::params![cust.id, cust.name, email_str, phone_str, cust.notes, now, now],
                    )?;
                }
                total += 1;
            }
        }
    }

    // ── Users ───────────────────────────────────────────────────
    if let Some(ref users) = payload.users {
        for val in users {
            if let Ok(user) = serde_json::from_value::<oz_core::User>(val.clone()) {
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let exists = tx
                    .query_row(
                        "SELECT 1 FROM users WHERE id = ?1",
                        rusqlite::params![user.id],
                        |_| Ok(()),
                    )
                    .is_ok();
                if exists {
                    tx.execute(
                        "UPDATE users SET username = ?1, display_name = ?2, role_id = ?3, updated_at = ?4 WHERE id = ?5",
                        rusqlite::params![user.username, user.display_name, user.role_id, now, user.id],
                    )?;
                } else {
                    // PIN hash not included in export; imported users are inactive
                    tx.execute(
                        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
                         VALUES (?1, ?2, '', ?3, ?4, 0, ?5, ?6)",
                        rusqlite::params![user.id, user.username, user.display_name, user.role_id, now, now],
                    )?;
                }
                total += 1;
            }
        }
    }

    // ── Settings ────────────────────────────────────────────────
    if let Some(ref settings) = payload.settings {
        for val in settings {
            if let Some(key) = val.get("key").and_then(|v| v.as_str())
                && let Some(value) = val.get("value").and_then(|v| v.as_str())
            {
                let _ = Settings::set(&tx, key, value);
                total += 1;
            }
        }
    }

    tx.commit().context("committing import transaction")?;

    eprintln!("import complete — {total} records written.");
    Ok(())
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
