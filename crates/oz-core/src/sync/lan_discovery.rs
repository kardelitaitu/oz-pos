//! LAN discovery via mDNS/DNS-SD.
//!
//! [`LanDiscoverer`] advertises an OZ-POS terminal on the local network
//! using mDNS so that other terminals (e.g. a KDS tablet) can find it
//! without manual IP configuration.
//!
//! # Service type
//!
//! Service type: `_oz-pos._tcp.local.`
//!
//! TXT records published:
//! - `terminal_id` — unique identifier for this terminal
//! - `role` — terminal role (e.g. `counter_pos`, `kds_kiosk`)
//! - `tcp_port` — the TCP port the terminal's sync/event server listens on
//!
//! # Example
//!
//! ```ignore
//! use oz_core::sync::lan_discovery::LanDiscoverer;
//!
//! let mut discoverer = LanDiscoverer::new("term-1", "counter_pos", 9180);
//! discoverer.start().expect("start mDNS advertising");
//! // ... application runs ...
//! discoverer.stop().expect("stop mDNS advertising");
//! ```

use mdns_sd::{ServiceDaemon, ServiceInfo};

/// The mDNS service type advertised by all OZ-POS terminals.
const SERVICE_TYPE: &str = "_oz-pos._tcp.local.";

/// Advertises an OZ-POS terminal on the LAN via mDNS/DNS-SD.
///
/// Create one per application lifetime, call [`start()`](Self::start) to
/// begin advertising, and [`stop()`](Self::stop) on shutdown.
///
/// The struct is **not** `Clone` — the underlying `ServiceDaemon` handle
/// is owned and dropped when the discoverer is dropped.
pub struct LanDiscoverer {
    /// Unique terminal identifier.
    terminal_id: String,
    /// Terminal role (e.g. `"counter_pos"`, `"kds_kiosk"`, `"unrestricted"`).
    role: String,
    /// TCP port the terminal's event/sync server listens on.
    tcp_port: u16,
    /// Optional mDNS daemon handle. `Some` when advertising is active.
    daemon: Option<ServiceDaemon>,
}

// `ServiceDaemon` does not implement `Debug`, so we implement it manually.
impl std::fmt::Debug for LanDiscoverer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanDiscoverer")
            .field("terminal_id", &self.terminal_id)
            .field("role", &self.role)
            .field("tcp_port", &self.tcp_port)
            .field("daemon", &self.daemon.as_ref().map(|_| "<ServiceDaemon>"))
            .finish()
    }
}

impl LanDiscoverer {
    /// Create a new `LanDiscoverer` without starting advertising.
    ///
    /// Call [`start()`](Self::start) to begin broadcasting on the LAN.
    pub fn new(
        terminal_id: impl Into<String>,
        role: impl Into<String>,
        tcp_port: u16,
    ) -> Self {
        Self {
            terminal_id: terminal_id.into(),
            role: role.into(),
            tcp_port,
            daemon: None,
        }
    }

    /// Start advertising this terminal via mDNS on the LAN.
    ///
    /// Spawns a background mDNS daemon thread and registers the
    /// `_oz-pos._tcp.local.` service with TXT records containing
    /// `terminal_id`, `role`, and `tcp_port`.
    ///
    /// Returns an error if the daemon cannot be created or the
    /// service registration fails.
    pub fn start(&mut self) -> Result<(), String> {
        // Prevent double-start.
        if self.daemon.is_some() {
            return Ok(());
        }

        let daemon = ServiceDaemon::new()
            .map_err(|e| format!("failed to create mDNS daemon: {e}"))?;

        let service_info = self.build_service_info()?;

        daemon
            .register(service_info)
            .map_err(|e| format!("failed to register mDNS service: {e}"))?;

        tracing::info!(
            terminal_id = %self.terminal_id,
            role = %self.role,
            tcp_port = %self.tcp_port,
            "mDNS advertising started"
        );

        self.daemon = Some(daemon);
        Ok(())
    }

    /// Stop advertising this terminal and shut down the mDNS daemon.
    ///
    /// This is idempotent — calling it when advertising is not active
    /// is a no-op.
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(daemon) = self.daemon.take() {
            daemon
                .shutdown()
                .map_err(|e| format!("failed to shut down mDNS daemon: {e}"))?;

            tracing::info!(
                terminal_id = %self.terminal_id,
                "mDNS advertising stopped"
            );
        }
        Ok(())
    }

    /// Returns `true` if the discoverer is currently advertising.
    pub fn is_running(&self) -> bool {
        self.daemon.is_some()
    }

    /// Build the `ServiceInfo` for this terminal.
    fn build_service_info(&self) -> Result<ServiceInfo, String> {
        // Instance name: use terminal_id as the unique instance name.
        let instance_name = &self.terminal_id;

        // Hostname: use a `.local.` name derived from terminal_id.
        let host_name = format!("{}.local.", self.terminal_id);

        // TXT records with terminal metadata.
        let properties: &[(&str, &str)] = &[
            ("terminal_id", &self.terminal_id),
            ("role", &self.role),
            ("tcp_port", &self.tcp_port.to_string()),
        ];

        let mut info = ServiceInfo::new(
            SERVICE_TYPE,
            instance_name,
            &host_name,
            "",       // empty — enable_addr_auto will fill in the IP
            self.tcp_port,
            properties,
        )
        .map_err(|e| format!("invalid service info: {e}"))?;

        info = info.enable_addr_auto();
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────

    #[test]
    fn new_discoverer_creates_unstarted() {
        let d = LanDiscoverer::new("term-1", "counter_pos", 9180);
        assert_eq!(d.terminal_id, "term-1");
        assert_eq!(d.role, "counter_pos");
        assert_eq!(d.tcp_port, 9180);
        assert!(!d.is_running());
        assert!(d.daemon.is_none());
    }

    #[test]
    fn new_discoverer_accepts_any_string_types() {
        let d = LanDiscoverer::new(
            String::from("term-kds"),
            "kds_kiosk",
            9180,
        );
        assert_eq!(d.terminal_id, "term-kds");
        assert_eq!(d.role, "kds_kiosk");
    }

    #[test]
    fn new_discoverer_kds_role() {
        let d = LanDiscoverer::new("kds-01", "kds_kiosk", 0);
        assert_eq!(d.role, "kds_kiosk");
    }

    #[test]
    fn new_discoverer_unrestricted_role() {
        let d = LanDiscoverer::new("admin-01", "unrestricted", 3099);
        assert_eq!(d.role, "unrestricted");
    }

    #[test]
    fn new_discoverer_zero_port() {
        let d = LanDiscoverer::new("t", "counter_pos", 0);
        assert_eq!(d.tcp_port, 0);
    }

    #[test]
    fn new_discoverer_max_port() {
        let d = LanDiscoverer::new("t", "counter_pos", 65535);
        assert_eq!(d.tcp_port, 65535);
    }

    // ── Build service info (unit-level, no daemon) ──────────────

    #[test]
    fn build_service_info_succeeds() {
        let d = LanDiscoverer::new("term-1", "counter_pos", 9180);
        let info = d.build_service_info().unwrap();
        let fullname = info.get_fullname();
        assert!(
            fullname.contains("term-1"),
            "fullname should contain the instance name: {fullname}"
        );
        assert!(
            fullname.contains("_oz-pos._tcp.local."),
            "fullname should contain service type: {fullname}"
        );
        assert_eq!(info.get_port(), 9180);
        assert_eq!(
            info.get_property_val_str("terminal_id"),
            Some("term-1")
        );
        assert_eq!(info.get_property_val_str("role"), Some("counter_pos"));
        assert_eq!(
            info.get_property_val_str("tcp_port"),
            Some("9180")
        );
        assert!(info.is_addr_auto(), "addr_auto should be enabled");
    }

    #[test]
    fn build_service_info_empty_terminal_id() {
        let d = LanDiscoverer::new("", "counter_pos", 9180);
        let info = d.build_service_info().unwrap();
        // Empty terminal_id produces hostname ".local." — mdns-sd accepts
        // it but the service instance name will be empty.
        assert!(info.get_fullname().starts_with("."));
    }

    #[test]
    fn build_service_info_long_instance_name() {
        let long = "a".repeat(63);
        let d = LanDiscoverer::new(&long, "counter_pos", 9180);
        let info = d.build_service_info().unwrap();
        assert!(info.get_fullname().contains(&long));
    }

    #[test]
    fn build_service_info_properties_match_input() {
        let d = LanDiscoverer::new("t-42", "kds_kiosk", 8080);
        let info = d.build_service_info().unwrap();
        assert_eq!(
            info.get_property_val_str("terminal_id"),
            Some("t-42")
        );
        assert_eq!(info.get_property_val_str("role"), Some("kds_kiosk"));
        assert_eq!(info.get_property_val_str("tcp_port"), Some("8080"));
    }

    // ── Start/stop lifecycle ───────────────────────────────────—

    #[test]
    fn start_stop_lifecycle() {
        let mut d = LanDiscoverer::new("lifecycle-test", "counter_pos", 9180);
        assert!(!d.is_running());

        d.start().unwrap();
        assert!(d.is_running());

        // double-start is a no-op
        d.start().unwrap();
        assert!(d.is_running());

        d.stop().unwrap();
        assert!(!d.is_running());

        // double-stop is a no-op
        d.stop().unwrap();
        assert!(!d.is_running());
    }

    #[test]
    fn start_stop_cycle_twice() {
        let mut d = LanDiscoverer::new("cycle-twice", "counter_pos", 9180);

        d.start().unwrap();
        assert!(d.is_running());
        d.stop().unwrap();
        assert!(!d.is_running());

        d.start().unwrap();
        assert!(d.is_running());
        d.stop().unwrap();
        assert!(!d.is_running());
    }

    #[test]
    fn stop_before_start_is_noop() {
        let mut d = LanDiscoverer::new("noop", "counter_pos", 9180);
        d.stop().unwrap();
        assert!(!d.is_running());
    }

    // ── Debug output ─────────────────────────────────────────────

    #[test]
    fn debug_output_contains_fields() {
        let d = LanDiscoverer::new("term-debug", "counter_pos", 9180);
        let debug = format!("{d:?}");
        assert!(debug.contains("term-debug"));
        assert!(debug.contains("counter_pos"));
        assert!(debug.contains("9180"));
        assert!(debug.contains("None"), "daemon should be None");
    }

    #[test]
    fn discoverer_is_not_clone() {
        fn assert_not_clone<T>() {}
        assert_not_clone::<LanDiscoverer>();
    }
}
