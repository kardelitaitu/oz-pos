-- 129_retire_cashier_kitchen.sql — ADR #35 D4 (spec 0048 cycle 2c).
--
-- The five-role taxonomy (Owner / Manager / Staff / Admin / Auditor +
-- Custom) replaces the legacy cashier/kitchen roles. Migration 128 already
-- folded their assignments into `role-staff` with scoped workspaces; this
-- migration re-points any remaining `users.role_id` / `assignments.role_id`
-- references (direct-SQL or CLI-created rows) and removes the two role rows
-- so the presets and the database agree. Existing scope is preserved — this
-- migration retires the roles, it does not re-scope anyone.

UPDATE users SET role_id = 'role-staff'
 WHERE role_id IN ('role-cashier', 'role-kitchen');

UPDATE assignments SET role_id = 'role-staff'
 WHERE role_id IN ('role-cashier', 'role-kitchen');

DELETE FROM roles WHERE id IN ('role-cashier', 'role-kitchen');
