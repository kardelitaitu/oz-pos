# Improvement Opportunities — July 31, 2026

> **Legend:** ✅ Complete · 🔷 Phase active · ⏳ Planned

---

## ✅ Phase A — E2E Test Infrastructure (Complete)

### Unified E2E Runner (`npm run e2e`)

Created a cross-platform Node.js E2E runner at `scripts/run-e2e.mjs`:
- Starts Docker backend (cloud server + license server + Redis) if available
- Starts Vite dev server as subprocess, waits for `localhost:1420`
- Runs Playwright with `--headed`, `--no-docker`, `--api-only`, `--ui-only` flags
- Cleans up Vite + Docker on exit (SIGINT/SIGTERM handlers)
- Cross-platform port cleanup: `netstat + taskkill` (Win) or `lsof + kill` (Unix)

**npm scripts added (`ui/package.json`):**
| Command | What it does |
|---------|-------------|
| `npm run e2e` | Full suite: Docker → Vite → Playwright → cleanup |
| `npm run e2e:headed` | Same, with browser visible |
| `npm run e2e:api` | API integration tests only |
| `npm run e2e:ui` | All UI tests (excluding API) |

### 3 Critical-Path E2E Specs

| Spec | Flow |
|------|------|
| `e2e-sale-to-history.spec.ts` | Add product → complete cash payment → verify in Sales History |
| `e2e-shift-reconciliation.spec.ts` | Open shift → complete sale → close shift → verify summary |
| `e2e-settings-persist.spec.ts` | Change receipt width / store name → navigate away → verify persisted |

---

## 🔷 Phase B — Next Recommendations (in priority order)

### B1 — KDS Critical-Path E2E Test (Phase Active)

**Goal:** Full ticket lifecycle E2E — pending → preparing → ready → served, plus layout switching, per-item status, and settings interaction.

Current `kds.spec.ts` covers basic render + single advance. Missing:
- Full lifecycle through all 4 statuses
- Layout switching (Kanban ↔ Focus ↔ Metro)
- Settings panel interaction (sound, thresholds)
- Per-item line item status advance (TODO 3e)
- History panel toggle

**File:** `ui/e2e/e2e-kds-critical-path.spec.ts`

### B2 — KDS E2E: End-to-End POS → KDS Flow

**Goal:** Complete a sale with kitchen items in Restaurant POS, then verify the ticket appears on the KDS screen.

Requires the dev-mock to support cross-workspace order propagation. Currently the sale mock and KDS mock are independent — completing a sale doesn't populate KDS orders.

**Steps:**
1. Add a `pendingKdsOrders` buffer to `dev-mock/tauri-api.ts`
2. Wire `completeSale` to push a new KDS order into the buffer
3. Wire `getKdsQueueScoped` to include buffered orders
4. Write E2E test: Restaurant POS → add product → complete sale → KDS → verify ticket

### B3 — E2E CI Workflow for PRs

**Goal:** Add a GitHub Actions workflow that runs the e2e suite on PRs targeting `main`.

**Steps:**
1. Create `.github/workflows/e2e-pr.yml`
2. Steps: Install Node → Install Playwright browsers → Build Docker image → Start Vite → Run Playwright → Upload traces on failure
3. Make it non-blocking (informational) initially, then require after proving stable

### B4 — Test the E2E Runner Itself

**Goal:** Write a vitest unit test for `scripts/run-e2e.mjs` that mocks `execSync` and `spawn`, verifying Docker detection, Vite startup, and cleanup logic.

### B5 — `--changed-only` Mode

**Goal:** Add a `--changed-only` flag to `run-e2e.mjs` that skips Docker startup when only UI spec files have changed (detected via `git diff --name-only`).

---

## ✅ Previously Completed

See `git log --oneline` for the full commit chain. Key items:
- ShiftManagementScreen audit + fix (1171 lines)
- SalesHistoryScreen audit + fix (1131 lines)
- DataManagementScreen audit + fix (968 lines)
- TerminalManagementScreen audit + fix (945 lines)
- AppearanceSection + ReceiptSection test coverage (32 tests)
- Doc drift audit (ARCHITECTURE.md, RESTRUCTURING.md, api-reference.md)
- `npm run check:all` unified validation runner
