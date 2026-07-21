//! Cloud warehouse export destinations for the analytics bundle.
//!
//! Defines export targets (BigQuery, Snowflake) and their respective
//! connection configurations. The [`CloudExporter`] trait provides a
//! uniform interface for sending [`AnalyticsBundle`](super::AnalyticsBundle)
//! data to each destination via REST/HTTP.
//!
//! # Configuration
//!
//! The [`CloudExportConfig`] is persisted in the `settings` table under
//! key `cloud_export_config` as JSON (same pattern as
//! [`ReportScheduleConfig`](super::ReportScheduleConfig)).
//!
//! # Usage
//!
//! ```rust,ignore
//! let config = CloudExportConfig::load(&store)?;
//! let result = CloudExporter::export(&bundle, &config).await?;
//! ```

use serde::{Deserialize, Serialize};

/// Supported cloud warehouse destinations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportDestination {
    /// Google BigQuery — requires a service-account JSON key.
    BigQuery(BigQueryConfig),
    /// Snowflake — requires account URL + user credentials.
    Snowflake(SnowflakeConfig),
}

impl ExportDestination {
    /// Human-readable label for the destination type.
    pub fn label(&self) -> &str {
        match self {
            Self::BigQuery(_) => "BigQuery",
            Self::Snowflake(_) => "Snowflake",
        }
    }
}

/// BigQuery connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigQueryConfig {
    /// GCP project ID.
    pub project_id: String,
    /// BigQuery dataset name.
    pub dataset: String,
    /// BigQuery table name.
    pub table: String,
    /// Service-account JSON key (base64-encoded for safe storage).
    pub service_account_key_b64: String,
    /// GCP region for the dataset (e.g. "US", "asia-southeast2").
    pub location: String,
}

impl BigQueryConfig {
    /// Create a new BigQuery config.
    pub fn new(
        project_id: impl Into<String>,
        dataset: impl Into<String>,
        table: impl Into<String>,
        service_account_key_b64: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            dataset: dataset.into(),
            table: table.into(),
            service_account_key_b64: service_account_key_b64.into(),
            location: location.into(),
        }
    }
}

/// Snowflake connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnowflakeConfig {
    /// Snowflake account URL (e.g. "https://xyz12345.us-east-1.snowflakecomputing.com").
    pub account_url: String,
    /// Snowflake warehouse name.
    pub warehouse: String,
    /// Database name.
    pub database: String,
    /// Schema name.
    pub schema: String,
    /// Table name for ingestion.
    pub table: String,
    /// Username for authentication.
    pub username: String,
    /// Password or private key for authentication.
    pub password: String,
}

impl SnowflakeConfig {
    /// Create a new Snowflake config.
    pub fn new(
        account_url: impl Into<String>,
        warehouse: impl Into<String>,
        database: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            account_url: account_url.into(),
            warehouse: warehouse.into(),
            database: database.into(),
            schema: schema.into(),
            table: table.into(),
            username: username.into(),
            password: password.into(),
        }
    }
}

/// Persisted configuration for cloud warehouse export.
///
/// Stored in the `settings` table under key `cloud_export_config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudExportConfig {
    /// Whether cloud export is enabled.
    pub enabled: bool,
    /// Selected destination.
    pub destination: ExportDestination,
    /// Include all report types or only selected ones.
    pub include_all_reports: bool,
    /// Specific report types to include when `include_all_reports` is false.
    pub report_types: Vec<String>,
}

impl Default for CloudExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            destination: ExportDestination::BigQuery(BigQueryConfig::new("", "", "", "", "")),
            include_all_reports: true,
            report_types: Vec::new(),
        }
    }
}

/// Settings key used to persist the cloud export config.
pub const CLOUD_EXPORT_SETTINGS_KEY: &str = "cloud_export_config";

/// Result of a cloud export operation.
#[derive(Debug, Clone, Serialize)]
pub struct CloudExportResult {
    /// Whether the export succeeded.
    pub success: bool,
    /// Number of rows exported.
    pub rows_exported: u64,
    /// Human-readable message (success detail or error).
    pub message: String,
}

/// Trait for exporting analytics data to cloud warehouse destinations.
pub struct CloudExporter;

impl CloudExporter {
    /// Export an analytics bundle to the configured cloud destination.
    ///
    /// Dispatches to the appropriate destination-specific implementation
    /// based on the `config.destination` variant.
    pub async fn export(
        bundle: &super::AnalyticsBundle,
        config: &CloudExportConfig,
    ) -> Result<CloudExportResult, crate::error::CoreError> {
        match &config.destination {
            ExportDestination::BigQuery(bq_config) => {
                Self::export_to_bigquery(bundle, bq_config).await
            }
            ExportDestination::Snowflake(sf_config) => {
                Self::export_to_snowflake(bundle, sf_config).await
            }
        }
    }

    /// Send analytics data to Google BigQuery via the Storage Write API
    /// (REST endpoint using service-account authentication).
    async fn export_to_bigquery(
        bundle: &super::AnalyticsBundle,
        config: &BigQueryConfig,
    ) -> Result<CloudExportResult, crate::error::CoreError> {
        // Decode the service-account key.
        let key_bytes = base64_decode(&config.service_account_key_b64)
            .map_err(|e| crate::error::CoreError::Internal(format!("invalid base64 key: {e}")))?;

        let key_json = String::from_utf8(key_bytes).map_err(|_| {
            crate::error::CoreError::Internal("service-account key is not valid UTF-8".into())
        })?;

        let _key: serde_json::Value = serde_json::from_str(&key_json).map_err(|e| {
            crate::error::CoreError::Internal(format!("invalid service-account JSON: {e}"))
        })?;

        // Serialise the bundle to NDJSON — one JSON object per row.
        let ndjson = bundle_to_ndjson(bundle);
        let row_count = ndjson.len();

        // Call BigQuery's tabledata.insertAll REST API.
        // This is a streaming insert — suitable for real-time analytics.
        let url = format!(
            "https://bigquery.googleapis.com/bigquery/v2/projects/{}/datasets/{}/tables/{}/insertAll",
            config.project_id, config.dataset, config.table
        );

        let client = reqwest::Client::new();

        // Obtain an OAuth2 access token from the service-account key.
        let access_token = get_gcp_access_token(&key_json).await.map_err(|e| {
            crate::error::CoreError::Internal(format!("failed to get GCP token: {e}"))
        })?;

        let payload = serde_json::json!({
            "kind": "bigquery#tableDataInsertAllRequest",
            "rows": ndjson.iter().map(|row| {
                serde_json::json!({"json": row})
            }).collect::<Vec<_>>(),
        });

        let resp = client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                crate::error::CoreError::Internal(format!("BigQuery request failed: {e}"))
            })?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();

        if status.is_success() {
            Ok(CloudExportResult {
                success: true,
                rows_exported: row_count as u64,
                message: format!(
                    "Exported {} rows to BigQuery {}.{}",
                    row_count, config.dataset, config.table
                ),
            })
        } else {
            Ok(CloudExportResult {
                success: false,
                rows_exported: 0,
                message: format!(
                    "BigQuery returned {}: {}",
                    status,
                    body.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error")
                ),
            })
        }
    }

    /// Send analytics data to Snowflake via the SQL REST API.
    ///
    /// Uses Snowflake's SQL statement execution endpoint to INSERT rows
    /// into the configured table via bulk INSERT statements (batched
    /// for efficiency).
    async fn export_to_snowflake(
        bundle: &super::AnalyticsBundle,
        config: &SnowflakeConfig,
    ) -> Result<CloudExportResult, crate::error::CoreError> {
        let ndjson = bundle_to_ndjson(bundle);
        let row_count = ndjson.len();

        if row_count == 0 {
            return Ok(CloudExportResult {
                success: true,
                rows_exported: 0,
                message: "No data to export — bundle is empty.".to_string(),
            });
        }

        let client = reqwest::Client::new();

        // Step 1: Authenticate and get a session token.
        let login_url = format!("{}/session/v1/login-request", config.account_url);

        // Obtain a session token via basic-auth login.
        let auth_resp = client
            .post(&login_url)
            .json(&serde_json::json!({
                "data": {
                    "LOGIN_NAME": config.username,
                    "PASSWORD": config.password,
                }
            }))
            .send()
            .await
            .map_err(|e| {
                crate::error::CoreError::Internal(format!("Snowflake auth request failed: {e}"))
            })?;

        let auth_body: serde_json::Value = auth_resp.json().await.unwrap_or_default();
        let token = auth_body["data"]["token"].as_str().ok_or_else(|| {
            crate::error::CoreError::Internal("Snowflake auth failed — no token returned".into())
        })?;

        // Step 2: Build INSERT statements in batches (50 rows per batch).
        let batch_size = 50;
        let columns = [
            "exported_at",
            "tenant_id",
            "store_name",
            "report_type",
            "report_data",
        ];

        for chunk in ndjson.chunks(batch_size) {
            let mut sql = format!(
                "INSERT INTO {}.{}.{} ({}) VALUES ",
                config.database,
                config.schema,
                config.table,
                columns.join(", ")
            );

            let rows: Vec<String> = chunk
                .iter()
                .map(|row| {
                    let exported_at = sql_escape(
                        row.get("exported_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    );
                    let tenant_id =
                        sql_escape(row.get("tenant_id").and_then(|v| v.as_str()).unwrap_or(""));
                    let store_name =
                        sql_escape(row.get("store_name").and_then(|v| v.as_str()).unwrap_or(""));
                    let report_type = sql_escape(
                        row.get("report_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown"),
                    );
                    let report_data = sql_escape(&serde_json::to_string(row).unwrap_or_default());
                    format!(
                        "('{}', '{}', '{}', '{}', PARSE_JSON('{}'))",
                        exported_at, tenant_id, store_name, report_type, report_data
                    )
                })
                .collect();

            sql.push_str(&rows.join(", "));
            sql.push(';');

            let stmt_url = format!("{}/api/v2/statements", config.account_url);

            let stmt_resp = client
                .post(&stmt_url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&serde_json::json!({
                    "statement": sql,
                    "timeout": 60,
                    "database": config.database,
                    "schema": config.schema,
                    "warehouse": config.warehouse,
                }))
                .send()
                .await
                .map_err(|e| {
                    crate::error::CoreError::Internal(format!(
                        "Snowflake INSERT request failed: {e}"
                    ))
                })?;

            let stmt_status = stmt_resp.status();
            let stmt_body: serde_json::Value = stmt_resp.json().await.unwrap_or_default();

            if !stmt_status.is_success() {
                return Ok(CloudExportResult {
                    success: false,
                    rows_exported: 0,
                    message: format!(
                        "Snowflake INSERT returned {}: {}",
                        stmt_status,
                        stmt_body
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error")
                    ),
                });
            }
        }

        Ok(CloudExportResult {
            success: true,
            rows_exported: row_count as u64,
            message: format!(
                "Exported {} rows to Snowflake {}.{}.{}",
                row_count, config.database, config.schema, config.table
            ),
        })
    }
}

/// Convert an AnalyticsBundle to NDJSON rows, one per report type.
fn bundle_to_ndjson(bundle: &super::AnalyticsBundle) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    let meta = &bundle.metadata;

    // Helper to stamp each row with export metadata.
    let stamp = |report_type: &str, data: serde_json::Value| -> serde_json::Value {
        serde_json::json!({
            "exported_at": meta.exported_at,
            "tenant_id": meta.tenant_id,
            "store_name": meta.store_name,
            "version": meta.version,
            "report_type": report_type,
            "data": data,
        })
    };

    // Daily revenue
    for r in &bundle.daily_revenue {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("daily_revenue", val));
        }
    }

    // Weekly revenue
    for r in &bundle.weekly_revenue {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("weekly_revenue", val));
        }
    }

    // Monthly revenue
    for r in &bundle.monthly_revenue {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("monthly_revenue", val));
        }
    }

    // Top products
    for r in &bundle.top_products {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("top_products", val));
        }
    }

    // Hourly heatmap
    for r in &bundle.hourly_heatmap {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("hourly_heatmap", val));
        }
    }

    // Category breakdown
    for r in &bundle.category_breakdown {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("category_breakdown", val));
        }
    }

    // Low stock alerts
    for r in &bundle.low_stock_alerts {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("low_stock_alerts", val));
        }
    }

    // Active stock alerts
    for r in &bundle.active_stock_alerts {
        if let Ok(val) = serde_json::to_value(r) {
            rows.push(stamp("active_stock_alerts", val));
        }
    }

    rows
}

/// Base64-decode a string.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    Engine::decode(&engine, input.as_bytes()).map_err(|e| format!("base64 decode: {e}"))
}

/// Escape a string for SQL (single-quote escaping).
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

/// Obtain a GCP OAuth2 access token using a service-account JSON key.
async fn get_gcp_access_token(key_json: &str) -> Result<String, String> {
    let key: serde_json::Value =
        serde_json::from_str(key_json).map_err(|e| format!("parse key: {e}"))?;

    let client_email = key["client_email"]
        .as_str()
        .ok_or("missing client_email in service-account key")?;
    let private_key = key["private_key"]
        .as_str()
        .ok_or("missing private_key in service-account key")?;

    // Create a JWT assertion for GCP OAuth.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "JWT",
        "kid": key["private_key_id"],
    });

    let claims = serde_json::json!({
        "iss": client_email,
        "scope": "https://www.googleapis.com/auth/bigquery.insertdata",
        "aud": "https://oauth2.googleapis.com/token",
        "exp": now + 3600,
        "iat": now,
    });

    use base64::Engine;
    let b64_engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_b64 = b64_engine.encode(
        serde_json::to_string(&header)
            .unwrap_or_default()
            .as_bytes(),
    );
    let claims_b64 = b64_engine.encode(
        serde_json::to_string(&claims)
            .unwrap_or_default()
            .as_bytes(),
    );

    let message = format!("{header_b64}.{claims_b64}");

    // Sign the JWT with the RSA private key.
    let signature = sign_rsa256(&message, private_key)?;
    let signature_b64 = b64_engine.encode(signature);

    let assertion = format!("{message}.{signature_b64}");

    // Exchange the assertion for an access token.
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &assertion),
        ])
        .send()
        .await
        .map_err(|e| format!("token request: {e}"))?;

    let body: serde_json::Value = resp.json().await.map_err(|e| format!("parse token: {e}"))?;
    body["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "no access_token in response: {}",
                serde_json::to_string(&body).unwrap_or_default()
            )
        })
}

/// Sign a string with an RSA256 private key (PKCS#8 PEM format).
///
/// GCP service-account keys are always PKCS#8 format, so only
/// PKCS#8 PEM parsing is needed.
fn sign_rsa256(message: &str, private_key_pem: &str) -> Result<Vec<u8>, String> {
    use rsa::RsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::signature::{SignatureEncoding, Signer};
    use sha2::Sha256;

    // Parse the PEM-encoded private key directly via rsa's PEM feature.
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .map_err(|e| format!("RSA key parse from PEM: {e}"))?;

    // PKCS#1 v1.5 sign using the established pattern from license_verification.rs.
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(message.as_bytes());

    Ok(signature.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::AnalyticsBundle;
    use crate::export::ExportMetadata;

    fn sample_bundle() -> AnalyticsBundle {
        AnalyticsBundle {
            metadata: ExportMetadata {
                exported_at: "2026-07-21T00:00:00.000Z".to_string(),
                tenant_id: "test-tenant".to_string(),
                store_name: "Test Store".to_string(),
                version: "0.0.17".to_string(),
            },
            daily_revenue: Vec::new(),
            weekly_revenue: Vec::new(),
            monthly_revenue: Vec::new(),
            top_products: Vec::new(),
            hourly_heatmap: Vec::new(),
            category_breakdown: Vec::new(),
            low_stock_alerts: Vec::new(),
            active_stock_alerts: Vec::new(),
        }
    }

    #[test]
    fn export_config_default() {
        let cfg = CloudExportConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.include_all_reports);
    }

    #[test]
    fn destination_labels() {
        let bq = ExportDestination::BigQuery(BigQueryConfig::new("p", "d", "t", "k", "US"));
        assert_eq!(bq.label(), "BigQuery");

        let sf = ExportDestination::Snowflake(SnowflakeConfig::new(
            "https://test.snowflake.com",
            "wh",
            "db",
            "s",
            "t",
            "u",
            "p",
        ));
        assert_eq!(sf.label(), "Snowflake");
    }

    #[test]
    fn export_destination_serde_roundtrip() {
        let bq = ExportDestination::BigQuery(BigQueryConfig::new(
            "my-project",
            "my_dataset",
            "my_table",
            "base64key==",
            "US",
        ));
        let json = serde_json::to_string(&bq).unwrap();
        let back: ExportDestination = serde_json::from_str(&json).unwrap();
        match back {
            ExportDestination::BigQuery(cfg) => {
                assert_eq!(cfg.project_id, "my-project");
                assert_eq!(cfg.dataset, "my_dataset");
                assert_eq!(cfg.table, "my_table");
                assert_eq!(cfg.location, "US");
            }
            _ => panic!("expected BigQuery"),
        }
    }

    #[test]
    fn cloud_export_config_serde_roundtrip() {
        let cfg = CloudExportConfig {
            enabled: true,
            destination: ExportDestination::Snowflake(SnowflakeConfig::new(
                "https://test.snowflake.com",
                "COMPUTE_WH",
                "ANALYTICS",
                "PUBLIC",
                "EXPORT_TABLE",
                "svc_user",
                "supersecret",
            )),
            include_all_reports: false,
            report_types: vec!["daily_revenue".to_string(), "top_products".to_string()],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: CloudExportConfig = serde_json::from_str(&json).unwrap();
        assert!(back.enabled);
        assert!(!back.include_all_reports);
        assert_eq!(back.report_types.len(), 2);
        match back.destination {
            ExportDestination::Snowflake(s) => {
                assert_eq!(s.username, "svc_user");
                assert_eq!(s.warehouse, "COMPUTE_WH");
            }
            _ => panic!("expected Snowflake"),
        }
    }

    #[test]
    fn bundle_to_ndjson_empty_bundle() {
        let bundle = sample_bundle();
        let rows = bundle_to_ndjson(&bundle);
        assert!(rows.is_empty(), "empty bundle should produce no rows");
    }

    #[test]
    fn sql_escape_handles_quotes() {
        assert_eq!(sql_escape("hello"), "hello");
        assert_eq!(sql_escape("it's"), "it''s");
        assert_eq!(sql_escape("'single'"), "''single''");
    }

    #[test]
    fn export_result_serialization() {
        let result = CloudExportResult {
            success: true,
            rows_exported: 42,
            message: "Exported to BigQuery".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"rows_exported\":42"));
    }
}
