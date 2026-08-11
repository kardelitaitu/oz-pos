-- 128_assignments.sql
-- ADR #35 D5 (spec 0048): role assignments with explicit-all scopes.
--
-- Users currently carry one global `users.role_id`. This migration adds the
-- assignment model: a single effective assignment per user with a `scope_mode`
-- (`global` | `scoped`) and, for scoped mode, an explicit `all` | `list` per
-- dimension (branches, workspaces). Empty lists never mean "all" — the
-- per-dimension flags are the explicit marker, so `list` with no rows is a
-- deny, not an implicit "all" (ADR #35 D5).
--
-- Behavior-preserving backfill: every existing user gets one assignment.
--   * Owner / Manager / Staff / custom roles -> `global` mode, role kept.
--   * Legacy role-cashier / role-kitchen users -> `role-staff` with `scoped`
--     mode and the workspace their current permission set implies
--     (`retail-pos` / `kds`), so their operational access survives the role
--     retirement that follows in a later migration.
--
-- The legacy `users.role_id` column is intentionally NOT re-pointed or dropped
-- here: re-pointing it would change what the 0047 gate (which still resolves
-- through `role_id`) grants to kitchen users until the gate rewires to read
-- assignments in the next 0048 cycle. This migration is purely additive and
-- behavior-neutral; the retirement + re-point land with the gate rewire.
--
-- `retail-pos` is seeded as a workspace here (migration 048 seeded `kds`)
-- because the cashier remap references it and `assignment_workspaces` FKs to
-- `workspaces(key)`.

-- One effective assignment per user.
CREATE TABLE IF NOT EXISTS assignments (
    user_id         TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    role_id         TEXT NOT NULL REFERENCES roles(id),
    scope_mode      TEXT NOT NULL DEFAULT 'global' CHECK (scope_mode IN ('global', 'scoped')),
    branch_scope    TEXT NOT NULL DEFAULT 'all'  CHECK (branch_scope IN ('all', 'list')),
    workspace_scope TEXT NOT NULL DEFAULT 'all'  CHECK (workspace_scope IN ('all', 'list')),
    expires_at      TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Scoped-mode branch dimension: rows exist only when branch_scope = 'list'
-- (ignored when 'all'). branch_id references store_profiles(id) semantically;
-- no FK — branch rows live in per-store databases, and the global identity DB
-- holds the assignment (ADR #4 / ADR #35 D5).
CREATE TABLE IF NOT EXISTS assignment_branches (
    assignment_user_id TEXT NOT NULL REFERENCES assignments(user_id) ON DELETE CASCADE,
    branch_id          TEXT NOT NULL,
    PRIMARY KEY (assignment_user_id, branch_id)
);
CREATE INDEX IF NOT EXISTS idx_assignment_branches_user ON assignment_branches(assignment_user_id);

-- Scoped-mode workspace dimension: rows exist only when workspace_scope = 'list'.
CREATE TABLE IF NOT EXISTS assignment_workspaces (
    assignment_user_id TEXT NOT NULL REFERENCES assignments(user_id) ON DELETE CASCADE,
    workspace_key      TEXT NOT NULL REFERENCES workspaces(key) ON DELETE CASCADE,
    PRIMARY KEY (assignment_user_id, workspace_key)
);
CREATE INDEX IF NOT EXISTS idx_assignment_workspaces_user ON assignment_workspaces(assignment_user_id);

-- Ensure the cashier remap target exists as a workspace.
INSERT OR IGNORE INTO workspaces (id, key, name, description, icon)
VALUES ('ws-retail-pos', 'retail-pos', 'Retail POS', 'Cashier terminal for retail checkout', 'store');

-- Backfill every existing user's single effective assignment.
INSERT OR IGNORE INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
SELECT id,
       CASE WHEN role_id IN ('role-cashier', 'role-kitchen') THEN 'role-staff' ELSE role_id END,
       CASE WHEN role_id IN ('role-cashier', 'role-kitchen') THEN 'scoped' ELSE 'global' END,
       'all',
       CASE WHEN role_id IN ('role-cashier', 'role-kitchen') THEN 'list' ELSE 'all' END
FROM users;

-- Cashier -> `retail-pos`, kitchen -> `kds` (the workspaces their grants imply).
INSERT OR IGNORE INTO assignment_workspaces (assignment_user_id, workspace_key)
SELECT id, CASE WHEN role_id = 'role-kitchen' THEN 'kds' ELSE 'retail-pos' END
FROM users
WHERE role_id IN ('role-cashier', 'role-kitchen');
