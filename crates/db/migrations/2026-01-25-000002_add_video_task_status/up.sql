-- Add video_pending and video_processing status to gm_aipub_tasks
-- This enables the scheduler to track video generation state

-- Drop existing constraint
ALTER TABLE gm_aipub_tasks 
DROP CONSTRAINT IF EXISTS aipub_tasks_valid_status;

-- Add updated constraint with video states
ALTER TABLE gm_aipub_tasks 
ADD CONSTRAINT aipub_tasks_valid_status CHECK (
    status IN ('pending', 'video_pending', 'video_processing', 'ready', 'processing', 'completed', 'failed')
);

-- Add index for video_pending status (scheduler will query this)
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_video_pending ON gm_aipub_tasks(status) WHERE status = 'video_pending';
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_video_processing ON gm_aipub_tasks(status) WHERE status = 'video_processing';
