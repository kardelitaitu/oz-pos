//! Per-terminal hardware profile — stores printer and scanner configuration
//! in per-terminal JSON files under `terminal_profiles/`.
//!
//! ## File layout
//!
//! ```text
//! {app_data_dir}/terminal_profiles/
//!   ├── terminal-001.json
//!   ├── terminal-002.json
//!   └── unknown.json
//! ```
//!
//! ## Crash-safe writes (ADR #22)
//!
//! Every save uses write-to-temp-then-atomic-rename:
//! 1. Write to `<path>.tmp`
//! 2. Rename old → `<path>.bak` (best-effort backup)
//! 3. Rename `<path>.tmp` → `<path>` (atomic on most filesystems)
//! 4. Remove `<path>.bak` on success
//!
//! If the process crashes mid-write, either the original or the new
//! profile survives — never a partial write.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::PlatformError;

/// Per-terminal hardware configuration stored in `terminal_profile.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalProfile {
    /// Printer connection type: `"network"`, `"usb"`, `"serial"`, `"auto"`.
    #[serde(default = "default_printer_connection")]
    pub printer_connection: String,

    /// Printer device path or IP address.
    #[serde(default)]
    pub printer_device_path: String,

    /// Printer paper size: `"58"`, `"80"`, `"a4"`, `"letter"`.
    #[serde(default = "default_printer_paper_size")]
    pub printer_paper_size: String,

    /// Selected scanner device ID.
    #[serde(default)]
    pub scanner_device_id: String,

    /// Scanner input mode: `"auto"`, `"keyboard"`, `"serial"`.
    #[serde(default = "default_scanner_input_mode")]
    pub scanner_input_mode: String,
}

fn default_printer_connection() -> String {
    "auto".into()
}

fn default_printer_paper_size() -> String {
    "80".into()
}

fn default_scanner_input_mode() -> String {
    "auto".into()
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            printer_connection: default_printer_connection(),
            printer_device_path: String::new(),
            printer_paper_size: default_printer_paper_size(),
            scanner_device_id: String::new(),
            scanner_input_mode: default_scanner_input_mode(),
        }
    }
}

impl TerminalProfile {
    /// Build the filesystem path for a terminal's profile.
    ///
    /// Returns `<base_dir>/terminal_profiles/<terminal_id>.json`.
    pub fn profile_path(base_dir: &Path, terminal_id: &str) -> PathBuf {
        base_dir
            .join("terminal_profiles")
            .join(format!("{terminal_id}.json"))
    }

    /// Load a profile from disk. Returns `Ok(Some(profile))` if the file
    /// exists, `Ok(None)` if the file is missing (caller should use
    /// defaults), or `Err` on read/parse failure.
    pub fn load(path: &Path) -> Result<Option<Self>, PlatformError> {
        match fs::read_to_string(path) {
            Ok(json) => {
                let profile: TerminalProfile = serde_json::from_str(&json).map_err(|e| {
                    PlatformError::Internal(format!(
                        "failed to parse terminal profile {}: {e}",
                        path.display()
                    ))
                })?;
                Ok(Some(profile))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(PlatformError::Internal(format!(
                "failed to read terminal profile {}: {e}",
                path.display()
            ))),
        }
    }

    /// Save a profile to disk using three-phase commit for crash safety.
    ///
    /// 1. Write to `<path>.tmp`
    /// 2. Rename `<path>` → `<path>.bak` (if exists)
    /// 3. Rename `<path>.tmp` → `<path>`
    /// 4. Remove `<path>.bak`
    pub fn save(&self, path: &Path) -> Result<(), PlatformError> {
        let tmp_path = path.with_extension("tmp");
        let bak_path = path.with_extension("bak");

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                PlatformError::Internal(format!(
                    "failed to create terminal profile dir {}: {e}",
                    parent.display()
                ))
            })?;
        }

        // Phase 1: Write to temp file.
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            PlatformError::Internal(format!("failed to serialize terminal profile: {e}"))
        })?;
        fs::write(&tmp_path, &json).map_err(|e| {
            PlatformError::Internal(format!(
                "failed to write terminal profile tmp {}: {e}",
                tmp_path.display()
            ))
        })?;

        // Phase 2: Rename existing → backup (best-effort).
        if path.exists() {
            let _ = fs::rename(path, &bak_path);
        }

        // Phase 3: Rename temp → final.
        fs::rename(&tmp_path, path).map_err(|e| {
            // Best-effort recovery: restore from backup if it exists.
            // fs::rename returns Err if source doesn't exist; let it fail silently.
            let _ = fs::rename(&bak_path, path);
            PlatformError::Internal(format!(
                "failed to commit terminal profile {}: {e}",
                path.display()
            ))
        })?;

        // Phase 4: Clean up backup.
        let _ = fs::remove_file(&bak_path);

        Ok(())
    }

    /// Create a default profile and save it to disk if no profile exists
    /// for the given terminal. Returns `true` if a new profile was created.
    pub fn ensure_default(base_dir: &Path, terminal_id: &str) -> Result<bool, PlatformError> {
        let path = Self::profile_path(base_dir, terminal_id);
        if path.exists() {
            return Ok(false);
        }
        let profile = TerminalProfile::default();
        profile.save(&path)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn default_profile_has_sensible_values() {
        let p = TerminalProfile::default();
        assert_eq!(p.printer_connection, "auto");
        assert_eq!(p.printer_device_path, "");
        assert_eq!(p.printer_paper_size, "80");
        assert_eq!(p.scanner_device_id, "");
        assert_eq!(p.scanner_input_mode, "auto");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir();
        let path = TerminalProfile::profile_path(dir.path(), "term-001");

        let mut profile = TerminalProfile::default();
        profile.printer_connection = "network".into();
        profile.printer_device_path = "192.168.1.100".into();
        profile.printer_paper_size = "58".into();
        profile.scanner_device_id = "scanner-001".into();
        profile.scanner_input_mode = "serial".into();

        profile.save(&path).unwrap();
        assert!(path.exists());

        let loaded = TerminalProfile::load(&path).unwrap().unwrap();
        assert_eq!(loaded, profile);
    }

    #[test]
    fn load_returns_none_for_missing_file() {
        let dir = temp_dir();
        let path = TerminalProfile::profile_path(dir.path(), "nonexistent");
        assert!(TerminalProfile::load(&path).unwrap().is_none());
    }

    #[test]
    fn profile_path_uses_terminal_id() {
        let path = TerminalProfile::profile_path(Path::new("/data"), "reg-42");
        assert_eq!(path, PathBuf::from("/data/terminal_profiles/reg-42.json"));
    }

    #[test]
    fn ensure_default_creates_profile() {
        let dir = temp_dir();
        let created = TerminalProfile::ensure_default(dir.path(), "new-term").unwrap();
        assert!(created);

        let path = TerminalProfile::profile_path(dir.path(), "new-term");
        assert!(path.exists());

        let loaded = TerminalProfile::load(&path).unwrap().unwrap();
        assert_eq!(loaded, TerminalProfile::default());
    }

    #[test]
    fn ensure_default_is_idempotent() {
        let dir = temp_dir();
        assert!(TerminalProfile::ensure_default(dir.path(), "term").unwrap());
        assert!(!TerminalProfile::ensure_default(dir.path(), "term").unwrap());
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = temp_dir();
        let path = TerminalProfile::profile_path(dir.path(), "term");

        let mut p1 = TerminalProfile::default();
        p1.printer_connection = "usb".into();
        p1.save(&path).unwrap();

        let mut p2 = TerminalProfile::default();
        p2.printer_connection = "network".into();
        p2.save(&path).unwrap();

        let loaded = TerminalProfile::load(&path).unwrap().unwrap();
        assert_eq!(loaded.printer_connection, "network");
    }

    #[test]
    fn three_phase_commit_no_leftover_tmp_or_bak() {
        let dir = temp_dir();
        let path = TerminalProfile::profile_path(dir.path(), "term");

        let profile = TerminalProfile::default();
        profile.save(&path).unwrap();

        // No tmp or bak files should remain.
        assert!(!path.with_extension("tmp").exists());
        assert!(!path.with_extension("bak").exists());
        assert!(path.exists());
    }

    #[test]
    fn serde_roundtrip_preserves_all_fields() {
        let json = r#"{
            "printer_connection": "serial",
            "printer_device_path": "/dev/ttyUSB0",
            "printer_paper_size": "a4",
            "scanner_device_id": "scan-42",
            "scanner_input_mode": "keyboard"
        }"#;

        let profile: TerminalProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.printer_connection, "serial");
        assert_eq!(profile.printer_device_path, "/dev/ttyUSB0");
        assert_eq!(profile.printer_paper_size, "a4");
        assert_eq!(profile.scanner_device_id, "scan-42");
        assert_eq!(profile.scanner_input_mode, "keyboard");

        let out = serde_json::to_string_pretty(&profile).unwrap();
        let roundtrip: TerminalProfile = serde_json::from_str(&out).unwrap();
        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let json = r#"{"printer_connection": "usb"}"#;
        let profile: TerminalProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.printer_connection, "usb");
        assert_eq!(profile.printer_paper_size, "80"); // default
        assert_eq!(profile.scanner_input_mode, "auto"); // default
    }

    #[test]
    fn multiple_terminals_have_separate_profiles() {
        let dir = temp_dir();

        let mut p_a = TerminalProfile::default();
        p_a.printer_connection = "usb".into();
        p_a.save(&TerminalProfile::profile_path(dir.path(), "term-a"))
            .unwrap();

        let mut p_b = TerminalProfile::default();
        p_b.printer_connection = "network".into();
        p_b.save(&TerminalProfile::profile_path(dir.path(), "term-b"))
            .unwrap();

        let loaded_a = TerminalProfile::load(&TerminalProfile::profile_path(dir.path(), "term-a"))
            .unwrap()
            .unwrap();
        let loaded_b = TerminalProfile::load(&TerminalProfile::profile_path(dir.path(), "term-b"))
            .unwrap()
            .unwrap();

        assert_eq!(loaded_a.printer_connection, "usb");
        assert_eq!(loaded_b.printer_connection, "network");
    }
}
