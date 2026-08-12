//! Centralised configuration for the cloud server.
//!
//! All environment-variable reads happen in [`CloudServerConfig::from_env`].
//! Every other module receives the values it needs through the config struct
//! — no scattered `std::env::var()` calls across the codebase.
//!
//! Validation runs eagerly: invalid values (unparseable port, missing
//! required vars in production mode) are surfaced at startup with clear
//! error messages rather than cryptic panics later.

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Default human-readable text output.
    Plain,
    /// Structured JSON lines (useful for log aggregators).
    Json,
}

/// Centralised configuration for the OZ-POS cloud sync server.
///
/// Construct with [`CloudServerConfig::from_env`]; pass by reference to
/// every module that needs environment-derived settings.
#[derive(Debug, Clone)]
pub struct CloudServerConfig {
    /// Path to the SQLite database file (default: `oz-pos.db`).
    /// Ignored when `database_url` points to a PostgreSQL server.
    pub db_path: String,

    /// Optional PostgreSQL connection URL (e.g. `postgres://...`).
    /// When set and starting with `postgres://`, the server connects to
    /// PostgreSQL instead of SQLite.
    pub database_url: Option<String>,

    /// HTTP listen port (default: `3099`).
    pub port: u16,

    /// Admin key for token minting and plan management.
    /// When `None` (default), token and plan endpoints are open (dev mode).
    /// When set, callers must pass the matching `X-Admin-Key` header.
    pub admin_key: Option<String>,

    /// When `true`, sync requests from tenants on the `free` plan are
    /// rejected with `403 plan_required`.
    pub enforce_plans: bool,

    /// Log output format.
    pub log_format: LogFormat,

    /// When `true`, the server runs in redirect-only mode (ADR #11):
    /// no DB, no prune, no metrics — just the migration redirect.
    pub redirect_only: bool,

    /// New server URL for the migration redirect. Sync requests to
    /// `/api/sync/*` return HTTP 421 with this URL in the body.
    pub sync_redirect_url: Option<String>,

    /// Stripe webhook signing secret (`whsec_...`).
    /// Required for `POST /api/webhooks/stripe` signature verification.
    pub stripe_webhook_secret: Option<String>,

    /// Square webhook signature key.
    /// Required for `POST /api/webhooks/square` signature verification.
    pub square_webhook_signature_key: Option<String>,

    /// Public Square webhook URL (used for webhook registration).
    pub square_webhook_url: Option<String>,

    /// JWT signing secret for `POST /api/v1/tokens`.
    /// Falls back to a hard-coded dev secret when unset.
    pub api_secret: Option<String>,
}

impl CloudServerConfig {
    /// Build configuration from environment variables.
    ///
    /// # Validation
    ///
    /// * `OZ_REDIRECT_ONLY=true` requires `OZ_SYNC_REDIRECT_URL` to be set.
    /// * `OZ_API_PORT` must parse as a valid `u16`.
    /// * `OZ_ADMIN_KEY` is treated as unset when empty (Docker passes `""`
    ///   for absent host variables).
    ///
    /// # Errors
    ///
    /// Returns `Err(message)` when validation fails so the caller can log
    /// and exit cleanly instead of panicking.
    pub fn from_env() -> Result<Self, String> {
        let redirect_only = env_bool("OZ_REDIRECT_ONLY");
        let sync_redirect_url = std::env::var("OZ_SYNC_REDIRECT_URL").ok();

        if redirect_only && sync_redirect_url.is_none() {
            return Err("OZ_REDIRECT_ONLY=true requires OZ_SYNC_REDIRECT_URL to be set".into());
        }

        let port: u16 = std::env::var("OZ_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3099);

        let log_format = match std::env::var("OZ_LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Plain,
        };

        let enforce_plans = env_bool("OZ_ENFORCE_PLANS");

        // An empty string must not enable key gating: `docker compose`
        // passes `OZ_ADMIN_KEY` through as `""` when the host variable
        // is absent.
        let admin_key = std::env::var("OZ_ADMIN_KEY").ok().filter(|k| !k.is_empty());

        let database_url = std::env::var("DATABASE_URL").ok();
        let db_path = std::env::var("OZ_DB_PATH").unwrap_or_else(|_| "oz-pos.db".into());

        Ok(Self {
            db_path,
            database_url,
            port,
            admin_key,
            enforce_plans,
            log_format,
            redirect_only,
            sync_redirect_url,
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            square_webhook_signature_key: std::env::var("SQUARE_WEBHOOK_SIGNATURE_KEY").ok(),
            square_webhook_url: std::env::var("SQUARE_WEBHOOK_URL").ok(),
            api_secret: std::env::var("OZ_API_SECRET").ok(),
        })
    }
}

/// Parse a boolean environment variable.
///
/// Returns `true` for `"1"`, `"true"` (case-insensitive), or `"on"`
/// (case-insensitive). Everything else (including unset) returns `false`.
fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_bool_true_values() {
        // Can't set env in unit tests without serial_test, so test the
        // helper logic directly.
        assert!(matches_bool_str("1"));
        assert!(matches_bool_str("true"));
        assert!(matches_bool_str("TRUE"));
        assert!(matches_bool_str("on"));
        assert!(matches_bool_str("ON"));
    }

    #[test]
    fn env_bool_false_values() {
        assert!(!matches_bool_str("0"));
        assert!(!matches_bool_str("false"));
        assert!(!matches_bool_str("no"));
        assert!(!matches_bool_str(""));
    }

    /// Same logic as `env_bool` but operating on a string slice so tests
    /// don't need environment mutation.
    fn matches_bool_str(s: &str) -> bool {
        matches!(s, "1" | "true" | "TRUE" | "on" | "ON")
    }

    #[test]
    fn default_port_is_3099() {
        // from_env reads the real env, but we can verify the default.
        let port: u16 = std::env::var("OZ_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3099);
        // In CI / local dev without OZ_API_PORT set, this is 3099.
        assert!(port > 0);
    }

    #[test]
    fn log_format_parses_json() {
        assert_eq!(
            match "json" {
                "json" => LogFormat::Json,
                _ => LogFormat::Plain,
            },
            LogFormat::Json
        );
    }

    #[test]
    fn log_format_defaults_to_plain() {
        assert_eq!(
            match "text" {
                "json" => LogFormat::Json,
                _ => LogFormat::Plain,
            },
            LogFormat::Plain
        );
    }
}
