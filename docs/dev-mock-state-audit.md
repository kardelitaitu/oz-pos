# Dev-Mock Reload-State Audit

> **Audited:** 2026-08-06 · **File:** `ui/src/dev-mock/tauri-api.ts` (line numbers as of `a1d9fe14`)
> **Scope:** Every piece of module-level state in the browser dev-mock, whether it survives a page reload, and how that compares to the real Tauri backend (which persists to SQLite via `crates/oz-core`).

The dev-mock powers the browser preview (`npm run dev` / Vite). It stands in for the Tauri IPC surface, so its state must mirror the backend's persistence contract — otherwise a preview reload demonstrates behavior the real app does not have (or vice versa). This audit catalogs the state so gaps are explicit and can be closed one at a time.

---

## State map

| Mock state | Line | Kind | Survives reload? | Real backend equivalent | Backend persists? |
|---|---|---|---|---|---|
| `mockActiveShift` | 394 | `let` (null or shift) | ✅ Yes — `oz-dev-mock:active-shift` | `shifts` (021) | ✅ SQLite |
| `cartState` | 260 | `let` (lines array) | ✅ Yes — `oz-dev-mock:cart` | `active_carts` (037) | ✅ SQLite |
| `completedSales` + `saleDetails` | 328–329 | `const` arrays (seeded from `oz-dev-mock:sales`) | ✅ Yes — `oz-dev-mock:sales` | `sales` / `sale_lines` (001) | ✅ SQLite |
| `mockUserPrefs` | 446 | `const` object | ✅ Yes — `oz-dev-mock:user-prefs` | `user_preferences` (038) | ✅ SQLite |
| `mockKdsOrders` | 160→255 | `const` array (pushed/mutated) | ✅ Yes — `oz-dev-mock:kds` | `kds_orders` (032, + 048/053/064/101/103) | ✅ SQLite |
| `mockKdsLineItems` | 164→256 | `const` object (per-item status mutated) | ✅ Yes — `oz-dev-mock:kds` | `kds_line_items` (105) | ✅ SQLite |
| `kdsDisplayCounter` | 161→263 | `let` number | ✅ Yes — derived from max persisted `display_number` | `kds_daily_counters` (032) | ✅ SQLite (per day) |
| `loginAttempts` | 227→322 | `const` object (flat count, `LOCKOUT_THRESHOLD = 4`) | ✅ Yes — `oz-dev-mock:login-attempts` | `login_attempts` (074 + 111 device) | ✅ SQLite |
| `mockShiftHistory` | 413→544 | `const` array (pushed on close) | ✅ Yes — `oz-dev-mock:shift-history` | `shifts` (021) | ✅ SQLite |
| `MOCK_STAFF/PRODUCTS/CATEGORIES/…` | 20–111 | `const` static seed | n/a — never mutated | seeded tables / fixtures | ✅ SQLite (seed) |
| `_initialKdsOrders` | 122 | `const` static seed | n/a — cloned into `mockKdsOrders` | seed fixture | — |
| `mockHeldCarts` | 380→430 | `let` array (hold/resume/delete mutations) | ✅ Yes — `oz-dev-mock:held-carts` | `held_carts` (013, + 095) | ✅ SQLite |
| sessions (`create_session`) | — | stateless fresh token | n/a | `session_store` — in-memory `HashMap` (`apps/desktop-client/src/state.rs:132`) | ❌ **In-memory by design** |

---

## ✅ Persisted state (parity with backend)

Eight localStorage keys, all using the same pattern (seed on first load, `save*` on every mutation, `load*` on module load):

| Key | Backs | Writes |
|---|---|---|
| `oz-dev-mock:active-shift` | `mockActiveShift` | `open_shift*` writes; `close_shift*` writes the `__closed__` sentinel (line 368) so a reload after closing does not re-seed a fresh open shift; first-load auto-seed applies only when nothing was ever persisted |
| `oz-dev-mock:cart` | `cartState` | `start_sale*` clears, `add_line*` pushes, all three `complete_sale*` variants clear after completing |
| `oz-dev-mock:sales` | `completedSales` + `saleDetails` | saved whenever a sale completes (holds both the sale rows and their detail objects, so the per-sale detail view survives too) |
| `oz-dev-mock:user-prefs` | `mockUserPrefs` | `set_user_preferences*` |
| `oz-dev-mock:kds` | `mockKdsOrders` + `mockKdsLineItems` (+ `kdsDisplayCounter` derived from the stored orders) | `pushKdsOrderFromCart` on all `complete_sale*` variants; `update_kds_status*`; `update_kds_line_item_status*` — the whole queue + line items persist under one key, and the next ticket number is one past the highest persisted `display_number` (never below the 104 seed baseline) |
| `oz-dev-mock:login-attempts` | `loginAttempts` | `staff_login` failure increments (persisted so a reload cannot bypass a lockout); success deletes (persisted so the unlock survives) |
| `oz-dev-mock:shift-history` | `mockShiftHistory` | both `close_shift*` variants push the closed shift and save; fresh loads seed exactly the one pre-seeded closed shift |
| `oz-dev-mock:held-carts` | `mockHeldCarts` | `hold_cart*` stores the full cart payload; `list_held_carts*` / `list_open_bills*` return summaries; `get_held_cart*` returns detail; `delete_held_cart*` removes the row |

All loaders/savers are wrapped in try/catch — if storage is unavailable they fall back to in-memory behavior and the app still works.

## ❌ Gaps — resets on reload while the backend persists

> **None remaining.** All previously-identified state now persists under the `oz-dev-mock:*` keys listed above (closed 2026-08-06). Two known parity *fidelity* gaps remain, both by design of the mock:
>
> - **Lockout model** — the backend enforces sliding-window, per-account **and** per-device and global limits with exponential backoff; the mock keeps a flat count of 4. The reload contract now matches, but the enforcement model is intentionally simpler.
> - **Shift history** — the backend derives history from the live `shifts` table (reconciliation totals); the mock stores a flat array of closed-shift rows.

## ⚖️ By-design parity (no fix needed)

- **Sessions** — both mock and backend are in-memory/stateless. The real backend intentionally re-authenticates on restart (`session_store` is a `HashMap` with TTL), which is why the preview always lands back at the login screen after a reload.
- **Held carts / open bills** — persisted in `oz-dev-mock:held-carts`; the mock now exercises the same hold/list/detail/delete contract as the backend. Session scope remains intentionally simplified because the browser mock has one store.

## 📌 Static seed data

`MOCK_PRODUCTS`, `MOCK_CATEGORIES`, `MOCK_STORE`, `MOCK_CURRENCIES`, `MOCK_TERMINAL`, `MOCK_CUSTOMERS`, `MOCK_INVENTORY_LOCATIONS`, `MOCK_WORKSPACES`, `MOCK_STAFF` are seed constants with no mutation handlers. Product stock quantities reset — and the backend persists stock in inventory tables — but since the mock never changes them, there is nothing to lose.

## Recommended follow-ups (in priority order)

1. **(Stretch) Mirror the backend's sliding-window lockout** instead of the flat threshold of 4 — the reload contract is fixed; the enforcement model is still simpler than the backend.

> ✅ **Done (2026-08-06):** `loginAttempts` → `oz-dev-mock:login-attempts` and `mockShiftHistory` → `oz-dev-mock:shift-history`, both pinned by reload-survival contract tests in `dev-mock-auth-contract.test.ts`. The audit's gap list is now empty.

> ✅ **Done (2026-08-06):** KDS state (`mockKdsOrders`, `mockKdsLineItems`, `kdsDisplayCounter`) is now persisted under `oz-dev-mock:kds` — the biggest gap is closed, pinned by the reload-survival contract tests in `dev-mock-auth-contract.test.ts`.

> ✅ **Done (2026-08-09):** Held carts (`mockHeldCarts`) are now persisted under `oz-dev-mock:held-carts`; hold/list/detail/delete and reload-resume behavior are pinned by the held-cart contract tests in `dev-mock-auth-contract.test.ts`. Persisted rows are runtime-validated on load, and generated ids use `crypto.randomUUID()` with a compatibility fallback.

> last audited 09-08-26 by buffy
> audit: Phase 1 Core Architecture & API Docs Audit

> status: ACCURATE (0 findings) · verified accurate: cargo check passed, no structural orphans, no stale version headers, all file references valid
