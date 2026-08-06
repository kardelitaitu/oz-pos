# Validation plan

## Required checks

- `cargo fmt --all -- --check`
- `cargo test -p platform-sync queue::tests::apply_remote -- --nocapture`
- `cargo test -p platform-sync daemon::tests -- --nocapture`
- `cargo test -p oz-core db::offline::tests::sync_applied_items_tracks_ids -- --nocapture`
- `cargo test -p oz-core db::products::tests -- --nocapture` (or the focused
  stock tests available in the current test module)

## Acceptance criteria

- A remote item applied twice changes local stock only once.
- A duplicate remote stock movement does not create a second ledger row after
  the first atomic application is committed.
- A malformed or insufficient-stock remote mutation returns an error, leaves no
  receipt, and does not advance the pull anchor.
- The daemon uses the atomic path; no production daemon path performs a separate
  mutation followed by a best-effort receipt.
- Existing EventBus behavior and checkout publication contracts remain unchanged
  in this slice.
- Existing architecture-boundary pilot and pending CHANGELOG edit remain
  untouched except for explicitly documented additions.

## Follow-up acceptance criteria

- Critical and best-effort event handlers are classified in a reviewed ADR.
- A durable outbox or equivalent retry mechanism exists for critical post-commit
  effects.
- Event delivery and sync operations carry stable operation IDs and expose
  operator-visible retry/dead-letter state.
