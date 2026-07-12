-- Migration 070: Reset cached machine_id so existing installations regenerate
-- it from stable hardware identifiers (wmic UUID / Windows MachineGuid)
-- instead of the previous random UUID v4 value.
--
-- The machine_id row is deleted here; on next startup get_machine_id() will
-- run generate_machine_id() which now derives a deterministic fingerprint from
-- the physical hardware, persists it, and returns it. This makes the free-tier
-- device identity resilient to DB deletion or app reinstalls.

DELETE FROM settings WHERE key = 'machine_id';
