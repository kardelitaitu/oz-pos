# ADR #11: Zero-Downtime VPS Migration Strategy

**Status:** Proposed
**Date:** 2026-07-13
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** vps, migration, dns, routing, fallback, server-redirection, client-config

---

## Context

As the business scales, it will be necessary to migrate the cloud synchronization server (`oz-cloud-server`) to new virtual private servers (VPS) or hosting providers. During such migrations, we must prevent connection disruptions and avoid requiring store staff to manually reconfigure settings on local POS registers. 

We need a structured, automated, and self-healing migration strategy that redirects thousands of active, offline, or waking registers to the new VPS destination with zero manual intervention.

---

## Decision

To support seamless server migrations, we will implement a multi-layered routing and redirection architecture:

### 1. DNS-Based Routing (Primary Migration Path)
- **Rule**: Client registers must never connect to hardcoded IP addresses (e.g., `http://203.0.113.5:3099`). The `server_url` setting on the client must always point to a qualified domain name backed by SSL/TLS (e.g., `https://sync.ozpos.com`).
- **Migration Execution**:
  1. Deploy the new server instance on the target VPS.
  2. Copy the active SQLite tenant sync databases from the old VPS.
  3. Update the DNS `A`/`AAAA` record at the domain provider (e.g., Cloudflare) to point to the new VPS IP address.
  4. Client registers automatically begin resolving to the new VPS on their next sync checkin.

### 2. Server-Led Auto-Redirection (Secondary Fallback)
If the domain name itself must be changed (e.g., moving from `https://sync.ozpos.com` to `https://pos-cloud.com`), the old server will run a temporary "deprecated redirection mode" service:
- **Redirection Protocol**: If a register connects to the old server, the old server returns an HTTP response containing a `server_migrated` error state and the new destination URL:
  ```json
  {
    "error": "server_migrated",
    "new_url": "https://pos-cloud.com"
  }
  ```
- **Client Auto-Update**: The client's `SyncEngine` transport parser will intercept this error, update the local SQLite database settings via `Settings::set_sync_server_url`, and seamlessly reconnect to the new URL on the next cycle.

```rust
if response.error == "server_migrated" {
    Settings::set_sync_server_url(store.conn(), &response.new_url)?;
}
```

### 3. Manual UI Configuration (Final Safety Net)
- The settings UI will retain the `Server URL` input field. If a terminal has been shut down or offline past the deprecation window of the old redirection server, a store manager can manually input the new domain to restore connection.

---

## Consequences

### Positive
- **Zero Friction**: 99.9% of migrations are handled at the DNS layer and require no code execution.
- **Self-Healing**: If domain changes occur, active and waking terminals follow the new address dynamically without administrative support tickets.
- **Robustness**: Older offline devices can always be manually reconfigured as a final fallback.

### Negative
- **Old Server Costs**: The old VPS must be kept active for a brief period (e.g., 15–30 days) to run the redirection service until all active terminals have checked in and migrated.

---

## Related
- ADR #4 — Store-First Tenancy
- ADR #10 — Sync Performance, Ultra-Low-Cost Server, and 3-Month Retention Strategy
- `crates/oz-core/src/settings.rs` — Settings DB getters/setters
- `platform/sync/src/transport.rs` — Client transport parsing
