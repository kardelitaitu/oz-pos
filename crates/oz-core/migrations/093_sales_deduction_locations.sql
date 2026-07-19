-- 093_sales_deduction_locations.sql
-- ADR-19 §2.4: add a JSON column to `sales` that records the per-line,
-- per-location deduction breakdown for split-fulfillment sales.
--
-- When a sale is completed with deductions from a single location, this
-- column still populates with a single-entry-per-line JSON blob for
-- audit uniformity (every `completed` sale has non-NULL deduction_locations
-- post-093).
--
-- Nullable in the schema because pre-093 rows and currently-in-flight
-- sales will have NULL; the Rust command `complete_sale_with_resolved_shortfalls`
-- is the ONLY place that writes non-NULL values from this point forward.
-- Void/refund flows read this column to credit stock back to the original
-- deduction source (ADR-19 §5.3 FIFO oldest-credit).

ALTER TABLE sales ADD COLUMN deduction_locations TEXT;
