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
use oz_core::{CoreError, Currency, FeatureRegistry, Money, Settings, SaleStatus};

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
    /// Manage categories (list, get, create, delete).
    Category(CategoryArgs),
    /// Manage inventory (get, adjust).
    Inventory(InventoryArgs),
    /// Manage sales (list, get, update-status).
    Sale(SaleArgs),
}

#[derive(Debug, Args)]
struct CategoryArgs {
    #[command(subcommand)]
    action: CategoryAction,
}

#[derive(Debug, Subcommand)]
enum CategoryAction {
    /// List all categories.
    List,
    /// Show a category by id.
    Get {
        /// Category id.
        id: String,
    },
    /// Create a new category.
    Create {
        /// Unique category id (e.g. "cat-drinks").
        id: String,
        /// Display name.
        name: String,
        /// Hex colour (e.g. "#06b6d4").
        colour: String,
    },
    /// Delete a category by id.
    Delete {
        /// Category id.
        id: String,
    },
}

#[derive(Debug, Args)]
struct InventoryArgs {
    #[command(subcommand)]
    action: InventoryAction,
}

#[derive(Debug, Subcommand)]
enum InventoryAction {
    /// Show current stock for a product by SKU.
    Get {
        /// Product SKU.
        sku: String,
    },
    /// Adjust stock for a product by SKU (e.g. +10 to restock, -3 to sell).
    Adjust {
        /// Product SKU.
        sku: String,
        /// Signed delta (e.g. +10 or -3).
        delta: i64,
    },
}

#[derive(Debug, Args)]
struct SaleArgs {
    #[command(subcommand)]
    action: SaleAction,
}

#[derive(Debug, Subcommand)]
enum SaleAction {
    /// List all sales (most recent first).
    List,
    /// Show a sale by id.
    Get {
        /// Sale UUID.
        id: String,
        /// Output format (default: text, or "json").
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Transition a sale to a new status.
    UpdateStatus {
        /// Sale UUID.
        id: String,
        /// New status (pending, active, completed, voided).
        status: String,
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
        Some(Command::Backup { output }) => run_backup(&conn, &output),
        Some(Command::Export { kind }) => run_export(&conn, &kind),
        Some(Command::Category(args)) => run_category(&conn, args),
        Some(Command::Inventory(args)) => run_inventory(&conn, args),
        Some(Command::Sale(args)) => run_sale(&conn, args),
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

// ── Backup / Export ────────────────────────────────────────────────────

/// Create an online SQLite snapshot of the database.
fn run_backup(conn: &Connection, output: &str) -> Result<()> {
    let store = Store::new(conn);
    eprintln!("creating backup -> {output}...");
    store
        .backup(output)
        .with_context(|| format!("backup to {output}"))?;
    eprintln!("backup complete");
    Ok(())
}

/// Write a CSV report to stdout for the given kind.
fn run_export(conn: &Connection, kind: &str) -> Result<()> {
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
            std::process::exit(1);
        }
    }

    Ok(())
}

// ── Category commands ────────────────────────────────────────────────────

fn run_category(conn: &Connection, args: CategoryArgs) -> Result<()> {
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

fn run_category_list(store: &Store<'_>) -> Result<()> {
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

fn run_category_get(store: &Store<'_>, id: &str) -> Result<()> {
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

fn run_category_create(store: &Store<'_>, id: &str, name: &str, colour: &str) -> Result<()> {
    match store.create_category(id, name, colour) {
        Ok(cat) => {
            println!("Created category: {} ({})", cat.name, cat.id);
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

fn run_category_delete(store: &Store<'_>, id: &str) -> Result<()> {
    match store.delete_category(id) {
        Ok(()) => {
            println!("Deleted category: {id}");
        }
        Err(CoreError::NotFound { .. }) => {
            eprintln!("Category not found: {id}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

// ── Inventory commands ──────────────────────────────────────────────────

fn run_inventory(conn: &Connection, args: InventoryArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        InventoryAction::Get { sku } => run_inventory_get(&store, &sku),
        InventoryAction::Adjust { sku, delta } => run_inventory_adjust(&store, &sku, delta),
    }
}

fn run_inventory_get(store: &Store<'_>, sku: &str) -> Result<()> {
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

fn run_inventory_adjust(store: &Store<'_>, sku: &str, delta: i64) -> Result<()> {
    match store.adjust_stock(sku, delta) {
        Ok(new_qty) => {
            let verb = if delta >= 0 { "restocked" } else { "sold" };
            println!(
                "Stock {verb} for {sku} (delta: {delta:+}) — new qty: {new_qty}"
            );
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

// ── Sale commands ──────────────────────────────────────────────────────

fn run_sale(conn: &Connection, args: SaleArgs) -> Result<()> {
    let store = Store::new(conn);

    match args.action {
        SaleAction::List => run_sale_list(&store),
        SaleAction::Get { id, format } => run_sale_get(&store, &id, &format),
        SaleAction::UpdateStatus { id, status } => run_sale_update_status(&store, &id, &status),
    }
}

fn run_sale_list(store: &Store<'_>) -> Result<()> {
    let sales = store.list_sales().context("listing sales")?;

    if sales.is_empty() {
        println!("No sales found.");
        return Ok(());
    }

    println!("{:<40} {:>10} {:>6}  {:>10}  Date", "ID", "Total", "Items", "Status");
    println!("{:-<40} {:->10} {:->6}  {:->10}  {:-<4}", "", "", "", "", "");

    for s in &sales {
        let total_str = format!(
            "{}.{:02}",
            s.total.minor_units / 100,
            s.total.minor_units.abs() % 100,
        );
        let status_str = match s.status {
            SaleStatus::Pending => "pending",
            SaleStatus::Active => "active",
            SaleStatus::Completed => "done",
            SaleStatus::Voided => "voided",
        };
        // Show only date portion of ISO-8601 timestamp.
        let date_str = s.created_at.as_str();
        let date_str = if date_str.len() > 10 { &date_str[..10] } else { date_str };
        println!("{:<40} {:>10} {:>6}  {:>10}  {}", s.id, total_str, s.line_count, status_str, date_str);
    }

    Ok(())
}

fn run_sale_get(store: &Store<'_>, id: &str, format: &str) -> Result<()> {
    match store.get_sale(id).context("looking up sale")? {
        Some(sale) => {
            if format == "json" {
                let json = serde_json::to_string_pretty(&sale)
                    .context("serializing sale to JSON")?;
                println!("{json}");
            } else {
                let total_str = format!(
                    "{}.{:02} {}",
                    sale.total.minor_units / 100,
                    sale.total.minor_units.abs() % 100,
                    std::str::from_utf8(&sale.currency.0).unwrap_or("???"),
                );
                println!("ID:           {}", sale.id);
                println!("Status:       {:?}", sale.status);
                println!("Total:        {}", total_str);
                println!("Line count:   {}", sale.line_count);
                println!("Currency:     {}", std::str::from_utf8(&sale.currency.0).unwrap_or("???"));
                println!("Created:      {}", sale.created_at);
                println!("Updated:      {}", sale.updated_at);

                if !sale.lines.is_empty() {
                    println!();
                    println!("{:<4} {:<24} {:>6} {:>10}", "#", "SKU", "Qty", "Unit");
                    println!("{:-<4} {:-<24} {:->6} {:->10}", "", "", "", "");
                    for line in &sale.lines {
                        let unit_str = format!(
                            "{}.{:02}",
                            line.unit_price.minor_units / 100,
                            line.unit_price.minor_units.abs() % 100,
                        );
                        println!("{:<4} {:<24} {:>6} {:>10}", line.line_position, line.sku, line.qty, unit_str);
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

fn run_sale_update_status(store: &Store<'_>, id: &str, status_str: &str) -> Result<()> {
    let to = SaleStatus::from_stored_str(status_str).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid status '{status_str}'; expected one of: pending, active, completed, voided"
        )
    })?;

    match store.update_sale_status(id, to) {
        Ok(sale) => {
            println!("Sale {id} status updated to {:?}", sale.status);
        }
        Err(CoreError::NotFound { .. }) => {
            eprintln!("Sale not found: {id}");
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
