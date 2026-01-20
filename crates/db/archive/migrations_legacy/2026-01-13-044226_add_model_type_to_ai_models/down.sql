-- Remove default Sora2 models
DELETE FROM gm_ai_models WHERE provider = 'TIKHUB' AND model_type = 'video';

-- Drop index
DROP INDEX IF EXISTS idx_ai_models_model_type;

-- Drop check constraint
ALTER TABLE gm_ai_models DROP CONSTRAINT IF EXISTS valid_model_type;

-- Remove model_type column
ALTER TABLE gm_ai_models DROP COLUMN IF EXISTS model_type;
