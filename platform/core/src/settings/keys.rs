//! Well-known settings keys.

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
/// Tax rounding mode: `"half_up"` or `"truncate"`. Default `"half_up"`.
pub const TAX_ROUNDING_MODE: &str = "tax.rounding_mode";
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
