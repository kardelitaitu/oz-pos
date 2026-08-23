# Engineering Records

Unified registry for architectural decisions (ADRs), audits, verifications, and
system analyses. Files live at their current paths; this index is the single
entry point.

---

## Architectural Decision Records (`docs/decisions/`)

### Numbered ADRs

| # | Title | Status |
|---|-------|--------|
| 1 | [Module System Design](../decisions/2026-01-15-module-system-design.md) | — |
| 2 | [Event Bus Design](../decisions/2026-02-01-event-bus-design.md) | — |
| 3 | [Frontend Restructure](../decisions/2026-03-01-frontend-restructure.md) | — |
| 4 | [Store-First Tenancy & Workspace Type/Instance Architecture](../decisions/2026-07-10-workspace-type-instance-design.md) | — |
| 5 | [Subscription Tier & Entitlement Architecture](../decisions/2026-07-10-subscription-tier-entitlement.md) | Superseded for lineup/quotas |
| 6 | [CRDT Delta Ledger & Offline Sync](../decisions/2026-07-10-crdt-delta-ledger-offline-sync.md) | — |
| 7 | [Data Scope Guard & Query Enforcement](../decisions/2026-07-10-data-scope-guard.md) | — |
| 8 | [Scoped Real-Time Event Bus](../decisions/2026-07-10-scoped-event-bus.md) | — |
| 9 | [License Server Architecture (PocketBase on Northflank)](../decisions/2026-07-10-license-server.md) | Implemented (2026-07-15) |
| 10 | [Sync Performance Strategy](../decisions/2026-07-13-sync-performance-compression-batching.md) | — |
| 11 | [Zero-Downtime VPS Migration Strategy](../decisions/2026-07-13-zero-downtime-vps-migration.md) | — |
| 12 | [Whitelabel Branding System](../decisions/2026-07-15-whitelabel-branding-system.md) | — |
| 13 | [Desktop App Updater](../decisions/2026-07-16-desktop-app-updater.md) | — |
| 14 | [Release Automation](../decisions/2026-07-16-release-automation.md) | — |
| 15 | [Shadow Banding Mitigation — CSS Noise Dithering](../decisions/2026-07-18-shadow-banding-css-dither.md) | — |
| 17 | [KDS Multi-Layout System](../decisions/2026-07-18-kds-multi-layout-system.md) | — |
| 18 | [Multi-Location Inventory](../decisions/2026-07-18-multi-location-inventory.md) | — |
| 19 | [Sale-Deduction Flow for Multi-Location Inventory](../decisions/2026-07-19-sale-deduction-multi-location.md) | Implemented (see [status](../decisions/2026-07-19-sale-deduction-multi-location.status.md)) |
| 20 | [Payment-Capture Ordering — Stock Reservation Before Payment Capture](../decisions/2026-07-19-payment-capture-ordering.md) | Implemented (see [status](../decisions/2026-07-19-payment-capture-ordering.status.md)) |
| 21 | [Sync Conflict Resolution Strategy](../decisions/2026-07-20-sync-conflict-resolution-strategy.md) | — |
| 22 | [Visual Node-Based Store & Workspace Topology Builder](../decisions/2026-07-20-node-based-store-topology-builder.md) | — |
| 23 | [Free Trial Lifecycle & License Activation Workflow](../decisions/2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md) | Re-scoped by `subscription-tiers.md` §4 |
| 30 | [Domain Module Extraction & oz-core Decomposition](../decisions/2026-07-24-domain-module-extraction.md) | — |
| 30 | [React-only UI Decision](../decisions/2026-07-24-react-only-decision.md) | — |
| 31 | [Decentralized UI Feature Module Registration](../decisions/2026-07-24-decentralized-ui-module-registration.md) | — |
| 32 | [DB Layer Extraction (R2) & Platform File Split (R5)](../decisions/2026-07-25-db-extraction-and-platform-split.md) | — |
| 33 | [Panic Policy & Production unwrap/expect Enforcement](../decisions/2026-08-03-panic-policy.md) | — |
| 34 | [Topology Editor as the Business Logic Builder](../decisions/2026-08-07-business-logic-topology-builder.md) | — |
| 34 | [Typed Connection Gating & Live Validation (Implementation)](../decisions/2026-08-08-adr34-typed-connection-gating.md) | Implemented (2026-08-08) |
| 35 | [RBAC — Role Assignments with Branch/Workspace Scopes and User Profile Data](../decisions/2026-08-11-adr35-rbac-role-assignments-user-profile.md) | Accepted (ratified 2026-08-11) |
| 36 | [Retail POS Product Attributes — Cost, Brand, Rack, Notes + Configurable Columns](../decisions/2026-08-11-adr36-retail-product-attributes.md) | Implemented (2026-08-12) |
| 37 | [Product Popularity Index — Weighted Activity Score for Retail Sorting](../decisions/2026-08-11-adr37-product-popularity-index.md) | Implemented (2026-08-12) |
| 38 | [Retail POS Row Context Menu — View Product Images in Browser](../decisions/2026-08-11-adr38-retail-row-context-menu-browser-images.md) | Implemented (2026-08-12) |
| 39 | [Midtrans QRIS Subscription Payments (Phase 2)](../decisions/2026-08-18-adr39-midtrans-subscription-payments.md) | Approved |
| 40 | [Multi-Terminal Peer Model](../decisions/2026-08-20-adr40-multi-terminal-peer-model.md) | Implemented (2026-08-20) |

### Research Notes

- [On-Device ML for Demand Forecasting](../decisions/2026-07-20-ai-demand-forecasting-research.md)
- [Cloud Warehouse Analytics Export](../decisions/2026-07-20-cloud-warehouse-analytics-research.md)
- [CRDT-Based Conflict-Free Replication](../decisions/2026-07-20-crdt-sync-research.md)
- [Voice-Controlled Checkout Research](../decisions/2026-07-20-voice-controlled-checkout-research.md)

### Phased Implementation Docs

**Sync:**
- [Phase 1 diagnostics](../decisions/2026-08-09-local-sync-phase1-diagnostics.md)
- [Phase 2 startup](../decisions/2026-08-09-local-sync-phase2-startup.md)
- [Phase 3 Tauri diagnostics](../decisions/2026-08-09-local-sync-phase3-tauri-diagnostics.md)
- [Phase 4 verification](../decisions/2026-08-09-local-sync-phase4-verification.md)
- [Isolated E2E harness](../decisions/2026-08-09-local-sync-isolated-e2e-harness.md)
- [Status/retry](../decisions/2026-08-09-local-sync-status-retry.md)
- [Auth hardening](../decisions/2026-08-09-sync-auth-hardening.md)
- [Plan gating](../decisions/2026-08-09-sync-plan-gating.md)

**Topology:**
- [Phase 1 branch persistence](../decisions/2026-08-09-topology-phase1-branch-persistence.md)
- [Phase 2 KDS source parity](../decisions/2026-08-09-topology-phase2-kds-source-parity.md)
- [Phase 3 semantic wire parity](../decisions/2026-08-09-topology-phase3-semantic-wire-parity.md)
- [Phase 4 runtime compiler](../decisions/2026-08-09-topology-phase4-runtime-compiler.md)
- [Phase 5 cycle validation](../decisions/2026-08-09-topology-phase5-cycle-validation.md)
- [Phase 6 legacy wire hardening](../decisions/2026-08-09-topology-phase6-legacy-wire-hardening.md)
- [Phase 7 KDS runtime consumer](../decisions/2026-08-09-topology-phase7-kds-runtime-consumer.md)
- [Phase 8 KDS fan-out](../decisions/2026-08-09-topology-phase8-kds-fanout.md)
- [Phase 9 stock routing](../decisions/2026-08-09-topology-phase9-stock-routing.md)
- [Phase 10 multi-warehouse allocation](../decisions/2026-08-09-topology-phase10-multi-warehouse-allocation.md)

---

## Audit Reports (`audit/` + scattered)

### Plan Overview

| Area | Title | Status |
|------|-------|--------|
| Plan | [Audit Plan — July 2026](../../audit/AUDIT_JULY_2026.md) | 36 sectors identified |

### Sector Audits

| # | Sector | Title | Status |
|---|--------|-------|--------|
| 01 | CRM | [CRM Module Audit](../../audit/01-crm-module.md) | Partially remediated |
| 02 | Loyalty | [Loyalty Module Audit](../../audit/02-loyalty-module.md) | Audited; follow-up needed |
| 03 | Reporting | [Reporting Module Audit](../../audit/03-reporting-module.md) | Remediated in part |
| 04 | Currency | [Currency Module Audit](../../audit/04-currency-module.md) | Partially remediated |
| 05 | Tax | [Tax Module Audit](../../audit/05-tax-module.md) | ✅ Fully remediated |
| 06 | Staff | [Staff Module Audit](../../audit/06-staff-module.md) | Partially remediated |
| 07 | Inventory | [Inventory Module Audit](../../audit/07-inventory-module.md) | ✅ Fully remediated |
| 08 | Plugin | [Plugin System Audit](../../audit/08-plugin-system.md) | ✅ Fully remediated |
| 09 | Sync | [Sync Module Audit](../../audit/09-sync-module.md) | ✅ Fully remediated |
| 10 | Products | [Product Management Screen Audit](../../audit/10-product-management-screen.md) | ✅ Fully remediated |
| 11 | Categories | [Category Management Screen Audit](../../audit/11-category-management-screen.md) | ✅ Fully remediated |
| 12 | Customers | [Customer Management Screen Audit](../../audit/12-customer-management-screen.md) | ✅ Fully remediated |
| 13 | Audit Log | [Audit Log Screen Audit](../../audit/13-audit-log-screen.md) | ✅ Fully remediated |
| 14 | Locations | [Location Management Audit](../../audit/14-location-management.md) | ✅ Remediated |
| 15 | Tables | [Table Management Audit](../../audit/15-table-management.md) | ✅ Remediated |
| 16 | Accessibility | [Accessibility Audit](../../audit/16-accessibility.md) | ✅ Fully remediated |
| 17 | Performance | [Performance Audit](../../audit/17-performance.md) | ✅ Fully remediated |
| 18 | Error Handling | [Error-Handling Audit](../../audit/18-error-handling.md) | ✅ Fully remediated |
| 19 | Offline Resilience | [Offline Resilience Audit](../../audit/19-offline-resilience.md) | ✅ Fully remediated |
| 20 | Tablet | [Tablet & Mobile Responsiveness Audit](../../audit/20-tablet-responsiveness.md) | ✅ Fully remediated |
| 21 | Theme | [Theme System Audit](../../audit/21-theme-system.md) | ✅ Fully remediated |
| 22 | Keyboard | [Keyboard Shortcuts Audit](../../audit/22-keyboard-shortcuts.md) | ✅ Fully remediated |
| 23 | Loading States | [Loading States Audit](../../audit/23-loading-states.md) | Audited; needs remediation |
| 24 | Empty States | [Empty States Audit](../../audit/24-empty-states.md) | ✅ Remediated |
| 25 | Rust Backend | [Rust Backend Audit](../../audit/25-rust-backend.md) | ✅ Fully remediated |
| 26 | Docker | [Docker Images Audit](../../audit/26-docker-images.md) | ✅ Fully remediated |
| 27 | CI Pipeline | [CI Pipeline Audit](../../audit/27-ci-pipeline.md) | ✅ Fully remediated |
| 28 | Release Process | [Release Process Audit](../../audit/28-release-process.md) | ✅ Remediated |
| 29 | Migrations | [Database Migrations Audit](../../audit/29-database-migrations.md) | ✅ Fully remediated |
| 30 | Topology (Rust) | [Topology Rust Area Audit](../../audit/30-topology-rust.md) | ⚠️ 8 findings (P2/P3) |
| 31 | Money (Rust) | [Money Primitive & Arithmetic Safety Audit](../../audit/31-money-primitive.md) | ✅ Fully remediated |
| 32 | Money (Frontend) | [Frontend Money Arithmetic Audit](../../audit/32-money-frontend.md) | Partially remediated (1 open) |
| 33 | Tax Rounding | [Tax & Rounding Consistency Audit](../../audit/33-tax-rounding.md) | ✅ Fully remediated |
| 34 | Currency Exchange | [Exchange Rates & Multi-Currency Settlement Audit](../../audit/34-currency-exchange.md) | Partially remediated (6 open) |
| 35 | Payments | [Payment Gateways & Cash Accounting Audit](../../audit/35-payment-cash.md) | ✅ Fully remediated |
| 36 | Residuals | [Multi-Currency Settlement, Tip/Service Persistence, Plugin Money Path](../../audit/36-settlement-residuals.md) | ✅ Remediated |

### Scattered Audit Reports (`docs/`)

- [Retail POS Theming Audit — 2026-07-28](../2026-07-28-retail-pos-theming-audit.md)
- [Retail POS UX Audit — 2026-07-29](../2026-07-29-retail-pos-ux-audit.md)
- [Code Quality Audit — 0.0.14](../code-quality-2026-07-20.md)
- [Database Optimization Audit — 2026-07-20](../database-optimization-2026-07-20.md)
- [Developer Experience Audit — 2026-07-20](../dev-experience-2026-07-20.md)
- [Dev-Mock Reload-State Audit](../dev-mock-state-audit.md)
- [UI State Audit — 0.0.14](../ui-state-audit-2026-07-20.md)
- [Modal & Overlay Audit Checklist](../modal-audit-checklist.md)
- [Shadow Banding Audit — Task List](../TODO-shadow-audit.md)
- [Product Image Storage Plan — Review Summary](../plan-product-images-review.md)
- [Design Exceptions Register](../design-exceptions.md)

### System Analysis / Observability (`docs/observability/`)

| Title | Description |
|-------|-------------|
| [Error Handling Analysis](../observability/error-handling-2026-07-20.md) | Offline-grace, error-handling coverage |
| [Logging Analysis](../observability/logging-2026-07-20.md) | Logging coverage, structure |

---

## Conventions

- **ADR naming:** `YYYY-MM-DD-adrNN-<slug>.md` in `docs/decisions/`
- **Audit naming:** `NN-<slug>.md` in `audit/` (sector number)
- **Status vocabulary:** ADRs use *proposed / accepted / implemented / superseded / re-scoped*; audits use *remediated / partially remediated / audited / open*
- **Adding a new record:** add a row to the appropriate table above and link to the file at its current location