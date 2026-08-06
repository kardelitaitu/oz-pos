# Validation

## Focused checks

- `cargo fmt --all -- --check`
- `cargo test -p platform-sync queue::tests::apply_remote_atomic -- --nocapture`
- `cargo test -p platform-sync daemon::tests::daemon_applies_replayed_remote_item_only_once -- --nocapture`
- `cargo test -p platform-sync daemon::tests::daemon_retains_anchor_until_remote_item_is_dead_lettered -- --nocapture`
- `cargo test -p oz-core migrations::tests::migrations_create_expected_tables -- --nocapture`
- `cargo check -p platform-sync -p oz-core`

## Acceptance criteria

- Same-SKU catalog conflicts fail without a receipt.
- Remote failures are retained with attempts and payload.
- Third failure becomes dead-lettered; later replay is skipped.
- Retryable failure retains the old pull anchor.
- Successful replay is idempotent and clears stale failure state.
- Migration 119 is registered and expected-table coverage passes.
