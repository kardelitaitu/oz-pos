-- ADR sync-auth-hardening P3: registered sync terminals.
--
-- Each POS terminal registers once and receives a high-entropy device
-- secret. Only the SHA-256 hash of the secret is stored; the plaintext
-- secret is returned to the client exactly once at registration. The
-- terminal then mints short-lived API tokens with its credentials
-- (client-credentials style), giving sync real per-device identity.
CREATE TABLE IF NOT EXISTS sync_terminals (
    terminal_id   TEXT PRIMARY KEY,
    -- SHA-256 hex digest of the device secret (never the plaintext).
    secret_hash   TEXT NOT NULL,
    label         TEXT NOT NULL DEFAULT '',
    tenant_id     TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
