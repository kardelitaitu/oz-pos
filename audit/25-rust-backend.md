# Rust Backend Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Rust backend — clippy/unsafe policy, error propagation, database/API coherence, migration integrity, synchronization, and test coverage
> **Status:** AUDITED · backend correctness and reliability findings require remediation
> **Production code changed:** None

## Scope

This audit evaluates sector 25 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers workspace structure, unsafe-code policy, panic and unwrap usage, typed error propagation, Tauri/API command boundaries, SQLite transaction discipline, backup/repair behavior, migrations, offline queue semantics, sync transport, input validation, tenant boundaries, and Rust test coverage.

Inspected areas:

- `Cargo.toml`
- `crates/oz-core/Cargo.toml`
- `crates/oz-core/src/lib.rs`
- `crates/oz-core/src/error.rs`
- `crates/oz-core/src/db/mod.rs`
- `crates/oz-core/src/migrations.rs`
- `crates/oz-core/src/offline.rs`
- `crates/oz-core/tests/offline_integration.rs`
- `crates/oz-core/tests/backup_restore_integration.rs`
- `crates/oz-api/src/lib.rs`
- `crates/oz-cli/src/lib.rs`
- `crates/oz-hal/src/lib.rs`
- `crates/oz-lua/src/lib.rs`
- `crates/oz-security/src/lib.rs`
- `crates/oz-security/src/windows.rs`
- `platform/core/src/database/migrations.rs`
- `platform/sync/src/lib.rs`
- `platform/sync/src/transport.rs`
- `apps/desktop-client/src/commands/offline.rs`
- `apps/desktop-client/src/commands/data.rs`
- `apps/desktop-client/src/commands/sync.rs`
- Rust workspace tests and migration SQL

## Architecture summary

The workspace is split across domain crates (`oz-core`, `oz-api`, `oz-security`, `oz-payment`, and related crates), platform crates (`platform-core`, `platform-sync`, and startup/kernel support), feature modules, and Tauri desktop/tablet commands. `oz-core` exposes a typed `CoreError` with a serializable error-kind discriminator. SQLite migrations are embedded at compile time and applied by `platform-core` inside a transaction. Domain writes generally use `unchecked_transaction()` and commit explicitly.

Unsafe-code policy is strong but not uniform. `oz-core`, `oz-cli`, `oz-payment`, `oz-reporting`, and `oz-security` deny unsafe code at the crate root; `oz-security/windows.rs` deliberately allows FFI; `oz-hal` allows unsafe code at the crate level as a forward-looking hardware escape hatch; and `oz-lua` warns on unsafe code while allowing two explicit locations. The audit therefore treats the policy as a control that needs scoped review, not as proof that all backend paths are memory-safe or behaviorally safe.

The offline system currently has two paths: the general SQLite-backed queue exposed through Tauri commands, and the platform sync engine that talks to a remote server. The general `retry_offline_sync` command currently marks every pending item as synced without dispatching its action. The sync transport has richer push/pull/conflict machinery, but it is not wired into that generic retry command.

## Findings

### RUST-01 — Generic offline retry reports success without executing queued actions

**Evidence:** `apps/desktop-client/src/commands/offline.rs` documents `retry_offline_sync` as a placeholder. The implementation lists pending items and calls `store.mark_offline_synced(&item.id)` for every item; it does not dispatch `item.action`, invoke the remote sync engine, or verify that the payload was applied. The comment explicitly says real dispatch will be added later. The richer `platform-sync` crate has push/outcome handling, but the Tauri command does not call it. The same implementation is surfaced by the Offline Queue UI's Sync All action.

**Impact:** A user pressing “Sync All” can receive a successful result while sales, voids, inventory changes, or other queued operations were never executed. Marking an unapplied transaction as synced can permanently lose the user's intended operation and make reconciliation impossible.

**Severity:** P0 · data/transaction integrity

**Affected files:** `apps/desktop-client/src/commands/offline.rs`, `ui/src/features/offline/OfflineQueueScreen.tsx`, `platform/sync/src/lib.rs`, `platform/sync/src/transport.rs`, and the offline queue database methods.

**Recommendation:** Replace the placeholder with one authoritative dispatch path. For each action, validate and execute the payload through the appropriate domain command or enqueue it into the real sync engine; mark an item synced only after an accepted/idempotent server or local result. Preserve rejected/conflicted items as failed or conflict state, record the reason, and return per-item outcomes. Add an integration test proving a queued sale changes the database before it is marked synced and that a rejected action remains retryable.

**Status:** Open

### RUST-02 — Backup and repair interpolate filesystem paths into dynamic SQL

**Evidence:** `crates/oz-core/src/db/mod.rs:155-157` implements `vacuum_into` by replacing single quotes in `output_path` and interpolating the result into `VACUUM INTO '{escaped}'`; `backup()` at lines 165-167 and `repair_to()` at lines 218-224 both call this helper. The workspace already enables rusqlite's `backup` feature in `Cargo.toml`, and the repository has separate backup/restore integration tests.

**Impact:** Manual SQL construction makes backup behavior depend on escaping rules and SQLite path semantics rather than a typed backup API. Paths containing unusual characters, platform-specific names, or a future escaping mistake could cause backup failure or dynamic SQL injection. A backup destination should not need to be represented as executable SQL text.

**Severity:** P2 · security/reliability boundary

**Affected files:** `crates/oz-core/src/db/mod.rs`, `crates/oz-core/tests/backup_restore_integration.rs`, `crates/oz-cli/src/commands.rs`, and desktop data/sync backup commands.

**Recommendation:** Use rusqlite's online backup API (or a narrowly reviewed SQLite backup wrapper) instead of formatting a `VACUUM INTO` statement. Validate destination policy at the application boundary, preserve atomic/temporary-file behavior, and test paths containing quotes, Unicode, separators, existing files, and unwritable destinations. Do not treat a path string as SQL merely because quote doubling currently appears to work.

**Status:** Open

### RUST-03 — Repair silently ignores failure to remove an existing destination

**Evidence:** `crates/oz-core/src/db/mod.rs:218-222` calls `let _ = std::fs::remove_file(output_path);` before running `VACUUM INTO`. All removal errors, including permission failures and a destination that is a directory, are discarded. The subsequent error reports only that `VACUUM INTO` failed.

**Impact:** Operators receive an indirect SQLite error instead of the filesystem cause, and a stale destination may remain in place after a failed repair. This makes recovery tooling harder to diagnose and can lead an operator to believe a repaired database was produced when it was not.

**Severity:** P3 · recovery reliability

**Affected files:** `crates/oz-core/src/db/mod.rs`, `crates/oz-core/tests/backup_restore_integration.rs`, and callers of `Store::repair_to`.

**Recommendation:** Ignore only `ErrorKind::NotFound`; propagate all other removal errors as a typed/internal error containing the destination and OS error. Add tests for a missing target, an existing regular file, a directory target, and a permission failure where the platform permits it.

**Status:** Open

### RUST-04 — Snapshot import accepts untyped JSON and defaults required product fields

**Evidence:** `platform/sync/src/transport.rs` defines `SyncSnapshotResponse.products`, `tax_rates`, and `users` as `Vec<serde_json::Value>`. The import tests in `platform/sync/src/lib.rs` explicitly expect `import_snapshot_missing_sku_defaults_to_empty_string`, `import_snapshot_missing_name_defaults_to_empty_string`, and corrupted products to import with defaults. This bypasses a typed schema for reference data that is later written into SQLite.

**Impact:** Malformed or incomplete server data can create products with empty SKU/name values, collide with uniqueness constraints, or introduce records that cannot be reliably sold or searched. The permissive shape also makes API drift and tenant-data corruption harder to detect at the boundary.

**Severity:** P1 · data integrity

**Affected files:** `platform/sync/src/transport.rs`, `platform/sync/src/lib.rs`, `crates/oz-core/src/db/products.rs`, and the snapshot endpoint contract.

**Recommendation:** Define typed, versioned snapshot DTOs with required fields and domain validation. Reject missing/blank SKU, name, currency, and other required values before opening the import transaction; validate enums, numeric ranges, and tenant identity. Keep unknown fields compatible only where safe, and test malformed, null, blank, duplicate, and cross-tenant records as rejected rather than imported with defaults.

**Status:** Open

### RUST-05 — Sync HTTP client fallback can remove authentication and timeout guarantees

**Evidence:** `platform/sync/src/transport.rs:91-112` builds a configured client with default headers, `no_proxy`, gzip, and a 30-second timeout. If client construction fails, `unwrap_or_else` falls back to `reqwest::Client::new()`. The logged fallback explicitly says it operates with “no auth, no timeout,” and the same code path is used for sync transport construction.

**Impact:** An environmental TLS/client-construction failure can silently degrade the transport into requests without the configured bearer token and without a request timeout. Depending on endpoint authorization, this can cause authentication failures, indefinite sync hangs, or dangerous assumptions that a request was sent with tenant credentials.

**Severity:** P1 · security/reliability

**Affected files:** `platform/sync/src/transport.rs`, `platform/sync/src/lib.rs`, sync daemon construction, and server authentication middleware.

**Recommendation:** Make `SyncTransport::new` fallible and fail closed when the configured client cannot be built. If a fallback is unavoidable, recreate all security and timeout properties explicitly and return a degraded-state error rather than continuing silently. Add tests for client-construction failure where injectable configuration permits it, and verify authorization headers and timeout behavior at the transport boundary.

**Status:** Open

### RUST-06 — Unsafe policy has deliberate broad exceptions that need tighter scope

**Evidence:** `crates/oz-core/src/lib.rs`, `oz-cli`, `oz-payment`, `oz-reporting`, and `oz-security/src/lib.rs` deny unsafe code. However, `crates/oz-hal/src/lib.rs` uses `#![allow(unsafe_code)]`, `crates/oz-security/src/windows.rs` allows unsafe for platform FFI, and `crates/oz-lua/src/lib.rs` warns rather than denies unsafe and contains explicit allows. The repository search found no broad unsafe block in the inspected core paths, but the crate-level policy allows future unsafe additions without a hard compile-time gate in those scopes.

**Impact:** Hardware, Windows FFI, and embedded Lua integration are legitimate unsafe boundaries, but broad allows make accidental expansion harder to detect. A future unsafe block can bypass the review signal that protects the otherwise safe backend.

**Severity:** P2 · security/maintainability control

**Affected files:** `crates/oz-hal/src/lib.rs`, `crates/oz-security/src/windows.rs`, `crates/oz-lua/src/lib.rs`, and unsafe modules under those crates.

**Recommendation:** Prefer crate-level `deny(unsafe_code)` with narrowly scoped module or item allowances. Require a `// SAFETY:` rationale, encapsulate FFI behind small audited functions, and add CI that inventories unsafe blocks and compares them with an allowlist. Keep hardware/FFI exceptions documented with ownership and test requirements.

**Status:** Open

### RUST-07 — Panic-oriented APIs remain in production startup/API paths and make failure recovery process-fatal

**Evidence:** The repository search found `expect` in `crates/oz-api/src/lib.rs` for database opening, foreign-key/WAL setup, migrations, port binding, and server exit. `platform/sync/src/transport.rs` also uses fallbacks for client creation, while numerous `unwrap`/`expect` calls remain in production modules outside `#[cfg(test)]`. Some panics are appropriate for invariant-proven setup, but the current inventory does not enforce a distinction between startup boundaries, user input, and recoverable runtime operations.

**Impact:** A bad path, unavailable port, migration failure, or malformed runtime value can terminate a service without a structured error or actionable recovery. Panic behavior is especially costly in unattended desktop/cloud processes and can obscure the original failure from callers.

**Severity:** P2 · reliability and operability

**Affected files:** `crates/oz-api/src/lib.rs`, `platform/sync/src/transport.rs`, `crates/oz-cli/src/commands.rs`, and production modules containing non-test unwrap/expect calls.

**Recommendation:** Audit production-only `unwrap`/`expect` calls separately from test code. Return `Result` from startup and command boundaries, attach context with `thiserror`/`anyhow` at the application edge, and reserve panics for documented impossible invariants. Add a CI inventory or lint policy with reviewed exceptions rather than a blanket textual ban.

**Status:** Open

### RUST-08 — Database transaction discipline is strong but `unchecked_transaction()` obscures composability boundaries

**Evidence:** `crates/oz-core/src/db/mod.rs` documents that multi-row writes use `unchecked_transaction()`, and many domain methods open their own transactions. `platform/core/src/database/migrations.rs` uses a checked transaction for migration application. Tests in workspace/settings code explicitly document nested-transaction behavior and the inability of some repository methods to participate in an outer transaction.

**Impact:** A caller composing multiple domain operations may unknowingly trigger nested transaction errors, partial workflows, or architectural workarounds. The unsafe naming of `unchecked_transaction()` also makes it harder to tell which methods are intended to be transaction-aware versus standalone atomic commands.

**Severity:** P2 · data consistency/API coherence

**Affected files:** `crates/oz-core/src/db/*.rs`, `platform/core/src/database/migrations.rs`, `platform/core/src/settings/raw.rs`, and transaction-composition tests.

**Recommendation:** Define a repository transaction contract: standalone methods may own a transaction, while composable methods accept `&Transaction` or an internal transaction context. Prefer checked transactions where possible, document every intentional nested/savepoint boundary, and add a static review rule for new database methods that open transactions internally.

**Status:** Open

### RUST-09 — Migration registry has legacy shared numeric prefixes and relies on array order

**Evidence:** `crates/oz-core/src/migrations.rs` documents multiple files sharing prefixes `046` and `047`, and explains that the compile-time array order—not filename lexicographic order—controls application. The runner records migration IDs in `schema_migrations` and applies them transactionally, but the registry has historical ordering exceptions and many later migrations.

**Impact:** The documented legacy behavior is currently intentional, but adding or reordering an entry can apply schema changes in an unexpected order or make a fresh database differ from an upgraded database. Shared prefixes increase review and tooling ambiguity.

**Severity:** P3 · migration maintainability

**Affected files:** `crates/oz-core/src/migrations.rs`, `crates/oz-core/migrations/046_*.sql`, `crates/oz-core/migrations/047_*.sql`, `platform/core/src/database/migrations.rs`, and migration tests.

**Recommendation:** Keep existing IDs immutable, enforce unique IDs and one canonical order in a migration-registry test, and reject duplicate prefixes for new migrations. Add fresh-install versus upgrade-path schema comparison tests and document a rollback/repair strategy for failed migrations.

**Status:** Open

### RUST-10 — Backend tests are broad but do not fully enforce the highest-risk contracts

**Evidence:** Core error, offline queue, backup/restore, sync transport, API routes, and migration tests exist. However, the current tests explicitly lock in placeholder offline behavior (marking queued items synced), permissive snapshot defaults, and the backup path's current SQL-based implementation without adversarial path cases. No single backend contract suite was found that combines tenant isolation, action dispatch, idempotency, migration upgrade parity, and failure recovery.

**Impact:** High test volume can give a false sense of backend readiness while tests preserve known unsafe or incomplete semantics. Regressions in cross-layer behavior can pass crate-local unit tests because the command, database, and sync transport are tested separately.

**Severity:** P1 · quality assurance gap

**Affected files:** `apps/desktop-client/src/commands/offline.rs` tests, `platform/sync/src/lib.rs` tests, `platform/sync/src/transport.rs` tests, `crates/oz-core/tests/offline_integration.rs`, `crates/oz-core/tests/backup_restore_integration.rs`, API tests, and migration tests.

**Recommendation:** Add integration tests for queued action execution and idempotent replay; malformed snapshot rejection; tenant/store boundary enforcement; backup destination safety; fresh-versus-upgrade migration parity; and structured error mapping through Tauri/API boundaries. Keep tests that demonstrate current placeholder behavior only until the production behavior is replaced, then convert them into regression tests for the corrected contract.

**Status:** Open

## Positive controls observed

- The workspace uses typed `thiserror` errors in core and sync layers, with `CoreErrorKind` serialized for front-end branching.
- `oz-core`, payment, reporting, CLI, and most security-facing crates deny unsafe code at the crate root.
- SQLite migrations are embedded and applied transactionally by `platform-core`.
- Domain writes commonly use explicit transactions and tests cover rollback/atomicity in several modules.
- Sync transport distinguishes timeout, connection, request, HTTP, anchor-expired, and server-migrated failures.
- The sync protocol has explicit push outcomes, cursors, conflict handling, and snapshot recovery concepts.
- Offline queue tests cover enqueue, pending/synced/failed status, retry counts, serialization, and deletion.
- Backup/restore integration tests and API route tests provide a substantial foundation for hardening the identified gaps.

## Test and validation results

This was an evidence-only audit; no production Rust code was changed. Validation commands for the audit are:

```text
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Results:

- Static source inventory and report evidence review: **completed**
- Production code changed during this audit: **none**
- `cargo clippy --workspace --all-targets -- -D warnings`: **failed** on `clippy::await_holding_lock` in the existing staged loyalty changes at `apps/desktop-client/src/commands/loyalty.rs:346` and `apps/tablet-client/src/commands/loyalty.rs:350`
- `cargo test --workspace`: **not run**, because the chained validation stopped after clippy failed
- Audit report formatting/scope review: **passed** after the final report edit

The clippy failure is outside this audit report's Rust files and the staged loyalty work was intentionally not modified. It is nevertheless a workspace gate failure that should be resolved before claiming a clean backend validation. Even a clean clippy/test run would not invalidate RUST-01, RUST-02, RUST-04, or RUST-05 because those are behavioral contract findings rather than compiler failures.

## Recommended remediation order

1. **RUST-01:** Replace placeholder offline retry with real, idempotent action dispatch before enabling “Sync All” as a success path.
2. **RUST-04/RUST-05:** Fail closed on malformed snapshot data and transport-client construction failures.
3. **RUST-02/RUST-03:** Replace dynamic backup SQL and propagate destination filesystem errors.
4. **RUST-10:** Add cross-layer integration tests for queue dispatch, tenant boundaries, migration parity, and backup safety.
5. **RUST-06/RUST-07/RUST-08:** Tighten unsafe/panic policy and clarify transaction composition contracts.
6. **RUST-09:** Add migration registry/order and fresh-versus-upgrade consistency gates.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
