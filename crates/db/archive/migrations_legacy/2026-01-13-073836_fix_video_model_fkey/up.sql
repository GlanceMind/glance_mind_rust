-- Fix foreign key constraint: model_id should reference gm_ai_models, not gm_ai_video_models

-- Step 1: Drop the existing foreign key constraint
ALTER TABLE gm_video_generation_tasks
  DROP CONSTRAINT IF EXISTS gm_video_generation_tasks_model_id_fkey;

-- Step 2: Add the correct foreign key constraint to gm_ai_models
ALTER TABLE gm_video_generation_tasks
  ADD CONSTRAINT gm_video_generation_tasks_model_id_fkey 
  FOREIGN KEY (model_id) REFERENCES gm_ai_models(id);
