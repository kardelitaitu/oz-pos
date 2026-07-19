-- Add product_type column to distinguish retail, restaurant, service, and dual-purpose items.
--
-- product_type values:
--   'retail'      — product appears in Retail POS only (track_serial, weight scale, etc.)
--   'restaurant'  — product appears in Restaurant Menu only (prep time, modifiers, KDS)
--   'both'        — product appears in both workspaces
--   'service'     — non-inventory item sold at POS (e.g. "car wash", "delivery fee")
--                    no stock tracking, no KDS routing, no warehouse visibility
--
-- Existing rows default to 'retail' to preserve backward compatibility.

ALTER TABLE products ADD COLUMN product_type TEXT NOT NULL DEFAULT 'retail';
