/*
last audited 25-07-26 by RSA-Agent (platform-core slice E: terminal_profile verified)
crate: platform-core | status: SAFE | lint: CLEAN
findings: clean typed kiosk-profile persistence (save/load/ensure-default); PC-1 INFO: filename interpolates terminal_id without sanitization (line 173) - same hardening note as manager.rs store paths; ids UUID-minted in normal flows
next: sanitize terminal ids (PC-1) | perf: N/A
*/
//! Per-terminal hardware profile — stores printer, scanner, scale, and
//! local preference configuration in per-terminal JSON files under
//! `terminal_profiles/`.
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

/// Per-terminal hardware and local-preference configuration stored in
/// `terminal_profiles/<id>.json`.
///
/// Missing fields in existing (pre-expansion) JSON files fall back to
/// serde defaults so old profiles are forward-compatible.
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

    // ── Scale (ADR #22 Phase 2) ──────────────────────────────
    /// Scale connection type: `"serial"`, `"usb"`, `"none"`.
    #[serde(default = "default_scale_connection")]
    pub scale_connection: String,

    /// Scale device path (e.g. `/dev/ttyUSB0`, `COM3`).
    #[serde(default)]
    pub scale_device_path: String,

    /// Scale baud rate (default 9600).
    #[serde(default = "default_scale_baud_rate")]
    pub scale_baud_rate: u32,

    /// Zero the scale automatically on boot.
    #[serde(default = "default_scale_zero_on_boot")]
    pub scale_zero_on_boot: bool,

    // ── Kitchen printer ───────────────────────────────────
    /// Kitchen printer connection type: `"network"`, `"usb"`, `"serial"`, `"disabled"`.
    #[serde(default = "default_kitchen_printer_connection")]
    pub kitchen_printer_connection: String,

    /// Kitchen printer device path or IP address.
    #[serde(default)]
    pub kitchen_printer_device_path: String,

    // ── Local preferences ────────────────────────────────────
    /// Sound volume percentage (0–100, default 80).
    #[serde(default = "default_sound_volume")]
    pub sound_volume: u32,

    /// Dark mode enabled.
    #[serde(default)]
    pub dark_mode: bool,

    /// Scale auto-zero after each transaction.
    #[serde(default = "default_scale_auto_zero")]
    pub scale_auto_zero: bool,

    // ── Schema version for forward-compatible evolution ──────────
    /// Schema version of this profile (incremented when fields are
    /// added, removed, or renamed). Starts at 1.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
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

fn default_scale_connection() -> String {
    "none".into()
}

fn default_scale_baud_rate() -> u32 {
    9600
}

fn default_scale_zero_on_boot() -> bool {
    false
}

fn default_sound_volume() -> u32 {
    80
}

fn default_kitchen_printer_connection() -> String {
    "disabled".into()
}

fn default_schema_version() -> u32 {
    1
}

fn default_scale_auto_zero() -> bool {
    true
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            printer_connection: default_printer_connection(),
            printer_device_path: String::new(),
            printer_paper_size: default_printer_paper_size(),
            scanner_device_id: String::new(),
            scanner_input_mode: default_scanner_input_mode(),
            scale_connection: default_scale_connection(),
            scale_device_path: String::new(),
            scale_baud_rate: default_scale_baud_rate(),
            scale_zero_on_boot: default_scale_zero_on_boot(),
            kitchen_printer_connection: default_kitchen_printer_connection(),
            kitchen_printer_device_path: String::new(),
            schema_version: default_schema_version(),
            sound_volume: default_sound_volume(),
            dark_mode: false,
            scale_auto_zero: default_scale_auto_zero(),
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
        // Scale defaults
        assert_eq!(p.scale_connection, "none");
        assert_eq!(p.scale_device_path, "");
        assert_eq!(p.scale_baud_rate, 9600);
        assert!(!p.scale_zero_on_boot);
        // Kitchen printer defaults
        assert_eq!(p.kitchen_printer_connection, "disabled");
        assert_eq!(p.kitchen_printer_device_path, "");
        // Schema version
        assert_eq!(p.schema_version, 1);
        // Local-prefs defaults
        assert_eq!(p.sound_volume, 80);
        assert!(!p.dark_mode);
        assert!(p.scale_auto_zero);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir();
        let path = TerminalProfile::profile_path(dir.path(), "term-001");

        let profile = TerminalProfile {
            printer_connection: "network".into(),
            printer_device_path: "192.168.1.100".into(),
            printer_paper_size: "58".into(),
            scanner_device_id: "scanner-001".into(),
            scanner_input_mode: "serial".into(),
            scale_connection: "serial".into(),
            scale_device_path: "COM3".into(),
            scale_baud_rate: 115200,
            scale_zero_on_boot: true,
            kitchen_printer_connection: "network".into(),
            kitchen_printer_device_path: "192.168.1.51".into(),
            schema_version: 1,
            sound_volume: 60,
            dark_mode: true,
            scale_auto_zero: false,
        };

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

        let p1 = TerminalProfile {
            printer_connection: "usb".into(),
            ..Default::default()
        };
        p1.save(&path).unwrap();

        let p2 = TerminalProfile {
            printer_connection: "network".into(),
            ..Default::default()
        };
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
        // New fields get defaults (backward compatible)
        assert_eq!(profile.scale_connection, "none");
        assert_eq!(profile.sound_volume, 80);

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
        assert_eq!(profile.scale_connection, "none"); // new default
        assert!(!profile.dark_mode); // new default
    }

    #[test]
    fn multiple_terminals_have_separate_profiles() {
        let dir = temp_dir();

        let p_a = TerminalProfile {
            printer_connection: "usb".into(),
            ..Default::default()
        };
        p_a.save(&TerminalProfile::profile_path(dir.path(), "term-a"))
            .unwrap();

        let p_b = TerminalProfile {
            printer_connection: "network".into(),
            ..Default::default()
        };
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
