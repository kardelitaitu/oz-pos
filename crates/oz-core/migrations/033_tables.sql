CREATE TABLE IF NOT EXISTS tables (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    capacity        INTEGER NOT NULL DEFAULT 4,
    pos_x           REAL NOT NULL DEFAULT 0,
    pos_y           REAL NOT NULL DEFAULT 0,
    shape           TEXT NOT NULL DEFAULT 'circle',
    width           REAL NOT NULL DEFAULT 10,
    height          REAL NOT NULL DEFAULT 10,
    status          TEXT NOT NULL DEFAULT 'available',
    active_sale_id  TEXT REFERENCES sales(id),
    section         TEXT NOT NULL DEFAULT '',
    active          INTEGER NOT NULL DEFAULT 1,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
