//! Windows Credential Manager implementation of [`Keyring`].
//!
//! Wraps `wincred` / `CredWriteW` / `CredReadW` / `CredDeleteW`
//! from `advapi32.dll` to store secrets.

#![allow(unsafe_code)]

use crate::error::SecurityError;
use crate::Keyring;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FALSE};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

/// Windows Credential Manager keyring.
///
/// Stores secrets in the Windows Credential Manager under the
/// target name `OZ-POS:{name}`.
pub struct WindowsCredentialManager;

impl WindowsCredentialManager {
    /// Create a new Windows Credential Manager keyring instance.
    pub fn new() -> Result<Self, SecurityError> {
        Ok(Self)
    }
}

impl Keyring for WindowsCredentialManager {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecurityError> {
        let target = encode_utf16_null(&format!("OZ-POS:{name}"));

        let mut p_cred: *mut CREDENTIALW = std::ptr::null_mut();
        let rc = unsafe {
            CredReadW(
                target.as_ptr() as *mut u16,
                CRED_TYPE_GENERIC,
                0,
                &mut p_cred,
            )
        };

        if rc == FALSE {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(SecurityError::KeyUnavailable(format!(
                "CredReadW failed: {err}"
            )));
        }

        let secret = unsafe {
            let cred = &*p_cred;
            let blob = std::slice::from_raw_parts(
                cred.CredentialBlob as *const u8,
                cred.CredentialBlobSize as usize,
            );
            let s = String::from_utf8_lossy(blob).into_owned();
            CredFree(p_cred as *const core::ffi::c_void);
            s
        };

        Ok(Some(secret))
    }

    fn set_secret(&self, name: &str, value: &str) -> Result<(), SecurityError> {
        let target = encode_utf16_null(&format!("OZ-POS:{name}"));
        let blob = value.as_bytes().to_vec();

        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_ptr() as *mut u16,
            Comment: std::ptr::null_mut(),
            LastWritten: unsafe { std::mem::zeroed() },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: std::ptr::null_mut(),
        };

        let rc = unsafe { CredWriteW(&cred, 0) };
        if rc == FALSE {
            let err = unsafe { GetLastError() };
            return Err(SecurityError::KeyUnavailable(format!(
                "CredWriteW failed: {err}"
            )));
        }
        Ok(())
    }

    fn delete_secret(&self, name: &str) -> Result<bool, SecurityError> {
        let target = encode_utf16_null(&format!("OZ-POS:{name}"));

        let rc = unsafe {
            CredDeleteW(target.as_ptr() as *mut u16, CRED_TYPE_GENERIC, 0)
        };

        if rc == FALSE {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(false);
            }
            return Err(SecurityError::KeyUnavailable(format!(
                "CredDeleteW failed: {err}"
            )));
        }
        Ok(true)
    }
}

fn encode_utf16_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keyring() -> WindowsCredentialManager {
        WindowsCredentialManager::new().expect("failed to create keyring")
    }

    #[test]
    fn windows_roundtrip() {
        let k = test_keyring();
        let name = "oz-pos-test-windows-roundtrip";
        let _ = k.delete_secret(name);

        assert_eq!(k.get_secret(name).unwrap(), None);

        k.set_secret(name, "test-value-123").unwrap();
        assert_eq!(
            k.get_secret(name).unwrap(),
            Some("test-value-123".into())
        );

        assert!(k.delete_secret(name).unwrap());
        assert_eq!(k.get_secret(name).unwrap(), None);
    }

    #[test]
    fn windows_delete_nonexistent_returns_false() {
        let k = test_keyring();
        assert!(!k.delete_secret("oz-pos-test-nonexistent-delete").unwrap());
    }

    #[test]
    fn windows_overwrite_existing() {
        let k = test_keyring();
        let name = "oz-pos-test-overwrite";
        let _ = k.delete_secret(name);

        k.set_secret(name, "v1").unwrap();
        k.set_secret(name, "v2").unwrap();
        assert_eq!(k.get_secret(name).unwrap(), Some("v2".into()));

        k.delete_secret(name).unwrap();
    }
}
