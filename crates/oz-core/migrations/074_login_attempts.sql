-- Track failed login attempts per username with timestamps.
-- Persists across app restarts so brute-force attacks cannot
-- bypass the rate limiter by restarting the process.
CREATE TABLE IF NOT EXISTS login_attempts (
    id          TEXT PRIMARY KEY,
    username    TEXT NOT NULL,
    attempted_at INTEGER NOT NULL  -- Unix epoch seconds
);
CREATE INDEX IF NOT EXISTS idx_login_attempts_username ON login_attempts(username);
CREATE INDEX IF NOT EXISTS idx_login_attempts_attempted_at ON login_attempts(attempted_at);
