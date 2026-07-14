//! License Activation Tauri commands.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{State, command};

use chrono::{DateTime, Utc};
use oz_core::Settings;
use oz_core::crypto::{decrypt_api_key, encrypt_api_key};
use oz_core::license_verification::{
    ActivateLicenseRequest, RenewLicenseRequest, SignedSubscriptionPayload,
    activate_license as core_activate_license, check_license_status as core_check_license_status,
    renew_license as core_renew_license, store_subscription, verify_license_signature,
};
use oz_core::subscription::TenantSubscription;

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
    // H1 audit fix: read the previously-stored (now encrypted) api_key
    // so the server can authenticate the caller as the legitimate tenant
    // admin on re-activations. On first activation this returns None and a
    // new api_key is issued in the response which we encrypt before storing.
    //
    // The `machine_id` parameter is the persisted machine fingerprint
    // (the front-end calls get_machine_id before activate_license).
    // We use it as the encryption key material — this binds the
    // ciphertext to this specific installation's hardware.
    let stored_api_key: Option<String> = {
        let conn = state.db.lock().await;
        let raw = Settings::get(&conn, "license.api_key")?.filter(|s| !s.is_empty());
        raw.as_ref().map(|v| {
            // Try decryption first (new format: base64 ciphertext).
            // If that fails, assume the value is legacy plaintext and
            // return it as-is. It will be encrypted on the next write.
            decrypt_api_key(v, &machine_id).unwrap_or_else(|e| {
                tracing::warn!(
                    "license.api_key decryption failed, treating as legacy plaintext: {e}"
                );
                v.clone()
            })
        })
    };

    let phone_clone = phone.clone();
    let machine_id_for_encryption = machine_id.clone();

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

    // Encrypt the api_key before storing in Settings.
    // The key is derived from the persisted machine_id, binding the
    // ciphertext to this specific installation.
    let encrypted_api_key = encrypt_api_key(&resp.api_key, &machine_id_for_encryption)
        .map_err(|e| AppError::Internal(format!("failed to encrypt api_key: {e}")))?;

    // ── Update tenant_subscription for quota enforcement ──────
    // The activate_license response includes a signed_payload with
    // tier, max_stores, max_pos_instances, etc. We persist this to
    // the tenant_subscription table keyed as "default" (NOT the
    // server-assigned tenant_id from resp.tenant_id) so workspace
    // commands like create_workspace_instance_scoped pick it up via
    // TenantSubscription::load("default"). Without this write, the
    // quota system would remain stuck on the bootstrap Free tier
    // (seeded by migration 061) regardless of what tier the user
    // activated.
    //
    // This write comes BEFORE Settings::set_batch so a partial
    // failure here doesn't leave the system in an inconsistent state
    // where Settings reflect the new tier but tenant_subscription
    // still has the old Free tier.
    let conn = state.db.lock().await;
    store_subscription(
        &conn,
        "default",
        &resp.signed_payload,
        &resp.signature,
        &resp.api_key,
    )
    .map_err(|e| AppError::Internal(format!("failed to persist subscription: {e}")))?;

    // Store in settings table
    Settings::set_batch(
        &conn,
        &[
            ("license.payload".to_string(), resp.signed_payload),
            ("license.signature".to_string(), resp.signature),
            ("license.tenant_id".to_string(), resp.tenant_id),
            ("license.api_key".to_string(), encrypted_api_key),
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

/// Renews an existing license subscription with a new license key.
///
/// Calls the server's `/api/v1/license/renew` endpoint with the
/// stored tenant_id, api_key, and the new key. On success, updates
/// both the Settings table and the tenant_subscription table with
/// the fresh signed_payload from the server.
#[command]
pub async fn renew_license(state: State<'_, AppState>, new_key: String) -> Result<bool, AppError> {
    if new_key.trim().is_empty() {
        return Err(AppError::Invalid("new license key is required".into()));
    }

    // Read tenant_id and api_key from Settings.
    let (tenant_id, api_key_encrypted, machine_id) = {
        let conn = state.db.lock().await;
        let tid = Settings::get(&conn, "license.tenant_id")?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::Invalid("No license activated. Activate first.".into()))?;
        let api_key_enc = Settings::get(&conn, "license.api_key")?.filter(|s| !s.is_empty());
        let mid = Settings::get(&conn, "machine_id")?.unwrap_or_default();
        (tid, api_key_enc, mid)
    };

    let api_key = match api_key_encrypted {
        Some(ref v) => decrypt_api_key(v, &machine_id).unwrap_or_else(|e| {
            tracing::warn!("license.api_key decryption failed, treating as legacy plaintext: {e}");
            v.clone()
        }),
        None => {
            return Err(AppError::Invalid(
                "No license activated. Activate first.".into(),
            ));
        }
    };

    let req = RenewLicenseRequest {
        tenant_id,
        api_key: api_key.clone(),
        key: new_key,
    };

    let resp = core_renew_license(&req)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Persist the renewed subscription to both stores.
    let conn = state.db.lock().await;

    // tenant_subscription (quota enforcement)
    store_subscription(
        &conn,
        "default",
        &resp.signed_payload,
        &resp.signature,
        &api_key,
    )
    .map_err(|e| AppError::Internal(format!("failed to persist renewed subscription: {e}")))?;

    // Settings (license status checks)
    // Parse the tenant_id from the renewed payload so Settings stays
    // in sync — if the server issued the renewal for a different
    // tenant (edge case like merged accounts), the stored tenant_id
    // is now correct for subsequent renew/status calls.
    let renewed_tenant_id: Option<String> =
        serde_json::from_str::<serde_json::Value>(&resp.signed_payload)
            .ok()
            .and_then(|v| v.get("tenant_id")?.as_str().map(String::from));

    let mut settings_entries = vec![
        ("license.payload".to_string(), resp.signed_payload),
        ("license.signature".to_string(), resp.signature),
    ];
    if let Some(tid) = renewed_tenant_id {
        settings_entries.push(("license.tenant_id".to_string(), tid));
    }

    Settings::set_batch(&conn, &settings_entries)?;

    Ok(true)
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

/// Data transfer object for server-authoritative license status.
/// Mirrors `oz_core::LicenseStatusResponse` but lives in this crate
/// so Tauri can serialize it over IPC.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLicenseStatusDto {
    /// The tenant ID.
    pub tenant_id: String,
    /// The subscription status.
    pub status: String,
    /// The tier key (free, pro, premium, enterprise).
    pub tier: String,
    /// Whether the subscription is active.
    pub active: bool,
    /// When the subscription expires (RFC 3339).
    pub expires_at: Option<String>,
    /// When the grace period ends (RFC 3339).
    pub grace_until: Option<String>,
    /// Maximum stores allowed.
    pub max_stores: i64,
}

/// Checks the license status against the PocketBase license server.
///
/// Unlike [`get_license_status`] which reads locally-stored data, this
/// command calls the server's `/api/v1/license/status` endpoint to get
/// the authoritative current status (e.g. whether the license has been
/// revoked or downgraded since last activation).
///
/// The stored API key is decrypted and sent as a Bearer token for
/// authentication. Returns the server's response directly.
#[command]
pub async fn check_license_status(
    state: State<'_, AppState>,
) -> Result<ServerLicenseStatusDto, AppError> {
    let (api_key_encrypted, machine_id) = {
        let conn = state.db.lock().await;
        let api_key_enc = Settings::get(&conn, "license.api_key")?.filter(|s| !s.is_empty());
        let mid = Settings::get(&conn, "machine_id")?.unwrap_or_default();
        (api_key_enc, mid)
    };

    let api_key = match api_key_encrypted {
        Some(ref v) => decrypt_api_key(v, &machine_id).unwrap_or_else(|e| {
            tracing::warn!("license.api_key decryption failed, treating as legacy plaintext: {e}");
            v.clone()
        }),
        None => {
            return Err(AppError::Invalid(
                "No license activated. Activate first.".into(),
            ));
        }
    };

    let resp = core_check_license_status(&api_key)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ServerLicenseStatusDto {
        tenant_id: resp.tenant_id,
        status: resp.status,
        tier: resp.tier,
        active: resp.active,
        expires_at: resp.expires_at,
        grace_until: resp.grace_until,
        max_stores: resp.max_stores,
    })
}

/// Analyzes the local license state and returns a comprehensive status response.
#[command]
pub async fn get_license_status(state: State<'_, AppState>) -> Result<LicenseStatusDto, AppError> {
    let conn = state.db.lock().await;

    // ── Clock rollback check (H1 audit gap fix) ─────────────
    // validate_clock_rollback compares the max ledger timestamp
    // against Utc::now(). If the OS clock was rolled back, return
    // ClockTampered so the UI can display a warning before the user
    // makes sales that would have future timestamps.
    if let Err(e) = TenantSubscription::validate_clock_rollback(&conn) {
        return Ok(LicenseStatusDto {
            is_active: false,
            status: LicenseVerificationStatus::ClockTampered,
            payload: None,
            message: Some(e.to_string()),
        });
    }

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
            // ── Expired (past grace period) ───────────────────
            #[cfg(debug_assertions)]
            {
                tracing::debug!("License expired in debug mode — returning Valid");
                Ok(LicenseStatusDto {
                    is_active: true,
                    status: LicenseVerificationStatus::Valid,
                    payload: None,
                    message: None,
                })
            }
            #[cfg(not(debug_assertions))]
            {
                return Ok(LicenseStatusDto {
                    is_active: false,
                    status: LicenseVerificationStatus::Expired,
                    payload: None,
                    message: Some(format!(
                        "License expired on {}. Grace period ended on {}.",
                        expires_at.format("%Y-%m-%d"),
                        grace_until.format("%Y-%m-%d")
                    )),
                });
            }
        }
    } else {
        // ── No stored payload/signature ─────────────────────
        #[cfg(debug_assertions)]
        {
            tracing::debug!("No license payload found in debug mode — returning Valid");
            Ok(LicenseStatusDto {
                is_active: true,
                status: LicenseVerificationStatus::Valid,
                payload: None,
                message: None,
            })
        }
        #[cfg(not(debug_assertions))]
        {
            return Ok(LicenseStatusDto {
                is_active: false,
                status: LicenseVerificationStatus::Missing,
                payload: None,
                message: Some("No license found. Please activate.".to_string()),
            });
        }
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
    fn generate_machine_id_is_deterministic() {
        // The machine ID is derived from the system UUID (or a random
        // fallback), hashed via SHA-256.  On the same machine it must
        // always return the same value — the first 15 hex chars of the
        // hash are stable.
        let id1 = generate_machine_id();
        for _ in 0..10 {
            assert_eq!(
                generate_machine_id(),
                id1,
                "machine ID changed between calls"
            );
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

    // ── ServerLicenseStatusDto tests ────────────────────────────

    #[test]
    fn server_license_status_dto_camel_case() {
        let dto = ServerLicenseStatusDto {
            tenant_id: "test-tenant".into(),
            status: "active".into(),
            tier: "pro".into(),
            active: true,
            expires_at: Some("2027-01-01T00:00:00Z".into()),
            grace_until: Some("2027-01-15T00:00:00Z".into()),
            max_stores: 2,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"tenantId\""));
        assert!(json.contains("\"expiresAt\""));
        assert!(json.contains("\"graceUntil\""));
        assert!(json.contains("\"maxStores\""));
        assert!(json.contains("\"active\":true"));
    }

    #[test]
    fn server_license_status_dto_null_optionals() {
        let dto = ServerLicenseStatusDto {
            tenant_id: "t1".into(),
            status: "canceled".into(),
            tier: "free".into(),
            active: false,
            expires_at: None,
            grace_until: None,
            max_stores: 1,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"expiresAt\":null"));
        assert!(json.contains("\"graceUntil\":null"));
    }

    // ── store_subscription → TenantSubscription round-trip ───────

    #[test]
    fn store_subscription_updates_tenant_subscription_default() {
        use oz_core::migrations;
        let conn = migrations::fresh_db();

        // Verify bootstrap Free tier is seeded
        let sub = TenantSubscription::load(&conn, "default")
            .expect("load")
            .expect("bootstrap row should exist");
        assert_eq!(sub.tier, oz_core::SubscriptionTier::Free);

        // Simulate a Pro activation — store_subscription should
        // replace the bootstrap row with the activated tier.
        let payload = r#"{
            "tenant_id": "default",
            "tier_key": "pro",
            "status": "active",
            "max_stores": 2,
            "max_pos_instances": 3,
            "allowed_types": ["restaurant-pos", "store-pos", "admin"],
            "starts_at": "2026-07-12T00:00:00Z",
            "expires_at": "2027-07-12T00:00:00Z",
            "grace_until": "2027-07-26T00:00:00Z",
            "issued_at": "2026-07-12T00:00:00Z"
        }"#;

        store_subscription(&conn, "default", payload, "SIG_PRO", "oz_apikey_pro")
            .expect("store_subscription should succeed");

        let updated = TenantSubscription::load(&conn, "default")
            .expect("load")
            .expect("row should exist after update");
        assert_eq!(updated.tier, oz_core::SubscriptionTier::Pro);
        assert_eq!(updated.max_stores, 2);
        assert_eq!(updated.max_pos_instances, 3);
        assert_eq!(updated.signature, "SIG_PRO");
        assert_eq!(updated.api_key, "oz_apikey_pro");
        assert_eq!(updated.signed_payload, payload);
    }

    // ── RenewLicenseRequest serialization ────────────────────────

    #[test]
    fn renew_license_request_serializes_snake_case() {
        let req = RenewLicenseRequest {
            tenant_id: "test-tenant".into(),
            api_key: "oz_test_key".into(),
            key: "OZ-PRO-NEW-KEY".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"tenant_id\""));
        assert!(json.contains("\"api_key\""));
        assert!(json.contains("\"key\""));
        assert!(json.contains("test-tenant"));
        assert!(json.contains("OZ-PRO-NEW-KEY"));
    }

    #[test]
    fn renew_license_request_deserializes() {
        let json = r#"{"tenant_id":"t1","api_key":"k1","key":"OZ-KEY"}"#;
        let req: RenewLicenseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tenant_id, "t1");
        assert_eq!(req.api_key, "k1");
        assert_eq!(req.key, "OZ-KEY");
    }
}
