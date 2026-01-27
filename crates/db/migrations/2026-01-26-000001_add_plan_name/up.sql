-- Add name field to gm_aipub_plans
ALTER TABLE gm_aipub_plans ADD COLUMN IF NOT EXISTS name VARCHAR(200);

-- Add comment
COMMENT ON COLUMN gm_aipub_plans.name IS 'Plan name for easy identification';
