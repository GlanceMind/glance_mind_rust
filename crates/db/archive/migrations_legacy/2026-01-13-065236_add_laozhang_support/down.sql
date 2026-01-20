-- Revert LaoZhang changes
-- Step 1: Remove LaoZhang video models
DELETE FROM gm_ai_models WHERE model_key IN ('sora-2', 'sora-2-pro');

-- Step 2: Drop index
DROP INDEX IF EXISTS idx_video_tasks_provider_post_id;

-- Step 3: Remove LaoZhang-specific fields
ALTER TABLE gm_video_generation_tasks
  DROP COLUMN IF EXISTS video_size,
  DROP COLUMN IF EXISTS video_seconds;

-- Step 4: Rename columns back to TikHub-specific names
ALTER TABLE gm_video_generation_tasks
  RENAME COLUMN provider_response TO tikhub_response;

ALTER TABLE gm_video_generation_tasks
  RENAME COLUMN provider_post_id TO tikhub_post_id;
