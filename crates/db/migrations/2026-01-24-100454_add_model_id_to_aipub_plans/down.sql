-- Remove model_id column from gm_aipub_plans table

DROP INDEX IF EXISTS idx_aipub_plans_model;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS model_id;
