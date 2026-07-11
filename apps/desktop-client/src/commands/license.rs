//! License Activation Tauri commands.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::Settings;
use oz_core::license_verification::{
    ActivateLicenseRequest, activate_license as core_activate_license, verify_license_signature,
};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
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
    let payload = Settings::get(&conn, "license.payload")?;
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
