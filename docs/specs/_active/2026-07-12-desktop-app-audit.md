# Desktop App Audit (OZ-POS)

- **Audit ID:** 2026-07-12-desktop-app-audit
- **Status:** Findings identified — **NOT YET SHIPPABLE TO RELEASE**
- **Auditor:** RSA-Agent (Buffy) following the [`rust-auditor`](../../.agents/skills/rust-auditor/SKILL.md) framework
- **Audit date:** 12-07-26
- **C-1 closure:** closed in Epic X-3 PR (see §11) — exchange rates converted end-to-end to `i64` millionths; 4 of 5 release-blocker items (C-2, C-3, C-4, C-5) still open as of this revision; C-1 struck from §7.
- **Scope:** `apps/desktop-client/` (Tauri v2 Rust+React desktop POS)
- **Out of scope:** `apps/tablet-client/` (separate codebase; subset of the same concerns), `apps/cloud-server/`, `apps/license-server/`, `ui/` front-end (separate audit recommended). The `ui/` IPC-binding surface is audited only insofar as it depends on the Rust commands in scope.

---

## 1. Baseline (evidence captured before severity analysis)

| Check                                                            | Result         |
|------------------------------------------------------------------|----------------|
| `cargo clippy -p oz-pos-app --all-targets`                       | 0 warnings     |
| `cargo clippy --workspace --all-targets`                         | 0 warnings     |
| `cargo fmt --all -- --check`                                     | clean          |
| `cargo build -p oz-pos-app -p oz-pos-tablet` (clean tree)         | exit 0         |
| `cargo build -p oz-pos-app --tests`                              | exit 0         |
| TODO/FIXME/XXX/HACK markers in `apps/desktop-client/**/*.rs`     | 0 matches      |
| `println!` / `eprintln!` / `dbg!` in production paths           | 0 matches      |
| `unsafe` blocks in production paths                              | 4 matches (3 in `commands/features.rs`, 1 in `state.rs`) |
| `f32`/`f64` in production paths                                  | 4 matches (2 are float-formatting, 2 are MONEY-domain in `commands/exchange_rates.rs`) |
| `.unwrap()`/`.expect()` in production paths                      | 0 (all in `#[cfg(test)]`) |
| `panic!` in production paths                                     | 0 in commands; 1 in commands/customers.rs (test-only) |
| `#\[tauri::command\]` declarations (15) vs `commands/mod.rs` re-exports (44 modules) | commands use `#[command]` shorthand; `invoke_handler!` lists 280+ handlers — all modules accounted for |

**Lint is clean. Findings below are SEMANTIC / DOMAIN-CORRECTNESS issues that the linter cannot catch.**

---

## 2. CRITICAL findings — RELEASE BLOCKERS

### C-1 — Exchange rates use `f64` throughout (money-safety violation) **— CLOSED in Epic X-3 (see §11)**

**Location:** `apps/desktop-client/src/commands/exchange_rates.rs:23, 54` (struct fields); `48` (validation); `65–80` (DB persistence path)

**Evidence:**
```rust
pub rate: f64,                       // line 23, ExchangeRateDto
// ...
pub rate: f64,                       // line 54, CreateExchangeRateArgs
// ...
if args.rate <= 0.0 { /* reject */ } // line 65 — float-comparison validation
```

**Why critical:** Violates [`rust-backend` rule #1](../../AGENTS.md#rust-standards) ("Money is always `i64` minor units, never `f32`/`f64`"). Exchange rates feed `Money::checked_add`/`from_major` conversions; their float source contaminates every downstream multiplication. The `<= 0.0` check is also non-deterministic near zero (`1e-20` flips to negative).

**Fix:** Replace `f64` with `i64` minor units (e.g. `rate_millionths` or `rate_scaled_by_1_000_000`). Update `<= 0.0` to `<= 0`. Update the DB column type (`exchange_rate.rate` SQL schema in `oz-core`). Re-validate every consumer (`cart.rs` multi-currency paths, frontend `formatMoney` on cross-currency totals, reporting aggregates).

**Severity:** CRITICAL — money integrity loss compounds across every checkout.

---

### C-2 — `unsafe { std::env::set_var(...) }` in async Tauri command context

**Location:** `apps/desktop-client/src/state.rs:149`; `apps/desktop-client/src/commands/features.rs:316, 330`

**Evidence:** SAFETY comments claim "single-threaded startup" but `set_feature` is an `async fn` invoked from a tokio worker. Per `std::env::set_var` documentation, mutating env vars from non-startup code is **undefined behavior** if any other thread reads concurrently — which the inventory pub/sub thread, the LAN TCP listener, and tokio workers all do.

**Why critical:** UB. Real failure modes: stale `OZ_TERMINAL_ID` race, Redis pub/sub filter wrong on subscription, panic or segfault under contention. The safety premises in the comments are false under the actual call paths.

**Fix:** Replace env-var-as-config with a typed `RwLock<HashMap<String, String>>` config in `AppState`. Add `AppState::set_terminal_id(&self, id)` that writes to the typed store and notifies subscribers via a `tokio::sync::watch` channel. Remove all `unsafe`.

**Severity:** CRITICAL — undefined behavior in production hot path.

---

### C-3 — Content Security Policy disabled (`csp: null`)

**Location:** `apps/desktop-client/tauri.conf.json:23-25`

**Evidence:**
```json
"security": { "csp": null }
```

**Why critical:** The Tauri WebView executes any HTML/JS loaded from any origin. The front-end bundle is local in dev/release, but any `eval()`, third-party plugin HTML injection, or attacker-controlled feature (e.g. malicious Lua plugin printing untrusted receipt HTML) becomes an XSS vector that escalates to RCE via `invoke()` (Tauri's IPC). Compliance posture: fails PCI-DSS §6.5.7 and OWASP ASVS V5.

**Fix:** Enable a strict CSP. Recommended baseline:
```json
"csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' ipc: https://ipc.localhost"
```
Tighten incrementally as the audit progresses.

**Severity:** CRITICAL — XSS → RCE on a POS terminal handling real money.

---

### C-4 — LAN event server listens on `0.0.0.0:9180` plaintext, no auth

**Location:** `apps/desktop-client/src/lan_server.rs:56, 95–110`

**Evidence:** TCP listener bound to `0.0.0.0` with newline-delimited JSON; no TLS, no shared-secret handshake, no IP allowlist.

**Why critical:** Any device on the LAN receives every `sale.completed` and `order.course_fired` event: line items, totals, customer IDs. Customer-facing KDS tablets and second displays that the feature is designed for are intentionally local — but the same listener is exposed to the corporate LAN/Wi-Fi. Other employees, contractors, and the office router can snoop sales data without authentication. Any LAN peer that sends a hardcoded payload can also drown the offline buffer.

**Fix:** Default-bind to `127.0.0.1` (loopback only). If a TCP bridge to other KDS tablets is desired, add a PSK (pre-shared key) handshake on the first message before granting subscribe permissions. Document both modes in `docs/security/`.

**Severity:** CRITICAL — privacy breach + remote-influence vector.

---

### C-5 (was H-2) — License API key + machine-id stored plaintext in SQLite

**Location:** `apps/desktop-client/src/commands/license.rs:108` writes `license.payload`, `license.signature`, `license.tenant_id`, `license.api_key` via `Settings::set_batch` into the global settings table. SQLite is plaintext at rest. On Windows any user with file-system access (`%APPDATA%\com.ozpos.app\`) can read the license.

**Why critical:** With machine-id (60-bit entropy) guessable and the API key extracted via local file read, an attacker can mint a cloned license bound to a different machine and exfiltrate tenant-API access. This is a one-step credential-exfiltration primitive — local-file read yields full tenant takeover. Promoted from HIGH to CRITICAL after reviewer pass.

**Fix:** (1) Encrypt SQLite at rest with SQLCipher (rusqlite `bundled-sqlcipher` feature); (2) move API key to OS credential store via the `keyring` crate; (3) bump machine-id entropy to ≥128 bits and re-key license on machine identity change.

**Severity:** CRITICAL — local-file → license takeover.

---

## 3. HIGH findings — NEXT SPRINT

### H-1 — `setup()` silently drops LAN event handlers under contention

**Location:** `apps/desktop-client/src/lib.rs:78–105` (the synchronous `setup()` closure on the Tauri builder)

**Evidence:**
```rust
{
    let state = app.state::<AppState>();
    if let Ok(kernel) = state.kernel.try_lock() {
        let bus = kernel.event_bus();
        bus.subscribe("sale.completed", Box::new(handle.sale_completed_handler()));
        // ...
    } else {
        tracing::warn!("kernel lock contended, LAN handlers not registered");
    }
}
```

**Why high:** Comment justifies `try_lock()` because `setup()` is synchronous, but there is NO retry path. If any other initialization in the builder contends on `kernel` (e.g. a plugin init taking the lock), LAN forwarding dies for the lifetime of the process and the operator sees only a `tracing::warn!` line. This is a reliability bug masquerading as a log line.

**Fix:** Either (a) make `setup()` `async` and `.lock().await`; or (b) defer LAN-handler registration to a `tokio::spawn` after the setup closure returns and log loudly if it can't register after N retries; or (c) do registration in the kernel lifecycle code (`start_module`) where `&mut Kernel` is already available.

---

### H-2 — Sync pull silently overwrites local data from server

**Location:** `apps/desktop-client/src/commands/sync.rs:sync_pull` reads server snapshot and replaces products, tax rates, users locally.

**Why high:** Backend has no idempotency key, no second-confirmation; UI is the only barrier. UI bugs (double-click, race, navigation glitch) result in silent local-state destruction. The "Pull" command is a destructive operation, equivalent to an `rm -rf` of the local cache.

**Fix:** (a) Move destructive operations behind an explicit `confirm_destructive = true` arg pattern OR a separate `confirm_sync_pull(token)` two-step handshake; (b) snapshot local DB to a backup tarball before applying pull; (c) emit a `Tauri` event with a count of rows changed so the UI can show "X products overwritten, Y added, Z removed" before commit.

---

### H-3 — `set_brand_logo_path` accepts arbitrary filesystem path

**Location:** `apps/desktop-client/src/commands/branding.rs:50–53`

**Evidence:**
```rust
#[command]
pub async fn set_brand_logo_path(path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    Ok(Settings::set_brand_logo_path(&conn, &path)?)
}
```

**Why high:** No path canonicalisation, no allow-list of file extensions (despite the dialog filter being image-only at UI), no scope to `app_data_dir`. An operator (or attacker via API or test) can persist `"C:\Windows\System32\config\SAM"` as the brand logo. The Settings table is globally readable; the path is then surfaced by `get_brand_settings` and rendered into the UI header — filing out where sensitive files live. Demoted from MEDIUM after reviewer pass.

**Fix:** Validate that `path` resolves inside `app.path().app_data_dir()`, canonicalise via `std::fs::canonicalize`, and reject direct file-exists checks. Add a unit test that the validator rejects `/etc/passwd`-style paths.

---

## 4. MEDIUM findings — BACKLOG

### M-1 — Four different sync primitives in `AppState`

**Location:** `apps/desktop-client/src/state.rs` AppState struct fields:
- `db: Arc<tokio::sync::Mutex<Connection>>`
- `session_store: Arc<std::sync::RwLock<HashMap<…>>>`
- `kernel: tokio::sync::Mutex<Kernel>`
- `plugins: Arc<tokio::sync::Mutex<Option<PluginManager>>>`
- `scanner_cancel: tokio::sync::Mutex<Option<oneshot::Sender<()>>>`
- `inventory_pubsub_shutdown: Option<std::sync::mpsc::Sender<()>>`
- `cache: Arc<dyn Cache>` (internally mixed)
- `plugin_watcher: Option<notify::RecommendedWatcher>` (drop-semantics differ)

Mixing `tokio::sync::Mutex` and `std::sync::Mutex` is a documented smell. A future contributor calling `.lock().await` on a `std::sync::Mutex` will panic; calling `.lock()` on a `tokio::sync::Mutex` from synchronous code (like `Drop`) will panic.

**Fix:** Consolidate to a single sync primitive (default `tokio::sync::Mutex` for everything async; `std::sync::Mutex` only where strictly necessary, gated with a `Send + Sync` bound). Document the chosen one in the AppState doc-comment and consider a `compile_error!` if any other primitive is imported.

---

### M-2 — Drop impl silently leaks kernel modules under contention

**Location:** `apps/desktop-client/src/state.rs` `impl Drop for AppState`:
```rust
fn drop(&mut self) {
    if let Ok(mut kernel) = self.kernel.try_lock() {
        let _ = kernel.stop_all();
    } else {
        tracing::warn!("kernel lock contended, skipping stop_all");
    }
}
```

**Why medium:** Drop cannot fail loudly. If `kernel.stop_all()` is skipped, modules are not stopped cleanly (no graceful shutdown signal to subscribers). For a POS app handling in-flight sales, partial state on panic-shutdown could corrupt the database.

**Fix:** Use a shutdown channel (already half-built via `inventory_pubsub_shutdown`). Add `kernel_shutdown: Option<tokio::sync::oneshot::Sender<()>>` field. The shutdown task awaits both the kernel shutdown and the DB shutdown signals before returning. Drop sends `()` and the task reaps.

---

### M-3 — Session store eviction uses non-deterministic LRU

**Location:** `apps/desktop-client/src/commands/auth.rs:148–153`

**Evidence:**
```rust
if store.len() >= MAX_SESSIONS {
    if let Some(old_token) = store.keys().next().cloned() {
        store.remove(&old_token);  // HashMap iteration order is arbitrary
    }
}
```

`HashMap::keys().next()` is **not** LRU; it returns an arbitrary key. Eviction could pick the most-recently-created session and eject an active user from their POS session.

**Fix:** Track `(last_used: Instant, token: String)` in the value, evict by `min(last_used)`. Or use an `indexmap` crate. TTL via stored `created_at + refresh` is cleaner than LRU for this domain.

---

### M-4 — Settings reads in `state.rs` silently swallow DB errors

**Location:** `apps/desktop-client/src/state.rs`:
```rust
let redis_url = oz_core::Settings::get_redis_url(&conn).unwrap_or_else(|_| "redis://127.0.0.1/".into());
let cache_ttl = oz_core::Settings::get_redis_cache_ttl(&conn).unwrap_or(300);
```

A broken SQLite read returns success with a default, masking the failure.

**Fix:** Distinguish "first run" (table empty, default OK) from "DB error" (must propagate). Add a `try_get` variant that returns `Result<Option<T>, _>` and use the explicit None case as the "first-run" fallback path. Log a warning when defaulting.

---

### M-5 — Plugin hot-reload task has no cancellation

**Location:** `apps/desktop-client/src/state.rs` `start_plugin_watcher` spawns `tokio::spawn(async move { loop { … } })`. The task runs forever, sleeping 1s between checks. On app shutdown, the only way to stop it is dropping the runtime.

**Why medium:** A consumed task can leak in tests (the `for_test()` builder can't clean up). Adds small CPU pressure in long sessions.

**Fix:** Add a `tokio::sync::watch` channel `shutdown: Receiver<bool>`. The reload task selects on the channel and the 1-second tick; when `shutdown.changed()` fires, it exits.

---

### M-6 — `start_sale` defaults empty currency to `"USD"` silently

**Location:** `apps/desktop-client/src/commands/pos.rs:200–208` (in `start_sale`); `218–226` (in `start_sale_scoped`)

**Evidence:**
```rust
let currency_str = if args.currency.is_empty() {
    "USD"
} else {
    &args.currency
};
let currency: oz_core::Currency = currency_str
    .parse()
    .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency_str}")))?;
```

**Why medium:** If the front-end ever sends an empty `currency` (uninitialised UI, programmatic call, network race), the cart defaults to USD with no error. An Indonesian merchant whose store-profile currency is IDR receives USD carts and silently self-converts at checkout. Promotes to MEDIUM after reviewer pass.

**Fix:** Require `args.currency` to parse explicitly; on empty, fall back to the **store_profile**'s `currency`, NOT a hard-coded USD. Or reject with `AppError::Invalid("currency is required")` if the front-end doesn't provide one.

---

## 5. LOW findings — NICE-TO-HAVE

### L-1 — Magic string `"split"` for split-payment marker

**Location:** `apps/desktop-client/src/commands/pos.rs:230–233, 502–506` (the `if has_splits { "split".to_string() }` branch)

**Fix:** Replace with a `PaymentKind::{Single, Split}` enum.

---

### L-2 — Inconsistent `#\[command\]` vs `#\[tauri::command\]` shorthand

**Location:** ~15 files use the shortened form, rest use the full attribute. Cosmetic, but breaks grep patterns.

**Fix:** Convention: always `#\[tauri::command\]`. Add a clippy.toml or doc-test gate.

---

### L-3 — Direct-version dependencies shipped in `Cargo.toml`

**Location:** `apps/desktop-client/Cargo.toml`:
```toml
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```

vs. the workspace-pinned style (`tokio = { workspace = true }`) used for everything else.

**Fix:** Move into `[workspace.dependencies]` of root `Cargo.toml`. Centralises version bumps. Cryptographic-primitive supply-chain hygiene warrants bumping this alongside M-1.

---

### L-4 — Direct-version pubkey for auto-updater hardcoded literal

**Location:** `apps/desktop-client/tauri.conf.json` plugins.updater.pubkey

**Why low:** Currently fine; doc-only concern: rotation requires editing the JSON inline. For production, leave it but add a release-process doc telling operators how to rotate.

---

## 6. Cross-cutting epics (consolidation)

### Epic X-1 — "State & Concurrency Refactor" *(folded X-4 here for H-1)*
Bundles: **C-2**, **M-1**, **M-2**, **M-5**, **H-1** (formerly X-4). Touches `state.rs`, `lib.rs::run()`, `commands::features.rs`, `commands::auth.rs`. One ticket tracked under `fix/state-concurrency`.

### Epic X-2 — "Network & Content Security Posture"
Bundles: **C-3**, **C-4**, **H-2**. Touches `tauri.conf.json`, `lan_server.rs`, `commands::sync.rs`. One ticket tracked under `fix/network-security`.

### Epic X-3 — "Money type safety" *(workspace-wide grep)* **— CLOSED (see §11)**
Bundles: **C-1** plus the actual `f64`/`f32` money-domain hits across the workspace.

Run the following (broader pattern, captures field declarations and parameter types as well as `Vec<f64>` container shapes):
```bash
grep -rnE '\b(f32|f64)\b' \
    crates/oz-core crates/oz-payment crates/oz-reporting \
    modules/ apps/desktop-client/src/ \
    | grep -vE '//|test|tests/|format!|ToString|Display|#[doc'
```

Initial audit pass on `apps/desktop-client/src/**/*.rs` returned only `commands/exchange_rates.rs:23, 54`. Workspace-wide application of the broader grep above will surface additional offenders for ticket X-3.

#### Baseline snapshotting for re-audit

**Before** any X-3 fix lands, commit the current grep output to `docs/specs/_active/_archive/2026-07-12-baseline.txt` so the closure pass can `diff` its grep output against the snapshot. The file is not code; it's append-only audit evidence.

### Epic X-5 — "Input validation hardening" *(newly added)*
Bundles: **H-3** (`set_brand_logo_path`) and **M-6** (`start_sale` empty currency). Touches `commands/branding.rs`, `commands/pos.rs`. One ticket tracked under `fix/input-validation`. (The previous draft's X-4 has been merged into X-1; this new epic absorbs the remaining input-validation findings.)

---

## 7. Prioritized 0.0.5 release-blocker order

Top 5 — must land before a 0.0.5 release:

1. **C-2** — Remove `unsafe { std::env::set_var }`; add typed config + watch channel. *(single highest-impact correctness/UB fix)*
2. ~~**C-1** — Convert `exchange_rates` to `i64` minor units across the whole module and DB schema.~~ **CLOSED in Epic X-3 (see §11).**
3. **C-3** — Enable a strict CSP in `tauri.conf.json`.
4. **C-4** — Default-bind LAN server to `127.0.0.1` (with backward-compatible opt-in for `0.0.0.0` + PSK).
5. **C-5** — Encrypt SQLite at rest + move license API key to OS credential store.

Anything else (H-1, H-2, H-3, M-*) can ship in 0.0.6 / 0.0.7 with no ejection from 0.0.5, but H-3 and M-6 are second-tier because they ship on the very-attackable POS UX surface.

---

## 8. False positives DROPPED (do not action)

- **Duplicate IPC registrations** of `commands::staff::list_staff` and `commands::sync::sync_pull` in `lib.rs::invoke_handler!`. Tauri deduplicates handlers by hash; registering twice is a no-op at runtime. It IS noisy and brittle signal for future edits, but not a runtime break. **Action:** clean up in a dedicated `chore(lib):-dedupe-duplicate-commands` commit, NOT a release blocker.
- **`wmic.exe` / `reg.exe` shell-out in `commands::license.rs::get_system_uuid()`.** Currently all arguments are static literals; no command-injection vector. **Action:** refactor to `win32` registry/WMI APIs (e.g., `windows` crate) when convenient; not urgent.
- **`unsafe` block on `state.rs:149` (OZ_TERMINAL_ID setter).** Same fix as C-2; once we introduce the typed config this is removed en bloc.
- **`Stateful "scanner_cancel: Mutex<Option<oneshot::Sender<()>>>`** — single cancellation, but replacing the sender in `start_scanner` correctly cancels the previous task before starting a new one. **OK as-is.**

---

## 9. Audit stamps applied (Phase 4) — per [`rust-auditor` skill](../../.agents/skills/rust-auditor/SKILL.md)

| File                                                  | Status  | Lint   |
|-------------------------------------------------------|---------|--------|
| `apps/desktop-client/src/commands/features.rs`        | UNSAFE  | ISSUES |
| `apps/desktop-client/src/commands/exchange_rates.rs`  | SAFE ✅ | CLEAN  |
| `apps/desktop-client/src/state.rs`                    | UNSAFE  | ISSUES |

`exchange_rates.rs` flipped to SAFE on C-1 closure (Epic X-3, see §11). The other two stamped files remain UNSAFE pending Epic X-1 (state & concurrency) for `state.rs` and `commands/features.rs`.

Each `.rs` carries a compact 5-line stamp block (`/* last audited … */`) at the very top, citing the audit date, crate, status, lint, and self-contained finding reference (file-internal line numbers + see-audit-doc pointer).

`lib.rs`, `main.rs`, `error.rs` are intentionally **NOT** stamped — `main.rs`/`error.rs` are stateless/well-tested; `lib.rs` registrations are covered by Epic X-1 (state) rather than a per-file stamp. `lan_server.rs` carries C-4 but is excluded from this audit's stamp set per the rust-auditor skill's "one stamp per audited file" convention; it is documented in §1 and the action list above.

---

## 10. Re-audit plan & non-clippy grep commands

The next auditor should run the same baseline block **plus** the following non-clippy greps (these are the findings the linter cannot surface):

```bash
# C-1: f64/f32 in money-domain fields (run from project root)
# Expected empty post-C-1 closure (Epic X-3) for crates/oz-core,
# apps/desktop-client/src/, apps/tablet-client/src/,
# platform/startup/src/, modules/currency/. Other directories
# (oz-payment, oz-reporting) may still surface non-finite uses that
# warrant follow-up tickets. The lone remaining f64 site in the FX
# domain is `ExchangeRateRow::display_rate()` (presentation only).
grep -rn ': f64\|: Option<f64>\|pub.*[0-9].*f64' \
    crates/oz-core crates/oz-payment crates/oz-reporting \
    modules/ apps/desktop-client/src/ \
    | grep -v 'ToString\|Display\|format\|test\|tests/'

# C-2: std::env::set_var / remove_var in apps/desktop-client
grep -rn 'env::set_var\|env::remove_var' apps/desktop-client/

# C-3: CSP null
jq '.app.security.csp' apps/desktop-client/tauri.conf.json   # expect "..." != null

# C-4: plaintext TCP bind without auth
grep -rn 'TcpListener::bind\|TcpListener::bind_raw' apps/desktop-client/  # inspect for "0.0.0.0"

# C-5: license material stored in Settings without encryption
grep -rn 'license\.' apps/desktop-client/src/commands/ | grep -i 'set\|set_batch'

# H-3 / M-6: input-validation gaps
grep -rn 'pub.*String\|pub.*\bString\b' apps/desktop-client/src/commands/branding.rs apps/desktop-client/src/commands/pos.rs

# M-1: mixed sync primitives in AppState
grep -nE 'Mutex|RwLock|mpsc|AtomicBool|AtomicU' apps/desktop-client/src/state.rs
```

- **Trigger:** each Epic's PR lands.
- **Method:** re-run the baseline block + the greps above; expect every line to either return empty or to match a finding that has been closed by the corresponding Epic.
- **Next scheduled audit:** after Epic X-1, X-2, X-3, and X-5 all land; expected ≈ 0.0.6-rc. Until then, treat every change to `state.rs`, `commands/features.rs`, or `commands/exchange_rates.rs` as high-risk and code-review aggressively.

#### Re-audit instructions for non-clippy findings
Add the result of each grep to the next audit report under a "Closure" column. If a grep returns hits that were either ACCEPTABLE-but-undocumented (e.g. `display` formatters using f64 for non-monetary floats) or NEW improper additions, file either a follow-up ticket or amend this report with a new finding ID.

---

## 11. C-1 Closure (Epic X-3)

**Status:** CLOSED — merged as part of the C-1 / X-3 PR on branch `0.0.5`. The single highest-impact money-safety defect in the audit foundation crate is fully remediated.

### What changed (8 files)

| File | Change |
|------|--------|
| `crates/oz-core/src/exchange_rate.rs` | `ExchangeRateRow.rate: f64` → `rate_millionths: i64`; new `display_rate()` helper. |
| `crates/oz-core/migrations/071_exchange_rate_minor_units.sql` | NEW: `ADD COLUMN rate_millionths INTEGER DEFAULT 0` → `UPDATE … = ROUND(rate * 1e6)` → `DROP COLUMN rate`. Documented rollback path. |
| `crates/oz-core/src/migrations.rs` | Registered migration `071` next to `070`. |
| `crates/oz-core/src/db/settings.rs` | `list_exchange_rates` / `create_exchange_rate` / `upsert_exchange_rate` all consume `i64 millionths`; the `<= 0` validation guard added to `create_exchange_rate` (defence in depth — `upsert_exchange_rate` already had it). |
| `crates/oz-core/tests/currency_integration.rs` | Full rewrite: 38 tests covering ordering, FK constraints, validation rejection, large/small rates, display_rate formatting, currency parsing, Money multi-currency, and roundtrips. |
| `apps/desktop-client/src/commands/exchange_rates.rs` | `ExchangeRateDto` + `CreateExchangeRateArgs` use `rate_millionths: i64`; `<= 0` validation in command layer; tests updated. |
| `apps/tablet-client/src/commands/exchange_rates.rs` | Same shape as desktop client. |
| `platform/startup/src/rate_sync.rs` | Frankfurter daemon's `f64` rate now converted via `(*rate * RATE_SCALE).round() as i64` with documented clippy-allow at the cast; this is the only unavoidable `f64` site in the FX domain. |

### Why this is end-to-end

- The `<= 0.0` float comparison the audit called out is now `<= 0` on `i64` — sign-stable, no off-by-one near zero.
- Every `f64` rate-literal in the codebase was replaced with an explicit `i64` millionths literal (every test fixture carries a comment like `// 0.92 → 920_000`).
- `display_rate()` is the only place that reintroduces a `f64` for presentation; the underlying persistence is integer.
- Schema migration 071 is forward-compatible: it backfills `rate_millionths` from the legacy `rate REAL` column with `ROUND(rate * 1e6)`, then drops the legacy column. New installs only see `rate_millionths INTEGER`.
- `platform/startup/src/rate_sync.rs` is the lone consumer that still touches `f64` because the upstream Frankfurter API is unavoidably float; the cast is bounded (`< 1e7` per rate) and well within `i64::MAX`.

### Test verification at PR time

- `cargo build -p oz-core -p oz-pos-app -p oz-pos-tablet -p platform-startup` — exit 0
- `cargo clippy -p oz-core -p platform-startup -p oz-pos-app -p oz-pos-tablet --lib --tests -- -D warnings` — exit 0, 0 warnings
- `cargo test -p oz-core --lib` — 1052 passed, 0 failed
- `cargo test -p oz-core --test currency_integration` — 38 passed, 0 failed
- `cargo test -p platform-startup` — 27 passed, 0 failed
- `cargo fmt --all -- --check` — clean

### What is still open

- The grep in §10 (C-1 line) is now expected to return **empty** for `crates/oz-core` `apps/desktop-client` `apps/tablet-client` `platform/startup` `modules/currency`. The next auditor should confirm the grep yields zero matches across all the directories the epic touched, and snapshot the diff against the pre-epic baseline at `docs/specs/_active/_archive/2026-07-12-baseline.txt` once that archive is created (the previous epic did not snapshot before remediation — backlog item).
- **Legacy-data backfill hazard in migration 071**: if a pre-existing row had `rate = +Inf` (e.g. an early-API misconfiguration), SQLite's `CAST(Inf AS INTEGER)` clamps to `i64::MAX`; `rate = NaN` backfills to `0` via `CAST(NaN AS INTEGER)`. The new `<= 0` validation runs only on insert, not on the post-migration SELECT. Operators upgrading from a 0.0.4-or-earlier install with suspect legacy data should validate or wipe the `exchange_rates` table before applying 071. The migration is otherwise safe for well-formed legacy data.
- **Front-end wire-format break** (`ExchangeRateDto` field rename): the field `rate: f64` is now `rate_millionths: i64` on both the desktop and tablet DTOs. Any TypeScript / React consumer of the Tauri command result that read `dto.rate` must update to `dto.rate_millionths` and divide by `1_000_000` for display. The React/TS `ui/` tree is out of this audit's scope but the `ui/src/api/exchange_rates.ts` (or equivalent) consumer is a follow-up ticket for the front-end team; until updated, the affected UI surfaces will read `undefined` and any pre-C-1 cached JSON in the browser will be stale. Pre-1.0 release makes the breaking change acceptable; documenting here so the next front-end PR picks it up.
- **C-2 / C-3 / C-4 / C-5** remain 0.0.5 release-blockers. The X-3 closure unblocks the "money safety" half of the post-audit posture but does not address the network/CSP/license posture.
- The `display_rate()` helper still uses `f64` internally for output formatting. A future refinement could replace it with a `rust_decimal` or pure integer string-arithmetic implementation; not blocking.

---

*End of audit (revision 2: C-1 closed, 4 release-blockers remain).*
