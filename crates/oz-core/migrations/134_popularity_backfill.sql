-- 134_popularity_backfill.sql — backfill popularity signals from pre-feature
-- history so the retail grid's default popularity sort is meaningful on the
-- first launch after upgrade (ADR #37).
--
-- When this feature ships, `product_activity` starts empty: search and edit
-- signals only exist from launch onward. The sales signal needs no seeding —
-- the full-catalog pass at store open reads `sale_lines` directly (completed
-- sales are the durable, synced ledger). This migration backfills the EDIT
-- signal from the durable product timestamps: one synthetic 'edit' event per
-- product, dated at its most recent modification (`updated_at`, preferring
-- `price_updated_at` when newer), inside the formula's decay window. The
-- recency-decay formula then ranks recently-managed products correctly from
-- day one.
--
-- Search history was never recorded before this feature, so the search
-- signal legitimately starts cold; sales (weight 0.6) dominate the blend and
-- searches begin accumulating immediately after launch.
--
-- Local-only, like the ledger it seeds (ADR #37 D4): the score and these
-- rows never sync. Idempotence is guaranteed by the migration runner (each
-- migration runs exactly once); the synthetic ids derive from product ids.
-- The window bound must match `popularity::WINDOW_DAYS` (90).

INSERT INTO product_activity (id, sku, event_type, created_at)
SELECT 'backfill-edit-' || p.id, p.sku, 'edit',
       MAX(p.updated_at, COALESCE(NULLIF(p.price_updated_at, ''), p.updated_at))
FROM products p
WHERE MAX(p.updated_at, COALESCE(NULLIF(p.price_updated_at, ''), p.updated_at))
      >= datetime('now', '-90 days');
