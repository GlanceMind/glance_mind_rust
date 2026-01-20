-- 通知表（全局，管理员发布）
CREATE TABLE IF NOT EXISTS gm_notifications (
    id SERIAL PRIMARY KEY,
    notification_type VARCHAR(20) NOT NULL,  -- feature, update, announcement, tip
    title VARCHAR(255) NOT NULL,
    title_zh VARCHAR(255),
    description TEXT,
    description_zh TEXT,
    link VARCHAR(500),
    link_text VARCHAR(100),
    link_text_zh VARCHAR(100),
    important BOOLEAN DEFAULT FALSE,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 用户已读记录表
CREATE TABLE IF NOT EXISTS gm_user_notification_reads (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    notification_id INT NOT NULL REFERENCES gm_notifications(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, notification_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_notification_reads_user ON gm_user_notification_reads(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_published ON gm_notifications(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_type ON gm_notifications(notification_type);

-- 插入第一条通知：多平台支持
INSERT INTO gm_notifications (notification_type, title, title_zh, description, description_zh, important, published_at)
VALUES (
    'feature',
    'Multi-Platform Support Released',
    '多平台支持正式发布',
    'We now support Instagram, Facebook, Reddit, and Twitter! You can analyze videos, posts, and comments from these platforms. Automation tools for these platforms will be available soon.',
    '我们现已支持 Instagram、Facebook、Reddit 和 Twitter！您可以分析这些平台的视频、帖子和评论。这些平台的自动化工具将陆续上线。',
    true,
    NOW()
);
