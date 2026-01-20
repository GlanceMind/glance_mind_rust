-- Add LaoZhang support and remove TikHub-specific naming
-- Step 1: Rename TikHub-specific columns to generic names
ALTER TABLE gm_video_generation_tasks
  RENAME COLUMN tikhub_post_id TO provider_post_id;

ALTER TABLE gm_video_generation_tasks
  RENAME COLUMN tikhub_response TO provider_response;

-- Step 2: Add LaoZhang-specific fields
ALTER TABLE gm_video_generation_tasks
  ADD COLUMN video_seconds VARCHAR(10),
  ADD COLUMN video_size VARCHAR(20);

-- Step 3: Add index for provider_post_id
CREATE INDEX IF NOT EXISTS idx_video_tasks_provider_post_id 
  ON gm_video_generation_tasks(provider_post_id);

-- Step 4: Insert LaoZhang video models into gm_ai_models table
INSERT INTO gm_ai_models (
  model_key, 
  name, 
  provider, 
  cost_multiplier, 
  is_active, 
  model_type,
  created_at
)
SELECT 'sora-2', 'Sora 2', 'laozhang', 1.0, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'sora-2')
UNION ALL
SELECT 'sora-2-pro', 'Sora 2 Pro', 'laozhang', 1.5, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'sora-2-pro');
