-- Revert Reddit plan types and media pipeline status

-- 1. Revert plan_type to previous values
ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN ('batch_text', 'single_video', 'account_grooming'));

-- 2. Revert publish task status (remove media_pending, media_processing)
ALTER TABLE gm_aipub_tasks DROP CONSTRAINT IF EXISTS aipub_tasks_valid_status;
ALTER TABLE gm_aipub_tasks ADD CONSTRAINT aipub_tasks_valid_status
    CHECK (status IN (
        'pending', 'video_pending', 'video_processing',
        'ready', 'processing', 'completed', 'failed'
    ));
