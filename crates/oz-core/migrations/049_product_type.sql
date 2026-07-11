-- Add product_type column to distinguish retail, restaurant, and dual-purpose items.
--
-- product_type values:
--   'retail'      — product appears in Retail POS only (track_serial, weight scale, etc.)
--   'restaurant'  — product appears in Restaurant Menu only (prep time, modifiers, KDS)
--   'both'        — product appears in both workspaces
--
-- Existing rows default to 'retail' to preserve backward compatibility.

ALTER TABLE products ADD COLUMN product_type TEXT NOT NULL DEFAULT 'retail';
