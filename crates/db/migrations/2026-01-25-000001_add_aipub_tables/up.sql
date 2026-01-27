-- =============================================================================
-- AI Publish Module - Database Migration
-- 自动发布模块数据库表
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. 发布计划表 gm_aipub_plans
-- 用户创建的发布计划主记录
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_plans (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id),
    
    -- 目标配置
    group_id INTEGER REFERENCES gm_social_groups(id),            -- 目标 Group
    social_account_id INTEGER REFERENCES gm_social_accounts(id), -- 或单个账户
    platform_id INTEGER NOT NULL REFERENCES gm_platforms(id),
    
    -- 内容配置
    content_type VARCHAR(20) NOT NULL,       -- post/video/reel/story
    
    -- AI 配置 (直接存储，不使用独立模板表)
    ai_task_types TEXT[],                    -- 需要的 AI 任务类型: ['video_gen', 'content_gen']
    ai_service_config JSONB,                 -- AI 服务配置
    ai_input JSONB,                          -- AI 输入参数 (用户填写)
    
    -- 直接提供的内容 (不使用AI时)
    content JSONB,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending' NOT NULL,  -- pending/ai_processing/ready/completed/failed
    
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
CREATE INDEX idx_aipub_plans_user ON gm_aipub_plans(user_id);
CREATE INDEX idx_aipub_plans_status ON gm_aipub_plans(status);
CREATE INDEX idx_aipub_plans_created_at ON gm_aipub_plans(created_at DESC);

-- 自动更新 updated_at
SELECT diesel_manage_updated_at('gm_aipub_plans');


-- -----------------------------------------------------------------------------
-- 2. AI 任务表 gm_aipub_ai_tasks
-- AI 生成任务，关联外部 AI 服务
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_ai_tasks (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES gm_aipub_plans(id) ON DELETE CASCADE,
    
    -- AI 任务类型
    task_type VARCHAR(20) NOT NULL,         -- video_gen/content_gen/image_gen
    
    -- 外部服务关联
    external_service VARCHAR(50) NOT NULL,  -- 服务标识: runway/pika/openai/mock
    external_job_id VARCHAR(200),           -- 外部服务返回的 job_id
    
    -- 输入输出 (JSONB 支持不同 AI 服务的灵活格式)
    input JSONB NOT NULL,                   -- AI 输入参数
    result JSONB,                           -- AI 生成结果
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending' NOT NULL,
    progress INTEGER DEFAULT 0,             -- 进度 0-100
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    
    -- 时间戳
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    CONSTRAINT aipub_ai_tasks_valid_task_type CHECK (task_type IN ('video_gen', 'content_gen', 'image_gen')),
    CONSTRAINT aipub_ai_tasks_valid_status CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    CONSTRAINT aipub_ai_tasks_valid_progress CHECK (progress >= 0 AND progress <= 100)
);

-- 索引
CREATE INDEX idx_aipub_ai_tasks_plan ON gm_aipub_ai_tasks(plan_id);
CREATE INDEX idx_aipub_ai_tasks_status ON gm_aipub_ai_tasks(status);
CREATE INDEX idx_aipub_ai_tasks_processing ON gm_aipub_ai_tasks(status) WHERE status = 'processing';

-- 自动更新 updated_at
SELECT diesel_manage_updated_at('gm_aipub_ai_tasks');


-- -----------------------------------------------------------------------------
-- 3. 发布任务表 gm_aipub_tasks
-- 每个账户的具体发布任务，Executor 执行单位
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_aipub_tasks (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES gm_aipub_plans(id) ON DELETE CASCADE,
    social_account_id INTEGER NOT NULL REFERENCES gm_social_accounts(id),
    
    -- 最终内容 (从 AI 结果或 Plan 复制, JSONB 支持灵活格式)
    content JSONB NOT NULL,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'ready' NOT NULL,
    
    -- 执行结果
    result_url VARCHAR(500),
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    
    -- 时间戳
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    
    CONSTRAINT aipub_tasks_valid_status CHECK (status IN ('pending', 'video_pending', 'video_processing', 'ready', 'processing', 'completed', 'failed'))
);

-- 索引
CREATE INDEX idx_aipub_tasks_plan ON gm_aipub_tasks(plan_id);
CREATE INDEX idx_aipub_tasks_account ON gm_aipub_tasks(social_account_id);
CREATE INDEX idx_aipub_tasks_status ON gm_aipub_tasks(status);
CREATE INDEX idx_aipub_tasks_ready ON gm_aipub_tasks(status) WHERE status = 'ready';
CREATE INDEX idx_aipub_tasks_video_pending ON gm_aipub_tasks(status) WHERE status = 'video_pending';
CREATE INDEX idx_aipub_tasks_video_processing ON gm_aipub_tasks(status) WHERE status = 'video_processing';

-- 自动更新 updated_at
SELECT diesel_manage_updated_at('gm_aipub_tasks');


-- -----------------------------------------------------------------------------
-- Comments
-- -----------------------------------------------------------------------------
COMMENT ON TABLE gm_aipub_plans IS 'AI自动发布计划主表';
COMMENT ON TABLE gm_aipub_ai_tasks IS 'AI生成任务表，关联外部AI服务';
COMMENT ON TABLE gm_aipub_tasks IS '发布任务表，每个账户一条记录，Executor执行单位';

COMMENT ON COLUMN gm_aipub_plans.ai_task_types IS '需要的AI任务类型数组，如 [video_gen, content_gen]';
COMMENT ON COLUMN gm_aipub_plans.ai_service_config IS 'AI服务配置JSON，如 {video_service: runway, video_duration: 30}';
COMMENT ON COLUMN gm_aipub_plans.ai_input IS '用户提供的AI输入参数JSON';
COMMENT ON COLUMN gm_aipub_plans.content IS '直接提供的发布内容JSON（不使用AI时）';

COMMENT ON COLUMN gm_aipub_ai_tasks.external_service IS '外部AI服务标识：runway/pika/openai/mock';
COMMENT ON COLUMN gm_aipub_ai_tasks.external_job_id IS '外部服务返回的任务ID';
COMMENT ON COLUMN gm_aipub_ai_tasks.input IS 'AI输入参数JSONB';
COMMENT ON COLUMN gm_aipub_ai_tasks.result IS 'AI生成结果JSONB';

COMMENT ON COLUMN gm_aipub_tasks.content IS '最终发布内容JSONB';
COMMENT ON COLUMN gm_aipub_tasks.result_url IS '发布成功后的链接';
