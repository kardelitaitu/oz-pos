//! Startup hardware registration — profile mapping and profile loading.

use std::path::Path;

use platform_core::terminal_profile::TerminalProfile;
use rusqlite::Connection;

use super::*;
use oz_hal::bootstrap::Connection as HalConnection;

/// Build a profile from a partial JSON object. Every field carries a serde
/// default, so this also pins forward-compatibility: an old profile missing
/// newer keys must still load.
fn profile(json: &str) -> TerminalProfile {
    serde_json::from_str(json).expect("profile fields all have serde defaults")
}

fn create_profiles_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE hardware_profiles (
             terminal_id    TEXT PRIMARY KEY,
             profile_json   TEXT NOT NULL,
             schema_version INTEGER NOT NULL DEFAULT 1,
             updated_at     TEXT NOT NULL DEFAULT ''
         );",
    )
    .expect("create table");
}

fn put_profile(conn: &Connection, terminal_id: &str, json: &str) {
    conn.execute(
        "INSERT INTO hardware_profiles (terminal_id, profile_json) VALUES (?1, ?2)",
        rusqlite::params![terminal_id, json],
    )
    .expect("insert profile");
}

// ── config_from_profile ──────────────────────────────────────────────

#[test]
fn a_network_printer_becomes_the_id_the_receipt_path_looks_up() {
    let cfg = config_from_profile(&profile(
        r#"{"printer_connection":"network","printer_device_path":"10.0.0.5:9100"}"#,
    ));
    assert_eq!(cfg.printers.len(), 1);
    assert_eq!(cfg.printers[0].id, MAIN_PRINTER_ID);
    assert_eq!(
        cfg.printers[0].connection,
        HalConnection::Network {
            addr: "10.0.0.5:9100".into()
        }
    );
}

#[test]
fn kitchen_and_main_printers_get_distinct_ids() {
    let cfg = config_from_profile(&profile(
        r#"{
            "printer_connection":"network","printer_device_path":"10.0.0.5:9100",
            "kitchen_printer_connection":"network","kitchen_printer_device_path":"10.0.0.6:9100"
        }"#,
    ));
    assert_eq!(cfg.printers.len(), 2);
    assert_eq!(cfg.printers[0].id, MAIN_PRINTER_ID);
    assert_eq!(cfg.printers[1].id, KITCHEN_PRINTER_ID);
}

#[test]
fn a_disabled_kitchen_printer_does_not_shadow_a_working_one() {
    let cfg = config_from_profile(&profile(
        r#"{
            "printer_connection":"network","printer_device_path":"10.0.0.5:9100",
            "kitchen_printer_connection":"disabled","kitchen_printer_device_path":"10.0.0.6:9100"
        }"#,
    ));
    assert_eq!(cfg.printers.len(), 1);
    assert_eq!(cfg.printers[0].id, MAIN_PRINTER_ID);
}

#[test]
fn auto_and_none_and_blank_are_not_connections() {
    // "auto" is the profile default. Treating it as a connection would
    // register a device the operator never chose.
    for kind in ["auto", "none", "disabled", "", "usb-but-typed-wrong"] {
        let cfg = config_from_profile(&profile(&format!(
            r#"{{"printer_connection":"{kind}","printer_device_path":"10.0.0.5:9100"}}"#
        )));
        assert_eq!(
            cfg.printers.len(),
            0,
            "{kind:?} must not register a printer"
        );
    }
}

#[test]
fn usb_is_a_connection_even_without_an_address() {
    // The HAL resolves it by enumerating the bus, so the profile saying
    // "usb" with no path is meaningful and must survive the mapping.
    let cfg = config_from_profile(&profile(r#"{"printer_connection":"usb"}"#));
    assert_eq!(cfg.printers.len(), 1);
    assert_eq!(cfg.printers[0].connection, HalConnection::Usb);
}

#[test]
fn serial_maps_to_serial_so_the_hal_can_report_it() {
    // The mapping does not decide what is supported; it forwards the
    // operator's intent and apply_config reports the missing driver.
    let cfg = config_from_profile(&profile(
        r#"{"printer_connection":"serial","printer_device_path":"COM3"}"#,
    ));
    assert_eq!(
        cfg.printers[0].connection,
        HalConnection::Serial {
            port: "COM3".into(),
            baud: 9600
        }
    );
}

#[test]
fn an_unconfigured_profile_describes_no_devices() {
    let cfg = config_from_profile(&profile("{}"));
    // Default printer_connection is "auto", default kitchen is "network"
    // with an empty path; neither is usable.
    assert!(cfg.is_empty(), "{cfg:?}");
}

#[test]
fn scanner_and_scale_settings_produce_no_drivers() {
    // Documented gaps, asserted so they cannot be "fixed" by guessing an id.
    let cfg = config_from_profile(&profile(
        r#"{
            "scanner_device_id":"vid:0x0c2e pid:0x0200","scanner_input_mode":"keyboard",
            "scale_connection":"serial","scale_device_path":"COM5","scale_baud_rate":9600
        }"#,
    ));
    assert!(cfg.is_empty());
}

#[test]
fn the_info_recorded_names_the_transport_and_address() {
    let cfg = config_from_profile(&profile(
        r#"{"printer_connection":"network","printer_device_path":"10.0.0.5:9100"}"#,
    ));
    let info = &cfg.printers[0].info;
    assert_eq!(info.vendor, "configured");
    assert_eq!(info.model, "printer");
    assert_eq!(info.serial, "10.0.0.5:9100");
}

// ── load_profile ─────────────────────────────────────────────────────

#[test]
fn the_database_row_wins_over_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = Connection::open_in_memory().expect("in-memory db");
    create_profiles_table(&conn);
    put_profile(
        &conn,
        "term-1",
        r#"{"printer_connection":"network","printer_device_path":"1.1.1.1:9100"}"#,
    );

    // A file that disagrees must lose.
    let path = TerminalProfile::profile_path(dir.path(), "term-1");
    let from_file = profile(r#"{"printer_connection":"usb"}"#);
    from_file.save(&path).expect("save file profile");

    let loaded = load_profile(&conn, "term-1", dir.path()).expect("db row loads");
    assert_eq!(loaded.printer_device_path, "1.1.1.1:9100");
}

#[test]
fn a_missing_database_row_falls_back_to_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = Connection::open_in_memory().expect("in-memory db");
    create_profiles_table(&conn);

    let path = TerminalProfile::profile_path(dir.path(), "term-2");
    profile(r#"{"printer_connection":"serial","printer_device_path":"COM4"}"#)
        .save(&path)
        .expect("save file profile");

    let loaded = load_profile(&conn, "term-2", dir.path()).expect("file loads");
    assert_eq!(loaded.printer_connection, "serial");
    assert_eq!(loaded.printer_device_path, "COM4");
}

#[test]
fn an_unparsable_database_row_falls_back_instead_of_failing_startup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = Connection::open_in_memory().expect("in-memory db");
    create_profiles_table(&conn);
    put_profile(&conn, "term-3", "{ not json");

    let path = TerminalProfile::profile_path(dir.path(), "term-3");
    profile(r#"{"printer_connection":"usb"}"#)
        .save(&path)
        .expect("save file profile");

    let loaded = load_profile(&conn, "term-3", dir.path()).expect("falls back to the file");
    assert_eq!(loaded.printer_connection, "usb");
}

#[test]
fn no_profile_anywhere_is_none_not_a_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = Connection::open_in_memory().expect("in-memory db");
    create_profiles_table(&conn);
    assert!(load_profile(&conn, "ghost", dir.path()).is_none());
}

#[test]
fn a_missing_table_does_not_panic_either() {
    // First run before migrations, or a store db that never got the table.
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = Connection::open_in_memory().expect("in-memory db");
    assert!(load_profile(&conn, "term-1", dir.path()).is_none());
}

// ── register_hardware ────────────────────────────────────────────────

#[tokio::test]
async fn register_hardware_makes_the_default_printer_resolvable() {
    let registry = DriverRegistry::default();
    let prof = profile(r#"{"printer_connection":"network","printer_device_path":"10.0.0.5:9100"}"#);
    let report = register_hardware(&registry, &prof).await;
    assert!(report.ok(), "{report}");
    assert_eq!(report.registered_count(), 1);
    assert!(
        registry.printer(MAIN_PRINTER_ID).await.is_some(),
        "the whole point: this lookup used to always return None"
    );
}

#[tokio::test]
async fn an_unconfigured_profile_registers_nothing_and_reports_no_failure() {
    let registry = DriverRegistry::default();
    let report = register_hardware(&registry, &profile("{}")).await;
    assert!(report.ok());
    assert_eq!(report.registered_count(), 0);
    assert!(registry.printer_ids().await.is_empty());
}

#[allow(dead_code)]
fn path_arg_is_a_str_slice() {
    // Keeps the signature honest: base_dir is a &Path, not a String.
    fn _takes(_: &Path) {}
}
