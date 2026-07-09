//! Terminal profile domain type — kiosk/kds lockdown per terminal.

use serde::{Deserialize, Serialize};

/// A terminal profile controls which UI is rendered on a POS device.
///
/// Profiles can lock a terminal to a specific screen (e.g. KDS kiosk)
/// and restrict navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProfile {
    /// FK to terminals.id.
    pub terminal_id: String,
    /// Profile type: 'counter_pos', 'kds_kiosk', 'customer_display', 'unrestricted'.
    pub profile_type: String,
    /// Optional locked screen route (e.g. 'kds' for KDS kiosk).
    pub locked_screen: Option<String>,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

/// Well-known terminal profile types.
impl TerminalProfile {
    /// Default unrestricted profile for admin/back-office terminals.
    pub const UNRESTRICTED: &'static str = "unrestricted";
    /// Front counter POS (full POS interface).
    pub const COUNTER_POS: &'static str = "counter_pos";
    /// KDS-only locked-down kiosk (no navigation, force KDS route).
    pub const KDS_KIOSK: &'static str = "kds_kiosk";
    /// Customer-facing secondary display.
    pub const CUSTOMER_DISPLAY: &'static str = "customer_display";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_profile_serde_roundtrip() {
        let profile = TerminalProfile {
            terminal_id: "t1".into(),
            profile_type: "kds_kiosk".into(),
            locked_screen: Some("kds".into()),
            updated_at: "2026-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["terminal_id"], "t1");
        assert_eq!(json["profile_type"], "kds_kiosk");
        assert_eq!(json["locked_screen"], "kds");

        let back: TerminalProfile = serde_json::from_value(json).unwrap();
        assert_eq!(back, profile);
    }

    #[test]
    fn terminal_profile_no_locked_screen() {
        let profile = TerminalProfile {
            terminal_id: "t2".into(),
            profile_type: "counter_pos".into(),
            locked_screen: None,
            updated_at: "2026-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert!(json["locked_screen"].is_null());
    }

    #[test]
    fn terminal_profile_constants() {
        assert_eq!(TerminalProfile::UNRESTRICTED, "unrestricted");
        assert_eq!(TerminalProfile::COUNTER_POS, "counter_pos");
        assert_eq!(TerminalProfile::KDS_KIOSK, "kds_kiosk");
        assert_eq!(TerminalProfile::CUSTOMER_DISPLAY, "customer_display");
    }
}
