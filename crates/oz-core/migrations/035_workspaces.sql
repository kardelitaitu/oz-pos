-- Workspace definitions
CREATE TABLE IF NOT EXISTS workspaces (
    id        TEXT PRIMARY KEY,
    key       TEXT NOT NULL UNIQUE,
    name      TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    icon      TEXT NOT NULL DEFAULT ''
);

-- Screens (navigation items) within each workspace
CREATE TABLE IF NOT EXISTS workspace_screens (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_key TEXT NOT NULL REFERENCES workspaces(key),
    screen_key    TEXT NOT NULL,
    label         TEXT NOT NULL DEFAULT '',
    sort_order    INTEGER NOT NULL DEFAULT 0,
    UNIQUE(workspace_key, screen_key)
);

-- Role-to-workspace access
CREATE TABLE IF NOT EXISTS role_workspaces (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    role_id       TEXT NOT NULL REFERENCES roles(id),
    workspace_key TEXT NOT NULL REFERENCES workspaces(key),
    UNIQUE(role_id, workspace_key)
);

-- Seed: Workspaces
INSERT OR IGNORE INTO workspaces (id, key, name, description, icon) VALUES
    ('ws-restaurant-pos', 'restaurant-pos', 'Restaurant POS', 'Cashier terminal for restaurant ordering with menu categories and table management', 'restaurant'),
    ('ws-store-pos', 'store-pos', 'Store POS', 'Cashier terminal for retail with product lookup, customer management, and loyalty', 'store'),
    ('ws-inventory', 'inventory', 'Inventory Management', 'Manage products, stock levels, bundles, categories, and inventory reports', 'inventory'),
    ('ws-admin', 'admin', 'Admin', 'System settings, staff management, reports, audit logs, and configuration', 'admin');

-- Seed: Workspace screens (which nav items appear in each workspace)
INSERT OR IGNORE INTO workspace_screens (workspace_key, screen_key, sort_order) VALUES
    ('restaurant-pos', 'sales', 1),
    ('restaurant-pos', 'kds', 2),
    ('restaurant-pos', 'orders', 3),
    ('restaurant-pos', 'tables', 4),
    ('store-pos', 'sales', 1),
    ('store-pos', 'customers', 2),
    ('store-pos', 'loyalty', 3),
    ('store-pos', 'sales-history', 4),
    ('store-pos', 'promotions', 5),
    ('inventory', 'products', 1),
    ('inventory', 'inventory', 2),
    ('inventory', 'inventory-adjustment', 3),
    ('inventory', 'bundles', 4),
    ('inventory', 'categories', 5),
    ('inventory', 'inventory-report', 6),
    ('admin', 'staff', 1),
    ('admin', 'settings', 2),
    ('admin', 'dashboard', 3),
    ('admin', 'reports', 4),
    ('admin', 'sales-dashboard', 5),
    ('admin', 'eod-report', 6),
    ('admin', 'features', 7),
    ('admin', 'data-management', 8),
    ('admin', 'audit-log', 9),
    ('admin', 'offline-queue', 10),
    ('admin', 'shifts', 11),
    ('admin', 'terminals', 12),
    ('admin', 'stores', 13),
    ('admin', 'exchange-rates', 14),
    ('admin', 'design', 15);
