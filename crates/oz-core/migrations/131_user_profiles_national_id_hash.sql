-- 131_user_profiles_national_id_hash.sql — ADR #35 D6 (spec 0049).
--
-- `national_id` is encrypted at rest (nonce-randomised ciphertext), so the
-- unique index on the ciphertext column can no longer enforce "national_id
-- unique when present". This migration adds a deterministic SHA-256 hash of
-- the plaintext national id, populated by the profile write path, and a
-- unique index on it — preserving the uniqueness invariant while the
-- ciphertext stays indistinguishable. The hash is one-way: it never reveals
-- the national id and is excluded from every read path.

ALTER TABLE users ADD COLUMN national_id_hash TEXT;
CREATE UNIQUE INDEX idx_users_national_id_hash ON users(national_id_hash);
