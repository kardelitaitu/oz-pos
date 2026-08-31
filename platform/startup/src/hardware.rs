/*
last audited 31-08-26 by DSH-Agent (hardware bootstrap, new)
crate: platform-startup | status: SAFE | lint: CLEAN
findings: the missing write side of the HAL registry. Both clients built an empty DriverRegistry and nothing ever registered into it, so every hardware command resolved None while the setup wizard could still list devices. Reads the profile the UI already saves; adds no new configuration surface. Deliberately does not wire scanners (nothing looks a scanner up by id - both clients only call scanner_ids() to populate the wizard) or scales (TerminalProfile records a device path, HidWeightScale needs a USB vendor/product pair the profile never captured). Card terminals stay unregistered until edc_terminals CRUD lands, which is the fail-closed behaviour chosen for the EDC path.
next: scale vid/pid in TerminalProfile; edc_terminals CRUD + registration | perf: one indexed row read plus optional file stat at startup
*/
//! Startup hardware registration — the missing write side of the HAL registry.
//!
//! The UI already lets an operator save a [`TerminalProfile`] describing
//! their printer, kitchen printer, scanner and scale. Until now nothing read
//! it back into drivers, so `AppState` held an empty
//! [`DriverRegistry`] and every hardware command resolved `None`.
//!
//! [`load_profile`] reads the profile the same way the settings command
//! does — database first, JSON file as fallback — and
//! [`register_hardware`] maps it onto [`HardwareConfig`] and applies it.
//! [`config_from_profile`] is pure, so the mapping is testable without a
//! database or a device.

use std::path::Path;

use platform_core::terminal_profile::TerminalProfile;
use rusqlite::Connection;

use oz_hal::DriverRegistry;
use oz_hal::bootstrap::{
    BootstrapReport, Connection as HalConnection, HardwareConfig, PrinterConfig,
};
use oz_hal::types::DeviceInfo;

/// Registry id the main receipt printer is looked up under.
///
/// `run_print_receipt_inner` asks for exactly this string; binding the
/// configured printer to any other id leaves receipt printing broken.
pub const MAIN_PRINTER_ID: &str = "default";

/// Registry id the kitchen display system looks its printer up under.
pub const KITCHEN_PRINTER_ID: &str = "kitchen";

/// Describe a configured device for logs and the setup wizard.
///
/// The profile records addressing, not identity, so the transport and its
/// address stand in for vendor and model.
fn configured_info(kind: &str, address: &str) -> DeviceInfo {
    if address.is_empty() {
        DeviceInfo::new("configured", kind, "")
    } else {
        DeviceInfo::new("configured", kind, address)
    }
}

/// Map a saved terminal profile onto the HAL's own hardware description.
///
/// Only what the profile can express becomes a driver:
///
/// | Profile field | Becomes |
/// |---|---|
/// | `printer_connection` + `printer_device_path` | printer under [`MAIN_PRINTER_ID`] |
/// | `kitchen_printer_connection` + path | printer under [`KITCHEN_PRINTER_ID`] |
/// | `scanner_*` | nothing — no code looks a scanner up by id |
/// | `scale_*` | nothing — see [`HardwareConfig`] docs |
///
/// A `"disabled"`, `"none"` or `"auto"` connection yields no entry rather
/// than a placeholder, so an unconfigured kitchen printer does not shadow a
/// working one.
#[must_use]
pub fn config_from_profile(profile: &TerminalProfile) -> HardwareConfig {
    let mut printers = Vec::new();

    if let Some(connection) =
        HalConnection::parse(&profile.printer_connection, &profile.printer_device_path, 0)
    {
        printers.push(PrinterConfig {
            id: MAIN_PRINTER_ID.to_string(),
            info: configured_info("printer", connection.address().unwrap_or("")),
            connection,
        });
    }

    if let Some(connection) = HalConnection::parse(
        &profile.kitchen_printer_connection,
        &profile.kitchen_printer_device_path,
        0,
    ) {
        printers.push(PrinterConfig {
            id: KITCHEN_PRINTER_ID.to_string(),
            info: configured_info("kitchen printer", connection.address().unwrap_or("")),
            connection,
        });
    }

    HardwareConfig {
        printers,
        ..HardwareConfig::default()
    }
}

/// Load the profile for `terminal_id`, preferring the database.
///
/// Mirrors the read order of the `get_hardware_settings` command:
/// `hardware_profiles.profile_json` first, then the crash-safe JSON file
/// under `base_dir/terminal_profiles/`. Returns `None` when neither has a
/// parsable row, which the caller treats as "nothing to register" rather
/// than an error — a first run has no profile yet.
#[must_use]
pub fn load_profile(
    conn: &Connection,
    terminal_id: &str,
    base_dir: &Path,
) -> Option<TerminalProfile> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT profile_json FROM hardware_profiles WHERE terminal_id = ?1",
            rusqlite::params![terminal_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(json) = stored {
        match serde_json::from_str::<TerminalProfile>(&json) {
            Ok(profile) => return Some(profile),
            Err(e) => tracing::warn!(
                terminal_id,
                error = %e,
                "hardware profile in DB is unparsable; falling back to the JSON file"
            ),
        }
    }

    let path = TerminalProfile::profile_path(base_dir, terminal_id);
    match TerminalProfile::load(&path) {
        Ok(profile) => profile,
        Err(e) => {
            tracing::warn!(
                terminal_id,
                error = %e,
                "could not read the hardware profile file"
            );
            None
        }
    }
}

/// Register every device the profile describes.
///
/// Does not call [`DriverRegistry::discover`]: discovery binds hardware-
/// derived ids and probes devices the operator never named. The two are
/// complementary and the caller decides whether to also discover.
pub async fn register_hardware(
    registry: &DriverRegistry,
    profile: &TerminalProfile,
) -> BootstrapReport {
    oz_hal::apply_config(registry, &config_from_profile(profile)).await
}

#[cfg(test)]
#[path = "hardware_tests.rs"]
mod tests;
