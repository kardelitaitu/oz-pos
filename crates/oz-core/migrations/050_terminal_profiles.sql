-- 050_terminal_profiles.sql
-- Terminal profiles — assigns a profile type and optional locked screen
-- to each terminal for kiosk lockdown and role-specific UIs.
--
-- Profile types:
--   'counter_pos'       — Front counter POS (default, full interface)
--   'kds_kiosk'         — KDS-only locked-down kiosk (no navigation)
--   'customer_display'  — Customer-facing secondary display
--   'unrestricted'      — No restrictions (admin/back-office)
--
-- When profile_type = 'kds_kiosk', the UI forces the KDS route and
-- hides all navigation.

CREATE TABLE IF NOT EXISTS terminal_profiles (
    terminal_id TEXT PRIMARY KEY REFERENCES terminals(id) ON DELETE CASCADE,
    profile_type TEXT NOT NULL DEFAULT 'unrestricted'
        CHECK (profile_type IN ('counter_pos', 'kds_kiosk', 'customer_display', 'unrestricted')),
    locked_screen TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
