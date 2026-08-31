//! Registry bootstrap — config parsing and fail-open registration.

use std::sync::Arc;

use super::*;
use crate::traits::edc::EdcTerminal;

fn info(model: &str) -> DeviceInfo {
    DeviceInfo::new("vendor", model, "SN-1")
}

fn printer(id: &str, connection: Connection) -> PrinterConfig {
    PrinterConfig {
        id: id.into(),
        connection,
        info: info(id),
    }
}

// ── Connection::parse ────────────────────────────────────────────────

#[test]
fn parse_accepts_the_profile_vocabulary() {
    assert_eq!(Connection::parse("usb", "", 0), Some(Connection::Usb));
    assert_eq!(
        Connection::parse("serial", "COM3", 115200),
        Some(Connection::Serial {
            port: "COM3".into(),
            baud: 115200
        })
    );
    assert_eq!(
        Connection::parse("bluetooth", "COM7", 0),
        Some(Connection::Bluetooth {
            port: "COM7".into()
        })
    );
    assert_eq!(
        Connection::parse("network", "10.0.0.5:9100", 0),
        Some(Connection::Network {
            addr: "10.0.0.5:9100".into()
        })
    );
}

#[test]
fn parse_is_case_insensitive_and_trims() {
    // Profiles saved by older builds used "USB" and "Network" verbatim from
    // the UI select; settings_tests.rs still asserts both spellings.
    assert_eq!(Connection::parse("USB", "", 0), Some(Connection::Usb));
    assert_eq!(
        Connection::parse("  Network ", " 10.0.0.5:9100 ", 0),
        Some(Connection::Network {
            addr: "10.0.0.5:9100".into()
        })
    );
    assert_eq!(
        Connection::parse("TCP", "host:1", 0),
        Some(Connection::Network {
            addr: "host:1".into()
        })
    );
}

#[test]
fn parse_treats_absent_disabled_and_auto_as_no_connection() {
    for kind in ["none", "disabled", "auto", "", "wat"] {
        assert_eq!(
            Connection::parse(kind, "COM3", 9600),
            None,
            "{kind:?} must not produce a connection"
        );
    }
    // A named transport with no address is unusable, not a default.
    assert_eq!(Connection::parse("serial", "   ", 9600), None);
    assert_eq!(Connection::parse("network", "", 9600), None);
}

#[test]
fn zero_baud_falls_back_to_the_documented_default() {
    assert_eq!(
        Connection::parse("serial", "COM3", 0),
        Some(Connection::Serial {
            port: "COM3".into(),
            baud: DEFAULT_BAUD
        })
    );
    assert_eq!(DEFAULT_BAUD, 9600);
}

// ── apply_config ─────────────────────────────────────────────────────

#[tokio::test]
async fn empty_config_registers_nothing_and_is_not_an_error() {
    let reg = DriverRegistry::default();
    let report = apply_config(&reg, &HardwareConfig::empty()).await;
    assert!(report.ok());
    assert_eq!(report.registered_count(), 0);
    assert!(reg.printer_ids().await.is_empty());
}

#[tokio::test]
async fn network_printer_lands_under_the_id_the_receipt_path_looks_up() {
    // run_print_receipt_inner does registry.printer("default"); before the
    // bootstrap existed that lookup could never resolve.
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![printer(
            "default",
            Connection::Network {
                addr: "10.0.0.5:9100".into(),
            },
        )],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert_eq!(report.registered, vec!["printer:default".to_string()]);
    let printer = reg.printer("default").await.expect("registered");
    assert_eq!(printer.device_info().model, "default");
}

#[tokio::test]
async fn registering_a_printer_also_gives_it_a_kick_drawer() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![printer(
            "default",
            Connection::Network { addr: "h:1".into() },
        )],
        ..HardwareConfig::default()
    };
    apply_config(&reg, &cfg).await;
    assert!(
        reg.cash_drawer("drawer:kick:default").await.is_some(),
        "the companion drawer must come from the same config entry"
    );
}

#[tokio::test]
async fn kitchen_and_main_printers_coexist_under_their_own_ids() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![
            printer("default", Connection::Network { addr: "a:1".into() }),
            printer(
                "kitchen",
                Connection::Bluetooth {
                    port: "COM8".into(),
                },
            ),
        ],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert_eq!(report.registered_count(), 2);
    assert!(reg.printer("default").await.is_some());
    assert!(reg.printer("kitchen").await.is_some());
}

#[tokio::test]
async fn serial_printer_is_reported_not_silently_rerouted() {
    // There is no serial printer driver. BtReceiptPrinter is serialport-
    // backed and would work mechanically, but naming a wired printer "bt"
    // in every log is worse than admitting the gap.
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![printer(
            "default",
            Connection::Serial {
                port: "COM3".into(),
                baud: 9600,
            },
        )],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert!(!report.ok(), "a serial printer must surface as rejected");
    assert_eq!(report.rejected.len(), 1);
    assert!(report.rejected[0].1.contains("no serial printer driver"));
    assert!(reg.printer("default").await.is_none());
}

#[tokio::test]
async fn one_bad_entry_does_not_stop_the_rest() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![
            printer("default", Connection::Network { addr: "a:1".into() }),
            printer(
                "kitchen",
                Connection::Serial {
                    port: "COM9".into(),
                    baud: 9600,
                },
            ),
            printer("bar", Connection::Network { addr: "c:3".into() }),
        ],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert_eq!(report.registered_count(), 2, "the good printers still land");
    assert_eq!(report.rejected.len(), 1);
    assert!(reg.printer("default").await.is_some());
    assert!(reg.printer("bar").await.is_some());
}

#[tokio::test]
async fn display_and_drawer_register_from_their_ports() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        displays: vec![DisplayConfig {
            id: "pole".into(),
            port: "COM2".into(),
            baud: 9600,
            info: info("Bixolon"),
        }],
        drawers: vec![DrawerConfig {
            id: "till".into(),
            port: "COM4".into(),
            baud: 9600,
            info: info("CD-110"),
        }],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert_eq!(report.registered_count(), 2);
    assert!(reg.display("pole").await.is_some());
    assert!(reg.cash_drawer("till").await.is_some());
}

#[tokio::test]
async fn blank_port_entries_are_skipped_not_registered() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        displays: vec![DisplayConfig {
            id: "pole".into(),
            port: "  ".into(),
            baud: 9600,
            info: info("x"),
        }],
        terminals: vec![TerminalConfig {
            id: "edc".into(),
            connection: TerminalConnection::Wired {
                port: String::new(),
                baud: 9600,
            },
            info: info("x"),
        }],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert!(report.ok(), "unconfigured is not a failure");
    assert_eq!(report.skipped.len(), 2);
    assert_eq!(report.registered_count(), 0);
}

#[tokio::test]
async fn card_terminals_register_wired_and_wireless() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        terminals: vec![
            TerminalConfig {
                id: "edc-front".into(),
                connection: TerminalConnection::Wired {
                    port: "COM3".into(),
                    baud: 115200,
                },
                info: info("iPP320"),
            },
            TerminalConfig {
                id: "edc-mobile".into(),
                connection: TerminalConnection::Wireless {
                    target: crate::drivers::edc::WirelessTarget::Network("10.0.0.9:9500".into()),
                },
                info: info("S920"),
            },
        ],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert_eq!(report.registered_count(), 2);

    let front = reg.terminal("edc-front").await.expect("registered");
    assert_eq!(front.device_info().model, "iPP320");
    // Registered means reachable, not working: the driver is still a stub
    // and must fail closed rather than approve a card.
    assert!(matches!(
        front
            .authorize(oz_core::Money {
                minor_units: 100,
                currency: "USD".parse::<oz_core::Currency>().unwrap(),
            })
            .await,
        Err(crate::error::HalError::Unsupported(_))
    ));
    assert!(reg.terminal("edc-mobile").await.is_some());
}

#[tokio::test]
async fn applying_the_same_config_twice_is_idempotent() {
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![printer(
            "default",
            Connection::Network { addr: "a:1".into() },
        )],
        ..HardwareConfig::default()
    };
    apply_config(&reg, &cfg).await;
    apply_config(&reg, &cfg).await;
    // Overwrite semantics mean one entry per id, not two.
    assert_eq!(reg.printer_ids().await.len(), 1);
    assert_eq!(reg.drawer_ids().await.len(), 1);
}

#[tokio::test]
async fn usb_printer_enumerates_and_never_faults_on_an_empty_bus() {
    // The one branch that touches hardware. On a machine with no printer it
    // must skip rather than reject or panic, and it must never claim a
    // registration the lookup cannot confirm.
    let reg = DriverRegistry::default();
    let cfg = HardwareConfig {
        printers: vec![printer("default", Connection::Usb)],
        ..HardwareConfig::default()
    };
    let report = apply_config(&reg, &cfg).await;
    assert!(report.ok(), "an absent USB printer is not a fault");
    assert_eq!(
        report.registered.is_empty(),
        reg.printer("default").await.is_none(),
        "a reported registration must match the lookup"
    );
}

#[test]
fn report_display_names_the_counts() {
    let report = BootstrapReport {
        registered: vec!["printer:default".into()],
        skipped: vec![],
        rejected: vec![("printer:kitchen".into(), "no serial printer driver".into())],
    };
    let text = report.to_string();
    assert!(text.contains("1 registered"), "{text}");
    assert!(text.contains("1 rejected"), "{text}");
    assert!(text.contains("no serial printer driver"), "{text}");
}

#[test]
fn config_len_counts_every_category() {
    let cfg = HardwareConfig {
        printers: vec![printer("a", Connection::Usb)],
        displays: vec![DisplayConfig {
            id: "d".into(),
            port: "COM1".into(),
            baud: 9600,
            info: info("d"),
        }],
        drawers: vec![],
        terminals: vec![],
    };
    assert_eq!(cfg.len(), 2);
    assert!(!cfg.is_empty());
    assert!(HardwareConfig::empty().is_empty());
}

#[allow(dead_code)]
fn trait_objects_are_send_sync() {
    // The registry stores Arc<dyn Trait>; this would fail to build if a
    // config-registered driver were not usable as one.
    let _printer: Option<Arc<dyn crate::traits::printer::ReceiptPrinter>> = None;
    let _terminal: Option<Arc<dyn EdcTerminal>> = None;
}
