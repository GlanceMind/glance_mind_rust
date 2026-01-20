-- Add model_type column to gm_ai_models table
ALTER TABLE gm_ai_models 
ADD COLUMN model_type VARCHAR(50) NOT NULL DEFAULT 'chat';

-- Add check constraint for valid model types
ALTER TABLE gm_ai_models 
ADD CONSTRAINT valid_model_type CHECK (model_type IN ('chat', 'video'));

-- Create index for model_type
CREATE INDEX idx_ai_models_model_type ON gm_ai_models(model_type);

-- Insert default Sora2 video model
INSERT INTO gm_ai_models (name, provider, model_key, cost_multiplier, is_active, model_type, created_at)
VALUES 
    ('Sora 2', 'TIKHUB', 'sora2', 1.0, true, 'video', NOW()),
    ('Sora 2 HD', 'TIKHUB', 'sora2-hd', 1.5, true, 'video', NOW())
ON CONFLICT DO NOTHING;

-- Update existing models to be chat type (if any exist)
UPDATE gm_ai_models 
SET model_type = 'chat' 
WHERE model_type IS NULL OR model_type = 'chat';
