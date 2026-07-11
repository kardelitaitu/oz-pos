//! License Activation Tauri commands.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{State, command};

use oz_core::Settings;
use oz_core::error::CoreError;
use oz_core::license_verification::{
    ActivateLicenseRequest, activate_license as core_activate_license, verify_license_signature,
};
use oz_core::subscription::TenantSubscription;

use crate::error::AppError;
use crate::state::AppState;

/// PocketBase requires IDs to be exactly 15 lowercase alphanumeric chars.
const MACHINE_ID_LEN: usize = 15;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LicenseVerificationStatus {
    Valid,
    Expired,
    GracePeriod,
    InvalidSignature,
    ClockTampered,
    Missing,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatusDto {
    pub is_active: bool,
    pub status: LicenseVerificationStatus,
    pub payload: Option<String>,
    pub message: Option<String>,
}

#[command]
pub async fn activate_license(
    state: State<'_, AppState>,
    key: String,
    email: String,
    machine_id: String,
) -> Result<bool, AppError> {
    let req = ActivateLicenseRequest {
        key,
        email,
        machine_id,
    };

    let resp = core_activate_license(&req)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Store in settings table
    let conn = state.db.lock().await;
    Settings::set_batch(
        &conn,
        &[
            ("license.payload".to_string(), resp.signed_payload),
            ("license.signature".to_string(), resp.signature),
            ("license.tenant_id".to_string(), resp.tenant_id),
            ("license.api_key".to_string(), resp.api_key),
        ],
    )?;

    Ok(true)
}

#[command]
pub async fn get_machine_id(state: State<'_, AppState>) -> Result<String, AppError> {
    let conn = state.db.lock().await;
    // Return the persisted machine ID if one already exists.
    if let Some(existing) = Settings::get(&conn, "machine_id")? {
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    // Generate a new one and persist it.
    let id = generate_machine_id();
    Settings::set_batch(&conn, &[("machine_id".to_string(), id.clone())])?;
    Ok(id)
}

/// Generate a cryptographically random 15-char lowercase alphanumeric
/// machine ID matching PocketBase's ID constraints.
///
/// Uses UUID v4 (OS entropy) hashed with SHA-256 to produce a unique
/// per-installation fingerprint. The ID is persisted in the local
/// Settings table and reused across activations.
fn generate_machine_id() -> String {
    let uuid = uuid::Uuid::new_v4();
    let mut hasher = Sha256::new();
    hasher.update(uuid.as_bytes());
    let hash = hasher.finalize();
    let hex_str = hex::encode(&hash[..16]);
    hex_str[..MACHINE_ID_LEN].to_string()
}

#[command]
pub async fn get_license_status(state: State<'_, AppState>) -> Result<LicenseStatusDto, AppError> {
    let conn = state.db.lock().await;

    // Check for clock rollback before evaluating licence expiry.
    // If the system clock has been set back, the ledger will contain
    // timestamps in the future relative to the wall clock.
    match TenantSubscription::validate_clock_rollback(&conn) {
        Err(CoreError::SystemClockTampered(msg)) => {
            return Ok(LicenseStatusDto {
                is_active: false,
                status: LicenseVerificationStatus::ClockTampered,
                payload: None,
                message: Some(format!("Clock tampering detected: {msg}")),
            });
        }
        Err(e) => return Err(AppError::Internal(e.to_string())),
        Ok(()) => {}
    }

    let payload_str = Settings::get(&conn, "license.payload")?;
    let signature = Settings::get(&conn, "license.signature")?;

    if let (Some(p), Some(s)) = (payload_str.clone(), signature) {
        match verify_license_signature(&p, &s) {
            Ok(_) => Ok(LicenseStatusDto {
                is_active: true,
                status: LicenseVerificationStatus::Valid,
                payload: Some(p),
                message: None,
            }),
            Err(_) => Ok(LicenseStatusDto {
                is_active: false,
                status: LicenseVerificationStatus::InvalidSignature,
                payload: None,
                message: None,
            }), // Invalid signature
        }
    } else {
        Ok(LicenseStatusDto {
            is_active: false,
            status: LicenseVerificationStatus::Missing,
            payload: None,
            message: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_tampered_serializes_camel_case() {
        let status = LicenseVerificationStatus::ClockTampered;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"clockTampered\"");
    }

    #[test]
    fn all_variants_round_trip() {
        let variants = [
            LicenseVerificationStatus::Valid,
            LicenseVerificationStatus::Expired,
            LicenseVerificationStatus::GracePeriod,
            LicenseVerificationStatus::InvalidSignature,
            LicenseVerificationStatus::ClockTampered,
            LicenseVerificationStatus::Missing,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: LicenseVerificationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(v, &back, "round-trip failed for {json}");
        }
    }

    #[test]
    fn clock_tampered_dto_is_inactive() {
        let dto = LicenseStatusDto {
            is_active: false,
            status: LicenseVerificationStatus::ClockTampered,
            payload: None,
            message: Some("Clock tampering detected: test".into()),
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"clockTampered\""));
        assert!(json.contains("\"isActive\":false"));
        assert!(json.contains("Clock tampering detected"));
    }

    #[test]
    fn generate_machine_id_returns_15_chars() {
        let id = generate_machine_id();
        assert_eq!(id.len(), 15, "machine ID must be 15 chars, got {id}");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "machine ID must be lowercase alphanumeric, got {id}"
        );
    }

    #[test]
    fn generate_machine_id_is_unique() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = generate_machine_id();
            assert!(ids.insert(id.clone()), "duplicate machine ID: {id}");
        }
    }

    #[test]
    fn machine_id_is_persisted_in_settings() {
        use oz_core::migrations;
        let conn = migrations::fresh_db();
        let id1 = generate_machine_id();
        // Simulate what get_machine_id does: persist to Settings.
        Settings::set_batch(&conn, &[("machine_id".to_string(), id1.clone())]).unwrap();
        let id2 = Settings::get(&conn, "machine_id").unwrap().unwrap();
        assert_eq!(
            id1, id2,
            "machine ID should survive round-trip through Settings"
        );
    }

    #[test]
    fn clock_tamper_detected_on_future_ledger_timestamps() {
        use oz_core::migrations;
        let conn = migrations::fresh_db();

        // Insert a sale with a timestamp far in the future
        // (simulates OS clock being rolled back).
        conn.execute(
            "INSERT INTO sales (id, status, total_minor, currency, line_count, created_at, updated_at)
             VALUES ('sale-clocktest', 'completed', 1000, 'USD', 1,
                     '2099-01-01T00:00:00.000Z', '2099-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let result = TenantSubscription::validate_clock_rollback(&conn);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CoreError::SystemClockTampered(_)),
            "should be SystemClockTampered, got: {err:?}"
        );
        assert!(err.to_string().contains("system clock tampered"));
    }
}
