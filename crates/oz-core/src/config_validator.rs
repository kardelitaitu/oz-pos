//! Startup configuration validator.
//!
//! Validates environment variables and configuration at application startup
//! with structured, operator-friendly error messages. Catches misconfigurations
//! before the application attempts to use them.
//!
//! # Usage
//!
//! ```no_run
//! use oz_core::config_validator::{validate_config, ConfigValidationError};
//!
//! match validate_config() {
//!     Ok(()) => tracing::info!("configuration validated"),
//!     Err(errors) => {
//!         for err in &errors {
//!             tracing::error!(%err, "configuration error");
//!         }
//!         std::process::exit(1);
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt;

/// A single configuration validation failure.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigValidationError {
    /// The environment variable or config key that failed.
    pub key: &'static str,
    /// Human-readable description of the problem.
    pub message: String,
    /// Suggested fix for the operator.
    pub fix: Option<String>,
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.message)?;
        if let Some(ref fix) = self.fix {
            write!(f, " ({fix})")?;
        }
        Ok(())
    }
}

/// Validate all known configuration variables from the live environment.
/// Returns a list of errors (empty list means all checks passed).
///
/// This function is designed to be called at startup — it collects *all*
/// failures rather than short-circuiting on the first one, so the operator
/// can fix everything in one pass.
pub fn validate_config() -> Result<(), Vec<ConfigValidationError>> {
    let vars: HashMap<String, String> = std::env::vars().collect();
    validate_config_inner(&vars)
}

/// Validate configuration from a caller-supplied map of key-value pairs.
/// Useful for testing without touching the real environment.
pub fn validate_config_with(
    vars: &HashMap<String, String>,
) -> Result<(), Vec<ConfigValidationError>> {
    validate_config_inner(vars)
}

/// Core validation logic — reads values from the given map instead of
/// the process environment. This keeps the logic testable without `unsafe`
/// env-var manipulation.
fn validate_config_inner(vars: &HashMap<String, String>) -> Result<(), Vec<ConfigValidationError>> {
    let mut errors = Vec::new();

    /// Helper: look up a key in the supplied map, returning `None` if absent.
    fn get<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
        vars.get(key).map(|s| s.as_str())
    }

    // ── OZ_API_PORT ──────────────────────────────────────────────
    if let Some(port_str) = get(vars, "OZ_API_PORT") {
        match port_str.parse::<u16>() {
            Ok(0) => errors.push(ConfigValidationError {
                key: "OZ_API_PORT",
                message: "port cannot be 0".into(),
                fix: Some("set OZ_API_PORT to a value between 1024 and 65535".into()),
            }),
            Ok(p) if p < 1024 => errors.push(ConfigValidationError {
                key: "OZ_API_PORT",
                message: "privileged port — requires root/admin".into(),
                fix: Some("set OZ_API_PORT to a value >= 1024".into()),
            }),
            Err(_) => errors.push(ConfigValidationError {
                key: "OZ_API_PORT",
                message: format!("'{port_str}' is not a valid port number"),
                fix: Some("set OZ_API_PORT to a value between 1024 and 65535".into()),
            }),
            _ => {} // valid port
        }
    }

    // ── DATABASE_URL ─────────────────────────────────────────────
    if let Some(db_url) = get(vars, "DATABASE_URL") {
        if db_url.is_empty() {
            errors.push(ConfigValidationError {
                key: "DATABASE_URL",
                message: "DATABASE_URL is set but empty".into(),
                fix: Some(
                    "set DATABASE_URL to a valid connection string, or unset it to use SQLite"
                        .into(),
                ),
            });
        } else if !db_url.starts_with("postgresql://") && !db_url.starts_with("postgres://") {
            errors.push(ConfigValidationError {
                key: "DATABASE_URL",
                message: format!(
                    "DATABASE_URL must start with 'postgresql://' or 'postgres://', got '{}'",
                    truncate_prefix(db_url, 40)
                ),
                fix: Some("set DATABASE_URL to a valid PostgreSQL connection string".into()),
            });
        }
    }

    // ── OZ_LICENSE_KEY / OZ_LICENSE_PRIVATE_KEY ──────────────────
    let license_key = get(vars, "OZ_LICENSE_PRIVATE_KEY").or_else(|| get(vars, "OZ_LICENSE_KEY"));

    if let Some(key) = license_key {
        if key.is_empty() {
            errors.push(ConfigValidationError {
                key: "OZ_LICENSE_PRIVATE_KEY",
                message: "license key is set but empty".into(),
                fix: Some("set OZ_LICENSE_PRIVATE_KEY to a PEM-encoded private key".into()),
            });
        } else if !key.contains("BEGIN") && !key.contains("PRIVATE KEY") {
            errors.push(ConfigValidationError {
                key: "OZ_LICENSE_PRIVATE_KEY",
                message: "does not appear to be a PEM-encoded private key".into(),
                fix: Some(
                    "ensure OZ_LICENSE_PRIVATE_KEY contains '-----BEGIN PRIVATE KEY-----'".into(),
                ),
            });
        }
    }

    // ── STRIPE_SECRET_KEY ────────────────────────────────────────
    if let Some(stripe_key) = get(vars, "STRIPE_SECRET_KEY") {
        if !stripe_key.is_empty() && !stripe_key.starts_with("sk_") {
            errors.push(ConfigValidationError {
                key: "STRIPE_SECRET_KEY",
                message: "should start with 'sk_' (Stripe secret keys are prefixed with 'sk_')"
                    .into(),
                fix: Some("check your Stripe dashboard for the correct secret key".into()),
            });
        }
    }

    // ── MIDTRANS_SERVER_KEY ──────────────────────────────────────
    if let Some(midtrans_key) = get(vars, "MIDTRANS_SERVER_KEY") {
        if midtrans_key.is_empty() {
            errors.push(ConfigValidationError {
                key: "MIDTRANS_SERVER_KEY",
                message: "Midtrans server key is set but empty".into(),
                fix: Some(
                    "set MIDTRANS_SERVER_KEY to your Midtrans server key from the dashboard".into(),
                ),
            });
        } else if !midtrans_key.starts_with("Mid-server-")
            && !midtrans_key.starts_with("SB-Mid-server-")
        {
            errors.push(ConfigValidationError {
                key: "MIDTRANS_SERVER_KEY",
                message:
                    "should start with 'Mid-server-' (production) or 'SB-Mid-server-' (sandbox)"
                        .into(),
                fix: Some(
                    "check your Midtrans dashboard → Settings → Access Keys for the correct server key"
                        .into(),
                ),
            });
        }
    }

    // ── OZ_SYNC_REDIRECT_URL requires OZ_REDIRECT_ONLY ──────────
    if get(vars, "OZ_SYNC_REDIRECT_URL").is_some() && get(vars, "OZ_REDIRECT_ONLY") != Some("true")
    {
        errors.push(ConfigValidationError {
            key: "OZ_SYNC_REDIRECT_URL",
            message: "OZ_SYNC_REDIRECT_URL is set but OZ_REDIRECT_ONLY is not 'true'".into(),
            fix: Some(
                "set OZ_REDIRECT_ONLY=true to run in redirect-only mode, or unset OZ_SYNC_REDIRECT_URL"
                    .into(),
            ),
        });
    }

    // ── REDIS_URL validity check ─────────────────────────────────
    if let Some(redis_url) = get(vars, "REDIS_URL") {
        if !redis_url.is_empty()
            && !redis_url.starts_with("redis://")
            && !redis_url.starts_with("rediss://")
        {
            errors.push(ConfigValidationError {
                key: "REDIS_URL",
                message: format!("should start with 'redis://' or 'rediss://', got '{redis_url}'"),
                fix: Some("set REDIS_URL to a valid Redis connection string".into()),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Truncate a string to `max_len` characters, keeping the beginning.
fn truncate_prefix(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal vars map for testing — no env var access needed.
    fn vars(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// An empty map simulates a clean environment with no config vars set.
    fn empty_vars() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn valid_port_accepted() {
        let v = vars(&[("OZ_API_PORT", "3099")]);
        assert!(validate_config_with(&v).is_ok());
    }

    #[test]
    fn port_zero_rejected() {
        let v = vars(&[("OZ_API_PORT", "0")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_API_PORT" && e.message.contains("0"))
        );
    }

    #[test]
    fn privileged_port_warns() {
        let v = vars(&[("OZ_API_PORT", "80")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_API_PORT" && e.message.contains("privileged"))
        );
    }

    #[test]
    fn non_numeric_port_rejected() {
        let v = vars(&[("OZ_API_PORT", "abc")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_API_PORT" && e.message.contains("valid port number"))
        );
    }

    #[test]
    fn empty_database_url_rejected() {
        let v = vars(&[("DATABASE_URL", "")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "DATABASE_URL" && e.message.contains("empty"))
        );
    }

    #[test]
    fn bad_database_url_prefix_rejected() {
        let v = vars(&[("DATABASE_URL", "mysql://localhost/db")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "DATABASE_URL" && e.message.contains("postgresql"))
        );
    }

    #[test]
    fn valid_postgres_url_accepted() {
        let v = vars(&[(
            "DATABASE_URL",
            "postgresql://user:pass@localhost:5432/ozpos",
        )]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "DATABASE_URL"));
        }
    }

    #[test]
    fn empty_license_key_rejected() {
        let v = vars(&[("OZ_LICENSE_PRIVATE_KEY", "")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_LICENSE_PRIVATE_KEY" && e.message.contains("empty"))
        );
    }

    #[test]
    fn non_pem_license_key_warns() {
        let v = vars(&[("OZ_LICENSE_PRIVATE_KEY", "not-a-real-key")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_LICENSE_PRIVATE_KEY" && e.message.contains("PEM"))
        );
    }

    #[test]
    fn valid_pem_license_key_accepted() {
        let pem = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQ\n-----END PRIVATE KEY-----";
        let v = vars(&[("OZ_LICENSE_PRIVATE_KEY", pem)]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "OZ_LICENSE_PRIVATE_KEY"));
        }
    }

    #[test]
    fn license_key_fallback_to_oz_license_key() {
        // OZ_LICENSE_KEY is checked as a fallback when OZ_LICENSE_PRIVATE_KEY is absent
        let pem = "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----";
        let v = vars(&[("OZ_LICENSE_KEY", pem)]);
        let result = validate_config_with(&v);
        assert!(result.is_ok());
    }

    #[test]
    fn bad_stripe_key_rejected() {
        let v = vars(&[("STRIPE_SECRET_KEY", "not-a-stripe-key")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "STRIPE_SECRET_KEY" && e.message.contains("sk_"))
        );
    }

    #[test]
    fn valid_stripe_test_key_accepted() {
        let v = vars(&[("STRIPE_SECRET_KEY", "sk_test_abc123")]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "STRIPE_SECRET_KEY"));
        }
    }

    #[test]
    fn bad_midtrans_key_rejected() {
        let v = vars(&[("MIDTRANS_SERVER_KEY", "not-midtrans")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "MIDTRANS_SERVER_KEY" && e.message.contains("Mid-server-"))
        );
    }

    #[test]
    fn valid_midtrans_sandbox_key_accepted() {
        let v = vars(&[("MIDTRANS_SERVER_KEY", "SB-Mid-server-test123")]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "MIDTRANS_SERVER_KEY"));
        }
    }

    #[test]
    fn empty_midtrans_key_rejected() {
        let v = vars(&[("MIDTRANS_SERVER_KEY", "")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "MIDTRANS_SERVER_KEY" && e.message.contains("empty"))
        );
    }

    #[test]
    fn redirect_url_without_redirect_only_rejected() {
        let v = vars(&[
            ("OZ_SYNC_REDIRECT_URL", "https://new-server.example.com"),
            ("OZ_REDIRECT_ONLY", ""),
        ]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "OZ_SYNC_REDIRECT_URL" && e.message.contains("OZ_REDIRECT_ONLY"))
        );
    }

    #[test]
    fn redirect_url_with_redirect_only_accepted() {
        let v = vars(&[
            ("OZ_SYNC_REDIRECT_URL", "https://new.example.com"),
            ("OZ_REDIRECT_ONLY", "true"),
        ]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "OZ_SYNC_REDIRECT_URL"));
        }
    }

    #[test]
    fn bad_redis_url_rejected() {
        let v = vars(&[("REDIS_URL", "mysql://localhost")]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.key == "REDIS_URL" && e.message.contains("redis://"))
        );
    }

    #[test]
    fn valid_redis_url_accepted() {
        let v = vars(&[("REDIS_URL", "redis://localhost:6379")]);
        let result = validate_config_with(&v);
        if let Err(errs) = &result {
            assert!(!errs.iter().any(|e| e.key == "REDIS_URL"));
        }
    }

    #[test]
    fn no_env_vars_is_clean() {
        let result = validate_config_with(&empty_vars());
        assert!(result.is_ok(), "empty config should validate cleanly");
    }

    #[test]
    fn collects_multiple_errors() {
        let v = vars(&[
            ("OZ_API_PORT", "0"),
            ("DATABASE_URL", ""),
            ("MIDTRANS_SERVER_KEY", ""),
        ]);
        let errs = validate_config_with(&v).unwrap_err();
        assert!(
            errs.len() >= 3,
            "should collect all errors, got {}",
            errs.len()
        );
    }

    #[test]
    fn error_display_format() {
        let err = ConfigValidationError {
            key: "TEST_KEY",
            message: "is broken".into(),
            fix: Some("try fixing it".into()),
        };
        let display = err.to_string();
        assert!(display.contains("TEST_KEY"));
        assert!(display.contains("is broken"));
        assert!(display.contains("try fixing it"));
    }

    #[test]
    fn error_display_without_fix() {
        let err = ConfigValidationError {
            key: "TEST_KEY",
            message: "is broken".into(),
            fix: None,
        };
        let display = err.to_string();
        assert!(display.contains("TEST_KEY"));
        assert!(display.contains("is broken"));
    }

    #[test]
    fn truncate_prefix_short() {
        assert_eq!(truncate_prefix("hello", 10), "hello");
    }

    #[test]
    fn truncate_prefix_long() {
        let long = "postgresql://user:verylongpassword@localhost:5432/dbname";
        let truncated = truncate_prefix(long, 40);
        assert!(truncated.len() <= 43); // 40 chars + '...'
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn validate_config_live_does_not_panic() {
        // Production entry point — must never panic even in messy environments.
        let _ = validate_config();
    }
}
