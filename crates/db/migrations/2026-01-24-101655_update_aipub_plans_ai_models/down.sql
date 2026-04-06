-- Revert: restore single model_id column

DROP INDEX IF EXISTS idx_aipub_plans_chat_model;
DROP INDEX IF EXISTS idx_aipub_plans_video_model;

ALTER TABLE gm_aipub_plans 
DROP COLUMN IF EXISTS chat_ai_model_id,
DROP COLUMN IF EXISTS video_ai_model_id;

ALTER TABLE gm_aipub_plans 
ADD COLUMN model_id INTEGER REFERENCES gm_ai_models(id);
