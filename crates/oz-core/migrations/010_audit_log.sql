-- OZ-POS Audit Log
--
-- Immutable, append-only log for sensitive actions.
-- No UPDATE or DELETE operations are permitted on this table
-- (enforced at the application layer + no application-level
-- UPDATE/DELETE methods in Store).
--
-- PCI-DSS Requirement 10.3.1: Audit logs must capture user ID,
-- event type, date/time, and success/failure.
-- PCI-DSS Requirement 10.3.2: Audit logs cannot be modified.

CREATE TABLE IF NOT EXISTS audit_log (
    id          TEXT PRIMARY KEY,                          -- UUID v4
    user_id     TEXT NOT NULL,                             -- FK to users.id (nullable if action is from system)
    action      TEXT NOT NULL,                             -- e.g. "sale.void", "sale.refund", "settings.change", "login", "export"
    target_type TEXT,                                      -- e.g. "sale", "user", "setting"
    target_id   TEXT,                                      -- e.g. sale UUID, username, setting key
    details     TEXT DEFAULT '{}',                         -- JSON blob with action-specific metadata
    outcome     TEXT NOT NULL DEFAULT 'success',           -- "success" or "failure"
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_target ON audit_log(target_type, target_id);
