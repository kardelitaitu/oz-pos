-- 073_offline_queue_priority.sql
-- P-2: Add sync priority tier to the offline queue.
-- Critical (0) items transmit before Normal (1), which transmit before Low (2).
-- Default is 1 (Normal) — backward-compatible with pre-P-2 deployments.

ALTER TABLE offline_queue ADD COLUMN priority INTEGER NOT NULL DEFAULT 1;
