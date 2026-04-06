-- =============================================================================
-- AI Publish Module - Manual Migration Script
-- 手动执行此脚本创建 AI 自动发布模块所需的表
-- =============================================================================

-- 检查表是否存在，如果存在则跳过
DO $$ 
BEGIN
    -- 如果表已存在，输出提示并退出
    IF EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'gm_aipub_plans') THEN
        RAISE NOTICE 'Table gm_aipub_plans already exists, skipping creation';
        RETURN;
    END IF;
END $$;

-- -----------------------------------------------------------------------------
-- 1. 发布计划表 gm_aipub_plans
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_plans (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id),
    
    -- 目标配置
    group_id INTEGER REFERENCES gm_social_groups(id),
    social_account_id INTEGER REFERENCES gm_social_accounts(id),
    platform_id INTEGER NOT NULL REFERENCES gm_platforms(id),
    
    -- 内容配置
    content_type VARCHAR(20) NOT NULL,
    
    -- AI 配置
    ai_task_types TEXT[],
    ai_service_config JSONB,
    ai_input JSONB,
    
    -- AI 模型配置 (chat 和 video 分开)
    chat_ai_model_id INTEGER REFERENCES gm_ai_models(id),
    video_ai_model_id INTEGER REFERENCES gm_ai_models(id),
    
    -- 直接提供的内容
    content JSONB,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending' NOT NULL,
    
    -- 时间戳
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,
    
    CONSTRAINT aipub_plans_valid_content_type CHECK (content_type IN ('post', 'video', 'reel', 'story')),
    CONSTRAINT aipub_plans_valid_status CHECK (status IN ('pending', 'ai_processing', 'ready', 'completed', 'failed')),
    CONSTRAINT aipub_plans_valid_target CHECK (
        (group_id IS NOT NULL AND social_account_id IS NULL) OR
        (group_id IS NULL AND social_account_id IS NOT NULL)
    )
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_aipub_plans_user ON gm_aipub_plans(user_id);
CREATE INDEX IF NOT EXISTS idx_aipub_plans_status ON gm_aipub_plans(status);
CREATE INDEX IF NOT EXISTS idx_aipub_plans_created_at ON gm_aipub_plans(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_aipub_plans_chat_model ON gm_aipub_plans(chat_ai_model_id);
CREATE INDEX IF NOT EXISTS idx_aipub_plans_video_model ON gm_aipub_plans(video_ai_model_id);

-- 自动更新 updated_at (如果函数存在)
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'diesel_manage_updated_at') THEN
        PERFORM diesel_manage_updated_at('gm_aipub_plans');
    END IF;
END $$;


-- -----------------------------------------------------------------------------
-- 2. AI 任务表 gm_aipub_ai_tasks
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_ai_tasks (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES gm_aipub_plans(id) ON DELETE CASCADE,
    
    task_type VARCHAR(20) NOT NULL,
    external_service VARCHAR(50) NOT NULL,
    external_job_id VARCHAR(200),
    
    input JSONB NOT NULL,
    result JSONB,
    
    status VARCHAR(20) DEFAULT 'pending' NOT NULL,
    progress INTEGER DEFAULT 0,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    CONSTRAINT aipub_ai_tasks_valid_task_type CHECK (task_type IN ('video_gen', 'content_gen', 'image_gen')),
    CONSTRAINT aipub_ai_tasks_valid_status CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    CONSTRAINT aipub_ai_tasks_valid_progress CHECK (progress >= 0 AND progress <= 100)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_aipub_ai_tasks_plan ON gm_aipub_ai_tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_aipub_ai_tasks_status ON gm_aipub_ai_tasks(status);
CREATE INDEX IF NOT EXISTS idx_aipub_ai_tasks_processing ON gm_aipub_ai_tasks(status) WHERE status = 'processing';

-- 自动更新 updated_at
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'diesel_manage_updated_at') THEN
        PERFORM diesel_manage_updated_at('gm_aipub_ai_tasks');
    END IF;
END $$;


-- -----------------------------------------------------------------------------
-- 3. 发布任务表 gm_aipub_tasks
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_tasks (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES gm_aipub_plans(id) ON DELETE CASCADE,
    social_account_id INTEGER NOT NULL REFERENCES gm_social_accounts(id),
    
    content JSONB NOT NULL,
    
    status VARCHAR(20) DEFAULT 'ready' NOT NULL,
    
    result_url VARCHAR(500),
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    
    CONSTRAINT aipub_tasks_valid_status CHECK (status IN ('pending', 'video_pending', 'video_processing', 'ready', 'processing', 'completed', 'failed'))
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_plan ON gm_aipub_tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_account ON gm_aipub_tasks(social_account_id);
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_status ON gm_aipub_tasks(status);
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_ready ON gm_aipub_tasks(status) WHERE status = 'ready';
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_video_pending ON gm_aipub_tasks(status) WHERE status = 'video_pending';
CREATE INDEX IF NOT EXISTS idx_aipub_tasks_video_processing ON gm_aipub_tasks(status) WHERE status = 'video_processing';

-- 自动更新 updated_at
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'diesel_manage_updated_at') THEN
        PERFORM diesel_manage_updated_at('gm_aipub_tasks');
    END IF;
END $$;


-- -----------------------------------------------------------------------------
-- Comments
-- -----------------------------------------------------------------------------
COMMENT ON TABLE gm_aipub_plans IS 'AI自动发布计划主表';
COMMENT ON TABLE gm_aipub_ai_tasks IS 'AI生成任务表，关联外部AI服务';
COMMENT ON TABLE gm_aipub_tasks IS '发布任务表，每个账户一条记录，Executor执行单位';

COMMENT ON COLUMN gm_aipub_plans.ai_task_types IS '需要的AI任务类型数组，如 [video_gen, content_gen]';
COMMENT ON COLUMN gm_aipub_plans.chat_ai_model_id IS 'Chat/文本生成使用的AI模型ID';
COMMENT ON COLUMN gm_aipub_plans.video_ai_model_id IS '视频生成使用的AI模型ID，仅视频内容类型需要';

-- 完成
DO $$ BEGIN RAISE NOTICE 'AI Publish tables created successfully!'; END $$;
