-- Revert: drop `page_manage` from the plan_type and ai_task_type CHECKs.
-- Restores the pre-page_manage state. 'seedance_video' is retained (it was
-- never part of page_manage and remains live in production).

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN (
        'batch_text', 'single_video', 'account_grooming',
        'reddit_text', 'reddit_image', 'reddit_link',
        'direct_publish'
    ));

ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT IF EXISTS aipub_ai_tasks_valid_task_type;
ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type
    CHECK (task_type IN (
        'video_gen', 'content_gen', 'image_gen', 'combined',
        'account_grooming', 'seedance_video'
    ));
