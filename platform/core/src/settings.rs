//! Settings store — typed access to a key-value settings table.
//!
//! The [`Settings`] struct provides read/write helpers for a generic
//! `settings` table (`key TEXT PRIMARY KEY, value TEXT`). All methods
//! take a `&rusqlite::Connection` so callers control transaction
//! boundaries.

use rusqlite::{Connection, params};

use crate::error::PlatformError;

/// Typed access to a key-value `settings` table.
pub struct Settings;

// ── Raw key-value helpers ────────────────────────────────────────────

impl Settings {
    /// Read a single setting by key. Returns `None` if the key doesn't exist.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, PlatformError> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Insert or update a setting.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), PlatformError> {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting. Returns `true` if the key existed.
    pub fn remove(conn: &Connection, key: &str) -> Result<bool, PlatformError> {
        let n = conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(n > 0)
    }

    /// Load every row from the `settings` table as `(key, value)` pairs.
    pub fn load_all(conn: &Connection) -> Result<Vec<(String, String)>, PlatformError> {
        let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Write multiple settings inside a single transaction.
    pub fn set_batch(conn: &Connection, rows: &[(String, String)]) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        for (key, value) in rows {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![key, value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

// ── Typed store configuration helpers ────────────────────────────────

/// Well-known settings keys for store configuration.
pub mod keys {
    /// Store display name. Default: `"OZ-POS Store"`.
    pub const STORE_NAME: &str = "store.name";
    /// Store street address (printed on receipts).
    pub const STORE_ADDRESS: &str = "store.address";
    /// Store tax / VAT registration number.
    pub const STORE_TAX_ID: &str = "store.tax_id";
    /// Default ISO-4217 currency code. Default: `"USD"`.
    pub const DEFAULT_CURRENCY: &str = "store.default_currency";
    /// Store branch name (e.g. "Downtown", "Mall Branch").
    pub const STORE_BRANCH: &str = "store.branch";
    /// Store logo (base64-encoded PNG). Empty string = no logo.
    pub const STORE_LOGO: &str = "store.logo";
    /// Store preset name (e.g., `"simple-retail"`, `"restaurant"`).
    pub const STORE_PRESET: &str = "store.preset";
    /// Whether the Setup Wizard has been completed.
    pub const SETUP_COMPLETE: &str = "store.setup_complete";
    /// Whether to show the Setup Wizard. `"true"` by default (absent).
    /// Set to `"false"` when the user completes or skips the wizard.
    pub const SHOW_SETUP_WIZARD: &str = "store.show_setup_wizard";

    // ── Receipt display settings ───────────────────────────────────
    /// Show currency symbol prefix on amounts. `"1"` or `"0"`. Default `"0"`.
    pub const RECEIPT_SHOW_CURRENCY: &str = "receipt.show_currency";
    /// Decimal separator style: `"dot"`, `"comma"`, or `"none"`. Default `"dot"`.
    pub const RECEIPT_DECIMAL_SEP: &str = "receipt.decimal_separator";
    /// Show tax line on receipts. `"1"` or `"0"`. Default `"1"`.
    pub const RECEIPT_SHOW_TAX: &str = "receipt.show_tax";
    /// Receipt footer text. Empty string means no footer.
    pub const RECEIPT_FOOTER: &str = "receipt.footer";
    /// Paper width: `"standard"` (80 mm) or `"narrow"` (58 mm). Default `"standard"`.
    pub const RECEIPT_PAPER_WIDTH: &str = "receipt.paper_width";
    /// Show table number on cart and receipts. `"1"` or `"0"`. Default `"0"`.
    pub const RECEIPT_SHOW_TABLE_NUMBER: &str = "receipt.show_table_number";
    /// Top margin in mm. Default `"0"`.
    pub const RECEIPT_MARGIN_TOP: &str = "receipt.margin_top";
    /// Bottom margin in mm. Default `"0"`.
    pub const RECEIPT_MARGIN_BOTTOM: &str = "receipt.margin_bottom";
    /// Left margin in mm. Default `"0"`.
    pub const RECEIPT_MARGIN_LEFT: &str = "receipt.margin_left";
    /// Right margin in mm. Default `"0"`.
    pub const RECEIPT_MARGIN_RIGHT: &str = "receipt.margin_right";

    // ── Printer settings ──────────────────────────────────────────
    /// Printer connection type: `"auto"`, `"usb"`, `"serial"`, `"network"`.
    pub const PRINTER_CONNECTION: &str = "printer.connection";
    /// Printer device path (e.g. `/dev/usb/lp0` or `COM1`).
    pub const PRINTER_DEVICE_PATH: &str = "printer.device_path";
    /// Printer paper size: `"58"`, `"80"`, `"a4"`, `"letter"`, `"9.5x11"`, `"9.5x5.5"`.
    pub const PRINTER_PAPER_SIZE: &str = "printer.paper_size";

    // ── Scanner settings ──────────────────────────────────────────
    /// Selected scanner device ID.
    pub const SCANNER_DEVICE_ID: &str = "scanner.device_id";
    /// Scanner input mode: `"auto"`, `"keyboard"`, `"serial"`.
    pub const SCANNER_INPUT_MODE: &str = "scanner.input_mode";

    // ── Cloud Sync settings ──────────────────────────────────────
    /// Remote server URL for syncing offline data.
    pub const SYNC_SERVER_URL: &str = "sync_server_url";
    /// API key for server authentication.
    pub const SYNC_API_KEY: &str = "sync_api_key";
    /// Whether cloud sync is enabled. `"1"` or `"0"`. Default `"0"`.
    pub const SYNC_ENABLED: &str = "sync_enabled";

    // ── PostgreSQL Sync settings ─────────────────────────────────
    /// Whether PostgreSQL sync is enabled. `"1"` or `"0"`. Default `"0"`.
    pub const PG_SYNC_ENABLED: &str = "pg_sync.enabled";
    /// PostgreSQL hostname or IP address.
    pub const PG_SYNC_HOST: &str = "pg_sync.host";
    /// PostgreSQL port (default `"5432"`).
    pub const PG_SYNC_PORT: &str = "pg_sync.port";
    /// PostgreSQL database name.
    pub const PG_SYNC_DBNAME: &str = "pg_sync.dbname";
    /// PostgreSQL user name.
    pub const PG_SYNC_USER: &str = "pg_sync.user";
    /// PostgreSQL password.
    pub const PG_SYNC_PASSWORD: &str = "pg_sync.password";

    // ── Redis Cache settings ─────────────────────────────────────
    /// Redis server URL. Default `"redis://localhost:6379"`.
    pub const REDIS_URL: &str = "redis.url";
    /// Redis cache TTL in seconds. Default `300`.
    pub const REDIS_CACHE_TTL: &str = "redis.cache_ttl";

    // ── Brand / White-label settings ────────────────────────────
    /// Primary brand colour (hex). Default `"#10b981"`.
    pub const BRAND_PRIMARY_COLOUR: &str = "brand.primary_colour";
    /// Filesystem path to the store logo image.
    pub const BRAND_LOGO_PATH: &str = "brand.logo_path";
    /// Store display name for the header. Default `""`.
    pub const BRAND_STORE_NAME: &str = "brand.store_name";

    // ── Credit settings ─────────────────────────────────────────
    /// Whether credit payment is enabled. `"1"` or `"0"`. Default `"0"`.
    pub const CREDIT_ENABLED: &str = "credit.enabled";
    /// Credit reminder interval in hours. Default `"24"`.
    pub const CREDIT_REMINDER_INTERVAL: &str = "credit.reminder_interval";
    /// Maximum credit limit in minor units. Default `"0"` (no limit).
    pub const CREDIT_MAX_LIMIT: &str = "credit.max_limit";

    // ── Exchange Rate Auto-Sync settings ─────────────────────────
    /// Whether exchange rate auto-sync is enabled. `"1"` or `"0"`. Default `"0"`.
    pub const RATE_SYNC_ENABLED: &str = "rate_sync.enabled";
    /// API key for the exchange rate provider.
    pub const RATE_SYNC_API_KEY: &str = "rate_sync.api_key";
    /// Sync interval in minutes. Default `"360"` (6 hours).
    pub const RATE_SYNC_INTERVAL: &str = "rate_sync.interval";
    /// Base currency for exchange rates. Default `"USD"`.
    pub const RATE_SYNC_BASE_CURRENCY: &str = "rate_sync.base_currency";
}

impl Settings {
    /// Get the store display name.
    pub fn get_store_name(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::STORE_NAME)
    }

    /// Set the store display name.
    pub fn set_store_name(conn: &Connection, name: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::STORE_NAME, name)
    }

    /// Get the store address (printed on receipts).
    pub fn get_store_address(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::STORE_ADDRESS)
    }

    /// Set the store address.
    pub fn set_store_address(conn: &Connection, addr: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::STORE_ADDRESS, addr)
    }

    /// Get the store tax / VAT registration number.
    pub fn get_store_tax_id(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::STORE_TAX_ID)
    }

    /// Set the store tax / VAT registration number.
    pub fn set_store_tax_id(conn: &Connection, id: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::STORE_TAX_ID, id)
    }

    /// Get the default currency code (ISO-4217).
    pub fn get_default_currency(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::DEFAULT_CURRENCY)
    }

    /// Set the default currency code.
    pub fn set_default_currency(conn: &Connection, code: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::DEFAULT_CURRENCY, code)
    }

    /// Get the store branch name.
    pub fn get_store_branch(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::STORE_BRANCH)
    }

    /// Set the store branch name.
    pub fn set_store_branch(conn: &Connection, branch: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::STORE_BRANCH, branch)
    }

    /// Get the store logo (base64-encoded PNG).
    pub fn get_store_logo(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::STORE_LOGO)
    }

    /// Set the store logo (base64-encoded PNG).
    pub fn set_store_logo(conn: &Connection, logo: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::STORE_LOGO, logo)
    }

    // ── Receipt display settings ───────────────────────────────────

    /// Whether to show the currency symbol prefix on receipt amounts.
    pub fn get_receipt_show_currency(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_SHOW_CURRENCY)?
            .as_deref()
            .unwrap_or("0")
            == "1")
    }

    /// Set whether to show the currency symbol prefix.
    pub fn set_receipt_show_currency(conn: &Connection, on: bool) -> Result<(), PlatformError> {
        Self::set(
            conn,
            keys::RECEIPT_SHOW_CURRENCY,
            if on { "1" } else { "0" },
        )
    }

    /// Decimal separator style: `"dot"`, `"comma"`, or `"none"`.
    pub fn get_receipt_decimal_separator(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_DECIMAL_SEP)?.unwrap_or_else(|| "dot".into()))
    }

    /// Set the decimal separator style.
    pub fn set_receipt_decimal_separator(
        conn: &Connection,
        val: &str,
    ) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_DECIMAL_SEP, val)
    }

    /// Whether to show the tax line on receipts.
    pub fn get_receipt_show_tax(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_SHOW_TAX)?
            .as_deref()
            .unwrap_or("1")
            == "1")
    }

    /// Set whether to show the tax line.
    pub fn set_receipt_show_tax(conn: &Connection, on: bool) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_SHOW_TAX, if on { "1" } else { "0" })
    }

    /// Get the receipt footer text (empty = no footer).
    pub fn get_receipt_footer(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_FOOTER)?.unwrap_or_default())
    }

    /// Set the receipt footer text.
    pub fn set_receipt_footer(conn: &Connection, text: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_FOOTER, text)
    }

    /// Paper width: `"standard"` (80 mm) or `"narrow"` (58 mm).
    pub fn get_receipt_paper_width(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_PAPER_WIDTH)?.unwrap_or_else(|| "standard".into()))
    }

    /// Set the paper width.
    pub fn set_receipt_paper_width(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_PAPER_WIDTH, val)
    }

    /// Whether to show the table number on cart and receipts.
    pub fn get_receipt_show_table_number(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_SHOW_TABLE_NUMBER)?
            .as_deref()
            .unwrap_or("0")
            == "1")
    }

    /// Set whether to show the table number.
    pub fn set_receipt_show_table_number(conn: &Connection, on: bool) -> Result<(), PlatformError> {
        Self::set(
            conn,
            keys::RECEIPT_SHOW_TABLE_NUMBER,
            if on { "1" } else { "0" },
        )
    }

    /// Margin from paper top edge in mm. Default `"0"`.
    pub fn get_receipt_margin_top(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_MARGIN_TOP)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set the top margin.
    pub fn set_receipt_margin_top(conn: &Connection, mm: i64) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_MARGIN_TOP, &mm.to_string())
    }

    /// Margin from paper bottom edge in mm. Default `"0"`.
    pub fn get_receipt_margin_bottom(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_MARGIN_BOTTOM)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set the bottom margin.
    pub fn set_receipt_margin_bottom(conn: &Connection, mm: i64) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_MARGIN_BOTTOM, &mm.to_string())
    }

    /// Margin from paper left edge in mm. Default `"0"`.
    pub fn get_receipt_margin_left(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_MARGIN_LEFT)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set the left margin.
    pub fn set_receipt_margin_left(conn: &Connection, mm: i64) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_MARGIN_LEFT, &mm.to_string())
    }

    /// Margin from paper right edge in mm. Default `"0"`.
    pub fn get_receipt_margin_right(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::RECEIPT_MARGIN_RIGHT)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set the right margin.
    pub fn set_receipt_margin_right(conn: &Connection, mm: i64) -> Result<(), PlatformError> {
        Self::set(conn, keys::RECEIPT_MARGIN_RIGHT, &mm.to_string())
    }

    // ── Printer settings ─────────────────────────────────────────

    /// Printer connection type.
    pub fn get_printer_connection(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::PRINTER_CONNECTION)?.unwrap_or_else(|| "auto".into()))
    }

    /// Set printer connection type.
    pub fn set_printer_connection(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PRINTER_CONNECTION, val)
    }

    /// Printer device path.
    pub fn get_printer_device_path(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::PRINTER_DEVICE_PATH)?.unwrap_or_default())
    }

    /// Set printer device path.
    pub fn set_printer_device_path(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PRINTER_DEVICE_PATH, val)
    }

    /// Printer paper size.
    pub fn get_printer_paper_size(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::PRINTER_PAPER_SIZE)?.unwrap_or_else(|| "80".into()))
    }

    /// Set printer paper size.
    pub fn set_printer_paper_size(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PRINTER_PAPER_SIZE, val)
    }

    // ── Scanner settings ─────────────────────────────────────────

    /// Selected scanner device ID.
    pub fn get_scanner_device_id(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::SCANNER_DEVICE_ID)?.unwrap_or_default())
    }

    /// Set scanner device ID.
    pub fn set_scanner_device_id(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::SCANNER_DEVICE_ID, val)
    }

    /// Scanner input mode.
    pub fn get_scanner_input_mode(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::SCANNER_INPUT_MODE)?.unwrap_or_else(|| "auto".into()))
    }

    /// Set scanner input mode.
    pub fn set_scanner_input_mode(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::SCANNER_INPUT_MODE, val)
    }

    // ── Credit settings ──────────────────────────────────────────

    /// Check if credit payment is enabled.
    pub fn is_credit_enabled(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::CREDIT_ENABLED)?.as_deref() == Some("1"))
    }

    /// Enable or disable credit payment.
    pub fn set_credit_enabled(conn: &Connection, enabled: bool) -> Result<(), PlatformError> {
        Self::set(conn, keys::CREDIT_ENABLED, if enabled { "1" } else { "0" })
    }

    /// Get credit reminder interval in hours.
    pub fn get_credit_reminder_interval(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::CREDIT_REMINDER_INTERVAL)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(24))
    }

    /// Set credit reminder interval in hours.
    pub fn set_credit_reminder_interval(
        conn: &Connection,
        hours: i64,
    ) -> Result<(), PlatformError> {
        Self::set(conn, keys::CREDIT_REMINDER_INTERVAL, &hours.to_string())
    }

    /// Get maximum credit limit in minor units (0 = no limit).
    pub fn get_credit_max_limit(conn: &Connection) -> Result<i64, PlatformError> {
        Ok(Self::get(conn, keys::CREDIT_MAX_LIMIT)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    /// Set maximum credit limit in minor units.
    pub fn set_credit_max_limit(conn: &Connection, limit: i64) -> Result<(), PlatformError> {
        Self::set(conn, keys::CREDIT_MAX_LIMIT, &limit.to_string())
    }

    // ── Cloud Sync ───────────────────────────────────────────────

    /// Get the configured sync server URL.
    pub fn get_sync_server_url(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::SYNC_SERVER_URL)
    }

    /// Set the sync server URL.
    pub fn set_sync_server_url(conn: &Connection, url: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::SYNC_SERVER_URL, url)
    }

    /// Get the sync API key.
    pub fn get_sync_api_key(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::SYNC_API_KEY)
    }

    /// Set the sync API key.
    pub fn set_sync_api_key(conn: &Connection, key: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::SYNC_API_KEY, key)
    }

    /// Check if sync is enabled.
    pub fn is_sync_enabled(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::SYNC_ENABLED)?.as_deref() == Some("1"))
    }

    /// Enable or disable sync.
    pub fn set_sync_enabled(conn: &Connection, enabled: bool) -> Result<(), PlatformError> {
        Self::set(conn, keys::SYNC_ENABLED, if enabled { "1" } else { "0" })
    }

    // ── PostgreSQL Sync ─────────────────────────────────────────

    /// Check if PostgreSQL sync is enabled.
    pub fn is_pg_sync_enabled(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::PG_SYNC_ENABLED)?.as_deref() == Some("1"))
    }

    /// Enable or disable PostgreSQL sync.
    pub fn set_pg_sync_enabled(conn: &Connection, enabled: bool) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_ENABLED, if enabled { "1" } else { "0" })
    }

    /// Get the PostgreSQL host.
    pub fn get_pg_sync_host(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::PG_SYNC_HOST)
    }

    /// Set the PostgreSQL host.
    pub fn set_pg_sync_host(conn: &Connection, host: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_HOST, host)
    }

    /// Get the PostgreSQL port.
    pub fn get_pg_sync_port(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::PG_SYNC_PORT)
    }

    /// Set the PostgreSQL port.
    pub fn set_pg_sync_port(conn: &Connection, port: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_PORT, port)
    }

    /// Get the PostgreSQL database name.
    pub fn get_pg_sync_dbname(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::PG_SYNC_DBNAME)
    }

    /// Set the PostgreSQL database name.
    pub fn set_pg_sync_dbname(conn: &Connection, dbname: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_DBNAME, dbname)
    }

    /// Get the PostgreSQL user.
    pub fn get_pg_sync_user(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::PG_SYNC_USER)
    }

    /// Set the PostgreSQL user.
    pub fn set_pg_sync_user(conn: &Connection, user: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_USER, user)
    }

    /// Get the PostgreSQL password.
    pub fn get_pg_sync_password(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::PG_SYNC_PASSWORD)
    }

    /// Set the PostgreSQL password.
    pub fn set_pg_sync_password(conn: &Connection, password: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::PG_SYNC_PASSWORD, password)
    }

    // ── Redis Cache ────────────────────────────────────────────────

    /// Get the Redis server URL.
    pub fn get_redis_url(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::REDIS_URL)?.unwrap_or_else(|| "redis://localhost:6379".into()))
    }

    /// Set the Redis server URL.
    pub fn set_redis_url(conn: &Connection, url: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::REDIS_URL, url)
    }

    /// Get the Redis cache TTL in seconds.
    pub fn get_redis_cache_ttl(conn: &Connection) -> Result<u64, PlatformError> {
        let val = Self::get(conn, keys::REDIS_CACHE_TTL)?;
        Ok(val.as_deref().unwrap_or("300").parse().unwrap_or(300))
    }

    /// Set the Redis cache TTL in seconds.
    pub fn set_redis_cache_ttl(conn: &Connection, ttl: u64) -> Result<(), PlatformError> {
        Self::set(conn, keys::REDIS_CACHE_TTL, &ttl.to_string())
    }

    // ── Brand / White-label ─────────────────────────────────────

    /// Get the primary brand colour (hex). Defaults to `"#10b981"`.
    pub fn get_brand_primary_colour(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::BRAND_PRIMARY_COLOUR)?.unwrap_or_else(|| "#10b981".into()))
    }

    /// Set the primary brand colour.
    pub fn set_brand_primary_colour(conn: &Connection, colour: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::BRAND_PRIMARY_COLOUR, colour)
    }

    /// Get the filesystem path to the store logo.
    pub fn get_brand_logo_path(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::BRAND_LOGO_PATH)
    }

    /// Set the filesystem path to the store logo.
    pub fn set_brand_logo_path(conn: &Connection, path: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::BRAND_LOGO_PATH, path)
    }

    /// Get the brand store display name.
    pub fn get_brand_store_name(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::BRAND_STORE_NAME)?.unwrap_or_default())
    }

    /// Set the brand store display name.
    pub fn set_brand_store_name(conn: &Connection, name: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::BRAND_STORE_NAME, name)
    }

    // ── Exchange Rate Auto-Sync ────────────────────────────────────

    /// Check if exchange rate auto-sync is enabled.
    pub fn is_rate_sync_enabled(conn: &Connection) -> Result<bool, PlatformError> {
        Ok(Self::get(conn, keys::RATE_SYNC_ENABLED)?.as_deref() == Some("1"))
    }

    /// Enable or disable exchange rate auto-sync.
    pub fn set_rate_sync_enabled(conn: &Connection, enabled: bool) -> Result<(), PlatformError> {
        Self::set(
            conn,
            keys::RATE_SYNC_ENABLED,
            if enabled { "1" } else { "0" },
        )
    }

    /// Get the exchange rate API key.
    pub fn get_rate_sync_api_key(conn: &Connection) -> Result<Option<String>, PlatformError> {
        Self::get(conn, keys::RATE_SYNC_API_KEY)
    }

    /// Set the exchange rate API key.
    pub fn set_rate_sync_api_key(conn: &Connection, key: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::RATE_SYNC_API_KEY, key)
    }

    /// Get the exchange rate sync interval in minutes.
    pub fn get_rate_sync_interval(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::RATE_SYNC_INTERVAL)?.unwrap_or_else(|| "360".into()))
    }

    /// Set the exchange rate sync interval in minutes.
    pub fn set_rate_sync_interval(conn: &Connection, val: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::RATE_SYNC_INTERVAL, val)
    }

    /// Get the base currency for exchange rate sync.
    pub fn get_rate_sync_base_currency(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::RATE_SYNC_BASE_CURRENCY)?.unwrap_or_else(|| "USD".into()))
    }

    /// Set the base currency for exchange rate sync.
    pub fn set_rate_sync_base_currency(
        conn: &Connection,
        currency: &str,
    ) -> Result<(), PlatformError> {
        Self::set(conn, keys::RATE_SYNC_BASE_CURRENCY, currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
        )
        .unwrap();
        conn
    }

    // ── Raw get / set / remove ───────────────────────────────────

    #[test]
    fn get_returns_none_for_missing_key() {
        let conn = fresh();
        assert_eq!(Settings::get(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn set_and_get_roundtrip() {
        let conn = fresh();
        Settings::set(&conn, "test.key", "hello").unwrap();
        assert_eq!(
            Settings::get(&conn, "test.key").unwrap(),
            Some("hello".into())
        );
    }

    #[test]
    fn set_overwrites_existing() {
        let conn = fresh();
        Settings::set(&conn, "k", "v1").unwrap();
        Settings::set(&conn, "k", "v2").unwrap();
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("v2".into()));
    }

    #[test]
    fn remove_existing_key() {
        let conn = fresh();
        Settings::set(&conn, "k", "v").unwrap();
        assert!(Settings::remove(&conn, "k").unwrap());
        assert_eq!(Settings::get(&conn, "k").unwrap(), None);
    }

    #[test]
    fn remove_missing_key_returns_false() {
        let conn = fresh();
        assert!(!Settings::remove(&conn, "nope").unwrap());
    }

    // ── Batch ────────────────────────────────────────────────────

    #[test]
    fn set_batch_inserts_multiple() {
        let conn = fresh();
        let rows: Vec<(String, String)> = vec![("a".into(), "1".into()), ("b".into(), "2".into())];
        Settings::set_batch(&conn, &rows).unwrap();
        assert_eq!(Settings::get(&conn, "a").unwrap(), Some("1".into()));
        assert_eq!(Settings::get(&conn, "b").unwrap(), Some("2".into()));
    }

    #[test]
    fn load_all_returns_all_rows() {
        let conn = fresh();
        Settings::set(&conn, "a", "1").unwrap();
        Settings::set(&conn, "b", "2").unwrap();
        let all = Settings::load_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&("a".into(), "1".into())));
        assert!(all.contains(&("b".into(), "2".into())));
    }

    #[test]
    fn set_batch_empty_vec() {
        let conn = fresh();
        Settings::set_batch(&conn, &[]).unwrap();
        assert_eq!(Settings::load_all(&conn).unwrap().len(), 0);
    }

    #[test]
    fn set_batch_overwrites_existing_keys() {
        let conn = fresh();
        Settings::set(&conn, "k", "old").unwrap();
        let rows = vec![("k".into(), "new".into())];
        Settings::set_batch(&conn, &rows).unwrap();
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("new".into()));
    }

    // ── Typed store config ───────────────────────────────────────

    #[test]
    fn store_name_default_is_none() {
        let conn = fresh();
        assert_eq!(Settings::get_store_name(&conn).unwrap(), None);
    }

    #[test]
    fn set_and_get_store_name() {
        let conn = fresh();
        Settings::set_store_name(&conn, "Acme POS").unwrap();
        assert_eq!(
            Settings::get_store_name(&conn).unwrap(),
            Some("Acme POS".into())
        );
    }

    #[test]
    fn set_and_get_default_currency() {
        let conn = fresh();
        Settings::set_default_currency(&conn, "EUR").unwrap();
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("EUR".into())
        );
    }

    #[test]
    fn remove_idempotent() {
        let conn = fresh();
        Settings::set(&conn, "k", "v").unwrap();
        assert!(Settings::remove(&conn, "k").unwrap());
        assert!(!Settings::remove(&conn, "k").unwrap());
    }

    #[test]
    fn load_all_empty_table() {
        let conn = fresh();
        let all = Settings::load_all(&conn).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn set_store_address_roundtrip() {
        let conn = fresh();
        Settings::set_store_address(&conn, "123 Main St").unwrap();
        assert_eq!(
            Settings::get_store_address(&conn).unwrap(),
            Some("123 Main St".into())
        );
    }

    #[test]
    fn set_store_tax_id_roundtrip() {
        let conn = fresh();
        Settings::set_store_tax_id(&conn, "TAX-12345").unwrap();
        assert_eq!(
            Settings::get_store_tax_id(&conn).unwrap(),
            Some("TAX-12345".into())
        );
    }

    #[test]
    fn get_store_address_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_store_address(&conn).unwrap(), None);
    }

    #[test]
    fn get_store_tax_id_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_store_tax_id(&conn).unwrap(), None);
    }

    #[test]
    fn get_default_currency_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_default_currency(&conn).unwrap(), None);
    }

    // ── Receipt settings ─────────────────────────────────────────

    #[test]
    fn receipt_show_currency_default_false() {
        let conn = fresh();
        assert!(!Settings::get_receipt_show_currency(&conn).unwrap());
    }

    #[test]
    fn set_receipt_show_currency_true() {
        let conn = fresh();
        Settings::set_receipt_show_currency(&conn, true).unwrap();
        assert!(Settings::get_receipt_show_currency(&conn).unwrap());
    }

    #[test]
    fn set_receipt_show_currency_false() {
        let conn = fresh();
        Settings::set_receipt_show_currency(&conn, true).unwrap();
        Settings::set_receipt_show_currency(&conn, false).unwrap();
        assert!(!Settings::get_receipt_show_currency(&conn).unwrap());
    }

    #[test]
    fn receipt_decimal_separator_default_dot() {
        let conn = fresh();
        assert_eq!(
            Settings::get_receipt_decimal_separator(&conn).unwrap(),
            "dot"
        );
    }

    #[test]
    fn set_receipt_decimal_separator_comma() {
        let conn = fresh();
        Settings::set_receipt_decimal_separator(&conn, "comma").unwrap();
        assert_eq!(
            Settings::get_receipt_decimal_separator(&conn).unwrap(),
            "comma"
        );
    }

    #[test]
    fn receipt_show_tax_default_true() {
        let conn = fresh();
        assert!(Settings::get_receipt_show_tax(&conn).unwrap());
    }

    #[test]
    fn set_receipt_show_tax_false() {
        let conn = fresh();
        Settings::set_receipt_show_tax(&conn, false).unwrap();
        assert!(!Settings::get_receipt_show_tax(&conn).unwrap());
    }

    #[test]
    fn receipt_footer_default_empty() {
        let conn = fresh();
        assert_eq!(Settings::get_receipt_footer(&conn).unwrap(), "");
    }

    #[test]
    fn set_receipt_footer() {
        let conn = fresh();
        Settings::set_receipt_footer(&conn, "Thank you!").unwrap();
        assert_eq!(Settings::get_receipt_footer(&conn).unwrap(), "Thank you!");
    }

    #[test]
    fn receipt_paper_width_default_standard() {
        let conn = fresh();
        assert_eq!(
            Settings::get_receipt_paper_width(&conn).unwrap(),
            "standard"
        );
    }

    #[test]
    fn set_receipt_paper_width_narrow() {
        let conn = fresh();
        Settings::set_receipt_paper_width(&conn, "narrow").unwrap();
        assert_eq!(Settings::get_receipt_paper_width(&conn).unwrap(), "narrow");
    }

    #[test]
    fn receipt_show_table_number_default_false() {
        let conn = fresh();
        assert!(!Settings::get_receipt_show_table_number(&conn).unwrap());
    }

    #[test]
    fn set_receipt_show_table_number_true() {
        let conn = fresh();
        Settings::set_receipt_show_table_number(&conn, true).unwrap();
        assert!(Settings::get_receipt_show_table_number(&conn).unwrap());
    }

    #[test]
    fn set_receipt_show_table_number_false() {
        let conn = fresh();
        Settings::set_receipt_show_table_number(&conn, true).unwrap();
        Settings::set_receipt_show_table_number(&conn, false).unwrap();
        assert!(!Settings::get_receipt_show_table_number(&conn).unwrap());
    }

    // ── Sync settings ───────────────────────────────────────────-

    #[test]
    fn sync_server_url_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_sync_server_url(&conn).unwrap(), None);
    }

    #[test]
    fn set_sync_server_url() {
        let conn = fresh();
        Settings::set_sync_server_url(&conn, "https://sync.example.com").unwrap();
        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap(),
            Some("https://sync.example.com".into())
        );
    }

    #[test]
    fn sync_api_key_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_sync_api_key(&conn).unwrap(), None);
    }

    #[test]
    fn set_sync_api_key() {
        let conn = fresh();
        Settings::set_sync_api_key(&conn, "sk-test123").unwrap();
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap(),
            Some("sk-test123".into())
        );
    }

    #[test]
    fn sync_enabled_default_false() {
        let conn = fresh();
        assert!(!Settings::is_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn set_sync_enabled() {
        let conn = fresh();
        Settings::set_sync_enabled(&conn, true).unwrap();
        assert!(Settings::is_sync_enabled(&conn).unwrap());
        Settings::set_sync_enabled(&conn, false).unwrap();
        assert!(!Settings::is_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn get_unknown_key_returns_none() {
        let conn = fresh();
        assert_eq!(
            Settings::get(&conn, "completely.unknown.key").unwrap(),
            None
        );
    }

    #[test]
    fn set_empty_value() {
        let conn = fresh();
        Settings::set(&conn, "empty.key", "").unwrap();
        assert_eq!(Settings::get(&conn, "empty.key").unwrap(), Some("".into()));
    }

    // ── Exchange Rate Auto-Sync ─────────────────────────────────

    #[test]
    fn rate_sync_enabled_default_false() {
        let conn = fresh();
        assert!(!Settings::is_rate_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn set_rate_sync_enabled() {
        let conn = fresh();
        Settings::set_rate_sync_enabled(&conn, true).unwrap();
        assert!(Settings::is_rate_sync_enabled(&conn).unwrap());
        Settings::set_rate_sync_enabled(&conn, false).unwrap();
        assert!(!Settings::is_rate_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn rate_sync_api_key_default_none() {
        let conn = fresh();
        assert_eq!(Settings::get_rate_sync_api_key(&conn).unwrap(), None);
    }

    #[test]
    fn set_rate_sync_api_key() {
        let conn = fresh();
        Settings::set_rate_sync_api_key(&conn, "my-key").unwrap();
        assert_eq!(
            Settings::get_rate_sync_api_key(&conn).unwrap(),
            Some("my-key".into())
        );
    }

    #[test]
    fn rate_sync_interval_default_360() {
        let conn = fresh();
        assert_eq!(Settings::get_rate_sync_interval(&conn).unwrap(), "360");
    }

    #[test]
    fn set_rate_sync_interval() {
        let conn = fresh();
        Settings::set_rate_sync_interval(&conn, "60").unwrap();
        assert_eq!(Settings::get_rate_sync_interval(&conn).unwrap(), "60");
    }

    #[test]
    fn rate_sync_base_currency_default_usd() {
        let conn = fresh();
        assert_eq!(Settings::get_rate_sync_base_currency(&conn).unwrap(), "USD");
    }

    #[test]
    fn set_rate_sync_base_currency() {
        let conn = fresh();
        Settings::set_rate_sync_base_currency(&conn, "EUR").unwrap();
        assert_eq!(Settings::get_rate_sync_base_currency(&conn).unwrap(), "EUR");
    }

    #[test]
    fn overwrite_with_same_value() {
        let conn = fresh();
        Settings::set(&conn, "k", "same").unwrap();
        Settings::set(&conn, "k", "same").unwrap();
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("same".into()));
    }

    #[test]
    fn keys_constants_are_non_empty() {
        assert!(!keys::STORE_NAME.is_empty());
        assert!(!keys::STORE_ADDRESS.is_empty());
        assert!(!keys::STORE_TAX_ID.is_empty());
        assert!(!keys::DEFAULT_CURRENCY.is_empty());
        assert!(!keys::STORE_PRESET.is_empty());
        assert!(!keys::SETUP_COMPLETE.is_empty());
        assert!(!keys::SHOW_SETUP_WIZARD.is_empty());
        assert!(!keys::RECEIPT_SHOW_CURRENCY.is_empty());
        assert!(!keys::RECEIPT_DECIMAL_SEP.is_empty());
        assert!(!keys::RECEIPT_SHOW_TAX.is_empty());
        assert!(!keys::RECEIPT_FOOTER.is_empty());
        assert!(!keys::RECEIPT_PAPER_WIDTH.is_empty());
        assert!(!keys::RECEIPT_SHOW_TABLE_NUMBER.is_empty());
        assert!(!keys::SYNC_SERVER_URL.is_empty());
        assert!(!keys::SYNC_API_KEY.is_empty());
        assert!(!keys::SYNC_ENABLED.is_empty());
        assert!(!keys::PG_SYNC_ENABLED.is_empty());
        assert!(!keys::PG_SYNC_HOST.is_empty());
        assert!(!keys::PG_SYNC_PORT.is_empty());
        assert!(!keys::PG_SYNC_DBNAME.is_empty());
        assert!(!keys::PG_SYNC_USER.is_empty());
        assert!(!keys::PG_SYNC_PASSWORD.is_empty());
        assert!(!keys::RATE_SYNC_ENABLED.is_empty());
        assert!(!keys::RATE_SYNC_API_KEY.is_empty());
        assert!(!keys::RATE_SYNC_INTERVAL.is_empty());
        assert!(!keys::RATE_SYNC_BASE_CURRENCY.is_empty());
        assert!(!keys::BRAND_PRIMARY_COLOUR.is_empty());
        assert!(!keys::BRAND_LOGO_PATH.is_empty());
        assert!(!keys::BRAND_STORE_NAME.is_empty());
    }
}
