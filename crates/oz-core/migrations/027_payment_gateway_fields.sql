-- Add gateway tracking fields to the payments table.
-- These store payment gateway reference IDs, status, and raw response
-- for external payment processors (card terminals, etc.).

ALTER TABLE payments ADD COLUMN gateway_reference TEXT;
ALTER TABLE payments ADD COLUMN gateway_status TEXT;
ALTER TABLE payments ADD COLUMN gateway_response TEXT;
