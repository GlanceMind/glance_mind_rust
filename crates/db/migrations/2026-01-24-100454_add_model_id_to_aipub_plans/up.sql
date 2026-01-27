-- Add model_id column to gm_aipub_plans table
-- References gm_ai_models table for AI model selection

ALTER TABLE gm_aipub_plans 
ADD COLUMN model_id INTEGER REFERENCES gm_ai_models(id);

-- Add index for model_id lookups
CREATE INDEX idx_aipub_plans_model ON gm_aipub_plans(model_id);

COMMENT ON COLUMN gm_aipub_plans.model_id IS '使用的AI模型ID，关联gm_ai_models表';
