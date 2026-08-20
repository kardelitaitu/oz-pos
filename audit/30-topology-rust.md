# Topology Rust Area Audit — August 2026

> **Audit date:** 2026-08-20
> **Sector:** Topology `.rs` area — `crates/oz-core/src/topology*.rs` and `apps/desktop-client/src/commands/topology*`
> **Status:** ⚠️ **8 FINDINGS** — TOP-01→TOP-08 (all P2/P3; no P0/P1). Verdict: solid, long-lived foundation; 6 small hygiene/consistency fixes recommended before heavy expansion.
> **Production code changed:** None (audit only — fixes pending user approval)

## Scope

Audited every topology-area Rust file (12 files, ~9,100 lines):

| File | Lines | Kind |
|---|---|---|
| `crates/oz-core/src/topology.rs` | 656 | production |
| `crates/oz-core/src/topology_tests.rs` | 337 | unit tests (42 tests) |
| `apps/desktop-client/src/commands/topology.rs` | 55 | module root |
| `apps/desktop-client/src/commands/topology/commands.rs` | 589 | production (3 Tauri commands) |
| `apps/desktop-client/src/commands/topology/model.rs` | 255 | production |
| `apps/desktop-client/src/commands/topology/persistence.rs` | 788 | production |
| `apps/desktop-client/src/commands/topology/semantics.rs` | 254 | production |
| `apps/desktop-client/src/commands/topology/topology_tests.rs` | 2,581 | unit tests (134) |
| `apps/desktop-client/src/commands/topology/topology_command_tests.rs` | 1,729 | command/crash-injection tests (53) |
| `apps/desktop-client/src/commands/topology/topology_stress_tests.rs` | 1,670 | stress/schema-evolution tests (54) |
| `apps/desktop-client/src/commands/topology/model_tests.rs` | 358 | unit tests (34) |
| `apps/desktop-client/src/commands/topology/persistence_tests.rs` | 216 | unit tests (29) |

## Verification (all green)

- `cargo check -p oz-core -p oz-pos-app` — **clean, zero warnings** (lib + bin).
- `cargo check -p oz-pos-app --tests` — **clean** (test targets compile).
- `cargo test -p oz-core --lib topology` — **42/42 pass**.
- `cargo test -p oz-pos-app --lib topology` — **306/306 pass** (includes the NUL-character rejection tests, which work — see TOP-01).
- No `unsafe`, `todo!`, `unimplemented!`, or `dbg!` anywhere in the area; the 9 `panic!` occurrences are all inside test code.
- Every DB write is inside a transaction; the revision-guarded save uses `TransactionBehavior::Immediate` (documented TOCTOU fix). All SQL is parameterized. Settings keys validate control chars / `/` / length before use.
- `lib.rs` registers exactly the 3 commands (`load_topology`, `can_save_topology`, `apply_topology_diff`) + startup recovery, matching the root's re-export surface.

## Architecture summary

The topology area was recently split from one ~8.5k-line file into a clean layered layout:

- **`oz-core::topology`** — Tauri-free, value-level semantic validation core (shared contract from `topologySemantics.json` via `include_str!`, cycle detection via Kahn's algorithm, typed-connection gates, ADR #34 gates). Reused by any client (desktop, tablet, tooling).
- **`commands/topology/model.rs`** — typed payloads + resilient serde (NaN/Infinity → 0.0, null → defaults, `#[serde(other)]` unknown folds).
- **`commands/topology/semantics.rs`** — desktop-side validation adapter (maps `CoreError::TopologyValidation` → `AppError::TopologyValidation`), apply-key/fingerprint/revision/ledger helpers.
- **`commands/topology/persistence.rs`** — branch-scoped settings keys, save/load, runtime-plan compilation, and the cross-database Apply **recovery journal** (forward-write + compensation with retryable journal; startup recovery in `lib.rs`).
- **`commands/topology/commands.rs`** — the 3 `#[tauri::command]` entry points; `apply_topology_diff` is a single-transaction workspace diff with idempotent request-ledger replay and compensation on global-DB write failure.

The split preserves the flat test namespace through root re-exports, and sibling `*_tests.rs` files are wired per AGENTS.md (`#[cfg(test)] #[path = "..."] mod tests;`). Test organization is strong: shared `pub(crate)` helpers in `topology_tests.rs`, subject-split suites, Tauri integration via a mock app, and crash-injection recovery tests.

## Findings

### TOP-01 — Raw control bytes (NUL/SOH) embedded in test string literals

**Evidence:** `apps/desktop-client/src/commands/topology/persistence_tests.rs:32-33` contains literal NUL (`0x00`) and SOH (`0x01`) bytes inside string literals (`"branch<NUL>test"`, `"branch<SOH>test"`) in `topology_setting_key_control_chars_rejected`. Introduced by commit `2bb06e9d` (2026-08-19, "style: normalize test module declarations across workspace"). The byte scan found exactly 2 control bytes in the whole topology area, both here. rustc accepts them (the 306-test run passes) and the test genuinely asserts the control-char rejection path.

**Impact:** The file is treated as binary by many tools — the read/grep tooling flags it, `git diff` degrades, code review and future refactors become harder. It compiles today only because rustc tolerates raw control bytes inside string literals; any toolchain tightening or an editor re-encoding the file could silently change or break it.

**Severity:** P2 · source hygiene

**Affected files:** `apps/desktop-client/src/commands/topology/persistence_tests.rs`

**Recommendation:** Replace the raw bytes with escapes: `"\u{0}"` and `"\u{1}"` (or `"\0"` / `"\x01"`). The assertion semantics are unchanged. Add a regression note so future control-char tests use escapes.

**Status:** Open (fix pending approval)

### TOP-02 — Runtime setting-key constant duplicated across three modules

**Evidence:** The string `"oz-pos/topology-runtime"` is declared in three places: `topology/model.rs:256` (`pub(crate) const TOPOLOGY_RUNTIME_SETTING_KEY`), `commands/kds.rs:21`, and `commands/pos.rs:24` (both module-private copies). `kds.rs:53` and `pos.rs:68` build their read keys by string-concatenating their local copy; the writer (`persistence.rs:28`) builds keys from the model copy.

**Impact:** The key scheme is the cross-module contract between the topology writer and the KDS/POS runtime consumers. Any future change to the key prefix must be edited in 3 files with no compiler error if one is missed — exactly the kind of drift that silently breaks routing at runtime (the branch-scoped write/read pairing is otherwise tested and correct today).

**Severity:** P2 · maintainability (single source of truth)

**Affected files:** `apps/desktop-client/src/commands/topology/model.rs`, `apps/desktop-client/src/commands/kds.rs`, `apps/desktop-client/src/commands/pos.rs`

**Recommendation:** Re-export the constants from the topology root (e.g. `pub(crate) use model::TOPOLOGY_RUNTIME_SETTING_KEY;` in `topology.rs`) and import them in `kds.rs`/`pos.rs`. Longer-term, move the settings-key constants into `oz-core::Settings` so any consumer (cloud-server, tablet) shares one definition.

**Status:** Open (fix pending approval)

### TOP-03 — Duplicated paragraph in `load_topology_data` doc comment

**Evidence:** `apps/desktop-client/src/commands/topology/persistence.rs:780-785` — "Returns `None` when no topology has been saved yet." appears twice back-to-back.

**Impact:** Cosmetic doc defect; makes the API look carelessly maintained, which matters for a long-lived surface.

**Severity:** P3 · docs

**Affected files:** `apps/desktop-client/src/commands/topology/persistence.rs`

**Recommendation:** Delete the duplicate paragraph.

**Status:** Open (fix pending approval)

### TOP-04 — `save_topology_data` duplicates structural validation with drift risk

**Evidence:** `persistence.rs:669-778` (`save_topology_data`) inlines the same checks as `validate_topology_structure` (`persistence.rs:603-656`): unique wire ids, unique node ids, unknown node types, unknown directions, unknown ports, ghost endpoints. The two already diverge in wording (`"wire {} has unknown from_port"` vs `"wire {} has unknown port"`).

**Impact:** Two validation paths that must stay semantically in sync. A future rule added to one (e.g. a new structural invariant for expansion) is easy to miss in the other, producing inconsistent save behavior between the legacy typed API and the Apply path.

**Severity:** P3 · maintainability

**Affected files:** `apps/desktop-client/src/commands/topology/persistence.rs`

**Recommendation:** In `save_topology_data`, normalize ports then delegate to `validate_topology_structure` (plus keep the load-side raw boundary). Error messages then come from one place.

**Status:** Open (fix pending approval)

### TOP-05 — Legacy typed API (`save_topology_data`/`load_topology_data`) is now production-dead

**Evidence:** Both are `pub` and re-exported from the topology root (`topology.rs:33`), but the only callers in the whole workspace are tests. Production persistence flows exclusively through `apply_topology_diff` (branch-scoped, revisioned, journaled). The legacy pair writes/reads only the unscoped `oz-pos/topology` key.

**Impact:** A public API surface that production no longer uses, with its own validation copy (TOP-04) and unscoped-key semantics that contradict the branch-scoped direction of the area. Future contributors may mistake it for the write path.

**Severity:** P3 · API surface hygiene

**Affected files:** `apps/desktop-client/src/commands/topology/persistence.rs`, `apps/desktop-client/src/commands/topology.rs`

**Recommendation:** Either (a) mark them `#[cfg(test)]` (they exist to preserve low-level round-trip coverage), or (b) keep them pub but add an explicit `#[deprecated]` note documenting the production path (`apply_topology_diff`). The tests stay either way.

**Status:** Open (fix pending approval)

### TOP-06 — Missing module docs / doc comments on some pub(crate) items

**Evidence:** `model_tests.rs` and `persistence_tests.rs` start with `use super::*;` and lack the `//!` module doc the other three test files carry. `pub(crate)` functions without doc comments: `model.rs:247` (`default_direction`), `semantics.rs:61/73/107/191` (`topology_apply_request_key`, `topology_revision_from_json`, `topology_apply_ledger_json`, `topology_validation`), `persistence.rs:112/240/252` (`save_topology_json_at_key_with_revision`, `persist_topology_recovery`, `clear_topology_recovery`).

**Impact:** AGENTS.md requires doc comments on public items; `pub(crate)` is internal so this is not a violation, but the area's own convention (everything else has docs) is inconsistent, and undocumented helpers are harder to extend safely later.

**Severity:** P3 · docs consistency

**Affected files:** `apps/desktop-client/src/commands/topology/model.rs`, `semantics.rs`, `persistence.rs`, `model_tests.rs`, `persistence_tests.rs`

**Recommendation:** Add one-line doc comments to the listed items and module docs to the two test files.

**Status:** Open (fix pending approval)

### TOP-07 — O(n²) scans in the validation core (scale note only)

**Evidence:** `crates/oz-core/src/topology.rs`: `workspace_ids.contains(...)` inside the location-wire loop (line 427), `nodes.iter().any(...)` per wire (line 428), and `nodes.iter().find(...)` per workspace (lines 501/507, and inside the KDS/warehouse loops). Same pattern in `compile_topology_runtime_plan` (index maps are already built there — good) and `validate_warehouse_capacity`.

**Impact:** None at current scale (topologies are tens of nodes). Worth revisiting when the area expands to hundreds/thousands of nodes, per the long-term growth plan. This is a note, not a defect.

**Severity:** P3 · performance (future)

**Affected files:** `crates/oz-core/src/topology.rs`, `apps/desktop-client/src/commands/topology/persistence.rs`

**Recommendation:** When topology scale grows, hoist id→node lookups into a `HashMap` (the cycle detector already models the right shape). No change now.

**Status:** Note (no action)

### TOP-08 — File sizes vs. guidelines (watch items)

**Evidence:** `persistence.rs` = 788 lines (under the 1,000 cap, above the preferred 600). Test files: `topology_tests.rs` 2,581, `topology_command_tests.rs` 1,729, `topology_stress_tests.rs` 1,670. All respect the commands-dir ~3k-line guideline the module docs cite.

**Impact:** Production is compliant; test files are large but well-organized by subject with shared helpers. As the area expands, the 2.5k-line `topology_tests.rs` will hit the internal guideline first.

**Severity:** P3 · maintainability (future)

**Affected files:** `apps/desktop-client/src/commands/topology/*`

**Recommendation:** No change now. When the next test subject is added, split `topology_tests.rs` first (the helpers in it are already `pub(crate)`-shared, so a split is mechanical).

**Status:** Note (no action)

## Strengths worth preserving

- **Layered split with a Tauri-free core** — the semantic engine lives in `oz-core` and is reusable beyond desktop; the desktop layer is a thin adapter + wire mapping. This is the right shape for expansion (tablet, cloud, tooling).
- **Invariant documentation is excellent** — TOCTOU fix rationale, non-reentrant-mutex deadlock note, nested-transaction explanation, raw-load-boundary rationale, frontend-parity comments. This is rare and valuable.
- **Idempotency + recovery design** — request-ledger fingerprint replay, retryable cross-DB compensation journal, startup recovery. Well tested (crash-injection suites).
- **Resilient serde** — NaN sanitization, null-tolerant deserialization, unknown-fold enums, load-time minimal shape gate that never bricks a topology over one legacy row.
- **Security boundaries** — settings-key validation, parameterized SQL, store-scoping checks, entitlement checks at Apply, strict semantic gate on the authenticated mutation path.

## Proposed remediation (pending approval)

| Fix | Items | Est. risk |
|---|---|---|
| A | TOP-01 control-byte escapes | trivial, test-only |
| B | TOP-02 single-source runtime key constant | trivial |
| C | TOP-03 doc dedupe | trivial |
| D | TOP-04 delegate to `validate_topology_structure` | low (covered by existing tests) |
| E | TOP-05 deprecate legacy typed API | trivial |
| F | TOP-06 doc-comment fill-in | trivial |

Each fix is followed by `cargo check` + the topology test suites (306 + 42) and a local commit. Audit stamps (`/* last audited … */`) to be added to the 6 production files after remediation.

## Verification commands used

```
cargo check -p oz-core -p oz-pos-app
cargo check -p oz-pos-app --tests
cargo test -p oz-core --lib topology
cargo test -p oz-pos-app --lib topology
```
