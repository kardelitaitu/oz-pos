# ADR #10: Sync Performance Strategy

**Status:** Active (Strategy — split into 3 phased specs below)
**Date:** 2026-07-13
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** sync, compression, batching, performance, network, low-bandwidth, hosting-cost, sqlite, retention

---

## Context

OZ-POS is designed to support low-end hardware (e.g., Android 9 tablets, Core2Duo PCs) operating in low-bandwidth, unstable, or offline environments.

Additionally, the project has a strict cost target: **the cloud sync server must cost no more than $2–$3 a month to host** (typically a tiny shared-core VPS with 512MB–1GB RAM and 1 shared vCPU), yet be capable of handling synchronization for thousands of active stores.

Currently:
- The client-side sync engine (`platform-sync`) retrieves all pending offline changes from the `offline_queue` SQLite table and sends them in a single massive HTTP request to the cloud sync server (`oz-cloud-server`). If a terminal has been offline for a long period, this causes OOM crashes, high CPU spikes, and request timeouts.
- The sync payload is currently sent uncompressed, consuming significant cellular or satellite data.
- High-frequency active polling by thousands of stores would easily overwhelm a low-tier $2 VPS with connection overhead and concurrent database queries.

## Strategy

This ADR sets the overall direction. The implementation is split into 3 independent phases, each with its own spec, acceptance criteria, and ship timeline:

| Phase | Spec | Focus | Est. effort |
|-------|------|-------|-------------|
| **1** | `docs/specs/_active/p1-sync-batching-compression-retention.md` | Client-side batching (64 KB adaptive), Gzip compression, capped exponential backoff, 90-day retention with `incremental_vacuum` | 2-3 weeks |
| **2** | `docs/specs/_active/p2-sync-priority-concurrency.md` | Sync priority tiers, per-route-group concurrency limits, 2-thread Tokio runtime | 1-2 weeks |
| **3** | `docs/specs/_active/p3-sync-pagination-snapshot-observability.md` | Pull-side pagination, snapshot endpoint, tiered heartbeat, metrics & monitoring | 2-3 weeks |

### Rationale for phasing

- **Phase 1** removes the immediate OOM risk and bandwidth cost. It is the highest-ROI change and can ship standalone.
- **Phase 2** improves fairness and resource isolation, but only matters under load that Phase 1 may already mitigate.
- **Phase 3** addresses edge cases (90-day offline terminals, large-scale observability) that are speculative until real usage data is available.

All phases are backward-compatible with existing clients. The server accepts both legacy (flat array) and batched (array-of-arrays) push formats. Clients can be upgraded independently.

## Sizing & Target Cost

| Workspace Count | Min SSD (90-day) | CPU | RAM | Monthly Cost |
|:---|:---|:---|:---|:---|
| 100 | 30 GB | 1 core | 1-2 GB | $2.50 – $4.00 |
| 500 | 100 GB | 1-2 cores | 2 GB | $6.00 – $10.00 |
| 2,000 | 375 GB | 2 cores | 4 GB | $15.00 – $25.00 |

Target: **100 tenants @ ≤$4/mo**. The $2-$3 VPS constraint applies only to the sub-500-tenant tier.

## Related

- ADR #4 — Store-First Tenancy & Workspace Tenancy
- ADR #6 — CRDT Delta Ledger & Offline Sync
- `docs/specs/_active/p1-sync-batching-compression-retention.md`
- `docs/specs/_active/p2-sync-priority-concurrency.md`
- `docs/specs/_active/p3-sync-pagination-snapshot-observability.md`
