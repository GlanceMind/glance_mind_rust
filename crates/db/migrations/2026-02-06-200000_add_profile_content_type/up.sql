-- Account grooming feature: constraints + image_ai_model_id column + image models

-- 1. Extend constraints for new types
ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_content_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_content_type
    CHECK (content_type IN ('post', 'video', 'reel', 'story', 'profile'));

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN ('batch_text', 'single_video', 'account_grooming'));

ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT IF EXISTS aipub_ai_tasks_valid_task_type;
ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type
    CHECK (task_type IN ('video_gen', 'content_gen', 'image_gen', 'combined', 'account_grooming'));

-- 2. Add image_ai_model_id column for account grooming (stores the selected image generation model)
ALTER TABLE gm_aipub_plans ADD COLUMN IF NOT EXISTS image_ai_model_id INTEGER REFERENCES gm_ai_models(id);

-- 3. Seed image generation models (LaoZhang API supported models)
INSERT INTO gm_ai_models (name, provider, model_key, model_type, cost_multiplier, is_active, created_at)
VALUES
    ('GPT-4o Image', 'laozhang', 'gpt-4o-image', 'image', 1.00, true, NOW()),
    ('DALL-E 3', 'laozhang', 'dall-e-3', 'image', 4.00, true, NOW()),
    ('DALL-E 2', 'laozhang', 'dall-e-2', 'image', 2.00, true, NOW()),
    ('Flux Pro v1.1', 'laozhang', 'black-forest-labs/flux-pro-v1.1', 'image', 3.50, true, NOW()),
    ('Sora Image', 'laozhang', 'sora-image', 'image', 2.00, true, NOW())
ON CONFLICT DO NOTHING;
