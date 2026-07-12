//! License Activation Tauri commands.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{State, command};

use chrono::{DateTime, Utc};
use oz_core::Settings;
use oz_core::license_verification::{
    ActivateLicenseRequest, SignedSubscriptionPayload, activate_license as core_activate_license,
    verify_license_signature,
};

use crate::error::AppError;
use crate::state::AppState;

/// PocketBase requires IDs to be exactly 15 lowercase alphanumeric chars.
const MACHINE_ID_LEN: usize = 15;

/// Represents the front-end state of a license.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LicenseVerificationStatus {
    /// License is active and within the expiry window.
    Valid,
    /// License is past expiry and past the grace period limit.
    Expired,
    /// License is past expiry but remains active within the 14-day grace window.
    GracePeriod,
    /// Signature verification failed, indicating possible tampering or corruption.
    InvalidSignature,
    /// System clock tampering detected via ledger timestamps.
    ClockTampered,
    /// No license has been activated for this installation.
    Missing,
}

/// Data transfer object representing the current state of the local license.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatusDto {
    /// Whether the license is currently active and usable.
    pub is_active: bool,
    /// Categorized verification status of the license.
    pub status: LicenseVerificationStatus,
    /// Raw JSON payload of the signed license, if available.
    pub payload: Option<String>,
    /// Human-readable message explaining the status or providing error details.
    pub message: Option<String>,
}

/// Activates a license key for the given email, phone, and machine ID.
#[command]
pub async fn activate_license(
    state: State<'_, AppState>,
    key: String,
    email: String,
    machine_id: String,
    phone: String,
) -> Result<bool, AppError> {
    // H1 audit fix: read the previously-stored api_key so the server can
    // authenticate the caller as the legitimate tenant admin on re-
    // activations. On first activation this returns None and a new
    // api_key is issued in the response.
    let stored_api_key: Option<String> = {
        let conn = state.db.lock().await;
        Settings::get(&conn, "license.api_key")?.filter(|s| !s.is_empty())
    };

    let phone_clone = phone.clone();

    let req = ActivateLicenseRequest {
        key,
        email,
        machine_id,
        phone,
        api_key: stored_api_key,
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
            ("license.phone".to_string(), phone_clone),
        ],
    )?;

    Ok(true)
}

/// Retrieves the unique hardware identifier for this installation.
#[command]
pub async fn get_machine_id(state: State<'_, AppState>) -> Result<String, AppError> {
    let conn = state.db.lock().await;
    // Return the persisted machine ID if one already exists.
    if let Some(existing) = Settings::get(&conn, "machine_id")?
        && !existing.is_empty()
    {
        return Ok(existing);
    }
    // Generate a new one and persist it.
    let id = generate_machine_id();
    Settings::set_batch(&conn, &[("machine_id".to_string(), id.clone())])?;
    Ok(id)
}

/// Query the physical motherboard UUID or Windows MachineGuid as a stable hardware identifier.
fn get_system_uuid() -> Option<String> {
    use std::process::Command;

    // 1. Try motherboard UUID via wmic
    if let Ok(output) = Command::new("wmic")
        .args(["csproduct", "get", "uuid"])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() >= 2 {
            let uuid = lines[1];
            if !uuid.is_empty()
                && uuid != "00000000-0000-0000-0000-000000000000"
                && uuid != "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"
            {
                return Some(uuid.to_string());
            }
        }
    }

    // 2. Try Windows MachineGuid from Registry
    if let Ok(output) = Command::new("reg")
        .args([
            "query",
            "HKLM\\SOFTWARE\\Microsoft\\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("MachineGuid") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return Some(parts[2].to_string());
                }
            }
        }
    }

    None
}

/// Generate a stable 15-char lowercase alphanumeric machine ID based on
/// system/hardware UUID, falling back to a random UUID if queries fail.
///
/// Uses the hardware ID hashed with SHA-256 to produce a unique
/// per-installation fingerprint. The ID is persisted in the local
/// Settings table and reused across activations.
fn generate_machine_id() -> String {
    let raw_id = get_system_uuid().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mut hasher = Sha256::new();
    hasher.update(raw_id.as_bytes());
    let hash = hasher.finalize();
    let hex_str = hex::encode(&hash[..16]);
    hex_str[..MACHINE_ID_LEN].to_string()
}

/// Analyzes the local license state and returns a comprehensive status response.
#[command]
pub async fn get_license_status(state: State<'_, AppState>) -> Result<LicenseStatusDto, AppError> {
    let conn = state.db.lock().await;
    let payload_str = Settings::get(&conn, "license.payload")?;
    let signature = Settings::get(&conn, "license.signature")?;

    if let (Some(p), Some(s)) = (payload_str, signature) {
        if let Err(e) = verify_license_signature(&p, &s) {
            return Ok(LicenseStatusDto {
                is_active: false,
                status: LicenseVerificationStatus::InvalidSignature,
                payload: None,
                message: Some(format!("Invalid signature: {}", e)),
            });
        }

        // Parse payload
        let payload: SignedSubscriptionPayload = match serde_json::from_str(&p) {
            Ok(parsed) => parsed,
            Err(e) => {
                return Ok(LicenseStatusDto {
                    is_active: false,
                    status: LicenseVerificationStatus::InvalidSignature,
                    payload: None,
                    message: Some(format!("Failed to parse payload: {}", e)),
                });
            }
        };

        let now = Utc::now();

        let expires_at = DateTime::parse_from_rfc3339(&payload.expires_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);

        let grace_until = DateTime::parse_from_rfc3339(&payload.grace_until)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);

        if now < expires_at {
            Ok(LicenseStatusDto {
                is_active: true,
                status: LicenseVerificationStatus::Valid,
                payload: Some(p),
                message: None,
            })
        } else if now < grace_until {
            Ok(LicenseStatusDto {
                is_active: true,
                status: LicenseVerificationStatus::GracePeriod,
                payload: Some(p),
                message: Some(format!(
                    "License expired on {}. You are in the grace period until {}.",
                    expires_at.format("%Y-%m-%d"),
                    grace_until.format("%Y-%m-%d")
                )),
            })
        } else {
            Ok(LicenseStatusDto {
                is_active: false,
                status: LicenseVerificationStatus::Expired,
                payload: None,
                message: Some(format!(
                    "License expired on {}. Grace period ended on {}.",
                    expires_at.format("%Y-%m-%d"),
                    grace_until.format("%Y-%m-%d")
                )),
            })
        }
    } else {
        Ok(LicenseStatusDto {
            is_active: false,
            status: LicenseVerificationStatus::Missing,
            payload: None,
            message: Some("No license found. Please activate.".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::error::CoreError;
    use oz_core::subscription::TenantSubscription;

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
