-- Add per-account Facebook Pages storage as a JSON-array TEXT column.
-- NULLABLE, no backfill: existing rows stay NULL (=> [] in API responses).
-- DEPLOY SAFETY: old code ignores the new column; new code tolerates NULL.
-- No deploy-coordination hazard.
ALTER TABLE gm_social_accounts ADD COLUMN fb_pages_id TEXT;
