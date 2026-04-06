-- Revert all account grooming changes

-- Remove seeded image models
DELETE FROM gm_ai_models WHERE model_type = 'image' AND model_key IN (
    'gpt-4o-image', 'dall-e-3', 'dall-e-2', 'black-forest-labs/flux-pro-v1.1', 'sora-image'
);

ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS image_ai_model_id;

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_content_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_content_type
    CHECK (content_type IN ('post', 'video', 'reel', 'story'));

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN ('batch_text', 'single_video'));

ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT IF EXISTS aipub_ai_tasks_valid_task_type;
ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type
    CHECK (task_type IN ('video_gen', 'content_gen', 'image_gen', 'combined'));
