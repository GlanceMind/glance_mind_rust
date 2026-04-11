ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_plan_type;
ALTER TABLE gm_aipub_plans ADD CONSTRAINT aipub_plans_valid_plan_type
    CHECK (plan_type IN (
        'batch_text', 'single_video', 'account_grooming',
        'reddit_text', 'reddit_image', 'reddit_link'
    ));
