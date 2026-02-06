-- Remove image models
DELETE FROM gm_ai_models WHERE model_type = 'image';

-- Revert the CHECK constraint to only allow 'chat' and 'video'
ALTER TABLE gm_ai_models DROP CONSTRAINT IF EXISTS valid_model_type;

ALTER TABLE gm_ai_models
ADD CONSTRAINT valid_model_type CHECK (model_type IN ('chat', 'video'));
