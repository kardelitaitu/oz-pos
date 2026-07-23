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

    // ── Delta ledger methods ──────────────────────────────────────

    /// Standalone delta writer — uses a savepoint for nesting safety.
    ///
    /// Computes `version = MAX(version) + 1` for the `(key, terminal_id)`
    /// pair and inserts a new row. Uses a savepoint so the SELECT MAX +
    /// INSERT are atomic and the call is safe from within an existing
    /// transaction (no nested `BEGIN` error).
    pub fn write_delta(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        // Use a savepoint so this works both standalone and when called
        // from within an existing transaction (e.g. set_tracked).
        // `execute_batch` is used instead of `conn.savepoint()` because
        // the latter requires `&mut Connection`.
        let sp = format!("_oz_delta_{}", std::process::id());
        conn.execute_batch(&format!("SAVEPOINT {sp}"))?;
        let result = (|| -> Result<(), PlatformError> {
            let version: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(version), 0) + 1
                     FROM setting_updated
                     WHERE key = ?1 AND terminal_id = ?2",
                    params![key, terminal_id],
                    |row| row.get(0),
                )
                .unwrap_or(1);

            conn.execute(
                "INSERT INTO setting_updated (key, value, terminal_id, version)
                 VALUES (?1, ?2, ?3, ?4)",
                params![key, value, terminal_id, version],
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute_batch(&format!("RELEASE {sp}"))?;
                Ok(())
            }
            Err(e) => {
                tracing::warn!(key, terminal_id, error = %e, "delta write failed, rolling back savepoint");
                let _ = conn.execute_batch(&format!("ROLLBACK TO {sp}"));
                Err(e)
            }
        }
    }

    /// Get the latest version number for a `(key, terminal_id)` pair.
    ///
    /// Returns `None` if no deltas exist for that pair. Used by shared
    /// settings cards to detect concurrent edits (compare known version
    /// against the stored version before writing).
    pub fn get_version(
        conn: &Connection,
        key: &str,
        terminal_id: &str,
    ) -> Result<Option<i64>, PlatformError> {
        let mut stmt = conn.prepare(
            "SELECT version FROM setting_updated
             WHERE key = ?1 AND terminal_id = ?2
             ORDER BY version DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![key, terminal_id], |row| row.get::<_, i64>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Set a value AND write a delta record — both in a single transaction.
    ///
    /// This is the recommended method for Tauri command handlers that have
    /// access to a terminal ID. Calls `Settings::set()` for the value and
    /// `Settings::write_delta()` for the versioned audit trail, both
    /// within a single transaction. Since `write_delta()` uses a nested
    /// savepoint, the delta write failure does not roll back the `set()`.
    ///
    /// Delta write failures are logged but do not roll back the `set()` —
    /// delta loss is non-fatal; the sync layer can reconstruct from the
    /// settings table.
    pub fn set_tracked(
        conn: &Connection,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        Self::set(conn, key, value)?;
        // Inline delta write within the existing transaction to avoid
        // nested BEGIN (SQLite does not support nested transactions).
        if let Err(e) = Self::write_delta_on_tx(&tx, key, value, terminal_id) {
            tracing::warn!(key, terminal_id, error = %e, "delta write failed (non-fatal)");
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch write with delta tracking for every row.
    ///
    /// Like `set_batch()`, but also writes a delta row for each key/value
    /// pair. All operations run in a single transaction.
    pub fn set_batch_tracked(
        conn: &Connection,
        rows: &[(String, String)],
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let tx = conn.unchecked_transaction()?;
        for (key, value) in rows {
            Self::set(conn, key, value)?;
            if let Err(e) = Self::write_delta_on_tx(&tx, key, value, terminal_id) {
                tracing::warn!(key, terminal_id, error = %e, "delta batch write failed (non-fatal)");
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Write a delta row using an existing transaction (no nested BEGIN).
    fn write_delta_on_tx(
        tx: &rusqlite::Transaction,
        key: &str,
        value: &str,
        terminal_id: &str,
    ) -> Result<(), PlatformError> {
        let version: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1
                 FROM setting_updated
                 WHERE key = ?1 AND terminal_id = ?2",
                params![key, terminal_id],
                |row| row.get(0),
            )
            .unwrap_or(1);
        tx.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version)
             VALUES (?1, ?2, ?3, ?4)",
            params![key, value, terminal_id, version],
        )?;
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
    pub const DEFAULT_CURRENCY: &str = "currency.default";
    /// Old store-specific key — used as fallback for backward compatibility.
    pub(crate) const OLD_DEFAULT_CURRENCY: &str = "store.default_currency";
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

    // ── Global Currency settings ─────────────────────────────────
    /// Currency display format: `"symbol"` (use symbol like $) or `"code"` (use code like USD). Default `"symbol"`.
    pub const CURRENCY_FORMAT: &str = "currency.format";
    /// Currency symbol position: `"prefix"` ($10) or `"suffix"` (10$). Default `"prefix"`.
    pub const CURRENCY_SYMBOL_POSITION: &str = "currency.symbol_position";
    /// Decimal separator: `"dot"` (1.50) or `"comma"` (1,50). Default `"dot"`.
    pub const CURRENCY_DECIMAL_SEPARATOR: &str = "currency.decimal_separator";
    /// Thousands separator: `"comma"`, `"dot"`, `"space"`, or `"none"`. Default `"comma"`.
    pub const CURRENCY_THOUSANDS_SEPARATOR: &str = "currency.thousands_separator";

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

    // ── LAN server settings (C-4) ────────────────────────────
    /// Bind address for the LAN event forwarder.
    /// Default `"127.0.0.1"` (loopback only). Set to `"0.0.0.0"`
    /// to allow external KDS tablet connections — requires
    /// `lan_server.psk` to be non-empty.
    pub const LAN_SERVER_BIND: &str = "lan_server.bind";
    /// Pre-shared key for the LAN event forwarder.
    /// Required when `lan_server.bind` is `"0.0.0.0"`.
    /// Peers must send `{"op":"hello","psk":"<value>"}` as
    /// their first message or the connection is dropped.
    pub const LAN_SERVER_PSK: &str = "lan_server.psk";
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
    ///
    /// Prefers the new global `currency.default` key and falls back to the
    /// old `store.default_currency` key for databases that haven't been
    /// migrated yet.
    pub fn get_default_currency(conn: &Connection) -> Result<Option<String>, PlatformError> {
        if let Some(val) = Self::get(conn, keys::DEFAULT_CURRENCY)? {
            return Ok(Some(val));
        }
        Self::get(conn, keys::OLD_DEFAULT_CURRENCY)
    }

    /// Set the default currency code.
    ///
    /// Writes to the new global `currency.default` key and cleans up the
    /// old `store.default_currency` key.
    pub fn set_default_currency(conn: &Connection, code: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::DEFAULT_CURRENCY, code)?;
        let _ = Self::remove(conn, keys::OLD_DEFAULT_CURRENCY);
        Ok(())
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

    // ── Global Currency display settings ───────────────────────────

    /// Get the currency display format: `"symbol"` or `"code"`.
    pub fn get_currency_format(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::CURRENCY_FORMAT)?.unwrap_or_else(|| "symbol".into()))
    }

    /// Set the currency display format.
    pub fn set_currency_format(conn: &Connection, fmt: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::CURRENCY_FORMAT, fmt)
    }

    /// Get the currency symbol position: `"prefix"` or `"suffix"`.
    pub fn get_currency_symbol_position(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::CURRENCY_SYMBOL_POSITION)?.unwrap_or_else(|| "prefix".into()))
    }

    /// Set the currency symbol position.
    pub fn set_currency_symbol_position(conn: &Connection, pos: &str) -> Result<(), PlatformError> {
        Self::set(conn, keys::CURRENCY_SYMBOL_POSITION, pos)
    }

    /// Get the decimal separator: `"dot"` or `"comma"`.
    pub fn get_currency_decimal_separator(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::CURRENCY_DECIMAL_SEPARATOR)?.unwrap_or_else(|| "dot".into()))
    }

    /// Set the decimal separator.
    pub fn set_currency_decimal_separator(
        conn: &Connection,
        sep: &str,
    ) -> Result<(), PlatformError> {
        Self::set(conn, keys::CURRENCY_DECIMAL_SEPARATOR, sep)
    }

    /// Get the thousands separator: `"comma"`, `"dot"`, `"space"`, or `"none"`.
    pub fn get_currency_thousands_separator(conn: &Connection) -> Result<String, PlatformError> {
        Ok(Self::get(conn, keys::CURRENCY_THOUSANDS_SEPARATOR)?.unwrap_or_else(|| "comma".into()))
    }

    /// Set the thousands separator.
    pub fn set_currency_thousands_separator(
        conn: &Connection,
        sep: &str,
    ) -> Result<(), PlatformError> {
        Self::set(conn, keys::CURRENCY_THOUSANDS_SEPARATOR, sep)
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

    // ── Global Currency settings tests ──────────────────────────

    #[test]
    fn currency_format_default_is_symbol() {
        let conn = fresh();
        assert_eq!(Settings::get_currency_format(&conn).unwrap(), "symbol");
    }

    #[test]
    fn set_and_get_currency_format() {
        let conn = fresh();
        Settings::set_currency_format(&conn, "code").unwrap();
        assert_eq!(Settings::get_currency_format(&conn).unwrap(), "code");
    }

    #[test]
    fn currency_symbol_position_default_is_prefix() {
        let conn = fresh();
        assert_eq!(
            Settings::get_currency_symbol_position(&conn).unwrap(),
            "prefix"
        );
    }

    #[test]
    fn set_and_get_currency_symbol_position() {
        let conn = fresh();
        Settings::set_currency_symbol_position(&conn, "suffix").unwrap();
        assert_eq!(
            Settings::get_currency_symbol_position(&conn).unwrap(),
            "suffix"
        );
    }

    #[test]
    fn currency_decimal_separator_default_is_dot() {
        let conn = fresh();
        assert_eq!(
            Settings::get_currency_decimal_separator(&conn).unwrap(),
            "dot"
        );
    }

    #[test]
    fn set_and_get_currency_decimal_separator() {
        let conn = fresh();
        Settings::set_currency_decimal_separator(&conn, "comma").unwrap();
        assert_eq!(
            Settings::get_currency_decimal_separator(&conn).unwrap(),
            "comma"
        );
    }

    #[test]
    fn currency_thousands_separator_default_is_comma() {
        let conn = fresh();
        assert_eq!(
            Settings::get_currency_thousands_separator(&conn).unwrap(),
            "comma"
        );
    }

    #[test]
    fn set_and_get_currency_thousands_separator() {
        let conn = fresh();
        Settings::set_currency_thousands_separator(&conn, "space").unwrap();
        assert_eq!(
            Settings::get_currency_thousands_separator(&conn).unwrap(),
            "space"
        );
    }

    #[test]
    fn get_default_currency_falls_back_to_old_key() {
        let conn = fresh();
        // Write old key, new key absent
        Settings::set(&conn, keys::OLD_DEFAULT_CURRENCY, "JPY").unwrap();
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("JPY".into())
        );
    }

    #[test]
    fn get_default_currency_prefers_new_key() {
        let conn = fresh();
        Settings::set(&conn, keys::DEFAULT_CURRENCY, "EUR").unwrap();
        Settings::set(&conn, keys::OLD_DEFAULT_CURRENCY, "JPY").unwrap();
        // New key takes precedence
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("EUR".into())
        );
    }

    #[test]
    fn set_default_currency_cleans_up_old_key() {
        let conn = fresh();
        Settings::set(&conn, keys::OLD_DEFAULT_CURRENCY, "GBP").unwrap();
        Settings::set_default_currency(&conn, "USD").unwrap();
        assert_eq!(
            Settings::get_default_currency(&conn).unwrap(),
            Some("USD".into())
        );
        // Old key should be gone
        assert_eq!(
            Settings::get(&conn, keys::OLD_DEFAULT_CURRENCY).unwrap(),
            None
        );
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

    // ── Delta ledger tests ──────────────────────────────────────

    /// Helper that creates the `setting_updated` table needed by delta tests.
    fn fresh_with_delta() -> Connection {
        let conn = fresh();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS setting_updated (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                key         TEXT    NOT NULL,
                value       TEXT    NOT NULL,
                terminal_id TEXT    NOT NULL DEFAULT 'unknown',
                version     INTEGER NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );
             CREATE INDEX IF NOT EXISTS idx_setting_updated_key_version
                 ON setting_updated(key, version DESC);
             CREATE INDEX IF NOT EXISTS idx_setting_updated_terminal
                 ON setting_updated(terminal_id, created_at DESC);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn write_delta_creates_row_with_version_one() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "receipt.footer", "Thanks!", "term-a").unwrap();
        let v = Settings::get_version(&conn, "receipt.footer", "term-a").unwrap();
        assert_eq!(v, Some(1));
    }

    #[test]
    fn write_delta_increments_version_per_key_terminal() {
        let conn = fresh_with_delta();
        // Write twice to same (key, terminal) — versions 1, 2.
        Settings::write_delta(&conn, "receipt.footer", "v1", "term-a").unwrap();
        Settings::write_delta(&conn, "receipt.footer", "v2", "term-a").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-a").unwrap(),
            Some(2)
        );
    }

    #[test]
    fn write_delta_different_keys_track_separate_versions() {
        let conn = fresh_with_delta();
        // Different keys should both start at version 1.
        Settings::write_delta(&conn, "store.name", "Shop", "term-a").unwrap();
        Settings::write_delta(&conn, "receipt.footer", "Bye", "term-a").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "store.name", "term-a").unwrap(),
            Some(1)
        );
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-a").unwrap(),
            Some(1)
        );
    }

    #[test]
    fn write_delta_different_terminals_track_separate_versions() {
        let conn = fresh_with_delta();
        // Same key, different terminals — independent version counters.
        Settings::write_delta(&conn, "receipt.footer", "v-a", "term-a").unwrap();
        Settings::write_delta(&conn, "receipt.footer", "v-b", "term-b").unwrap();
        Settings::write_delta(&conn, "receipt.footer", "v-a2", "term-a").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-a").unwrap(),
            Some(2)
        );
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-b").unwrap(),
            Some(1)
        );
    }

    #[test]
    fn get_version_returns_none_for_missing() {
        let conn = fresh_with_delta();
        assert_eq!(
            Settings::get_version(&conn, "nonexistent", "term-a").unwrap(),
            None
        );
    }

    #[test]
    fn set_tracked_writes_setting_and_delta_atomically() {
        let conn = fresh_with_delta();
        Settings::set_tracked(&conn, "receipt.footer", "Hello!", "term-a").unwrap();
        // Setting should be stored.
        assert_eq!(
            Settings::get(&conn, "receipt.footer").unwrap(),
            Some("Hello!".into())
        );
        // Delta should be version 1.
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-a").unwrap(),
            Some(1)
        );
    }

    #[test]
    fn set_tracked_overwrites_and_increments_version() {
        let conn = fresh_with_delta();
        Settings::set_tracked(&conn, "k", "v1", "term-a").unwrap();
        Settings::set_tracked(&conn, "k", "v2", "term-a").unwrap();
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("v2".into()));
        assert_eq!(
            Settings::get_version(&conn, "k", "term-a").unwrap(),
            Some(2)
        );
    }

    #[test]
    fn set_batch_tracked_writes_all_deltas() {
        let conn = fresh_with_delta();
        let rows: Vec<(String, String)> = vec![
            ("receipt.footer".into(), "Batch1".into()),
            ("store.name".into(), "Batch2".into()),
        ];
        Settings::set_batch_tracked(&conn, &rows, "term-a").unwrap();

        assert_eq!(
            Settings::get(&conn, "receipt.footer").unwrap(),
            Some("Batch1".into())
        );
        assert_eq!(
            Settings::get(&conn, "store.name").unwrap(),
            Some("Batch2".into())
        );
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-a").unwrap(),
            Some(1)
        );
        assert_eq!(
            Settings::get_version(&conn, "store.name", "term-a").unwrap(),
            Some(1)
        );
    }

    #[test]
    fn set_tracked_is_idempotent_same_value_same_version() {
        let conn = fresh_with_delta();
        Settings::set_tracked(&conn, "k", "same", "term-a").unwrap();
        Settings::set_tracked(&conn, "k", "same", "term-a").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "k", "term-a").unwrap(),
            Some(2)
        );
    }

    // ── Delta ledger resilience & edge cases ───────────────────

    /// Verify that `set_tracked` still persists the setting value even when
    /// the `setting_updated` table does not exist (delta write fails).
    /// The ADR specifies delta loss is non-fatal — the settings table write
    /// must succeed regardless.
    #[test]
    fn set_tracked_survives_missing_delta_table() {
        // Fresh DB WITHOUT the `setting_updated` table.
        let conn = fresh();
        let result = Settings::set_tracked(&conn, "receipt.footer", "NoDelta!", "term-a");
        // Should succeed — delta failure is caught and logged, not propagated.
        assert!(result.is_ok());
        // The setting value must still be persisted.
        assert_eq!(
            Settings::get(&conn, "receipt.footer").unwrap(),
            Some("NoDelta!".into())
        );
        // The delta write must have failed — version lookup will fail
        // because the `setting_updated` table does not exist.
        assert!(Settings::get_version(&conn, "receipt.footer", "term-a").is_err());
    }

    /// Empty terminal_id is valid — `set_tracked` should not panic or error.
    #[test]
    fn set_tracked_empty_terminal_id() {
        let conn = fresh_with_delta();
        Settings::set_tracked(&conn, "store.name", "Shop", "").unwrap();
        assert_eq!(
            Settings::get(&conn, "store.name").unwrap(),
            Some("Shop".into())
        );
        assert_eq!(
            Settings::get_version(&conn, "store.name", "").unwrap(),
            Some(1)
        );
    }

    /// `set_batch_tracked` with an empty slice is a no-op — no panic, no error.
    #[test]
    fn set_batch_tracked_empty_batch() {
        let conn = fresh_with_delta();
        let rows: Vec<(String, String)> = vec![];
        Settings::set_batch_tracked(&conn, &rows, "term-a").unwrap();
        assert_eq!(Settings::load_all(&conn).unwrap().len(), 0);
    }

    /// Delta writes handle special characters in keys and values
    /// (Unicode, quotes, backslashes).
    #[test]
    fn write_delta_special_characters() {
        let conn = fresh_with_delta();
        let key = "store.name";
        let value = "Caf\u{00e9} \"OZ\" — 100% natural";
        Settings::write_delta(&conn, key, value, "term-\u{2603}").unwrap();
        // The version should be 1 — special chars don't affect SQL execution.
        assert_eq!(
            Settings::get_version(&conn, key, "term-\u{2603}").unwrap(),
            Some(1)
        );
    }

    /// Prove that `write_delta()` works when called from within an outer
    /// transaction. rusqlite converts the nested BEGIN to a SAVEPOINT,
    /// so the delta write is atomic within the outer transaction.
    #[test]
    fn write_delta_works_inside_outer_transaction() {
        let conn = fresh_with_delta();
        // Start an outer transaction.
        let outer_tx = conn.unchecked_transaction().unwrap();
        // Call `write_delta()` — it internally creates another
        // `unchecked_transaction()` which becomes a SAVEPOINT.
        Settings::write_delta(&conn, "store.name", "nested-val", "term-nest").unwrap();
        // Commit the outer transaction.
        outer_tx.commit().unwrap();
        // Delta must be visible.
        assert_eq!(
            Settings::get_version(&conn, "store.name", "term-nest").unwrap(),
            Some(1)
        );
    }

    /// Prove that a `write_delta()` inside an outer transaction that gets
    /// rolled back also rolls back the delta — confirming the SAVEPOINT
    /// is nested within the outer transaction.
    #[test]
    fn write_delta_rolls_back_with_outer_transaction() {
        let conn = fresh_with_delta();
        // Start outer tx, write delta, then rollback.
        {
            let _outer_tx = conn.unchecked_transaction().unwrap();
            Settings::write_delta(&conn, "receipt.footer", "rollback-me", "term-rb").unwrap();
            // _outer_tx dropped → ROLLBACK
        }
        // Delta must NOT be visible after rollback.
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-rb").unwrap(),
            None
        );
    }

    /// Different keys from the same terminal track independent version counters.
    #[test]
    fn set_tracked_multiple_keys_independent_versions() {
        let conn = fresh_with_delta();
        // Key "a" gets versions 1, 2.
        Settings::set_tracked(&conn, "a", "v1", "term-x").unwrap();
        Settings::set_tracked(&conn, "a", "v2", "term-x").unwrap();
        // Key "b" should start at version 1, NOT 3.
        Settings::set_tracked(&conn, "b", "v1", "term-x").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "a", "term-x").unwrap(),
            Some(2)
        );
        assert_eq!(
            Settings::get_version(&conn, "b", "term-x").unwrap(),
            Some(1)
        );
    }

    /// `set_tracked` with many sequential writes verifies monotonically
    /// increasing version numbers.
    #[test]
    fn set_tracked_monotonic_version_sequence() {
        let conn = fresh_with_delta();
        for i in 1..=5 {
            Settings::set_tracked(&conn, "receipt.footer", &format!("v{i}"), "term-mono").unwrap();
            assert_eq!(
                Settings::get_version(&conn, "receipt.footer", "term-mono").unwrap(),
                Some(i)
            );
        }
    }

    // ── Transaction atomicity proof & concurrency safety ──────

    /// Prove that `conn.execute()` inside an `unchecked_transaction()`
    /// is truly transactional: if the transaction is dropped (rolled back),
    /// the `set()` must NOT persist. This verifies `set_tracked` is atomic.
    #[test]
    fn conn_execute_is_part_of_active_transaction() {
        let conn = fresh();
        // Open a transaction, call set(), then drop without commit.
        {
            let _tx = conn.unchecked_transaction().unwrap();
            Settings::set(&conn, "tx.key", "should-rollback").unwrap();
            // _tx dropped here → ROLLBACK
        }
        // Value must NOT be visible after rollback.
        assert_eq!(Settings::get(&conn, "tx.key").unwrap(), None);
    }

    /// Verify that `Settings::set()` called via `conn.execute()` inside
    /// an `unchecked_transaction()` commits atomically when `tx.commit()`
    /// is called. This is the happy path of `set_tracked`.
    #[test]
    fn conn_execute_commits_with_transaction() {
        let conn = fresh();
        let tx = conn.unchecked_transaction().unwrap();
        Settings::set(&conn, "tx.key", "should-commit").unwrap();
        tx.commit().unwrap();
        assert_eq!(
            Settings::get(&conn, "tx.key").unwrap(),
            Some("should-commit".into())
        );
    }

    /// Concurrent `set_tracked` calls on different keys must not interfere.
    /// Each key gets its own version counter regardless of thread scheduling.
    /// Concurrent `set_tracked` on independent keys from separate threads.
    /// Uses a temp file DB so each thread can open its own `Connection`.
    /// In-memory DBs can't be shared across threads because
    /// `rusqlite::Connection` contains a `RefCell` (not `Sync`).
    #[test]
    fn set_tracked_concurrent_threads_independent_keys() {
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create the DB with schema first.
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                );
                CREATE TABLE IF NOT EXISTS setting_updated (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    key         TEXT    NOT NULL,
                    value       TEXT    NOT NULL,
                    terminal_id TEXT    NOT NULL DEFAULT 'unknown',
                    version     INTEGER NOT NULL,
                    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                );
                CREATE INDEX IF NOT EXISTS idx_setting_updated_key_version
                    ON setting_updated(key, version DESC);
                CREATE INDEX IF NOT EXISTS idx_setting_updated_terminal
                    ON setting_updated(terminal_id, created_at DESC);
                PRAGMA journal_mode=WAL;
            ",
            )
            .unwrap();
        }

        let p1 = db_path.clone();
        let p2 = db_path.clone();

        let t1 = thread::spawn(move || {
            let conn = Connection::open(&p1).unwrap();
            for i in 1..=10 {
                Settings::set_tracked(&conn, "thread.a", &format!("v{i}"), "t1").unwrap();
            }
        });
        let t2 = thread::spawn(move || {
            let conn = Connection::open(&p2).unwrap();
            for i in 1..=10 {
                Settings::set_tracked(&conn, "thread.b", &format!("v{i}"), "t2").unwrap();
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        // Each key should have exactly 10 versions.
        assert_eq!(
            Settings::get_version(&conn, "thread.a", "t1").unwrap(),
            Some(10)
        );
        assert_eq!(
            Settings::get_version(&conn, "thread.b", "t2").unwrap(),
            Some(10)
        );
    }

    /// `set_batch_tracked` with one failing delta must still write all
    /// settings and the succeeding deltas. Partial failure = non-fatal.
    #[test]
    fn set_batch_tracked_partial_delta_failure() {
        // DB without `setting_updated` — all delta writes will fail.
        let conn = fresh();
        let rows: Vec<(String, String)> = vec![
            ("key.1".into(), "val1".into()),
            ("key.2".into(), "val2".into()),
            ("key.3".into(), "val3".into()),
        ];
        // Should succeed despite all delta writes failing.
        Settings::set_batch_tracked(&conn, &rows, "term-a").unwrap();
        // All settings must be persisted.
        assert_eq!(Settings::get(&conn, "key.1").unwrap(), Some("val1".into()));
        assert_eq!(Settings::get(&conn, "key.2").unwrap(), Some("val2".into()));
        assert_eq!(Settings::get(&conn, "key.3").unwrap(), Some("val3".into()));
    }

    /// Verify that `set_tracked`'s error does NOT roll back the `set()`.
    /// The ADR specifies delta loss is non-fatal — the setting persists
    /// even if the delta write failed. This is the production behavior.
    ///
    /// (Already proven by `set_tracked_survives_missing_delta_table`;
    /// this test confirms the behavior holds for multiple keys.)
    #[test]
    fn set_tracked_delta_failure_does_not_rollback_set() {
        let conn = fresh();
        // 5 keys, all without delta table — all delta writes fail.
        for i in 0..5 {
            let key = format!("fail.{i}");
            let val = format!("val{i}");
            Settings::set_tracked(&conn, &key, &val, "term-fail").unwrap();
            assert_eq!(Settings::get(&conn, &key).unwrap(), Some(val));
        }
        assert_eq!(Settings::load_all(&conn).unwrap().len(), 5);
    }

    /// Prove that `get_version` is case-sensitive — SQLite uses BINARY
    /// collation by default. 'Key' and 'key' are different keys.
    #[test]
    fn get_version_case_sensitive_keys() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "KEY", "val1", "term-x").unwrap();
        Settings::write_delta(&conn, "key", "val2", "term-x").unwrap();
        // Uppercase KEY is independent from lowercase key.
        assert_eq!(
            Settings::get_version(&conn, "KEY", "term-x").unwrap(),
            Some(1)
        );
        assert_eq!(
            Settings::get_version(&conn, "key", "term-x").unwrap(),
            Some(1)
        );
    }

    /// `write_delta_on_tx` correctly picks the MAX version when hundreds
    /// of existing delta rows exist for the same (key, terminal_id) pair.
    #[test]
    fn write_delta_with_large_version_count() {
        let conn = fresh_with_delta();
        // Insert 100 deltas manually so MAX(version) = 100.
        let tx = conn.unchecked_transaction().unwrap();
        for v in 1..=100 {
            tx.execute(
                "INSERT INTO setting_updated (key, value, terminal_id, version) VALUES (?1, ?2, ?3, ?4)",
                params![&format!("key.v"), &format!("val{v}"), "term-bulk", v],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        // Now call write_delta — it should compute version 101.
        Settings::write_delta(&conn, "key.v", "latest", "term-bulk").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "key.v", "term-bulk").unwrap(),
            Some(101)
        );
    }

    /// `set_batch_tracked` with duplicate keys in a single batch.
    /// The second write must overwrite the first, and the delta version
    /// must increment per key.
    #[test]
    fn set_batch_tracked_duplicate_keys_in_batch() {
        let conn = fresh_with_delta();
        let rows: Vec<(String, String)> = vec![
            ("receipt.footer".into(), "v1".into()),
            ("receipt.footer".into(), "v2".into()),
        ];
        Settings::set_batch_tracked(&conn, &rows, "term-dup").unwrap();

        // Final value should be v2 (last write wins).
        assert_eq!(
            Settings::get(&conn, "receipt.footer").unwrap(),
            Some("v2".into())
        );
        // Delta should reflect both writes: version 2.
        assert_eq!(
            Settings::get_version(&conn, "receipt.footer", "term-dup").unwrap(),
            Some(2)
        );
    }

    /// Verify the `get_version` query uses the
    /// `idx_setting_updated_key_version` index for efficient lookups
    /// rather than a full table scan.
    #[test]
    fn get_version_uses_covering_index() {
        let conn = fresh_with_delta();
        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN SELECT version FROM setting_updated
                 WHERE key = 'k' AND terminal_id = 't'
                 ORDER BY version DESC LIMIT 1",
            )
            .unwrap();
        let plans: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        let plan_text = plans.join(" ");
        assert!(
            plan_text.contains("idx_setting_updated_key_version")
                || plan_text.contains("COVERING INDEX"),
            "get_version query should use idx_setting_updated_key_version, got: {plan_text}"
        );
    }

    /// `get_version` returns Only the MAX version, even when multiple
    /// version rows exist for the same (key, terminal_id) pair.
    #[test]
    fn get_version_returns_max_for_multiple_versions() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "k", "v1", "term-x").unwrap();
        Settings::write_delta(&conn, "k", "v2", "term-x").unwrap();
        Settings::write_delta(&conn, "k", "v3", "term-x").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "k", "term-x").unwrap(),
            Some(3)
        );
        // Verify we have exactly 3 delta rows, not just the latest.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM setting_updated WHERE key = 'k' AND terminal_id = 'term-x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3, "all three delta rows should be preserved");
    }

    /// `set_tracked` with the same key called from two different terminals
    /// must track completely independent version counters (LWW per-terminal).
    #[test]
    fn set_tracked_independent_terminal_version_counters() {
        let conn = fresh_with_delta();
        // Terminal A writes key "k" twice.
        Settings::set_tracked(&conn, "k", "a1", "term-a").unwrap();
        Settings::set_tracked(&conn, "k", "a2", "term-a").unwrap();
        // Terminal B writes key "k" once.
        Settings::set_tracked(&conn, "k", "b1", "term-b").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "k", "term-a").unwrap(),
            Some(2)
        );
        assert_eq!(
            Settings::get_version(&conn, "k", "term-b").unwrap(),
            Some(1)
        );
        // The settings value should be the last write across all terminals.
        assert_eq!(Settings::get(&conn, "k").unwrap(), Some("b1".into()));
    }

    /// Verify `write_delta` handles version numbers near `i64::MAX`
    /// without overflow or panic. SQLite integer arithmetic wraps on
    /// overflow, but the delta write itself should succeed.
    #[test]
    fn write_delta_near_version_overflow() {
        let conn = fresh_with_delta();
        // Insert a row at i64::MAX to set up the overflow scenario.
        conn.execute(
            "INSERT INTO setting_updated (key, value, terminal_id, version)
             VALUES ('overflow.key', 'max', 'term-of', ?1)",
            params![i64::MAX],
        )
        .unwrap();
        // `write_delta` should still succeed — the version computation
        // wraps around, but the method should not panic.
        let result = Settings::write_delta(&conn, "overflow.key", "post-max", "term-of");
        assert!(result.is_ok(), "write_delta should not panic near i64::MAX");
        // The version stored is implementation-defined (wrap or saturate),
        // but we must have a version row.
        assert!(Settings::get_version(&conn, "overflow.key", "term-of").is_ok());
    }

    /// `write_delta` with a very long key (near SQLite's default
    /// limit) should not panic or truncate silently.
    #[test]
    fn write_delta_long_key() {
        let conn = fresh_with_delta();
        let long_key = "k".repeat(500); // 500 chars, well under limit
        Settings::write_delta(&conn, &long_key, "long-val", "term-lg").unwrap();
        let v = Settings::get_version(&conn, &long_key, "term-lg").unwrap();
        assert_eq!(v, Some(1));
    }

    /// `write_delta` with a very long value (multi-kilobyte JSON blob)
    /// should not panic or truncate.
    #[test]
    fn write_delta_long_value() {
        let conn = fresh_with_delta();
        let long_value = "x".repeat(10_000); // 10 KB value
        Settings::write_delta(&conn, "bulk.key", &long_value, "term-bulk").unwrap();
        let v = Settings::get_version(&conn, "bulk.key", "term-bulk").unwrap();
        assert_eq!(v, Some(1));
    }

    #[test]
    fn keys_constants_are_non_empty() {
        assert!(!keys::STORE_NAME.is_empty());
        assert!(!keys::STORE_ADDRESS.is_empty());
        assert!(!keys::STORE_TAX_ID.is_empty());
        assert!(!keys::DEFAULT_CURRENCY.is_empty());
        assert!(!keys::OLD_DEFAULT_CURRENCY.is_empty());
        assert!(!keys::CURRENCY_FORMAT.is_empty());
        assert!(!keys::CURRENCY_SYMBOL_POSITION.is_empty());
        assert!(!keys::CURRENCY_DECIMAL_SEPARATOR.is_empty());
        assert!(!keys::CURRENCY_THOUSANDS_SEPARATOR.is_empty());
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
        assert!(!keys::LAN_SERVER_BIND.is_empty());
        assert!(!keys::LAN_SERVER_PSK.is_empty());
    }

    // ── ADR #22 resilience & correctness tests ───────────────────

    /// SQL-injection-like strings in keys/values should be handled
    /// safely by parameterized queries — no syntax errors, no data leaks.
    #[test]
    fn write_delta_sql_injection_resilience() {
        let conn = fresh_with_delta();
        let malicious = "'; DROP TABLE settings; --";
        // Key with injection attempt.
        Settings::write_delta(&conn, malicious, "safe-val", "term-sql").unwrap();
        assert_eq!(
            Settings::get_version(&conn, malicious, "term-sql").unwrap(),
            Some(1)
        );
        // Value with injection attempt.
        Settings::write_delta(&conn, "safe.key", malicious, "term-sql").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "safe.key", "term-sql").unwrap(),
            Some(1)
        );
        // The settings table should still exist (not dropped).
        assert!(Settings::load_all(&conn).is_ok());
    }

    /// `set_tracked` with SQL-injection-like inputs should also be safe.
    #[test]
    fn set_tracked_sql_injection_resilience() {
        let conn = fresh_with_delta();
        let malicious = "'; DROP TABLE setting_updated; --";
        Settings::set_tracked(&conn, malicious, malicious, "term-sql").unwrap();
        // Delta table should still exist.
        assert!(Settings::get_version(&conn, "safe.check", "term-sql").is_ok());
        // The malicious value should be stored literally.
        assert_eq!(
            Settings::get(&conn, malicious).unwrap(),
            Some(malicious.into())
        );
    }

    /// Empty string key is valid in SQLite — `write_delta` should
    /// handle it without panicking.
    #[test]
    fn write_delta_empty_string_key() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "", "empty-key-val", "term-empty").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "", "term-empty").unwrap(),
            Some(1)
        );
    }

    /// Empty string terminal_id should work — it's a valid default.
    #[test]
    fn write_delta_empty_terminal_id() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "k", "v", "").unwrap();
        assert_eq!(Settings::get_version(&conn, "k", "").unwrap(), Some(1));
    }

    /// Delta rows should have a non-null `created_at` timestamp
    /// populated by the DEFAULT clause.
    #[test]
    fn write_delta_created_at_is_populated() {
        let conn = fresh_with_delta();
        Settings::write_delta(&conn, "ts.key", "ts-val", "term-ts").unwrap();
        let ts: String = conn
            .query_row(
                "SELECT created_at FROM setting_updated WHERE key = 'ts.key' AND terminal_id = 'term-ts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!ts.is_empty(), "created_at should not be empty");
        // Should be an ISO 8601-like timestamp.
        assert!(ts.contains('T'), "timestamp should contain 'T': {ts}");
    }

    /// `write_delta` should not leave partial data when an error occurs
    /// inside the savepoint. We can't easily trigger a constraint violation
    /// on the INSERT (all columns accept any text), but we verify the
    /// ROLLBACK path exists and the function signature handles errors.
    ///
    /// Instead, verify that a successful write_delta followed by another
    /// write on the same key correctly increments (proving the savepoint
    /// lifecycle is clean — no stale savepoints).
    #[test]
    fn write_delta_savepoint_cleanup_allows_reuse() {
        let conn = fresh_with_delta();
        // First write — savepoint created, released.
        Settings::write_delta(&conn, "sp.k", "v1", "term-sp").unwrap();
        // Second write — new savepoint, must not collide with stale state.
        Settings::write_delta(&conn, "sp.k", "v2", "term-sp").unwrap();
        assert_eq!(
            Settings::get_version(&conn, "sp.k", "term-sp").unwrap(),
            Some(2)
        );
    }

    /// Single-quote (apostrophe) in key names is a common real-world
    /// edge case — e.g. store names like "Joe's Caf\u{00e9}".
    #[test]
    fn write_delta_single_quote_in_key() {
        let conn = fresh_with_delta();
        let key = "store.it's";
        Settings::write_delta(&conn, key, "val", "term-sq").unwrap();
        assert_eq!(
            Settings::get_version(&conn, key, "term-sq").unwrap(),
            Some(1)
        );
    }

    // ── ADR #22 error-path bug hunting ──────────────────────────

    /// `write_delta` should not panic when the `setting_updated` table
    /// does not exist. The ROLLBACK path must execute cleanly.
    /// This proves the savepoint error-recovery code path is reachable
    /// and works correctly.
    #[test]
    fn write_delta_rolls_back_cleanly_on_missing_table() {
        // DB without `setting_updated` — SELECT will fail.
        let conn = fresh();
        let result = Settings::write_delta(&conn, "k", "v", "term-rb");
        // write_delta should return an error, not panic.
        assert!(result.is_err(), "write_delta should error on missing table");
        // The settings table should be unaffected (no partial writes).
        assert!(Settings::load_all(&conn).is_ok());
        // Connection must still be usable after the savepoint rollback —
        // a subsequent write to the settings table should succeed.
        Settings::set(&conn, "recovery.key", "recovered").unwrap();
        assert_eq!(
            Settings::get(&conn, "recovery.key").unwrap(),
            Some("recovered".into())
        );
    }

    /// `set_batch_tracked` should persist settings even when delta
    /// writes fail for every row. Delta loss is non-fatal.
    /// This proves the batch-level error resilience.
    #[test]
    fn set_batch_tracked_survives_all_delta_failures() {
        // DB without `setting_updated` — all delta writes will fail.
        let conn = fresh();
        let rows: Vec<(String, String)> = vec![
            ("batch.a".into(), "va".into()),
            ("batch.b".into(), "vb".into()),
            ("batch.c".into(), "vc".into()),
        ];
        let result = Settings::set_batch_tracked(&conn, &rows, "term-batch");
        // Should succeed — delta failures are non-fatal.
        assert!(
            result.is_ok(),
            "set_batch_tracked should succeed despite delta failures"
        );
        // All three settings must be persisted.
        assert_eq!(Settings::get(&conn, "batch.a").unwrap(), Some("va".into()));
        assert_eq!(Settings::get(&conn, "batch.b").unwrap(), Some("vb".into()));
        assert_eq!(Settings::get(&conn, "batch.c").unwrap(), Some("vc".into()));
        // No deltas should have been written — the delta table doesn't exist.
        assert!(Settings::get_version(&conn, "batch.a", "term-batch").is_err());
        assert!(Settings::get_version(&conn, "batch.b", "term-batch").is_err());
        assert!(Settings::get_version(&conn, "batch.c", "term-batch").is_err());
    }

    /// `set_tracked` should return an error (not panic) when the
    /// `settings` table itself is missing — the outer transaction
    /// must roll back cleanly.
    #[test]
    fn set_tracked_errors_when_settings_table_missing() {
        let conn = Connection::open_in_memory().unwrap();
        // No tables at all — `set()` will fail.
        let result = Settings::set_tracked(&conn, "k", "v", "term");
        assert!(
            result.is_err(),
            "set_tracked should error when settings table missing"
        );
    }

    // ── ADR #22 completeness: timestamps, batch edges, stress ─────

    /// The `settings` table has an `updated_at` column with a DEFAULT
    /// that should be populated automatically on INSERT.
    #[test]
    fn settings_updated_at_is_populated_on_set() {
        let conn = fresh();
        Settings::set(&conn, "ts.key", "ts-val").unwrap();
        let ts: String = conn
            .query_row(
                "SELECT updated_at FROM settings WHERE key = 'ts.key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!ts.is_empty(), "updated_at should be populated");
        assert!(ts.contains('T'), "updated_at should be ISO 8601: {ts}");
    }

    /// `updated_at` should change when a setting value is overwritten,
    /// providing a per-key modification timestamp.
    #[test]
    fn settings_updated_at_changes_on_overwrite() {
        let conn = fresh();
        Settings::set(&conn, "ts.k", "v1").unwrap();
        let ts1: String = conn
            .query_row(
                "SELECT updated_at FROM settings WHERE key = 'ts.k'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // Small delay to ensure timestamp changes (10ms margin).
        std::thread::sleep(std::time::Duration::from_millis(10));
        Settings::set(&conn, "ts.k", "v2").unwrap();
        let ts2: String = conn
            .query_row(
                "SELECT updated_at FROM settings WHERE key = 'ts.k'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(ts1, ts2, "updated_at should change on overwrite");
    }

    /// `set_batch_tracked` with an empty terminal_id should work
    /// — empty string is a valid terminal identifier.
    #[test]
    fn set_batch_tracked_empty_terminal_id() {
        let conn = fresh_with_delta();
        let rows: Vec<(String, String)> =
            vec![("bt.k1".into(), "v1".into()), ("bt.k2".into(), "v2".into())];
        Settings::set_batch_tracked(&conn, &rows, "").unwrap();
        assert_eq!(Settings::get(&conn, "bt.k1").unwrap(), Some("v1".into()));
        assert_eq!(Settings::get(&conn, "bt.k2").unwrap(), Some("v2".into()));
        assert_eq!(Settings::get_version(&conn, "bt.k1", "").unwrap(), Some(1));
        assert_eq!(Settings::get_version(&conn, "bt.k2", "").unwrap(), Some(1));
    }

    /// Stress test: 1,000 consecutive `write_delta` calls on the same
    /// (key, terminal_id) pair. The version counter should reach 1,000
    /// without stalling, wrapping, or erroring.
    #[test]
    fn write_delta_1000_consecutive_calls() {
        let conn = fresh_with_delta();
        for i in 1..=1000 {
            Settings::write_delta(&conn, "stress.k", &format!("v{i}"), "term-stress").unwrap();
        }
        assert_eq!(
            Settings::get_version(&conn, "stress.k", "term-stress").unwrap(),
            Some(1000)
        );
    }
}
