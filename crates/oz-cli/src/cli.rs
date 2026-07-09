#![allow(missing_docs)]
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

use clap::{Args, Parser, Subcommand};

/// OZ-POS command-line tool.
#[derive(Debug, Parser)]
#[command(name = "oz", version, about = "OZ-POS maintenance and migration CLI")]
pub struct Cli {
    /// Path to the SQLite database (default: ./oz-pos.db).
    #[arg(short, long, global = true, default_value = "oz-pos.db")]
    pub db: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
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
    /// Manage customers (list, get, create).
    Customer(CustomerArgs),
    /// Manage users (list, get, create).
    User(UserArgs),
    /// Restore the database from a backup file.
    Restore {
        /// Path to the backup file.
        #[arg(short, long)]
        input: String,
    },
    /// Export data to an encrypted .ozpkg file.
    ExportOzpkg {
        /// Output file path.
        #[arg(short, long)]
        output: String,
        /// Data types to include (comma-separated: products,categories,sales,customers,users,settings).
        #[arg(short, long, default_value = "all")]
        types: String,
        /// Encryption password.
        #[arg(short, long)]
        password: String,
    },
    /// Import data from an encrypted .ozpkg file.
    ImportOzpkg {
        /// Input .ozpkg file path.
        #[arg(short, long)]
        input: String,
        /// Decryption password.
        #[arg(short, long)]
        password: String,
        /// Dry-run mode: show what would be imported without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Args)]
pub struct InitDbArgs {
    /// Feature preset to apply (simple-retail, restaurant, full-store, custom).
    #[arg(long, default_value = "simple-retail")]
    pub preset: String,
}

#[derive(Debug, Args)]
pub struct ProductArgs {
    #[command(subcommand)]
    pub action: ProductAction,
}

#[derive(Debug, Subcommand)]
pub enum ProductAction {
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

#[derive(Debug, Args)]
pub struct CategoryArgs {
    #[command(subcommand)]
    pub action: CategoryAction,
}

#[derive(Debug, Subcommand)]
pub enum CategoryAction {
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
pub struct InventoryArgs {
    #[command(subcommand)]
    pub action: InventoryAction,
}

#[derive(Debug, Subcommand)]
pub enum InventoryAction {
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
pub struct SaleArgs {
    #[command(subcommand)]
    pub action: SaleAction,
}

#[derive(Debug, Subcommand)]
pub enum SaleAction {
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
pub struct CustomerArgs {
    #[command(subcommand)]
    pub action: CustomerAction,
}

#[derive(Debug, Subcommand)]
pub enum CustomerAction {
    /// List all customers.
    List,
    /// Show a customer by id.
    Get {
        /// Customer id.
        id: String,
    },
    /// Create a new customer.
    Create {
        /// Display name.
        name: String,
        /// Email address.
        #[arg(long)]
        email: Option<String>,
        /// Phone number.
        #[arg(long)]
        phone: Option<String>,
        /// Free-form notes.
        #[arg(long)]
        notes: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub action: UserAction,
}

#[derive(Debug, Subcommand)]
pub enum UserAction {
    /// List all users.
    List,
    /// Show a user by id.
    Get {
        /// User id.
        id: String,
    },
    /// Create a new user.
    Create {
        /// Login username.
        username: String,
        /// Hashed PIN/password.
        pin_hash: String,
        /// Display name shown on the POS UI.
        display_name: String,
        /// Role id (e.g. "role-owner", "role-cashier").
        role_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parse_migrate() {
        let cli = Cli::try_parse_from(["oz", "migrate"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Migrate)));
    }

    #[test]
    fn cli_parse_product_list() {
        let cli = Cli::try_parse_from(["oz", "product", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Product(ProductArgs {
                action: ProductAction::List,
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_product_create() {
        let cli =
            Cli::try_parse_from(["oz", "product", "create", "SKU-1", "Widget", "999"]).unwrap();
        match cli.command {
            Some(Command::Product(ProductArgs {
                action:
                    ProductAction::Create {
                        sku, name, price, ..
                    },
            })) => {
                assert_eq!(sku, "SKU-1");
                assert_eq!(name, "Widget");
                assert_eq!(price, 999);
            }
            _ => panic!("expected Product::Create"),
        }
    }

    #[test]
    fn cli_parse_product_get() {
        let cli = Cli::try_parse_from(["oz", "product", "get", "ABC"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Product(ProductArgs {
                action: ProductAction::Get { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_category_list() {
        let cli = Cli::try_parse_from(["oz", "category", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Category(CategoryArgs {
                action: CategoryAction::List,
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_inventory_get() {
        let cli = Cli::try_parse_from(["oz", "inventory", "get", "SKU-001"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Inventory(InventoryArgs {
                action: InventoryAction::Get { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_sale_list() {
        let cli = Cli::try_parse_from(["oz", "sale", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Sale(SaleArgs {
                action: SaleAction::List,
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_customer_list() {
        let cli = Cli::try_parse_from(["oz", "customer", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Customer(CustomerArgs {
                action: CustomerAction::List,
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_user_list() {
        let cli = Cli::try_parse_from(["oz", "user", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::User(UserArgs {
                action: UserAction::List,
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_backup() {
        let cli = Cli::try_parse_from(["oz", "backup", "-o", "backup.db"]).unwrap();
        match cli.command {
            Some(Command::Backup { output }) => assert_eq!(output, "backup.db"),
            _ => panic!("expected Backup"),
        }
    }

    #[test]
    fn cli_parse_restore() {
        let cli = Cli::try_parse_from(["oz", "restore", "-i", "backup.db"]).unwrap();
        match cli.command {
            Some(Command::Restore { input }) => assert_eq!(input, "backup.db"),
            _ => panic!("expected Restore"),
        }
    }

    #[test]
    fn cli_parse_default_db() {
        let cli = Cli::try_parse_from(["oz", "migrate"]).unwrap();
        assert_eq!(cli.db, "oz-pos.db");
    }

    #[test]
    fn cli_parse_custom_db() {
        let cli = Cli::try_parse_from(["oz", "--db", "custom.db", "migrate"]).unwrap();
        assert_eq!(cli.db, "custom.db");
    }

    #[test]
    fn cli_parse_export_ozpkg() {
        let cli =
            Cli::try_parse_from(["oz", "export-ozpkg", "-o", "data.ozpkg", "-p", "secret123"])
                .unwrap();
        match cli.command {
            Some(Command::ExportOzpkg {
                output, password, ..
            }) => {
                assert_eq!(output, "data.ozpkg");
                assert_eq!(password, "secret123");
            }
            _ => panic!("expected ExportOzpkg"),
        }
    }

    #[test]
    fn cli_parse_import_ozpkg() {
        let cli = Cli::try_parse_from([
            "oz",
            "import-ozpkg",
            "-i",
            "data.ozpkg",
            "-p",
            "secret123",
            "--dry-run",
        ])
        .unwrap();
        match cli.command {
            Some(Command::ImportOzpkg {
                input,
                password,
                dry_run,
            }) => {
                assert_eq!(input, "data.ozpkg");
                assert_eq!(password, "secret123");
                assert!(dry_run);
            }
            _ => panic!("expected ImportOzpkg"),
        }
    }

    #[test]
    fn cli_parse_sale_get() {
        let cli = Cli::try_parse_from(["oz", "sale", "get", "some-id"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Sale(SaleArgs {
                action: SaleAction::Get { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_sale_update_status() {
        let cli =
            Cli::try_parse_from(["oz", "sale", "update-status", "some-id", "completed"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Sale(SaleArgs {
                action: SaleAction::UpdateStatus { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_category_create() {
        let cli = Cli::try_parse_from([
            "oz",
            "category",
            "create",
            "cat-drinks",
            "Beverages",
            "#06b6d4",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Category(CategoryArgs {
                action: CategoryAction::Create { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_user_create() {
        let cli = Cli::try_parse_from([
            "oz",
            "user",
            "create",
            "jdoe",
            "hash123",
            "John Doe",
            "role-cashier",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::User(UserArgs {
                action: UserAction::Create { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_customer_create() {
        let cli = Cli::try_parse_from([
            "oz",
            "customer",
            "create",
            "Alice",
            "--email",
            "alice@test.com",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Customer(CustomerArgs {
                action: CustomerAction::Create { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_inventory_adjust() {
        let cli = Cli::try_parse_from(["oz", "inventory", "adjust", "SKU-001", "+5"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Inventory(InventoryArgs {
                action: InventoryAction::Adjust { .. },
                ..
            }))
        ));
    }

    #[test]
    fn cli_parse_export_csv() {
        let cli = Cli::try_parse_from(["oz", "export", "daily-summary"]).unwrap();
        match cli.command {
            Some(Command::Export { kind }) => assert_eq!(kind, "daily-summary"),
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn cli_parse_export_with_types_and_password() {
        let cli = Cli::try_parse_from([
            "oz",
            "export-ozpkg",
            "-o",
            "backup.ozpkg",
            "-p",
            "secret",
            "-t",
            "products,customers",
        ])
        .unwrap();
        match cli.command {
            Some(Command::ExportOzpkg {
                output,
                password,
                types,
                ..
            }) => {
                assert_eq!(output, "backup.ozpkg");
                assert_eq!(password, "secret");
                assert_eq!(types, "products,customers");
            }
            _ => panic!("expected ExportOzpkg"),
        }
    }
}
