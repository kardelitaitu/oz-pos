-- OZ-POS Audit Log Write-Once Enforcement
--
-- SQLite BEFORE UPDATE / BEFORE DELETE triggers enforce that
-- audit log entries are immutable once written. Any attempt to
-- modify or delete an audit entry will raise an ABORT error.
--
-- PCI-DSS Requirement 10.3.2: Audit logs cannot be modified.
-- PCI-DSS Requirement 10.5: Audit logs must be protected from
--   alteration and unauthorised deletion.
--
-- These triggers apply at the database layer, so they protect
-- against raw SQL access as well as application-level code.

CREATE TRIGGER IF NOT EXISTS audit_log_immutable_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'audit_log entries are immutable: UPDATE not allowed');
END;

CREATE TRIGGER IF NOT EXISTS audit_log_immutable_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'audit_log entries are immutable: DELETE not allowed');
END;
