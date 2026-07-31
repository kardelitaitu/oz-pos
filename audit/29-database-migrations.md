# Database Migrations Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Database migrations — ordering, idempotency, transaction safety, rollback coverage, index/constraint quality, schema documentation, and fresh-vs-upgrade parity
> **Status:** AUDITED · migration safety and schema-contract findings require remediation
> **Production code changed:** None

## Scope

This audit evaluates sector 29 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers the migration registry, generic runner, embedded SQL, transaction boundaries, migration ordering, idempotency, rollback behavior, foreign keys, tenant/location scoping, indexes, constraints, fresh-install behavior, upgrade behavior, and migration test coverage.

Inspected areas:

- `crates/oz-core/src/migrations.rs`
- `crates/oz-core/migrations/*.sql`
- `platform/core/src/database/migrations.rs`
- `docs/database-optimization-2026-07-20.md`
- `docs/ARCHITECTURE.md`
- Migration-related tests embedded in `crates/oz-core/src/migrations.rs` and `platform/core/src/database/migrations.rs`

## Architecture summary

OZ-POS embeds every core migration with `include_str!` and registers it manually in `crates/oz-core/src/migrations.rs:40-578`. The generic `platform-core` runner creates `schema_migrations`, loads applied IDs, executes each unapplied migration inside a SQLite transaction, records the ID, and commits at `platform/core/src/database/migrations.rs:33-43` and `:113-123`. Core startup then enables WAL, a busy timeout, `synchronous=NORMAL`, and foreign keys after migration execution at `crates/oz-core/src/migrations.rs:587-610`.

The design has useful safety controls: migration IDs are primary keys, application is transactional, fresh databases are tested through the complete registered list, and a small generic rollback helper prevents out-of-order rollback. The schema has substantial foreign-key, check-constraint, partial-index, and location-scoping coverage. However, the registry is manual and ID-only, rollback metadata is external to migrations, upgrade-path testing is thin, and several destructive rebuild migrations depend on SQLite PRAGMA behavior that is not tested against populated legacy data.

## Findings

### DB-01 — Manual migration registry can silently omit SQL files and contradicts ordering documentation

**Evidence:** `crates/oz-core/src/migrations.rs:3-5` says migrations run in lexicographic order, while `ALL` is a hand-maintained compile-time array at `:40-578` and the runner applies that array order at `platform/core/src/database/migrations.rs:33-43`. The registry itself documents that adding a migration requires both a new SQL file and a new array entry at `migrations.rs:9-12`. The same file documents shared numeric prefixes at `:30-39`, including the non-lexicographic placement of `047_purchase_orders.sql` before `046_track_serial.sql` in the array at `:218-230`.

**Impact:** A new SQL file can exist in the repository but never execute if its registry entry is forgotten. A future maintainer following the module-level lexicographic-order claim can also place an entry incorrectly relative to dependencies. The existing uniqueness test validates registered IDs, not parity between the filesystem and the registry.

**Severity:** P2 · schema delivery and maintainability

**Affected files:** `crates/oz-core/src/migrations.rs`, `crates/oz-core/migrations/`, `platform/core/src/database/migrations.rs`, and migration CI/tests.

**Recommendation:** Make the registry the explicit source of truth and correct the stale lexicographic wording, or generate/validate the registry from the SQL directory. Add a test or CI script that compares every migration filename with exactly one registered ID, rejects orphaned SQL files, rejects registry entries without files, and verifies dependency-sensitive order. Keep historical IDs immutable and forbid reuse of gaps.

**Status:** Open

### DB-02 — Applied migration IDs are tracked without SQL checksums or compatibility metadata

**Evidence:** `platform/core/src/database/migrations.rs:20-26` defines a migration as only an `id` and raw `sql`. `schema_migrations` stores only `id` and `applied_at` at `:82-90`; `load_applied` checks only whether the ID exists at `:92-100`. If an already-applied SQL file changes while retaining its ID, the runner skips it without detecting the definition drift.

**Impact:** A modified historical migration can make fresh databases and upgraded databases produce different schemas while both report the same migration IDs. This is particularly dangerous for data migrations and table rebuilds: operators cannot tell whether an installed database was created by the committed definition or by an older mutation of that file.

**Severity:** P1 · schema reproducibility and upgrade safety

**Affected files:** `platform/core/src/database/migrations.rs`, `crates/oz-core/src/migrations.rs`, `schema_migrations`, release procedures, and migration tests.

**Recommendation:** Record a cryptographic checksum and migration format/version alongside each applied ID. On startup, compare the committed checksum with the stored value and fail closed with an actionable repair message when a historical definition changed. If a compatibility exception is required, make it an explicit migration or a reviewed checksum allowlist—never silently accept changed SQL under the same ID.

**Status:** Open

### DB-03 — Rollback support is generic and caller-supplied, but registered migrations have no down SQL

**Evidence:** `platform/core/src/database/migrations.rs:46-80` exposes `rollback(conn, migration_id, down_sql)`, where the caller supplies arbitrary reverse SQL. `Migration` has no `down_sql` field at `:20-26`, and the core registry entries at `crates/oz-core/src/migrations.rs:40-578` contain only `id` and `sql`. The tests prove rollback for synthetic tables, but do not provide rollback definitions for the production schema migrations.

**Impact:** The documented rollback capability is not an operational rollback plan for the real database. An operator cannot safely ask the runner to reverse migration 081, 089, 091, or other rebuild/data migrations without hand-authoring destructive SQL. A guessed down script can lose data, violate foreign keys, or leave the schema inconsistent.

**Severity:** P2 · recovery and operability

**Affected files:** `platform/core/src/database/migrations.rs`, `crates/oz-core/src/migrations.rs`, release/backup procedures, and migration runbooks.

**Recommendation:** Treat production migrations as forward-only unless a reviewed down migration is actually maintained. Document that distinction explicitly. For reversible schema-only changes, add versioned, reviewed down SQL or a repair command; for destructive/data migrations, require a backup-plus-forward-repair procedure and test restore. Never accept ad-hoc rollback SQL as the primary recovery contract.

**Status:** Open

### DB-04 — Location/store scoping columns remain nullable and the schema intentionally accepts unscoped domain rows

**Evidence:** `069_data_scoping_columns.sql:4-7` defines NULL as “unscoped / legacy / global shared” and adds nullable `store_id`/`warehouse_id` columns at `:17-28`. The migration creates scoped indexes at `:36-49`, but no foreign keys or non-null enforcement tie those columns to store/location tables. The migration tests explicitly insert products and sales without a `store_id` and assert NULL at `crates/oz-core/src/migrations.rs:906-943`.

**Impact:** The database permits rows that are not attributable to a tenant/store. If any read or write path forgets its application-level scope predicate, an unscoped row can appear across stores or a new row can be created without ownership. The migration is intentionally transitional, so this is not proof of an exploitable cross-tenant read by itself, but it is a material defense-in-depth gap for a multi-store deployment.

**Severity:** P1 · tenant/data isolation

**Affected files:** `069_data_scoping_columns.sql`, related domain tables and queries, `crates/oz-core/src/migrations.rs`, and store/workspace access policy.

**Recommendation:** Define an explicit transition policy: backfill legacy rows to a known store or quarantine them, then make tenant ownership non-null for tables that must be isolated. Add foreign keys where the store catalog is authoritative, composite uniqueness/indexes that include the scope, and integration tests proving a store-scoped query cannot return NULL/other-store rows. If global rows remain supported, represent that state explicitly and test every caller's policy rather than relying on NULL semantics.

**Status:** Open

### DB-05 — Migration 081 relies on an ineffective foreign-key PRAGMA during a destructive table rebuild

**Evidence:** The runner opens a transaction before executing each migration at `platform/core/src/database/migrations.rs:113-121`. Migration 081 executes `PRAGMA foreign_keys = OFF` and later `PRAGMA foreign_keys = ON` inside that transaction (`081_stock_transfers_received_partial.sql:60-64`); SQLite does not change the connection-level `foreign_keys` setting while a transaction is active. The migration then drops and renames `stock_transfers` at `:109-111` while `stock_transfer_lines` retains a foreign-key relationship to it. Migration 089 also uses the same PRAGMA pattern for its `stock_summary` rebuild (`089_stock_summary_composite_pk.sql:35-64`), but the inspected schema does not show an equivalent child-table reference for that table. The current core migration tests exercise a fresh database and schema/index assertions, but do not seed populated transfer-line rows and run migration 081 as an upgrade fixture.

**Impact:** For migration 081, enabled foreign keys can make `DROP TABLE stock_transfers` fail or invoke the child table's `ON DELETE CASCADE`, risking loss of `stock_transfer_lines` during a populated upgrade. The SQL comments assume foreign-key checks are disabled, but the runner's transaction boundary can make that assumption false. Migration 089 still deserves a PRAGMA/upgrade test, but it is not evidence of the same child-reference data-loss risk. Fresh migration tests can pass because they do not exercise populated dependent records.

**Severity:** P1 · upgrade data integrity

**Affected files:** `platform/core/src/database/migrations.rs`, `081_stock_transfers_received_partial.sql`, `047_stock_transfers.sql`, related foreign-key tables, and migration upgrade tests.

**Recommendation:** Do not rely on toggling `foreign_keys` inside a transaction. Use a rebuild pattern that preserves dependent tables and foreign-key metadata under the active enforcement mode, or move the controlled prerequisite outside the transaction only with an explicitly verified atomicity plan. Add an upgrade fixture containing populated `stock_transfer_lines`, stock summaries, and representative references; assert row counts, FK targets, `PRAGMA foreign_key_check`, and rollback behavior after each rebuild.

**Status:** Open

### DB-06 — Fresh-install and upgrade-path parity is not comprehensively tested

**Evidence:** `crates/oz-core/src/migrations.rs:676-706` tests that the full registered list applies and is idempotent on a fresh in-memory connection. Additional tests inspect selected tables, indexes, location seeds, and migration 100. The generic runner tests at `platform/core/src/database/migrations.rs:151-172` cover synthetic first/second runs. No inspected migration test constructs a representative pre-migration database, applies only the migrations available at that historical point, seeds realistic rows, upgrades through 106, and compares the resulting schema/data invariants with a fresh install.

**Impact:** A migration can work on an empty database while failing on real legacy data, losing dependent rows during a rebuild, or leaving stale indexes/foreign keys. This is the highest-risk blind spot for the many table-rebuild migrations and for data transformations such as 092's delete-and-rebuild of `stock_summary`.

**Severity:** P1 · upgrade correctness

**Affected files:** `crates/oz-core/src/migrations.rs`, `platform/core/src/database/migrations.rs`, `crates/oz-core/migrations/*.sql`, migration integration fixtures, and release upgrade validation.

**Recommendation:** Maintain versioned upgrade fixtures for each destructive/rebuild milestone. Seed realistic rows, apply the remaining migrations, run `PRAGMA foreign_key_check`, compare normalized `sqlite_master` schemas and indexes against a fresh install, and assert business data conservation. Include interrupted/failing migration recovery and backup-restore tests in the release gate.

**Status:** Open

### DB-07 — Migration 092 performs a destructive aggregate rebuild whose source-of-truth assumptions are not enforced by schema checks

**Evidence:** `092_rebuild_stock_summary_group_by_location.sql:42-70` deletes every row from `stock_summary` and recreates it from `stock_movements`, grouped by `(item_id, location_id)`. The migration also zeroes `inventory.qty` based on an item-level aggregate at `:72-88`. The SQL is transaction-wrapped by the runner, but the schema does not enforce that every valid stock summary is derivable from the movement ledger, nor does the migration test seed a non-trivial ledger and verify conservation across locations.

**Impact:** If stock summary contains an intentional adjustment or if legacy movement rows are incomplete, the migration silently discards the summary state and reconstructs a different balance. The item-level inventory zero-out can also affect all inventory rows for a product when the schema evolves toward multiple locations. Transactionality prevents a half-applied state, but it does not prevent a semantically incorrect rebuild.

**Severity:** P2 · inventory data integrity

**Affected files:** `092_rebuild_stock_summary_group_by_location.sql`, stock movement/summary domain code, `crates/oz-core/src/migrations.rs`, and inventory migration fixtures.

**Recommendation:** Specify the authoritative ledger contract and validate it before rebuilding: compare pre/post totals, reject or quarantine orphaned movement rows, and record a migration audit summary. Seed positive, negative, multi-location, and legacy rows in an upgrade test. Keep the rebuild inside a transaction, but add conservation assertions rather than treating atomicity as correctness.

**Status:** Open

### DB-08 — Several integrity rules remain application-only despite being described as data invariants

**Evidence:** `083_workspace_inventory_locations.sql` documents that the single-binding/multi-binding XOR is enforced in the application layer and uses a partial unique index only for one primary at `:61-93`. `100_setting_updated.sql` defines a per-terminal version ledger and indexes at `:12-32`, but has no unique constraint on `(key, terminal_id, version)`. The comments describe version allocation as `MAX(version) + 1`, which is vulnerable to duplicate versions if concurrent writers do not serialize the operation in application code. Other configuration columns, such as `enabled` in `087_stock_thresholds_alerts.sql` and `schema_version` in `104_hardware_profiles.sql`, likewise have no domain checks.

**Impact:** A caller bypassing the intended repository path, a concurrent writer, or a future API can create duplicate setting versions, multiple binding modes, or invalid flag/version values. Indexes improve lookup but do not enforce the invariants described by the schema comments.

**Severity:** P2 · constraint quality/concurrency

**Affected files:** `083_workspace_inventory_locations.sql`, `087_stock_thresholds_alerts.sql`, `100_setting_updated.sql`, `104_hardware_profiles.sql`, settings/location repositories, and migration tests.

**Recommendation:** For each invariant, explicitly choose database enforcement or application enforcement and document the reason. Add `UNIQUE(key, terminal_id, version)` (or a stronger event identity) where duplicate versions are invalid, allocate versions in a serialized transaction, and add `CHECK` constraints for boolean/version domains. For cross-table XOR rules, use a carefully tested trigger or enforce and test the repository boundary with concurrent integration tests.

**Status:** Open

## Positive controls observed

- Migration SQL is embedded at compile time, preventing a deployed binary from depending on a mutable SQL directory.
- Applied migration IDs are primary keys, so the runner does not apply the same registered ID twice.
- `apply_one` executes migration SQL and its tracking insert in one transaction at `platform/core/src/database/migrations.rs:113-123`.
- Core tests cover first-run application, second-run idempotency, registered-ID uniqueness, expected tables, selected indexes, canonical location seeds, and migration 100 schema/idempotency behavior.
- Generic runner tests cover rollback of the final synthetic migration and reject out-of-order rollback requests.
- Foreign keys, check constraints, partial unique indexes, and location-specific indexes are used extensively in the newer inventory/KDS schema.
- Migration 091 uses `PRAGMA defer_foreign_keys = ON` for its multi-table primary-key rename, showing awareness that ordinary FK toggling is insufficient for that operation.
- Runtime startup configures WAL, busy timeout, synchronous mode, and foreign-key enforcement after migration execution.

## Test and validation results

This was an evidence-only audit; no migration SQL, runner, or production code was changed.

Validation performed:

- Migration registry, runner, SQL, index/constraint, and documentation inventory: **completed**
- Ordering/idempotency/transaction/rollback and fresh-vs-upgrade evidence review: **completed**
- Targeted migration tests: **passed** — `cargo test -p oz-core migrations:: --lib` (17 passed, 0 failed)
- Full populated legacy upgrade fixture, schema snapshot comparison, and destructive-rebuild conservation tests: **not present / not run**
- Full workspace clippy/test validation: **not run for this documentation-only audit**
- Audit report formatting, whitespace, `git diff --check`, and audit-only scope review: **passed**

The targeted passing tests verify the current fresh-install and selected idempotency contracts; they do not close DB-02, DB-04, DB-05, DB-06, DB-07, or DB-08 because those findings concern checksum drift, tenant policy, populated upgrades, data conservation, and concurrency/constraint enforcement.

## Recommended remediation order

1. **DB-05/DB-06:** Build populated upgrade fixtures and correct the rebuild/foreign-key transaction strategy before the next schema-changing release.
2. **DB-02:** Add migration checksums and fail-closed historical-definition drift detection.
3. **DB-04:** Define and test the end state for nullable store/warehouse ownership.
4. **DB-07:** Add stock-ledger conservation checks around destructive summary rebuilds.
5. **DB-01/DB-03:** Make registry/file parity explicit and document forward-only versus reversible migrations.
6. **DB-08:** Enforce or concurrency-test the invariants currently left to application code.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to upgrade fixtures, schema-contract tests, and validation results.
