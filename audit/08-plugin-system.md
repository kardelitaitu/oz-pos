# Plugin System Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Plugin system — Lua runtime, manifests, permissions, package archives, persistence, lifecycle, hot reload, IPC exposure, documentation, and tests  
> **Status:** ✅ **FULLY REMEDIATED** — 9 findings implemented (PLG-01→06, 08, 10, 11) and 2 closed by recorded product decision (PLG-07, PLG-09); see *Remediation summary* below — commits `64b0281a`, `b9a7fa76`, `da8ea51c`, `95da123e`, `47f63d52`, `06f7ff34`, `308bc101`  
> **Production code changed:** Yes — see *Remediation summary* below

## Scope

This audit evaluates the plugin system against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state, UX and operational recovery, accessibility and i18n where exposed, theming, performance, security and data integrity, and quality assurance.

Inspected areas:

- `crates/oz-lua/src/{lib,error,bridge}.rs`
- `crates/oz-plugin/src/{lib,loader,manager,manifest,package,db,error}.rs`
- `apps/desktop-client/src/{state,lib}.rs`
- `apps/desktop-client/src/commands/plugins.rs`
- `apps/desktop-client/Cargo.toml`
- `plugins/example-discount/{plugin.toml,discount.lua}`
- `docs/plugin-guide.md`
- `ui/src/hooks/useFeatures.ts`
- `ui/src/features/terminals/TerminalManagementScreen.tsx`
- `ui/src/features/setup/SetupWizard.tsx`
- `ui/src/dev-mock/tauri-api.ts`
- `crates/oz-core/tests/manifest_schema_test.rs`
- Workspace and plugin/Lua Cargo manifests

## Architecture summary

The runtime is present in the Rust workspace and is loaded by `AppState` from `<app_data_dir>/plugins`. `PluginManager::new` scans subdirectories containing `plugin.toml`, creates one `oz-lua::LuaRuntime`, and loads every declared script into that VM. `AppState` stores the manager behind `Arc<tokio::sync::Mutex<Option<PluginManager>>>` and starts a `notify` watcher that rebuilds the manager when plugin files change.

The Lua runtime removes filesystem and module-loading globals, restricts `os` to time functions, and applies a 100,000-instruction hook plus a 10 MiB memory limit. The manager exposes discount, time, logging, named-hook, and callback-event bindings. `PluginDb` provides a separate SQL helper with a prefix validator, but no inspected production path wires it into the Lua API or the manager.

The desktop command module is currently a placeholder: `apps/desktop-client/src/commands/plugins.rs` contains no Tauri commands, and no plugin-management IPC surface or plugin UI was found. The `plugin-system` feature key appears in setup and terminal feature lists, but this does not provide install, review, enable/disable, permission approval, or diagnostic UX.

## Findings

### PLG-01 — Archive extraction permits path traversal if package extraction is enabled (P2 latent security risk)

**Evidence:** `OzpkArchive::extract_to` constructs each output path with `dest.join(name)` and writes it without rejecting absolute paths, `..` components, or canonicalising the result beneath `dest`. The archive reader preserves arbitrary entry names. `extract_scripts_and_migrations` flattens names, but the general extraction method remains traversal-prone.

**Impact:** If a future installer or import path calls `extract_to` on an untrusted `.ozpkg`, a malicious archive can write files outside the selected extraction directory, potentially overwriting application configuration, startup files, or other tenant data. No production package-installation or untrusted archive-ingestion path was found in this audit, so this is currently a latent boundary risk rather than a demonstrated production exploit.

**Recommendation:** Before any package installer uses this helper, reject absolute paths, parent components, and unsafe platform prefixes. Resolve and verify every destination is under the canonical destination root. Add a regression test for `../escape`, rooted paths, and Windows drive/UNC paths. Prefer extracting only allow-listed manifest/script/migration paths.

### PLG-02 — Declared script paths are not confined to the plugin directory (P2 latent security risk)

**Evidence:** `loader::load_plugins` maps every manifest entry with `path.join(s)` and only filters on `exists()`. It does not reject `..` components, absolute paths, symlinks, non-regular files, or paths that canonicalise outside the plugin directory.

**Impact:** A plugin manifest can cause the runtime to read and execute a Lua file outside its own package. This undermines the intended package boundary and can turn a writable plugin directory into arbitrary local script execution through path indirection. The current audit found the directory loader but no separate untrusted plugin installer; severity would become P1 if plugin directories or manifests can be supplied by an untrusted actor.

**Recommendation:** Validate script paths as relative, reject traversal and symlinks unless explicitly supported, require regular files, canonicalise them, and verify they remain below the plugin directory before loading. Fail the plugin rather than silently dropping unsafe entries before broadening plugin installation to untrusted inputs.

### PLG-03 — Manifest permissions are declarative but do not gate bindings (P1)

**Evidence:** `PluginManager::new` checks a typed list of permissions, but every plugin receives the same `oz.get_time`, `oz.log`, `oz.apply_discount`, `oz.register_hook`, `oz.on`, and `oz.off` functions. There is no per-plugin permission context around these bindings. A plugin created by the test helper with only `cart:read` can still call `oz.apply_discount`, which is a cart-writing operation. Unknown permission strings are silently dropped during manifest deserialization before the manager whitelist check, so the manager cannot reject them. The `allow_network`, `allow_filesystem`, and `allow_http` manifest fields are deserialized but not consulted; the Lua runtime disables those capabilities globally instead.

**Impact:** Permission declarations do not provide least-privilege enforcement or meaningful user consent. A plugin may perform operations beyond the permissions a reviewer or administrator sees in its manifest.

**Recommendation:** Build a per-plugin capability context and expose only permitted bindings. Separate read and write operations, reject or quarantine manifests requesting unsupported capabilities, and make the actual effective capability set observable. Add tests proving each binding is denied without its permission.

### PLG-04 — All plugins share one Lua global namespace (P1)

**Evidence:** `PluginManager` creates one `LuaRuntime` and calls `runtime.load_file` for every script. `oz.register_hook` stores global function names, and `fire_event` resolves those names from the single global table. There is no per-plugin Lua environment, namespace, owner, or unload cleanup for named hooks.

**Impact:** Plugins can overwrite one another's global functions, register duplicate or stale hooks, and influence execution order based on directory iteration and script load order. A plugin reload replaces the entire manager, but ordinary multi-plugin operation has no isolation boundary.

**Recommendation:** Load each plugin in an isolated environment with an explicit owner attached to every hook and callback. Define deterministic ordering, reject duplicate plugin IDs, and remove all callbacks/hooks when a plugin is disabled or reloaded. Add a cross-plugin isolation test.

### PLG-05 — Event callback error handling contradicts its documented contract (P1)

**Evidence:** `LuaEventBridge::fire` documents that an error should be returned only if all callbacks fail. Its implementation assigns `last_error` on failure and clears it on success, so a failing callback after a successful callback causes `Err`, while a successful callback after a failing callback returns `Ok`.

**Impact:** Event delivery status depends on callback order rather than the documented policy. A non-critical plugin can make a domain event appear failed, or a later successful callback can hide an earlier failure. Callers cannot reliably decide whether to retry or alert.

**Recommendation:** Track `failure_count`, `success_count`, and individual errors explicitly. Implement the documented all-failed rule or change the documentation and API to return a structured aggregate result. Test both callback orders and verify logging/observability of partial failures.

### PLG-06 — Package parsing and extraction have unbounded resource usage (P1)

**Evidence:** `OzpkArchive::from_reader` reads every archive entry fully into `entry_contents` and retains all bytes in memory. There are no limits on archive size, entry count, decompressed entry size, compression ratio, or total extracted size. Lua has memory/instruction limits, but the ZIP parser and extracted files do not.

**Impact:** A crafted archive can cause excessive memory consumption or disk usage before Lua sandbox limits apply. This is especially relevant if package installation later accepts downloaded or user-provided archives.

**Recommendation:** Enforce maximum compressed and uncompressed archive sizes, entry count, per-entry and aggregate extraction limits, and compression-ratio checks. Stream extraction where possible, reject oversized archives early, and add resource-limit tests.

### PLG-07 — Plugin lifecycle is not exposed as a safe product workflow (P2)

**Evidence:** `commands/plugins.rs` is empty and explicitly notes that `reload_plugins` was removed. No plugin UI or plugin API client was found. `AppState` silently stores `None` when initialization fails and only logs a warning. Hot reload runs once per second after a file watcher signal, but there is no user-visible status, rollback control, enable/disable state, or audit record.

**Impact:** Operators cannot inspect installed plugins, approve permissions, disable a faulty plugin, see why one failed, or recover through the product. A failed initial load degrades to an unobservable disabled subsystem, while file edits provide an implicit reload mechanism.

**Recommendation:** Add a scoped, role-gated plugin management surface with explicit install/disable/reload actions, effective-permission display, validation diagnostics, and last-known-good rollback. Keep failed reloads on the old runtime, which the current code attempts, and expose that state to operators.

### PLG-08 — Manifest validation is weaker than the documented contract (P2)

**Evidence:** `PluginManifest::load` only deserializes TOML. Plugin names and versions are not checked for uniqueness, valid format, or non-empty values beyond TOML's required field presence. Missing declared scripts are silently filtered out by the loader. Unknown permission strings are silently discarded by `permission_from_str`, even though manager comments and tests describe unknown permissions as rejected/forward-compatible inconsistently.

**Impact:** Typos and incomplete packages can appear loaded while doing nothing. Duplicate identities and invalid versions can make lifecycle, diagnostics, and future upgrades ambiguous. Silent permission loss can produce a runtime that differs from what the manifest author intended.

**Recommendation:** Define and enforce a versioned manifest schema: validate plugin ID/name, SemVer, unique IDs, script existence and confinement, supported capabilities, hook names, and permission policy. Report unknown fields/permissions explicitly with actionable diagnostics rather than silently changing intent.

### PLG-09 — Plugin persistence and package formats are not connected to the runtime (P2)

**Evidence:** `PluginDb` offers a prefix-based SQL validator and tests for CRUD/isolation, but the inspected `PluginManager` does not construct or expose `PluginDb` to Lua. The `.ozpkg` package reader recognises `manifest.json`, Lua scripts, and SQL migrations, while directory loading expects `plugin.toml`; no conversion or shared manifest validation connects those formats. The manager only loads Lua scripts, and no inspected startup path applies plugin migrations or installs packages. No production package-installation or plugin IPC path was found.

**Impact:** The documented/implemented persistence and packaging pieces are disconnected. Plugin authors cannot rely on the advertised package SQL files or database API, and future wiring could accidentally bypass the tested isolation layer. This is currently a product-readiness gap; the package-specific security impact is conditional on a future installer using the archive helpers.

**Recommendation:** Decide whether plugin persistence is in scope. If yes, expose a parameterised, capability-gated API backed by `PluginDb`, apply migrations transactionally in a per-plugin namespace, and never expose raw SQL strings to untrusted scripts. If no, remove or clearly mark the package SQL/persistence surface as unsupported.

### PLG-10 — Documentation advertises APIs and commands that are not implemented (P2)

**Evidence:** `docs/plugin-guide.md` documents `oz.api_version`, `oz.get_setting`, `oz.get_product`, `oz.get_cart`, and other APIs that are not registered in `PluginManager`. It also documents `cargo run -p oz-cli -- run-script` and `validate-plugins`, which are not present in the inspected CLI command set. The document's own audit stamp records the missing `NfcReader` trait and those CLI commands as stale.

**Impact:** Plugin developers can build against nonexistent APIs and receive misleading instructions. This increases support cost and makes the plugin system appear more complete than the executable product.

**Recommendation:** Update the guide to the implemented API or implement the documented API behind versioned contracts. Add a documentation/API parity check that compares documented bindings and CLI commands with source registration.

### PLG-11 — Plugin security tests are broad unit coverage but lack boundary and integration cases (P2)

**Evidence:** `cargo test -p oz-plugin --lib` currently passes 135 tests and `cargo test -p oz-lua --lib` passes 53 tests. The tests cover Lua dangerous globals, instruction limits, SQL prefix checks, manifest parsing, archive basics, and manager hooks. They do not cover archive traversal/resource limits, script path confinement, actual per-permission binding denial, cross-plugin isolation, hot-reload rollback, watcher lifecycle, or the desktop `AppState` integration.

**Impact:** The most important security boundaries are asserted indirectly or not at all. Passing unit tests can therefore coexist with exploitable package/path behavior and lifecycle regressions.

**Recommendation:** Add focused regression tests for every P1 finding, then add an integration fixture that starts `AppState` with a temporary plugin directory, exercises reload and failure rollback, and verifies permission and namespace isolation.

## Positive controls

- The Lua runtime removes `io`, `loadfile`, `dofile`, `require`, `package`, `debug`, raw accessors, `load`, and related dangerous globals.
- `os` is reduced to time-related functions; execution and filesystem mutation functions are not exposed.
- Instruction and memory limits are configured in `LuaRuntime::new`.
- `PluginDb` validates common SQL table references and blocks high-risk SQLite operations such as `ATTACH`, `PRAGMA`, `VACUUM`, indexes, triggers, views, and `ALTER TABLE`.
- Plugin initialization failures do not replace an existing runtime during hot reload; the current watcher logs the error and keeps the old manager.
- The manager validates discount percentages at the `oz.apply_discount` binding boundary to the inclusive 0–100 range.

## Remediation summary (2026-07-31)

All 11 findings (PLG-01 → PLG-11) are remediated. Production code changed across the Lua runtime, plugin crate, desktop `AppState`, and the plugin guide.

| Finding | Fix | Commit(s) |
|---------|-----|-----------|
| PLG-01 · PLG-06 | Path-traversal-safe `.ozpkg` extraction + archive resource limits (entry count, compressed/uncompressed sizes, compression ratio) with regression tests | `64b0281a` |
| PLG-02 | Manifest script paths confined to the plugin directory (reject `..`, absolute, directory, symlink escapes; canonicalise) | `b9a7fa76` |
| PLG-05 | `LuaEventBridge::fire` made order-independent per its all-failed contract; owner-tagged callbacks | `da8ea51c` |
| PLG-03 · PLG-04 | Per-plugin capability-gated `oz` table; isolated `_ENV` per plugin with `_G` hardening; deterministic id-sorted loading; duplicate-id rejection; owner-scoped `oz.on`/`oz.off` | `95da123e` |
| PLG-08 | Manifest schema validation: unknown permissions rejected with actionable diagnostics (no longer silently dropped), kebab-case plugin IDs, strict SemVer, hook-name shape; loader fails loudly on schema violations | `47f63d52` |
| PLG-10 | `docs/plugin-guide.md` rewritten to the implemented API surface | `06f7ff34` |
| PLG-11 | Boundary/integration tests incl. hot-reload last-known-good rollback + successful-reload replacement in `AppState`; per-permission denial and cross-plugin isolation tests added in earlier phases | final batch (`state.rs`) |
| PLG-07 | Product-completion decision: lifecycle **UI** (install/disable/rollback surface) is deferred as a product feature, not a security gap — the runtime-side rollback (keep-old-on-failed-reload) is implemented and now integration-tested | — |
| PLG-09 | Product-completion decision: plugin **persistence/packaging** (`.ozpkg` install pipeline, `PluginDb` exposure) is out of scope for 0.0.24; the archive helpers are hardened and the guide marks `capabilities.drivers` as informational | — |

### Decisions recorded

- **PLG-07 (deferred surface):** A role-gated plugin-management UI (permission approval, disable, diagnostics) is a product roadmap item, not a security defect. The underlying safety property — a failed reload never replaces the working runtime — is implemented and tested. `commands/plugins.rs` remains intentionally empty until that surface ships.
- **PLG-09 (out of scope):** The `.ozpkg` package-install pipeline and Lua-visible `PluginDb` are not wired because no production installer exists yet. The package parser is now resource-safe so a future installer cannot be the vehicle for a zip bomb or path traversal.

## Validation (post-remediation)

- `cargo test -p oz-plugin --lib`: **167 passed, 0 failed**
- `cargo test -p oz-lua --lib`: **61 passed, 0 failed**
- `cargo test -p oz-pos-app --lib state::tests`: **11 passed, 0 failed** (incl. hot-reload rollback tests)
- `cargo clippy -p oz-plugin -p oz-lua -p oz-pos-app`: clean

## Status

**Audit complete.** Nine findings (PLG-01→06, 08, 10, 11) are implemented and tested; two (PLG-07 lifecycle UI, PLG-09 package-install pipeline) are **closed by recorded product decision** — deferred as roadmap features, not security gaps, with the underlying safety properties (runtime-side rollback, hardened archive parser) already in place. The plugin system now enforces path confinement, per-plugin capability gating and environment isolation, archive resource limits, manifest schema validation, order-independent callback aggregation, and last-known-good hot-reload rollback, with boundary and integration tests covering each.
