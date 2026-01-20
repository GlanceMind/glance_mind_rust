-- 创建视频生成任务表
CREATE TABLE gm_video_generation_tasks (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id),
    task_id VARCHAR(255) NOT NULL UNIQUE,
    generation_id VARCHAR(255),
    
    -- 输入参数
    prompt TEXT,
    media_id VARCHAR(255),
    
    -- 任务状态
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    progress_pct NUMERIC(3,2),
    
    -- 结果信息 (从TikHub获取)
    video_width INTEGER,
    video_height INTEGER,
    video_url TEXT,
    thumbnail_url TEXT,
    tikhub_post_id VARCHAR(255),
    tikhub_response JSONB,
    
    -- 费用相关
    cost_points NUMERIC(10,2) NOT NULL DEFAULT 200.00,
    wallet_transaction_id INTEGER REFERENCES gm_wallet_transactions(id),
    
    -- 错误信息
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    
    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    CONSTRAINT valid_status CHECK (status IN ('pending', 'queued', 'processing', 'succeeded', 'failed', 'cancelled'))
);

-- 创建索引
CREATE INDEX idx_video_tasks_user_id ON gm_video_generation_tasks(user_id);
CREATE INDEX idx_video_tasks_task_id ON gm_video_generation_tasks(task_id);
CREATE INDEX idx_video_tasks_status ON gm_video_generation_tasks(status);
CREATE INDEX idx_video_tasks_created_at ON gm_video_generation_tasks(created_at);
CREATE INDEX idx_video_tasks_status_updated ON gm_video_generation_tasks(status, updated_at) 
    WHERE status IN ('pending', 'queued', 'processing');

-- 添加定价规则（使用 ON CONFLICT DO NOTHING 避免重复）
INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, created_at)
VALUES ('video_upload_image', NULL, 10.00, NOW())
ON CONFLICT (action_type, COALESCE(platform_id, 0)) DO NOTHING;

INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, created_at)
VALUES ('video_generate', NULL, 200.00, NOW())
ON CONFLICT (action_type, COALESCE(platform_id, 0)) DO NOTHING;
