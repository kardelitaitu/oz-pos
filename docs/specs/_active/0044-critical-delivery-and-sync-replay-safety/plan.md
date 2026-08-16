# Critical delivery and sync replay safety

## Decision boundary

The event bus currently catches individual handler failures and continues, while
checkout commands publish after their local sale transaction commits. Making
`publish()` fail for every handler would couple checkout correctness to audit,
reporting, loyalty, notifications, and UI bridges. This pilot therefore keeps
that API stable and hardens the separate sync replay boundary first.

## Phase 1 — atomic remote application

1. Begin one SQLite transaction for each remote item.
2. Check `sync_applied_items` inside that transaction.
3. Apply the supported remote mutation through transaction-aware helpers.
4. Insert the remote receipt in the same transaction.
5. Commit only after both mutation and receipt succeed.
6. Leave the pull anchor unchanged when an item fails.

The existing standalone `apply_remote` remains available for isolated callers and
compatibility tests. The daemon uses `apply_remote_atomic` so production replay
handling has one explicit transaction boundary.

## Phase 2 — tests and observability

- Verify a second application of the same remote item does not change stock.
- Verify a failed mutation does not leave a receipt.
- Verify a duplicate stock movement is blocked by its remote receipt.
- Record atomic-apply failures in daemon status and logs.
- Add metrics for applied, skipped, and failed remote items.

## Phase 3 — critical event policy design

Before changing EventBus behavior, classify handlers:

- **Critical/domain durability:** sale sync enqueue and audit persistence, with
  explicit policy on whether failure blocks checkout or creates an outbox retry.
- **Best effort:** reporting, UI bridge, LAN broadcast, telemetry, notifications.
- **Exactly once is not assumed:** every handler must be idempotent or consume a
  stable event/operation key.

The implementation should prefer a transactional outbox for post-commit critical
work rather than making the in-process bus globally fail-fast.

## Phase 4 — durable ordering and recovery

- Add per-aggregate sequence or causality keys for sale and stock events.
- Persist retry state and operator-visible failure reasons.
- Add crash/restart and concurrent-consumer tests.
- Reconcile desktop and tablet sync paths around the same application service.
