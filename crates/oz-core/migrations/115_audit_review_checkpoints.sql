-- ── 115: Server-side audit review checkpoints (audit/13 AUD-04) ──
-- Review status previously lived only in browser localStorage: device-local,
-- not shared across managers, reset by clearing browser data, and never
-- represented as an auditable event. This table persists each "Mark
-- Reviewed" checkpoint with the tenant store, reviewer, review timestamp,
-- and a (created_at, id) high-water mark so the badge state is durable,
-- tenant-scoped, shared, and verifiable.
CREATE TABLE IF NOT EXISTS audit_review_checkpoints (
    id                           TEXT PRIMARY KEY,
    store_id                     TEXT NOT NULL,
    reviewer_user_id             TEXT NOT NULL,
    reviewed_at                  TEXT NOT NULL,             -- ISO-8601 review action time
    reviewed_through_created_at  TEXT NOT NULL,             -- newest entry.created_at covered
    reviewed_through_id          TEXT NOT NULL,             -- tie-breaker entry id covered
    created_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_review_checkpoints_reviewed_at
    ON audit_review_checkpoints(reviewed_at DESC);
