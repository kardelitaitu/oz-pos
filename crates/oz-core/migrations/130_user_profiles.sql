-- 130_user_profiles.sql — ADR #35 D6 (spec 0049): user profile columns.
--
-- The 9 mandatory-at-creation items (username + full name live on `users`
-- already; the 8 profile fields below are new) are enforced at creation
-- time by `create_user_with_profile`. The columns are nullable in SQL so
-- legacy rows and direct-SQL inserts enter the incomplete-profile state
-- instead of being rejected; job_title and notes are NOT NULL with empty
-- defaults so every profile row has stable string slots for the UI.
--
-- The D6 not-collected fields (gender, religion, marital status, ethnicity,
-- blood type, bank account, shift/availability) are deliberately absent.

ALTER TABLE users ADD COLUMN date_of_birth TEXT;
ALTER TABLE users ADD COLUMN phone TEXT;
ALTER TABLE users ADD COLUMN national_id_type TEXT;
ALTER TABLE users ADD COLUMN national_id TEXT;
ALTER TABLE users ADD COLUMN email TEXT;
ALTER TABLE users ADD COLUMN monthly_take_home_minor INTEGER;
ALTER TABLE users ADD COLUMN emergency_contact_name TEXT;
ALTER TABLE users ADD COLUMN emergency_contact_phone TEXT;
ALTER TABLE users ADD COLUMN job_title TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN notes TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN address TEXT;
ALTER TABLE users ADD COLUMN language TEXT;
ALTER TABLE users ADD COLUMN avatar TEXT;
ALTER TABLE users ADD COLUMN tax_id TEXT;
ALTER TABLE users ADD COLUMN national_id_expires_at TEXT;
ALTER TABLE users ADD COLUMN emergency_contact_relationship TEXT;
ALTER TABLE users ADD COLUMN hire_date TEXT;

-- "Unique when present": SQLite UNIQUE allows multiple NULLs.
CREATE UNIQUE INDEX idx_users_email ON users(email);
CREATE UNIQUE INDEX idx_users_national_id ON users(national_id);
