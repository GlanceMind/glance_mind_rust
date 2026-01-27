-- Revert to original status constraint
-- Note: This may fail if there are rows with video_pending or video_processing status

-- Drop video status indexes
DROP INDEX IF EXISTS idx_aipub_tasks_video_pending;
DROP INDEX IF EXISTS idx_aipub_tasks_video_processing;

ALTER TABLE gm_aipub_tasks 
DROP CONSTRAINT IF EXISTS aipub_tasks_valid_status;

ALTER TABLE gm_aipub_tasks 
ADD CONSTRAINT aipub_tasks_valid_status CHECK (
    status IN ('ready', 'processing', 'completed', 'failed')
);
