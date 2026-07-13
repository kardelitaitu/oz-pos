# C-2 — Replace `unsafe { std::env::set_var(...) }` with typed config + watch channel

- **Status:** TODO
- **Sprint:** 0.0.5-rc
- **Severity:** CRITICAL
- **Owner:** TBD (audit-triage)
- **Implementer:** pending
- **Closes:** audit finding C-2 (2026-07-12-desktop-app-audit)
- **Audit source:** `docs/specs/_active/2026-07-12-desktop-app-audit.md` §2

## Summary

`apps/desktop-client/src/state.rs:149` and
`apps/desktop-client/src/commands/features.rs:316, 330` mutate
process-level environment variables from an `async fn` running on a
tokio worker. `std::env::set_var` is **undefined behavior** in
multi-threaded contexts. Replace the env-var-as-config pattern with a
typed `AppState` config field plus a `tokio::sync::watch` channel so
subscribers (Redis pub/sub, inventory listener, LAN server) get a
deterministic view of runtime configuration.

## Baseline (pre-fix)

```rust
// apps/desktop-client/src/state.rs:149
// SAFETY: only called at startup before any threads are spawned.
unsafe { std::env::set_var("OZ_TERMINAL_ID", &id); }

// apps/desktop-client/src/commands/features.rs:316
// SAFETY: single-threaded setup.
unsafe { std::env::set_var("OZ_FEATURE_BRANCH", &branch); }

// apps/desktop-client/src/commands/features.rs:330
unsafe { std::env::set_var("OZ_FEATURE_FLAG_BATCH", &batch); }
```

The SAFETY comments are false under the actual call paths:
`set_feature` is `async fn` invoked from a tokio worker; the inventory
pub/sub listener, the LAN TCP server, and other tokio tasks all read
the env vars concurrently. The audit catalogued the real failure modes
(stale `OZ_TERMINAL_ID` race, Redis pub/sub filter wrong, panic or
segfault under contention).

## Acceptance criteria

- [ ] No `unsafe { std::env::set_var(...) }` in any production path
      (grep `apps/desktop-client/src/` and `apps/tablet-client/src/`)
- [ ] No `unsafe { std::env::remove_var(...) }` in any production path
- [ ] `AppState` gains a typed `config: Arc<RwLock<AppConfig>>` field
      (or `Arc<tokio::sync::RwLock<AppConfig>>` if hot-path readers are async)
- [ ] `AppState::set_terminal_id(&self, id: &str)` writes to the typed
      config and notifies subscribers via a `tokio::sync::watch` channel
- [ ] All call sites that previously called
      `unsafe { std::env::set_var("OZ_TERMINAL_ID", ...) }` now call
      `AppState::set_terminal_id(...)` or an equivalent typed setter
- [ ] Inventory pub/sub, Redis client, and LAN server subscribe to the
      watch channel and re-read terminal_id / feature flags when it fires
- [ ] New unit test: two concurrent setters from different threads do not
      race; the watch fires once per write and the readers see the
      new value
- [ ] All previously-passing tests still pass
- [ ] `cargo fmt --all -- --check` and
      `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Audit stamp at `apps/desktop-client/src/state.rs` and
      `apps/desktop-client/src/commands/features.rs` updates to
      `status: SAFE` (or stays `UNSAFE` for any remaining C-2-related
      findings; explicit residual)

## Plan (proposed)

1. **Add `AppConfig` struct** in a new module
   `apps/desktop-client/src/config.rs` (or extend `state.rs`):
   ```rust
   pub struct AppConfig {
       pub terminal_id: String,
       pub feature_branch: Option<String>,
       pub feature_flag_batch: Option<String>,
   }
   ```
2. **Add typed config + watch channel to `AppState`**:
   ```rust
   pub struct AppState {
       // existing fields ...
       pub config: Arc<tokio::sync::RwLock<AppConfig>>,
       pub config_tx: tokio::sync::watch::Sender<AppConfig>,
   }
   ```
3. **Replace `unsafe { std::env::set_var(...) }` callsites** with
   `AppState::set_*` methods that update the typed config and emit on
   the watch channel:
   ```rust
   impl AppState {
       pub async fn set_terminal_id(&self, id: String) -> Result<(), AppError> {
           {
               let mut cfg = self.config.write().await;
               cfg.terminal_id = id.clone();
           }
           self.config_tx.send_modify(|c| c.terminal_id = id);
           Ok(())
       }
   }
   ```
4. **Subscribe inventory pub/sub, Redis client, and LAN server** to
   `config_tx.subscribe()` and re-read fields on `changed()`.
5. **Delete the env-var reads** in places like
   `commands/features.rs::feature_filter_for_branch` — they should
   accept the config as a parameter or read it from `AppState`.
6. **Add unit test `concurrent_set_terminal_id_no_race`** in a new
   `#[cfg(test)] mod tests` block in `state.rs` that fires 100
   concurrent `set_terminal_id` calls and asserts the watch fires
   exactly 100 times and every reader sees a value that was
   written by exactly one caller.
7. **Update audit doc** to mark C-2 closed in §2, §6 (X-1), §7
   release-blocker list, and the audit stamps at the two stamped
   files.

## Verification (post-implementation)

```bash
# 1. No more unsafe env mutation
grep -rn 'env::set_var\|env::remove_var' apps/desktop-client/src/ apps/tablet-client/src/
# expect: 0 matches in production paths

# 2. Tests pass
cargo test -p oz-pos-app --lib
cargo test -p oz-pos-tablet --lib
cargo test -p oz-core --lib

# 3. Lint + fmt clean
cargo clippy -p oz-pos-app -p oz-pos-tablet --lib --tests -- -D warnings
cargo fmt --all -- --check

# 4. Audit grep returns empty for C-2
# (the grep listed in the audit doc §10 line 2 should now return 0 matches)

# 5. Audit stamp at state.rs and commands/features.rs updates to SAFE
head -5 apps/desktop-client/src/state.rs
head -5 apps/desktop-client/src/commands/features.rs
```

## Risks

- **Subscriber drop race**: if a subscriber's `select!` arm doesn't
  re-enter the `changed().await` future, it can miss updates. Mitigate
  with `Receiver::borrow_and_update()` at task start and an explicit
  `changed().await` re-arm in the loop.
- **Tauri command hot path**: `set_terminal_id` is called from a
  Tauri command; holding the `RwLock` write guard for the full send
  is correct but should be profiled to confirm it doesn't add
  millisecond-scale latency to the IPC roundtrip.
- **Test environment**: tests that previously set env vars via
  `unsafe { env::set_var(...) }` need to construct `AppState` with
  the typed config directly. The migration of test scaffolding is
  part of this card.

## Non-goals

- Persistent config storage: typed config is in-memory; the existing
  SQLite-backed `Settings` table is the source of truth on restart.
- Multi-process config sync: the typed config is per-process; the
  sync-client (Epic X-2's C-5 follow-up) handles persistence.
- Encrypted config values: out of scope; C-5 covers encryption.

## References

- `docs/specs/_active/2026-07-12-desktop-app-audit.md` §2 C-2
- `docs/specs/_active/2026-07-12-desktop-app-audit.md` §6 X-1 (epic)
- `docs/specs/_active/2026-07-12-desktop-app-audit.md` §7 release-blocker list
- `apps/desktop-client/src/state.rs:149`
- `apps/desktop-client/src/commands/features.rs:316, 330`
- `apps/desktop-client/src/lib.rs` (Tauri builder + `setup()`)
