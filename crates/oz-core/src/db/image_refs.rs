//! Cloud image content spine — refcount maintenance, missing-hash computation,
//! GC, and push-queue management (spec 0046b §3.7, §3.6).
//!
//! `image_refs` tracks per-tenant hash refcounts so `GET /api/v1/images/{hash}`
//! can verify the requesting tenant actually references the hash (closing
//! cross-tenant fetch). `missing_hashes` drives the server-side nudge that
//! tells the desktop which hashes the cloud still needs.
//! `image_push_queue` persists pending desktop→cloud uploads.

use super::Store;
use crate::error::CoreError;

// ── Image refs (cloud content spine) ─────────────────────────────────

impl Store<'_> {
    /// Increment the refcount for `(tenant_id, hash)`, recording `bytes` on
    /// first insert. Idempotent when the (tenant, hash) pair already exists
    /// (refcount simply increases).
    pub fn ref_image(&self, tenant_id: &str, hash: &str, bytes: i64) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO image_refs (tenant_id, hash, refcount, bytes, updated_at)
             VALUES (?1, ?2, 1, ?3, ?4)
             ON CONFLICT(tenant_id, hash) DO UPDATE SET
                 refcount = refcount + 1,
                 bytes = excluded.bytes,
                 updated_at = excluded.updated_at",
            rusqlite::params![tenant_id, hash, bytes, now],
        )?;
        Ok(())
    }

    /// Decrement the refcount for `(tenant_id, hash)`.
    ///
    /// If the refcount reaches zero the row is kept for the grace window
    /// (cloud GC sweeps refcount=0 rows older than the configured grace).
    /// Returns the number of rows affected (0 means the pair did not exist).
    pub fn unref_image(&self, tenant_id: &str, hash: &str) -> Result<usize, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let affected = self.conn.execute(
            "UPDATE image_refs SET refcount = MAX(refcount - 1, 0), updated_at = ?1
             WHERE tenant_id = ?2 AND hash = ?3",
            rusqlite::params![now, tenant_id, hash],
        )?;
        Ok(affected)
    }

    /// Check whether a tenant has an active reference to `hash`.
    pub fn image_ref_exists(&self, tenant_id: &str, hash: &str) -> Result<bool, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM image_refs WHERE tenant_id = ?1 AND hash = ?2 AND refcount > 0",
            rusqlite::params![tenant_id, hash],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    /// Given a list of candidate hashes, return the subset that the tenant
    /// does NOT have an active reference for (set-difference: candidates -
    /// present). Used by the server to compute `missing_hashes` on the
    /// catalog snapshot response.
    pub fn missing_hashes<'a>(
        &self,
        tenant_id: &str,
        candidates: &[&'a str],
    ) -> Result<Vec<&'a str>, CoreError> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        use std::collections::HashSet;
        // Build parameterised query with placeholders for each candidate
        let placeholders: Vec<String> = (1..=candidates.len())
            .map(|i| format!("?{}", i + 1)) // ?1 = tenant_id, ?2.. = hashes
            .collect();
        let sql = format!(
            "SELECT hash FROM image_refs WHERE tenant_id = ?1 AND hash IN ({}) AND refcount > 0",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_refs: Vec<&dyn rusqlite::types::ToSql> =
            vec![&tenant_id as &dyn rusqlite::types::ToSql];
        for c in candidates {
            param_refs.push(c);
        }
        let present: HashSet<String> = stmt
            .query_map(param_refs.as_slice(), |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(candidates
            .iter()
            .filter(|c| !present.contains(**c))
            .copied()
            .collect())
    }

    /// Sweep rows where refcount = 0 and updated_at is older than
    /// `grace_secs`. Returns the deleted hashes so the caller can remove
    /// the corresponding files.
    pub fn gc_images(&self, tenant_id: &str, grace_secs: i64) -> Result<Vec<String>, CoreError> {
        let cutoff = format!("-{grace_secs} seconds");
        let mut stmt = self.conn.prepare(
            "DELETE FROM image_refs
             WHERE tenant_id = ?1 AND refcount = 0
               AND datetime(updated_at) <= datetime('now', ?2)
             RETURNING hash",
        )?;
        let hashes: Vec<String> = stmt
            .query_map(rusqlite::params![tenant_id, cutoff], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(hashes)
    }

    /// Sum of `bytes` for all active refs (refcount > 0) for the tenant.
    /// Used for the 4 GB soft-alert metric (§3.7).
    pub fn image_bytes_used(&self, tenant_id: &str) -> Result<i64, CoreError> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(bytes), 0) FROM image_refs WHERE tenant_id = ?1 AND refcount > 0",
                rusqlite::params![tenant_id],
                |r| r.get(0),
            )
            .map_err(CoreError::from)
    }

    // ── Push queue management (desktop) ───────────────────────────────

    /// Enqueue a hash for upload. Idempotent: if the hash is already in the
    /// queue, the existing row is kept (no-op).
    pub fn enqueue_image_push(&self, hash: &str, size_bytes: i64) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT OR IGNORE INTO image_push_queue (hash, size_bytes, next_attempt_at, enqueued_at)
             VALUES (?1, ?2, ?3, ?3)",
            rusqlite::params![hash, size_bytes, now],
        )?;
        Ok(())
    }

    /// Peek the next batch of up to `limit` images ready for upload (rows
    /// whose `next_attempt_at` is due). Returns (hash, size_bytes, attempts).
    pub fn peek_push_batch(&self, limit: usize) -> Result<Vec<(String, i64, i32)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT hash, size_bytes, attempts FROM image_push_queue
             WHERE datetime(next_attempt_at) <= datetime('now')
             ORDER BY next_attempt_at ASC, enqueued_at ASC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Record a push attempt for a hash. On success the row is deleted; on
    /// failure the attempt counter is bumped and `next_attempt_at` is set
    /// with AWS full-jitter backoff: delay = uniform(0, min(30 min, 60 s *
    /// 2^attempts)). After 8 attempts the entry is dead-lettered (deleted).
    pub fn mark_push_attempt(&self, hash: &str, success: bool) -> Result<(), CoreError> {
        use rand::Rng;
        if success {
            self.conn.execute(
                "DELETE FROM image_push_queue WHERE hash = ?1",
                rusqlite::params![hash],
            )?;
            return Ok(());
        }
        // Fetch current attempts
        let (attempts,): (i32,) = self
            .conn
            .query_row(
                "SELECT attempts FROM image_push_queue WHERE hash = ?1",
                rusqlite::params![hash],
                |r| Ok((r.get(0)?,)),
            )
            .unwrap_or((0,));
        let next_attempt = attempts + 1;
        if next_attempt > 8 {
            // Dead-letter after 8 attempts — delete and return
            self.conn.execute(
                "DELETE FROM image_push_queue WHERE hash = ?1",
                rusqlite::params![hash],
            )?;
            return Ok(());
        }
        // AWS full-jitter: delay = uniform(0, min(30 min, 60 s * 2^attempts))
        let max_base = 60_i64 * 2_i64.pow(attempts as u32);
        let limit = max_base.min(1800); // 30 minutes in seconds
        let delay_secs: i64 = rand::thread_rng().gen_range(0..=limit);
        let next_at = format!("+{delay_secs} seconds");
        self.conn.execute(
            "UPDATE image_push_queue
             SET attempts = ?1, next_attempt_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?2)
             WHERE hash = ?3",
            rusqlite::params![next_attempt, next_at, hash],
        )?;
        Ok(())
    }

    /// Delete a dead-lettered (or manually cleared) push entry.
    pub fn clear_push_entry(&self, hash: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM image_push_queue WHERE hash = ?1",
            rusqlite::params![hash],
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "image_refs_tests.rs"]
mod tests;
