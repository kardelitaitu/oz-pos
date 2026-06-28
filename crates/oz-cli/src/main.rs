//! `oz` — OZ-POS command-line tool.
//!
//! Subcommands:
//! - `oz migrate` — apply pending SQL migrations
//! - `oz init-db` — seed the database with default settings + feature preset
//! - `oz product list` — list all products
//! - `oz product get <sku>` — show a product by SKU
//! - `oz product create <sku> <name> <price>` — create a new product
//! - `oz product update <sku> <name> <price>` — update an existing product
//! - `oz product delete <sku>` — delete a product
//! - `oz backup` — snapshot the local SQLite store (scaffold)
//! - `oz export` — write a CSV report (scaffold)

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use rusqlite::Connection;

use oz_core::db::Store;
use oz_core::{CoreError, Currency, FeatureRegistry, Money, Settings};

// ── CLI structure ─────────────────────────────────────────────────────

/// OZ-POS command-line tool.
#[derive(Debug, Parser)]
#[command(name = "oz", version, about = "OZ-POS maintenance and migration CLI")]
struct Cli {
    /// Path to the SQLite database (default: ./oz-pos.db).
    #[arg(short, long, global = true, default_value = "oz-pos.db")]
    db: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Apply pending SQL migrations to the database.
    Migrate,
    /// Seed the database with default settings and feature flags.
    InitDb(InitDbArgs),
    /// Manage products (list, get, create, update, delete).
    Product(ProductArgs),
    /// Snapshot the local SQLite store to a backup file.
    Backup {
        /// Destination path for the backup file.
        #[arg(short, long)]
        output: String,
    },
    /// Write a CSV report for the given time window (scaffold).
    Export {
        /// Report kind (e.g. `daily-summary`, `sales-by-hour`).
        kind: String,
    },
}

#[derive(Debug, Args)]
struct InitDbArgs {
    /// Feature preset to apply (simple-retail, restaurant, full-store, custom).
    #[arg(long, default_value = "simple-retail")]
    preset: String,
}

#[derive(Debug, Args)]
struct ProductArgs {
    #[command(subcommand)]
    action: ProductAction,
}

#[derive(Debug, Subcommand)]
enum ProductAction {
    /// List all products.
    List,
    /// Show a product by SKU.
    Get {
        /// Product SKU.
        sku: String,
    },
    /// Create a new product.
    Create {
        /// Unique product SKU.
        sku: String,
        /// Display name.
        name: String,
        /// Price in minor units (e.g. 350 for $3.50).
        price: i64,
        /// ISO-4217 currency code (default: USD).
        #[arg(long, default_value = "USD")]
        currency: String,
    },
    /// Update an existing product by SKU.
    Update {
        /// Product SKU to update.
        sku: String,
        /// New display name.
        name: String,
        /// New price in minor units.
        price: i64,
        /// ISO-4217 currency code (default: USD).
        #[arg(long, default_value = "USD")]
        currency: String,
        /// New category id (or empty to clear).
        #[arg(long)]
        category_id: Option<String>,
        /// New barcode (or empty to clear).
        #[arg(long)]
        barcode: Option<String>,
    },
    /// Delete a product by SKU.
    Delete {
        /// Product SKU to delete.
        sku: String,
    },
}

// ── Entry point ───────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Open connection (shared by most commands).
    let conn = open_db(&cli.db)?;

    match cli.command {
        Some(Command::Migrate) => run_migrate(conn),
        Some(Command::InitDb(args)) => run_init_db(&conn, &args),
        Some(Command::Product(args)) => run_product(&conn, args),
        Some(Command::Backup { output }) => {
            println!("backup -> {output}: not yet implemented (scaffold)");
            Ok(())
        }
        Some(Command::Export { kind }) => {
            println!("export {kind}: not yet implemented (scaffold)");
            Ok(())
        }
        None => {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            Ok(())
        }
    }
}

// ── DB helpers ────────────────────────────────────────────────────────

fn open_db(path: &str) -> Result<Connection> {
    let conn =
        Connection::open(path).with_context(|| format!("opening database at {path}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enabling foreign_keys")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enabling WAL")?;
    Ok(conn)
}

// ── Migrate ──────────────────────────────────────────────────────────

fn run_migrate(conn: Connection) -> Result<()> {
    eprintln!("applying migrations...");
    let mut conn = conn;
    oz_core::migrations::run(&mut conn).context("applying migrations")?;
    eprintln!("migrations up to date");
    Ok(())
}

// ── Init-DB ──────────────────────────────────────────────────────────

/// Seed the database with default settings and a feature preset.
fn run_init_db(conn: &Connection, args: &InitDbArgs) -> Result<()> {
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

    eprintln!("database initialised successfully");
    Ok(())
}

// ── Product CRUD ─────────────────────────────────────────────────────

fn run_product(conn: &Connection, args: ProductArgs) -> Result<()> {
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

fn run_product_list(store: &Store<'_>) -> Result<()> {
    let products = store.list_products().context("listing products")?;

    if products.is_empty() {
        println!("No products found.");
        return Ok(());
    }

    println!("{:<12} {:<24} {:>10}  Stock", "SKU", "Name", "Price");
    println!("{:-<12} {:-<24} {:->10}  {:-}", "", "", "", "");

    for p in &products {
        let price_str = format!(
            "{}.{:02}",
            p.product.price.minor_units / 100,
            p.product.price.minor_units.abs() % 100
        );
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

fn run_product_get(store: &Store<'_>, sku: &str) -> Result<()> {
    match store.get_product(sku).context("looking up product")? {
        Some(p) => {
            let price_str = format!(
                "{}.{:02} {}",
                p.product.price.minor_units / 100,
                p.product.price.minor_units.abs() % 100,
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
                p.product.barcode.as_deref().unwrap_or("(none)")
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

fn run_product_create(
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

    match store.create_product(sku, name, money, None, None, 0) {
        Ok(product) => {
            println!("Created product: {} ({})", product.name, product.sku.as_str());
        }
        Err(CoreError::Validation { message, .. }) => {
            eprintln!("Validation error: {message}");
            std::process::exit(1);
        }
        Err(CoreError::Conflict { entity, field }) => {
            eprintln!("Conflict: {entity} already exists ({field})");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_product_update(
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
    let cat = category_id.and_then(|s| if s.is_empty() { None } else { Some(s) });
    let bar = barcode.and_then(|s| if s.is_empty() { None } else { Some(s) });

    match store.update_product(sku, name, money, cat, bar) {
        Ok(product) => {
            println!("Updated product: {} ({})", product.name, product.sku.as_str());
        }
        Err(CoreError::NotFound { .. }) => {
            eprintln!("Product not found: {sku}");
            std::process::exit(1);
        }
        Err(CoreError::Validation { message, .. }) => {
            eprintln!("Validation error: {message}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_product_delete(store: &Store<'_>, sku: &str) -> Result<()> {
    match store.delete_product(sku) {
        Ok(()) => {
            println!("Deleted product: {sku}");
        }
        Err(CoreError::NotFound { .. }) => {
            eprintln!("Product not found: {sku}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
