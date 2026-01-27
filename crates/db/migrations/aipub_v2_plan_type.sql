-- =============================================================================
-- AI Publish V2 Migration - Add plan_type and ai_task_id
-- 添加 plan_type 字段区分批量文字和单视频任务
-- 添加 ai_task_id 关联 publish task 到其生成的 AI task
-- =============================================================================

-- 1. Add plan_type to gm_aipub_plans
-- 两种类型: batch_text (批量文字, 目标是group), single_video (单视频, 目标是account)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_aipub_plans' AND column_name = 'plan_type'
    ) THEN
        ALTER TABLE gm_aipub_plans 
        ADD COLUMN plan_type VARCHAR(20) DEFAULT 'batch_text' NOT NULL;
        
        -- Add check constraint
        ALTER TABLE gm_aipub_plans
        ADD CONSTRAINT aipub_plans_valid_plan_type 
        CHECK (plan_type IN ('batch_text', 'single_video'));
        
        RAISE NOTICE 'Added plan_type column to gm_aipub_plans';
    ELSE
        RAISE NOTICE 'plan_type column already exists';
    END IF;
END $$;

-- Create index on plan_type
CREATE INDEX IF NOT EXISTS idx_aipub_plans_plan_type ON gm_aipub_plans(plan_type);

-- 2. Add ai_task_id to gm_aipub_tasks
-- 关联到生成该 task 内容的 AI task
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_aipub_tasks' AND column_name = 'ai_task_id'
    ) THEN
        ALTER TABLE gm_aipub_tasks 
        ADD COLUMN ai_task_id INTEGER REFERENCES gm_aipub_ai_tasks(id);
        
        RAISE NOTICE 'Added ai_task_id column to gm_aipub_tasks';
    ELSE
        RAISE NOTICE 'ai_task_id column already exists';
    END IF;
END $$;

-- Create index on ai_task_id
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_ai_task ON gm_aipub_tasks(ai_task_id);

-- 3. Add sequence to gm_aipub_ai_tasks for ordering
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'gm_aipub_ai_tasks' AND column_name = 'sequence'
    ) THEN
        ALTER TABLE gm_aipub_ai_tasks 
        ADD COLUMN sequence INTEGER DEFAULT 0 NOT NULL;
        
        RAISE NOTICE 'Added sequence column to gm_aipub_ai_tasks';
    ELSE
        RAISE NOTICE 'sequence column already exists';
    END IF;
END $$;

-- 4. Update existing plans with inferred plan_type
-- If has video_ai_model_id or video_gen in ai_task_types -> single_video
-- Otherwise -> batch_text
UPDATE gm_aipub_plans
SET plan_type = CASE
    WHEN video_ai_model_id IS NOT NULL THEN 'single_video'
    WHEN ai_task_types IS NOT NULL AND array_to_string(ai_task_types, ',') LIKE '%video_gen%' THEN 'single_video'
    ELSE 'batch_text'
END
WHERE plan_type = 'batch_text'; -- Only update if using default

-- 5. Update task_type constraint to support 'combined' type
-- combined = content_gen + video_gen in single task
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'aipub_ai_tasks_valid_task_type'
    ) THEN
        ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT aipub_ai_tasks_valid_task_type;
    END IF;
    
    ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type 
    CHECK (task_type IN ('video_gen', 'content_gen', 'image_gen', 'combined'));
    
    RAISE NOTICE 'Updated task_type constraint to include combined';
END $$;

-- Comments
COMMENT ON COLUMN gm_aipub_plans.plan_type IS 'Plan type: batch_text (multiple text posts for group) or single_video (one video for account)';
COMMENT ON COLUMN gm_aipub_tasks.ai_task_id IS 'Reference to the AI task that generated this publish task content';
COMMENT ON COLUMN gm_aipub_ai_tasks.sequence IS 'Execution order within the plan (0=first)';

-- Done
DO $$ BEGIN RAISE NOTICE 'AI Publish V2 migration completed successfully!'; END $$;
