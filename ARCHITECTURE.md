# OZ-POS Architecture

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (4 noted findings, all doc-staleness — architecture itself holds) · F1: "9 modules" → 10 now (loyalty/ added) · F2: "three ADRs" → 29 ADRs in docs/decisions/ · F3: "22+ crates" → 29 workspace members · F4: module-README rule met by 9/10 modules (loyalty/ lacks README) · verified: ui/src/frontend dirs exist, 3 named ADR paths exist, event-bus/module principles intact -->

**Version:** 2.0 (Post-Restructuring)
**Status:** Active — restructuring complete

This document defines the long-term target architecture for OZ-POS. The 6-phase
restructuring (tracked in `RESTRUCTURING.md`) has been completed, migrating the
codebase from a flat monolith to the modular architecture described below.

---

## Core Goals

- **Offline First** — POS works without internet. Cloud is optional, sync is eventual.
- **Modular** — Every feature is a self-contained module with its own backend + frontend.
- **Rust First** — Core business logic, database, and API are all Rust.
- **Multi-Platform** — Windows, Linux, Android Tablet, iPad.
- **Feature Toggle System** — Modules are enabled/disabled at runtime.
- **Multi-Store Ready** — Architecture supports single store, multi-store, and franchise.
- **Sync Ready** — Offline-first with eventual consistency.
- **Long Term Maintainability** — Clear boundaries, no spaghetti.
- **Fast Development** — Module isolation enables parallel teams.

---

## Technology Stack

| Layer            | Technology             |
| ---------------- | ---------------------- |
| Core Backend     | Rust                   |
| UI Shell         | Tauri v2               |
| Frontend         | React → SolidJS*       |
| Database         | SQLite                 |
| API              | Rust (Tauri IPC + HTTP)|
| State Management | Solid Store*           |
| Build System     | Cargo Workspace        |
| Testing          | Rust Test + Playwright |
| Documentation    | Markdown + ADRs        |

*\*Front-end migration from React to SolidJS is planned but not yet started.
The architecture is designed to be framework-agnostic at the module level.*

---

## Architecture Principles

### Rule 1 — Modules Own Business Logic

Modules are the atomic unit of business capability. Each module owns its entire
vertical slice: database models, services, API routes, and UI pages.

Inventory owns inventory logic.
Sales owns sales logic.
CRM owns CRM logic.

### Rule 2 — No Direct Module-to-Module Calls

Modules communicate exclusively through an event bus. This prevents coupling
and enables independent testing, loading, and replacement.

```
  Sales              Inventory
    |                    |
    ▼                    ▼
  ┌──────────────────────────┐
  │       Event Bus          │
  └──────────────────────────┘
    |                    |
    ▼                    ▼
  sale.completed      stock.updated
```

### Rule 3 — Platform Provides Infrastructure Only

The platform layer (kernel, core, sync, etc.) contains zero business logic.
It provides infrastructure that modules consume.

### Rule 4 — Integrations Are Adapters

External service integrations (Stripe, Midtrans, Epson Printer, WhatsApp) are
thin adapters with no business logic. Business rules live in modules.

### Rule 5 — SQLite Is the Source of Truth

Cloud is optional. The POS must continue working without internet. SQLite is
the authoritative data store. Cloud sync is eventual and non-blocking.

---

## Repository Structure (Target — Long-Term Vision)

> ⚠️ The diagram below shows the **long-term target architecture** the codebase
> is evolving toward. It is NOT the current state — many of these directories
> (`integrations/`, top-level `frontend/`, `tooling/`, `config/`, `tests/`,
> and modules like `loyalty`, `accounting`, `purchasing`, `warehouse`,
> `restaurant`, `ecommerce`) do not yet exist. See the **Project Layout
> (Post-Restructuring) — Current State** section below for the actual
> current directory structure.

```
oz-pos/
│
├─ apps/              Deployable applications
│   ├─ desktop-client/  Windows + Linux (keyboard/mouse)
│   └─ tablet-client/   Android + iPad (touch)
│
├─ platform/          System infrastructure
│   ├─ kernel/         Module system (load, unload, lifecycle)
│   ├─ core/           Shared services (auth, rbac, database, etc.)
│   ├─ sync/           Offline-first sync engine
│   ├─ api/            Backend HTTP API (today: crates/oz-api/)
│   └─ ui/             Frontend infrastructure (today: ui/src/frontend/)
│
├─ modules/           Business features (top 10 shown; today 9 exist)
│   ├─ sales/
│   ├─ inventory/
│   ├─ crm/
│   ├─ loyalty/        ← planned
│   ├─ reporting/
│   ├─ accounting/     ← planned
│   ├─ purchasing/     ← planned
│   ├─ warehouse/      ← planned
│   ├─ restaurant/     ← planned
│   └─ ecommerce/      ← planned
│
├─ integrations/      External adapters (planned; today in crates/oz-hal, crates/oz-payment)
│   ├─ payments/       (cash, stripe, midtrans, xendit)
│   ├─ hardware/       (printers, scanners, cash-drawers, scales)
│   ├─ messaging/      (whatsapp, email, telegram)
│   ├─ shipping/
│   └─ tax/
│
├─ foundation/        Reusable zero-business-logic code
│   ├─ contracts/      Core traits (Module, Service, EventHandler)
│   ├─ dto/            Shared DTOs                 (planned)
│   ├─ value-objects/  Money, Currency, Email, etc. (today: money.rs)
│   ├─ errors/         Shared error types
│   ├─ validation/     Validation utilities         (planned)
│   ├─ enums/          Shared enumerations
│   ├─ constants/      Shared constants             (planned)
│   └─ utils/          Pure utility functions       (planned)
│
├─ frontend/          Shared frontend infrastructure (today: ui/src/frontend/)
│   ├─ shell/          App host (layout, sidebar, routing)
│   ├─ shared/         Reusable UI components
│   ├─ desktop/        Desktop-specific layouts
│   ├─ tablet/         Tablet-specific layouts
│   ├─ widgets/        Dashboard widget framework
│   └─ themes/         Branding and theming
│
├─ tooling/           Build tools, scaffolding, generators (planned)
├─ config/            Shared configuration           (planned)
├─ docs/              Documentation + ADRs
│   └─ decisions/      Architecture Decision Records
├─ assets/            Icons, fonts, branding         (exists at root)
└─ tests/             End-to-end and integration tests (planned)
```

---

## Module Structure

Every module follows the same structure, owning both backend and frontend:

```
modules/inventory/
│
├─ manifest.json       Module metadata (id, name, version, dependencies)
├─ migrations/         SQLite migrations
├─ src/                Rust backend
│   ├─ services/        Business logic
│   ├─ repositories/    Database access
│   ├─ models/          Domain entities
│   ├─ events/          Published event types
│   ├─ permissions/     Module-specific permission keys
│   └─ lib.rs           Module entry point
├─ ui/                 Frontend
│   ├─ pages/           Full-page routes
│   ├─ components/      Module-specific components
│   ├─ routes/          Route definitions
│   └─ widgets/         Dashboard widgets
└─ tests/              Module-specific tests
```

### Module Manifest Example

```json
{
  "id": "inventory",
  "name": "Inventory",
  "version": "1.0.0",
  "dependencies": []
}
```

---

## Platform Core Services

```
platform/core/
│
├─ auth/              Authentication (login, logout, sessions, password reset)
├─ rbac/              Authorization (roles, permissions, policies)
├─ database/          SQLite management (connection, transactions, migrations)
├─ settings/          Application configuration (store name, tax, currency)
├─ logging/           System logs (errors, warnings, performance)
├─ audit/             Immutable audit trail (refund issued, price changed)
├─ storage/           File management (product images, reports, exports)
├─ cache/             In-memory caching layer
├─ notifications/     Notification dispatching (WhatsApp, email, push)
├─ scheduler/         Background jobs (backup, sync, report generation)
├─ localization/      i18n infrastructure
└─ tenancy/           Multi-tenant support (tenant, store, terminal)
```

### Permission Examples

Permissions follow a `domain.action` pattern:
- `inventory.read`
- `inventory.write`
- `sales.refund`

---

## Event Bus

The event bus is the critical architectural boundary. Modules publish events;
other modules subscribe. No module ever imports another module directly.

### Event Flow Example

```
Sale Completed
     │
     ▼
Event Bus
     │
     ├── Inventory   → stock.updated
     ├── CRM         → customer.history.updated
     ├── Loyalty     → points.awarded
     └── Reporting   → report.data.changed
```

### Core Traits

```rust
trait Module {
    fn id(&self) -> &str;
    fn load(&mut self, kernel: &Kernel) -> Result<()>;
    fn unload(&mut self) -> Result<()>;
}

trait Service {
    fn name(&self) -> &str;
}

trait EventHandler {
    fn handle(&self, event: &Event) -> Result<()>;
}

trait Integration {
    fn name(&self) -> &str;
}
```

---

## Module Loading Flow

```
Application Start
     │
     ▼
Load Settings
     │
     ▼
Load Enabled Modules
     │
     ▼
Register Routes
     │
     ▼
Register Menus
     │
     ▼
Register Widgets
     │
     ▼
Start Application
```

Feature toggles are persisted in settings and control which modules load:

```
Settings → Modules → Inventory  [ON]
                      CRM        [OFF]
                      Loyalty    [OFF]
                      Reporting  [ON]
     │
     ▼
Save → Restart → Load Enabled Modules Only
```

---

## Project Layout (Post-Restructuring) — Current State

The codebase has been restructured from a flat monolith into the modular architecture
defined above. This layout shows the **actual current state** after all 6
restructuring phases. For the long-term target vision (with `integrations/`,
top-level `frontend/`, additional modules, etc.), see the **Repository
Structure (Target — Long-Term Vision)** section above.

```
oz-pos/
│
├─ apps/              Deployable applications
│   ├─ desktop-client/  Windows + Linux (moved from src-tauri/)
│   │   └─ src/
│   │       ├─ commands/  IPC command handlers
│   │       ├─ error.rs
│   │       ├─ lib.rs     (uses platform_startup::init_module_system)
│   │       ├─ main.rs
│   │       └─ state.rs
│   └─ tablet-client/   Android + iPad (touch-optimized shell)
│       └─ src/
│           ├─ commands/  (shared with desktop-client)
│           └─ same structure
│
├─ platform/          System infrastructure
│   ├─ core/           Shared services (database, settings, auth stubs)
│   ├─ kernel/         Module system lifecycle (register → load → start → stop)
│   ├─ startup/        Shared startup: module registration + event wiring
│   └─ sync/           Offline-first sync engine (queue, transport, replication, LWW conflict)
│
├─ modules/           Business features (9 modules)
│   ├─ sales/          Point-of-sale (core cart, checkout, sales history)
│   ├─ inventory/      Product catalog, stock management
│   ├─ crm/            Customer management, loyalty
│   ├─ tax/            Tax rate configuration
│   ├─ settings/       Feature toggles, store configuration, sync settings
│   ├─ staff/          Employee management, roles
│   ├─ reporting/      Dashboard widgets, sales reports
│   ├─ terminal/       POS terminal management
│   └─ currency/       Multi-currency + exchange rates
│
├─ crates/            Low-level utility crates
│   ├─ oz-core/        Database migrations, domain types, Store, sync_client, events
│   ├─ oz-api/         HTTP API server (axum)
│   ├─ oz-cli/         CLI tool
│   ├─ oz-hal/         Hardware abstraction layer (printers, scanners, cash drawers)
│   ├─ oz-logging/     Structured logging setup
│   ├─ oz-lua/         Lua scripting integration
│   ├─ oz-payment/     Card payment processing
│   ├─ oz-reporting/   Report generation (PDF, CSV)
│   └─ oz-security/    Auth, hashing, encryption
│
├─ foundation/        Reusable zero-business-logic code
│   ├─ contracts/      Core traits (Module, Service, EventHandler)
│   ├─ errors/         Shared error types (MoneyError, SkuError)
│   ├─ enums/          Shared enumerations (SaleStatus, PaymentMethod)
│   └─ money.rs        Money, Currency value objects
│
├─ ui/                Frontend (React/TypeScript)
│   ├─ src/
│   │   ├─ api/         Per-domain API files (sales.ts, products.ts, etc.)
│   │   ├─ features/    Feature screens (sales, products, customers, etc.)
│   │   ├─ frontend/    Shell, shared components, themes, registries
│   │   ├─ platform/    UI registries (page, menu, widget)
│   │   ├─ locales/     Fluent i18n (domain-split .ftl files)
│   │   └─ main.tsx     Entry point with registrations
│   └─ package.json
│
├─ docs/
│   ├─ decisions/      ADRs (module-system, event-bus, frontend-restructure)
│   └─ specs/          Module manifest format spec
│
├─ ARCHITECTURE.md    This file
├─ RESTRUCTURING.md    Phase tracking checklist
├─ agents.md           AI agent configuration
└─ Cargo.toml          Workspace definition (22+ crates)
```

---

## Migration Roadmap (Complete ✅)

All 6 restructuring phases have been completed. See `RESTRUCTURING.md` for
the detailed task checklist.

### Phase 1 — Foundation ✅
- [x] Rust workspace with crate separation
- [x] Design tokens and component library
- [x] Shared modal/toast/empty-state components
- [x] Fluent localization infrastructure

### Phase 2 — Module Extraction ✅
- [x] Define `Module` trait and kernel skeleton (`foundation/src/contracts.rs`)
- [x] Extract `foundation/` crate (Money, Currency, contracts, errors, enums)
- [x] Create `platform/core/` (database, auth, rbac, settings stubs)
- [x] Create `platform/kernel/` (Kernel struct, lifecycle, dependency resolution)
- [x] Create 9 business modules (sales, inventory, crm, tax, settings, staff, reporting, terminal, currency)
- [x] Wire all modules into both desktop + tablet clients via shared startup

### Phase 3 — Event Bus ✅
- [x] Implement in-process event bus in `platform/kernel/`
- [x] Wire `sale.completed` → inventory stock update + CRM history + audit log + reporting
- [x] Wire `product.created` → audit log + sync enqueuer
- [x] Wire `stock.adjusted` → audit log + sync enqueuer
- [x] Remove all direct module-to-module Store calls

### Phase 4 — Frontend Infrastructure ✅
- [x] Split `api/pos.ts` into 12 per-domain API files
- [x] Create `frontend/shell/` (AppLayout, AppShell extracted from App.tsx)
- [x] Create `frontend/shared/` (Button, Card, Modal, etc. from components/)
- [x] Create `frontend/themes/` (tokens, components, reset CSS from styles/)
- [x] Build page-registry, menu-registry, widget-registry
- [x] Refactor `App.tsx` to render from registries with feature gating
- [x] Split `en-US.ftl` into 12 per-domain Fluent files

### Phase 5 — Tablet Client ✅
- [x] Create `apps/tablet-client/` — Tauri v2 mobile target (oz-pos-tablet)
- [x] Move `src-tauri/` → `apps/desktop-client/`
- [x] Build touch-optimized shell (bottom nav, larger hit targets)
- [x] Create `platform/startup/` — shared module registration + event wiring

### Phase 6 — Sync Engine ✅
- [x] Implement `platform/sync/` with queue, transport, push/pull replication
- [x] LWW conflict resolution (server-authoritative on tie)
- [x] Wire sync into sales module (SaleSyncEnqueuer)
- [x] Wire sync into inventory module (InventorySyncEnqueuer)
- [x] Integration tests (4 tests: single item, empty queue, multiple items, server error)

---

## Documentation Requirements

Every module must contain:
- `README.md` — Purpose, usage, configuration
- `CHANGELOG.md` — Version history

Every architectural change must create an Architecture Decision Record (ADR).
The three ADRs written to date are:
```
docs/decisions/2026-01-15-module-system-design.md
docs/decisions/2026-02-01-event-bus-design.md
docs/decisions/2026-03-01-frontend-restructure.md
```

---

## Non-Negotiable Rules

1. No business logic in platform.
2. No business logic in integrations.
3. No direct module-to-module calls.
4. Events first.
5. SQLite first.
6. Offline first.
7. Module owns backend and frontend.
8. Shared code contains no business logic.
9. Every module is independently testable.
10. Documentation updated with every architecture change.

---

*This document is a living specification. Phase boundaries are guidelines,
not hard deadlines. Every PR should move the codebase closer to the target
architecture.*
