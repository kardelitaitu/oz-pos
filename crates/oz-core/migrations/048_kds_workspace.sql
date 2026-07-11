-- KDS workspace seed
--
-- Adds the Kitchen Display System as a standalone workspace so
-- the backend can return it alongside the other workspaces.
-- Role-to-workspace mappings for the kitchen role are seeded
-- separately by seed_default_roles() or the admin UI.

INSERT OR IGNORE INTO workspaces (id, key, name, description, icon) VALUES
    ('ws-kds', 'kds', 'Kitchen Display', 'Order queue display for the kitchen — tap tickets to advance their status', 'kds');
