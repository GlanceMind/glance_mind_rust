-- Remove name field from gm_aipub_plans
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS name;
