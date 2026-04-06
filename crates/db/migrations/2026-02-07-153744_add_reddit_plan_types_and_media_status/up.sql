-- Add Reddit plan types and generic media pipeline status
--
-- 1. Extend plan_type CHECK to include reddit_text, reddit_image, reddit_link
-- 2. Extend publish task status CHECK to include media_pending, media_processing
--    (generic media generation states, replacing platform-specific video_pending etc.)

-- 1. plan_type: add Reddit types
ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN (
        'batch_text', 'single_video', 'account_grooming',
        'reddit_text', 'reddit_image', 'reddit_link'
    ));

-- 2. publish task status: add generic media states (keep old video states for backward compat)
ALTER TABLE gm_aipub_tasks DROP CONSTRAINT IF EXISTS aipub_tasks_valid_status;
ALTER TABLE gm_aipub_tasks ADD CONSTRAINT aipub_tasks_valid_status
    CHECK (status IN (
        'pending',
        'video_pending', 'video_processing',       -- historical (backward compat)
        'media_pending', 'media_processing',        -- generic media pipeline (new)
        'ready', 'processing', 'completed', 'failed'
    ));
