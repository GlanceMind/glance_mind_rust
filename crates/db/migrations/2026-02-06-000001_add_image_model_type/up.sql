-- Add 'image' to the valid_model_type CHECK constraint on gm_ai_models
-- First drop the existing constraint, then add the updated one

ALTER TABLE gm_ai_models DROP CONSTRAINT IF EXISTS valid_model_type;

ALTER TABLE gm_ai_models
ADD CONSTRAINT valid_model_type CHECK (model_type IN ('chat', 'video', 'image'));

-- Insert image generation models (verified working on LaoZhang default plan)
INSERT INTO gm_ai_models (name, provider, model_key, cost_multiplier, is_active, model_type, created_at)
VALUES
    ('GPT-4o Image', 'LAOZHANG', 'gpt-4o-image', 1.0, true, 'image', NOW()),
    ('Sora Image', 'LAOZHANG', 'sora-image', 1.0, true, 'image', NOW()),
    ('DALL-E 3', 'LAOZHANG', 'dall-e-3', 1.0, true, 'image', NOW())
ON CONFLICT DO NOTHING;
