-- Replace single model_id with chat_ai_model_id and video_ai_model_id
-- Both are optional, video model is only needed for video content types

-- Remove old model_id column
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS model_id;

-- Add new model columns
ALTER TABLE gm_aipub_plans 
ADD COLUMN chat_ai_model_id INTEGER REFERENCES gm_ai_models(id),
ADD COLUMN video_ai_model_id INTEGER REFERENCES gm_ai_models(id);

-- Add indexes
CREATE INDEX idx_aipub_plans_chat_model ON gm_aipub_plans(chat_ai_model_id);
CREATE INDEX idx_aipub_plans_video_model ON gm_aipub_plans(video_ai_model_id);

COMMENT ON COLUMN gm_aipub_plans.chat_ai_model_id IS 'Chat/文本生成使用的AI模型ID';
COMMENT ON COLUMN gm_aipub_plans.video_ai_model_id IS '视频生成使用的AI模型ID，仅视频内容类型需要';
