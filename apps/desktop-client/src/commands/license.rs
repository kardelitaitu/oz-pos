//! License Activation Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::Settings;
use oz_core::error::CoreError;
use oz_core::license_verification::{
    ActivateLicenseRequest, activate_license as core_activate_license, verify_license_signature,
};
use oz_core::subscription::TenantSubscription;

use crate::error::AppError;
use crate::state::AppState;

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
    pub payload: Option<String>,
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

    if let (Some(p), Some(s)) = (payload.clone(), signature) {
        match verify_license_signature(&p, &s) {
            Ok(_) => Ok(LicenseStatusDto {
                is_active: true,
                payload: Some(p.to_string()),
            }),
            Err(_) => Ok(LicenseStatusDto {
                is_active: false,
                payload: None,
            }), // Invalid signature
        }
    } else {
        Ok(LicenseStatusDto {
            is_active: false,
            payload: None,
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
