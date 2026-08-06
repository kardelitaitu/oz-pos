-- 110_sale_line_tax_breakdown.sql — per-line multi-rate tax breakdown (auditability).
--
-- `sale_lines.tax_rate_id` stores only the FIRST applicable rate id (see
-- compute_sale_tax), which is an auditability gap for multi-rate jurisdictions
-- (e.g. state + local). This column persists the full per-rate breakdown as a
-- JSON array so receipts/audit trails can reconstruct exactly how each line
-- was taxed, even if a rate is later archived (TAX-03) or renamed.
--
-- Element shape: `{ "rate_id": "…" | null, "rate_bps": int, "is_inclusive": bool, "tax_minor": int }`
-- `rate_id` is null for Lua-override lines. NULL when no tax applies or for
-- legacy records (pre-110), so reads must handle NULL.

ALTER TABLE sale_lines ADD COLUMN tax_breakdown_json TEXT;
