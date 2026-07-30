# Improvement Opportunities — July 29, 2026 Audits

> From two feature audits: **KDS (Kitchen Display System)** and **ADR #22 Workspace Settings**.
> 3324/3324 tests, zero type errors, zero clippy warnings, bundle parity clean.

---

## 1. KDS — Production Kitchen Readiness

### 🔴 Tier 1 — Blockers

- [x] **1a. No real-time push (polling only):** KDS polls at 2-30s intervals. A production kitchen needs sub-second delivery. Add a Tauri WebSocket or SSE stream so the POS pushes orders to KDS instantly rather than waiting for the next poll tick.
  - _Files:_ `ui/src/features/kds/KdsScreen.tsx` (polling useEffect), `crates/oz-core/src/db/kds.rs` (queue query), needs new event-bus emitter on `create_kds_order`.
  - _Effort:_ Medium (new backend event + frontend stream subscription)

- [x] **1b. Table number is an `as unknown` hack:** `KdsTicketCard.tsx` reads table numbers via `(order as unknown as Record<string, unknown>)['table_number']`. The `KdsOrder` Rust struct and DB schema have no `table_number` field.
  - _Files:_ `ui/src/features/kds/components/KdsTicketCard.tsx:80-82`, `crates/oz-core/src/kds.rs` (KdsOrder struct), `crates/oz-core/migrations/032_kds_orders.sql`, `crates/oz-core/src/db/kds.rs` (row_to_kds_order)
  - _Effort:_ Small (add column + field + wire through)

- [x] **1c. Auto-acknowledge toggle does nothing:** `KdsSettingsPanel` stores `autoAcknowledge` in component state, but nothing reads it to auto-advance tickets. The toggle is a placebo.
  - _Files:_ `ui/src/features/kds/KdsSettingsPanel.tsx`, `ui/src/features/kds/KdsScreen.tsx`
  - _Effort:_ Small (add `useEffect` timer in `KdsScreen` that calls `advanceStatus` after configurable delay when enabled)

### 🟠 Tier 2 — Significant Gaps

- [x] **2a. Phase 1 — Schema + Pipeline** ✅ (committed `409c471b`)
  - [x] Migration 105: `kds_line_items` table
  - [x] Migration 106: `sale_lines` enriched with `course` + `modifiers_json`
  - [x] Rust types: `KdsLineItem`, `KdsModifier`, `CreateKdsLineItemInput`
  - [x] DB methods: `create_kds_line_items`, `get_kds_order_lines`
  - [x] Pipeline: `complete_sale_to_kds` persists structured line items
  - [x] `get_kds_order_lines_scoped` Tauri command + TS API wrapper
  - [x] **Phase 2 — KDS front-end display** (course-grouped ticket cards)
  - [x] **Phase 3 — POS cart input** (course selector + modifier UI)

- [x] **2b. No recall / history view:** Once a ticket advances to "served", it vanishes from the queue. Kitchen staff can't pull up completed orders.
  - _Files:_ `ui/src/features/kds/KdsScreen.tsx` (queue only shows pending/preparing/ready)
  - _Effort:_ Medium (add history tab/panel, `list_kds_orders` API already exists)

- [x] **2c. No priority / rush flag:** FOH can't signal an urgent ticket that visually escalates above normal SLA.
  - _Files:_ `crates/oz-core/src/kds.rs` (KdsOrder struct needs `priority` field), `crates/oz-core/migrations/032_kds_orders.sql`, `ui/src/features/kds/components/KdsTicketCard.tsx`
  - _Effort:_ Small (add boolean + CSS class)

- [x] **2d. No keyboard shortcuts:** Kitchen staff must tap individual tickets on a tablet. Add number keys (1-9) to select, Space to advance, arrows to navigate.
  - _Files:_ `ui/src/features/kds/KdsScreen.tsx` (add `onKeyDown` handler)
  - _Effort:_ Small

- [x] **2e. Hardcoded dark theme (not tokenized):** KDS CSS uses private `--kds-bg: #1a1a2e` etc. Doesn't respond to global light/dark theme toggle. A KDS tablet in a bright daytime kitchen can't switch to light mode.
  - _Files:_ `ui/src/features/kds/KdsScreen.css:3-26` (all `--kds-*` custom properties)
  - _Effort:_ Medium (redefine `--kds-*` as `var(--color-*)` references per theme)

### 🟡 Tier 3 — Polish

- [x] **3a. No zone-switching UI on the KDS screen:** The `kdsZone` preference exists in `useKdsPreferences` but there's no visible toggle. A shared tablet covering "grill" and "fry" stations can't switch zones.
  - _Files:_ `ui/src/features/kds/KdsScreen.tsx`, `ui/src/features/kds/hooks/useKdsPreferences.ts`
  - _Effort:_ Small (add zone chip/tab bar in header)

- [x] **3b. No offline resilience:** If backend is unreachable, KDS shows an error banner with stale queue.
  - _Solution:_ localStorage cache + retry queue + optimistic updates — download-and-go, no Service Worker needed (see `useKdsOffline` hook)
  - _Effort:_ Large (localStorage cache + retry queue + optimistic UI)

- [x] **3c. No bump bar / hardware integration:** Physical USB bump bars and parallel thermal printer chits are standard in production kitchens. HAL has printer drivers but they're not wired to KDS.
  - _Files:_ `crates/oz-hal/src/drivers/kds_chit.rs`, `apps/desktop-client/src/commands/kds.rs`
  - _Effort:_ Medium (HAL input driver + KDS event wiring)

- [x] **3d. No "order up" voice callout:** When a ticket hits "ready", there should be a distinct TTS "Order 42 up!" rather than just the red-threshold chime.
  - _Files:_ `ui/src/features/kds/hooks/useNewTicketSound.ts`, `ui/src/frontend/shared/useSound.ts`
  - _Effort:_ Small (add TTS call in KdsScreen `advanceStatus` when new status is "ready")

- [x] **3e. Item-level status (all-day view):** Tickets with per-line-item cooked/served/bumped status via `item_status` column on `kds_line_items`. ✅ (`ad406c4c`)
  - [x] DB method: `update_kds_line_item_status` — validates status, auto-sets `started_at`/`ready_at`/`served_at` timestamps
  - [x] Tauri command: `update_kds_line_item_status_scoped` — scoped + `kds:orders-changed` event emission
  - [x] TS API wrapper: `updateKdsLineItemStatusScoped`
  - [x] Per-item status dots (5 colors: pending→grey, preparing→amber, ready→green, served→accent, cancelled→red)
  - [x] Click-to-advance on actionable items + `e.stopPropagation()` to avoid tripping ticket-level advance
  - [x] Wired through all 3 layouts (kanban, focus, metro)
  - _Effort:_ Large ✅

- [x] **3f. No ticket editing post-creation:** If a customer adds an item mid-preparation, the KDS ticket is frozen. No "on the fly" additions or modifier edits. ✅ (`5b86ff8e`)
  - [x] `UpdateKdsOrderItemsInput` extended with `line_items: Option<Vec<CreateKdsLineItemInput>>` (`#[serde(default)]` for backward compat)
  - [x] DB method `update_kds_order_items` now handles `line_items` — when provided, DELETEs existing + INSERTs new in a transaction, re-derives summary/count
  - [x] `create_kds_line_items_in_tx` refactored to avoid nested transactions
  - [x] `CreateKdsLineItemInput` TS interface + `line_items` field on `UpdateKdsOrderItemsInput`
  - [x] `KdsProductPickerModal` — searchable product list (restaurant/both), course dropdown, quantity stepper, remove, focus trap, Escape/backdrop
  - [x] `onAddItems` callback wired through KdsLayoutProps → 3 layouts → KdsTicketCard
  - [x] KdsScreen `onConfirm` merges existing line items with picked items, calls API
  - [x] 15 new FTL keys (EN + ID)
  - _Effort:_ Medium ✅

---

## 2. Workspace Settings — Cross-Card Reactivity & F10 Modal

### 🔴 Quick Wins (one-line fixes)

- [x] **4a. Wire `markSettingsUpdated` in all 5 save handlers:** The `SettingsContext` supports cross-card reactivity via `markSettingsUpdated(keys)`, but none of the workspace cards call it after save. Changing receipt paper width in Store POS Settings won't update Restaurant POS Settings without a page reload.
  - _Files:_ `WorkspaceStorePosSettings.tsx`, `WorkspaceRestaurantPosSettings.tsx`, `WorkspaceKdsSettings.tsx`, `WorkspaceInventorySettings.tsx`, `TerminalPreferencesCard.tsx`
  - _Each call site needs:_ `markSettingsUpdated([...changed keys])` after successful save + originals update
  - _Effort:_ ~5 lines per card

- [x] **4b. Register F10 keybinding to open WorkspaceSettingsModal:** The modal exists and works, but nothing opens it. The JSDoc says "Opens via F10 inside a workspace" but no `onKeyDown` handler triggers it.
  - _Files:_ Needs a `useEffect` in the workspace screen shells (`AppShell.tsx`, `KdsScreen.tsx`, `PosScreen.tsx`, `RetailPosScreen.tsx`) that listens for F10 and toggles the modal
  - _Effort:_ Small (per-screen keyboard listener)

### 🟠 Feature Gaps

- [x] **4c. Kitchen printer needs its own hardware profile field:** `WorkspaceRestaurantPosSettings` reuses `hw.profile.hardware.printer` (the receipt printer) for kitchen printing. In a real restaurant these are different devices — the receipt printer is at the POS station, the kitchen printer is in the kitchen.
  - _Files:_ `useTerminalHardware` (hook), `HardwareSettingsDto` (Rust DTO), `terminal_profile.json` (schema), `WorkspaceRestaurantPosSettings.tsx`
  - _Effort:_ Medium (schema change + DTO + UI field)

- [x] **4d. Migrate unscoped `setReceiptSettings` to scoped variants:** `WorkspaceStorePosSettings` and `WorkspaceRestaurantPosSettings` call `setReceiptSettings(...userId)` (unscoped) instead of `setReceiptSettingsScoped(sessionToken, ...)`. ADR #7 migrated 84 other commands to scoped — these two slipped through.
  - _Files:_ `WorkspaceStorePosSettings.tsx:71`, `WorkspaceRestaurantPosSettings.tsx:78`
  - _Effort:_ Small (swap API call + add sessionToken from context)

- [x] **4e. `terminal_profile.json` migrated to DB with schema versioning:** Hardware profiles now live in the DB with migration 104 (`hardware_profiles` table with `schema_version` column). Backward-compat writes to JSON preserved. (`1032c907`)
  - _Effort:_ Medium ✅

- [x] **4f. Missing FTL keys for some workspace card labels:** Several `aria-label` attributes in workspace cards are still hardcoded English: `aria-label="Sound volume"`, `aria-label="Yellow escalation threshold in minutes"`, `aria-label="Red escalation threshold in minutes"`.
  - _Files:_ `TerminalPreferencesCard.tsx:118`, `WorkspaceKdsSettings.tsx:130,145`
  - _Effort:_ Small (add 3 FTL keys + wire them)

---

## 3. Completed (Prior Session)

- [x] **SettingsPage.tsx** — 2 hardcoded strings fixed (`533247bc`)
- [x] **RestaurantMenu.tsx** — 13 FTL keys + 2 aria-labels (`b3307810`)
- [x] **Attribute-only FTL sweep** — 75 keys across 16 bundles (`104c4891`)
- [x] **PosScreen.tsx** — 5 hardcoded English strings (`0796d835`)
- [x] **ProductManagement + CategoryManagement** — 3 fixes (`13023004`)
- [x] **AuditLogScreen** — 1 unreviewed badge title (`268ecd81`)
- [x] **CustomerManagement** — zero bugs (clean sweep)
