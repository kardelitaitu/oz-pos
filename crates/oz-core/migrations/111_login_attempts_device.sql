-- STAFF-07: device-scoped and global abuse controls for login attempts.
--
-- Adds a device_id column so the rate limiter can combine per-account
-- throttling with per-device and global limits, and so lockouts survive
-- restarts just like the username rows. device_id is NULL for legacy rows
-- and when no device could be determined; per-device and global limits
-- only apply to rows with a device id.
ALTER TABLE login_attempts ADD COLUMN device_id TEXT;
CREATE INDEX IF NOT EXISTS idx_login_attempts_device ON login_attempts(device_id);
