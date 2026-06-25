-- Add the `page_manage` plan type + AI task type.
--
-- page_manage is an AI-orchestrated page operating plan. At scheduler
-- enqueue time, one page_manage plan is expanded into existing child task
-- types (account_grooming + batch_text), each scheduled independently. The
-- executor never sees a page_manage publish task — only the standard
-- children — so no new content_type / publish status is required.
--
-- NOTE: the task_type list below intentionally RETAINS 'seedance_video',
-- which was added out-of-band by crates/db/migrations/seedance_v1.sql and is
-- live in production. Re-stating the full set here makes this migration the
-- authoritative definition (matching entity::AiTaskType) and avoids dropping
-- a value that is in active use.

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN (
        'batch_text', 'single_video', 'account_grooming',
        'reddit_text', 'reddit_image', 'reddit_link',
        'direct_publish', 'page_manage'
    ));

ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT IF EXISTS aipub_ai_tasks_valid_task_type;
ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type
    CHECK (task_type IN (
        'video_gen', 'content_gen', 'image_gen', 'combined',
        'account_grooming', 'seedance_video', 'page_manage'
    ));
