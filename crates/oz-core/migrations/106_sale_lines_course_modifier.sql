-- 106_sale_lines_course_modifier.sql
-- Enrich sale_lines with course + modifier data for the KDS pipeline.
--
-- These columns carry per-line course assignment and modifier choices
-- from the POS cart through to the KDS ticket. Both are nullable so
-- existing sale records remain valid (they display as "OTHER" course
-- with no modifiers on the KDS).

ALTER TABLE sale_lines ADD COLUMN course TEXT;
ALTER TABLE sale_lines ADD COLUMN modifiers_json TEXT;
